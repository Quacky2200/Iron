// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The `.ppk` layout, version 1. Everything is little-endian; offsets are `u64` from the start of
//! the pack (a pack may sit inside a larger file, as in an Android APK).
//!
//! ```text
//! +--------------------+ 0
//! | Header (64 bytes)  |
//! +--------------------+ 64
//! | entry data         |  each entry's chunks back to back; an entry starts aligned to
//! |   (chunks)         |  1 << align_log2 when it is at least that big, else to 16
//! +--------------------+ toc_offset
//! | TOC                |  TocHeader (16) | Entry x E (64 each, sorted by GUID)
//! |                    |  | PathSlot x E (16 each, sorted by path hash, then variant, then path)
//! |                    |  | Chunk x C (24 each) | strings (UTF-8 paths and meta text)
//! +--------------------+ file_size
//! ```
//!
//! Header: `magic "IRPK"`, `version u16`, `flags u16`, `header_size u32 (64)`, `align_log2 u8`,
//! `chunk_log2 u8`, `reserved u16`, `toc_offset u64`, `toc_size u64`, `file_size u64`,
//! `pack_id [u8; 16]` (content hash of the TOC), `toc_crc u32`, `header_crc u32` (bytes 0..60).
//!
//! Entry: `guid [16]`, `content_hash [16]` (of the raw bytes), `size u64`, `first_chunk u32`,
//! `chunk_count u32`, `path_offset u32`, `path_len u16`, `kind u8`, `flags u8`, `meta_offset u32`,
//! `meta_len u32`.
//!
//! Chunk: `offset u64`, `stored_size u32`, `raw_size u32`, `codec u8`, `reserved [3]`, `crc u32`
//! (of the stored bytes). Every chunk of an entry but the last holds `1 << chunk_log2` raw bytes,
//! so the chunk holding byte `n` of an entry is `first_chunk + (n >> chunk_log2)`.

use crate::{Error, Result};

/// Kept from before the rename to Platypus (`.irpk` became `.ppk`), so older packs still open.
pub const MAGIC: [u8; 4] = *b"IRPK";
pub const VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 64;
pub const TOC_HEADER_SIZE: usize = 16;
pub const ENTRY_SIZE: usize = 64;
pub const PATH_SLOT_SIZE: usize = 16;
pub const CHUNK_SIZE: usize = 24;
/// Small entries are aligned to this; large ones to `1 << align_log2`.
pub const SMALL_ALIGN: u64 = 16;

/// Entry flag: the entry hides the same GUID and path in packs mounted before this one (a patch
/// that deletes an asset).
pub const FLAG_TOMBSTONE: u8 = 1;
/// Entry flag: every chunk is stored uncompressed and back to back, so the whole entry is one
/// byte range (read without copying from a memory map).
pub const FLAG_CONTIGUOUS: u8 = 2;
/// Entry flag: the payload is the cooked form (`IRTX` texture, `IRMD` model); without it the
/// payload is the source file's bytes. A path may have one entry of each.
pub const FLAG_COOKED: u8 = 4;

/// How a chunk is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Codec {
    None = 0,
    /// LZ4 block format ([`crate::lz4`]).
    Lz4 = 1,
    /// Raw deflate (RFC 1951) through `miniz_oxide`: smaller, slower to read.
    Deflate = 2,
    /// Zstandard (the `zstd` crate): each chunk is its own frame, so ranges still read per chunk.
    Zstd = 3,
}

impl Codec {
    pub fn from_u8(v: u8) -> Option<Codec> {
        Some(match v {
            0 => Codec::None,
            1 => Codec::Lz4,
            2 => Codec::Deflate,
            3 => Codec::Zstd,
            _ => return None,
        })
    }
}

/// What an entry holds, so loaders can pick a decoder without sniffing bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AssetKind {
    Data = 0,
    Texture = 1,
    Model = 2,
    Audio = 3,
    Font = 4,
    Scene = 5,
    Prefab = 6,
    Svg = 7,
    Css = 8,
    Material = 9,
    Animation = 10,
}

impl AssetKind {
    pub fn from_u8(v: u8) -> AssetKind {
        match v {
            1 => AssetKind::Texture,
            2 => AssetKind::Model,
            3 => AssetKind::Audio,
            4 => AssetKind::Font,
            5 => AssetKind::Scene,
            6 => AssetKind::Prefab,
            7 => AssetKind::Svg,
            8 => AssetKind::Css,
            9 => AssetKind::Material,
            10 => AssetKind::Animation,
            _ => AssetKind::Data,
        }
    }

    /// The kind of a game file by its name (the build's default; `.meta` importers may say more).
    pub fn of_path(path: &str) -> AssetKind {
        let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
        if name.ends_with(".scene.json") {
            return AssetKind::Scene;
        }
        if name.ends_with(".prefab.json") {
            return AssetKind::Prefab;
        }
        match name.rsplit_once('.').map_or("", |(_, e)| e) {
            "png" | "jpg" | "jpeg" | "hdr" => AssetKind::Texture,
            "glb" | "gltf" => AssetKind::Model,
            "wav" | "ogg" | "mp3" | "flac" | "opus" => AssetKind::Audio,
            "ttf" | "otf" | "woff" | "woff2" => AssetKind::Font,
            "svg" => AssetKind::Svg,
            "css" => AssetKind::Css,
            _ => AssetKind::Data,
        }
    }
}

/// A 128-bit asset id: the `guid` of an asset's `.meta` file (32 hex digits), or one derived from
/// the path for files without one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Guid(pub [u8; 16]);

impl Guid {
    /// Parses 32 hex digits (dashes, as in a UUID, are ignored).
    pub fn parse(text: &str) -> Option<Guid> {
        let digits: Vec<u8> = text.bytes().filter(|&b| b != b'-').collect();
        if digits.len() != 32 {
            return None;
        }
        let mut out = [0u8; 16];
        for (i, pair) in digits.chunks(2).enumerate() {
            let hex = std::str::from_utf8(pair).ok()?;
            out[i] = u8::from_str_radix(hex, 16).ok()?;
        }
        Some(Guid(out))
    }

    /// The id of a file without a `.meta`: a hash of its normalized path, so it is stable across
    /// builds.
    pub fn from_path(path: &str) -> Guid {
        let mut key = b"platypus.path:".to_vec();
        key.extend_from_slice(path.as_bytes());
        Guid(crate::hash::content_hash(&key))
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

pub(crate) fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(crate) fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

pub(crate) fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

pub(crate) fn bytes16(b: &[u8], at: usize) -> [u8; 16] {
    b[at..at + 16].try_into().expect("16 bytes")
}

/// The fixed-size header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u16,
    pub flags: u16,
    pub align_log2: u8,
    pub chunk_log2: u8,
    pub toc_offset: u64,
    pub toc_size: u64,
    pub file_size: u64,
    pub pack_id: [u8; 16],
    pub toc_crc: u32,
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut b = [0u8; HEADER_SIZE];
        b[0..4].copy_from_slice(&MAGIC);
        b[4..6].copy_from_slice(&self.version.to_le_bytes());
        b[6..8].copy_from_slice(&self.flags.to_le_bytes());
        b[8..12].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        b[12] = self.align_log2;
        b[13] = self.chunk_log2;
        b[16..24].copy_from_slice(&self.toc_offset.to_le_bytes());
        b[24..32].copy_from_slice(&self.toc_size.to_le_bytes());
        b[32..40].copy_from_slice(&self.file_size.to_le_bytes());
        b[40..56].copy_from_slice(&self.pack_id);
        b[56..60].copy_from_slice(&self.toc_crc.to_le_bytes());
        let crc = crate::hash::crc32(&b[0..60]);
        b[60..64].copy_from_slice(&crc.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8]) -> Result<Header> {
        if b.len() < HEADER_SIZE || b[0..4] != MAGIC {
            return Err(Error::Format("no IRPK header".into()));
        }
        if crate::hash::crc32(&b[0..60]) != u32_at(b, 60) {
            return Err(Error::Corrupt("header checksum".into()));
        }
        let version = u16_at(b, 4);
        if version != VERSION {
            return Err(Error::Format(format!(
                "version {version} (this reader reads {VERSION})"
            )));
        }
        let h = Header {
            version,
            flags: u16_at(b, 6),
            align_log2: b[12],
            chunk_log2: b[13],
            toc_offset: u64_at(b, 16),
            toc_size: u64_at(b, 24),
            file_size: u64_at(b, 32),
            pack_id: bytes16(b, 40),
            toc_crc: u32_at(b, 56),
        };
        if u32_at(b, 8) as usize != HEADER_SIZE || !(10..=30).contains(&h.chunk_log2) {
            return Err(Error::Format("header fields out of range".into()));
        }
        Ok(h)
    }
}

/// One TOC entry as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEntry {
    pub guid: Guid,
    pub content_hash: [u8; 16],
    pub size: u64,
    pub first_chunk: u32,
    pub chunk_count: u32,
    pub path_offset: u32,
    pub path_len: u16,
    pub kind: u8,
    pub flags: u8,
    pub meta_offset: u32,
    pub meta_len: u32,
}

impl RawEntry {
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.guid.0);
        out.extend_from_slice(&self.content_hash);
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.first_chunk.to_le_bytes());
        out.extend_from_slice(&self.chunk_count.to_le_bytes());
        out.extend_from_slice(&self.path_offset.to_le_bytes());
        out.extend_from_slice(&self.path_len.to_le_bytes());
        out.push(self.kind);
        out.push(self.flags);
        out.extend_from_slice(&self.meta_offset.to_le_bytes());
        out.extend_from_slice(&self.meta_len.to_le_bytes());
    }

    pub fn decode(b: &[u8]) -> RawEntry {
        RawEntry {
            guid: Guid(bytes16(b, 0)),
            content_hash: bytes16(b, 16),
            size: u64_at(b, 32),
            first_chunk: u32_at(b, 40),
            chunk_count: u32_at(b, 44),
            path_offset: u32_at(b, 48),
            path_len: u16_at(b, 52),
            kind: b[54],
            flags: b[55],
            meta_offset: u32_at(b, 56),
            meta_len: u32_at(b, 60),
        }
    }
}

/// One chunk as stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawChunk {
    pub offset: u64,
    pub stored_size: u32,
    pub raw_size: u32,
    pub codec: u8,
    pub crc: u32,
}

impl RawChunk {
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.stored_size.to_le_bytes());
        out.extend_from_slice(&self.raw_size.to_le_bytes());
        out.extend_from_slice(&[self.codec, 0, 0, 0]);
        out.extend_from_slice(&self.crc.to_le_bytes());
    }

    pub fn decode(b: &[u8]) -> RawChunk {
        RawChunk {
            offset: u64_at(b, 0),
            stored_size: u32_at(b, 8),
            raw_size: u32_at(b, 12),
            codec: b[16],
            crc: u32_at(b, 20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-API-1102
    #[test]
    fn header_entries_and_chunks_have_their_documented_sizes_and_round_trip() {
        let h = Header {
            version: VERSION,
            flags: 0,
            align_log2: 12,
            chunk_log2: 18,
            toc_offset: 5 << 32,
            toc_size: 1234,
            file_size: (5 << 32) + 1234,
            pack_id: [7; 16],
            toc_crc: 0xdead_beef,
        };
        let b = h.encode();
        assert_eq!(&b[0..4], b"IRPK");
        assert_eq!(Header::decode(&b).unwrap(), h);
        let mut damaged = b;
        damaged[20] ^= 1;
        assert!(matches!(Header::decode(&damaged), Err(Error::Corrupt(_))));
        let e = RawEntry {
            guid: Guid([1; 16]),
            content_hash: [2; 16],
            size: 6 << 30,
            first_chunk: 3,
            chunk_count: 4,
            path_offset: 5,
            path_len: 6,
            kind: AssetKind::Model as u8,
            flags: FLAG_COOKED,
            meta_offset: 8,
            meta_len: 9,
        };
        let mut out = Vec::new();
        e.encode(&mut out);
        assert_eq!(out.len(), ENTRY_SIZE);
        assert_eq!(RawEntry::decode(&out), e);
        let c = RawChunk {
            offset: 1 << 40,
            stored_size: 10,
            raw_size: 20,
            codec: Codec::Lz4 as u8,
            crc: 30,
        };
        let mut out = Vec::new();
        c.encode(&mut out);
        assert_eq!(out.len(), CHUNK_SIZE);
        assert_eq!(RawChunk::decode(&out), c);
        let g = Guid::parse("0123456789abcdef0123456789ABCDEF").unwrap();
        assert_eq!(g.to_hex(), "0123456789abcdef0123456789abcdef");
        assert_eq!(Guid::parse("01234567-89ab-cdef-0123-456789abcdef"), Some(g));
        assert_eq!(Guid::parse("xyz"), None);
        assert_eq!(AssetKind::of_path("scenes/a.scene.json"), AssetKind::Scene);
        assert_eq!(AssetKind::of_path("assets/A.PNG"), AssetKind::Texture);
    }
}
