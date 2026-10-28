// SPDX-License-Identifier: Apache-2.0

//! The payload of a single-file game: an archive appended to the engine host's executable
//! (`iron-build package` writes it, `iron_app` reads it; see `docs/iron-build.md`).
//!
//! ```text
//! [host exe bytes][entry data...][table of contents][trailer]
//! trailer (32 bytes, little endian): "IRONPAYL" | version u32 | reserved u32 | payload_len u64 | toc_len u64
//! toc: count u32, then per entry: path_len u16, path (UTF-8), offset u64 (from the payload's
//!      start), stored_len u64, raw_len u64, method u8 (0 stored, 1 deflate), crc32 u32
//! ```
//!
//! The trailer is the last thing in the file, so a reader needs only the end of the file:
//! an executable without a payload has no `IRONPAYL` there. Signing (Authenticode) appends
//! a certificate and hides the trailer: sign last and keep the payload before it (see
//! `docs/research/iron-code-signing.md`).

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};

const MAGIC: &[u8; 8] = b"IRONPAYL";
/// The format version this code reads and writes.
pub const VERSION: u32 = 1;
const TRAILER_LEN: u64 = 32;
/// Entries smaller than this are stored as they are (deflate would not help).
const MIN_COMPRESS: usize = 64;
/// The most entries and the largest table of contents accepted when reading.
const MAX_ENTRIES: u32 = 1_000_000;
const MAX_TOC: u64 = 256 << 20;

/// One file of the archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// `/`-separated, relative, checked by [`safe_path`].
    pub path: String,
    offset: u64,
    stored_len: u64,
    pub raw_len: u64,
    method: u8,
    crc: u32,
}

/// Whether `path` may be an archive entry: relative, `/` separated, no empty, `.` or `..`
/// parts, no drive letters, backslashes or NUL (the rules of `resolve_asset`).
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= u16::MAX as usize
        && !path.contains(['\\', '\0', ':'])
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn err<T>(msg: impl Into<String>) -> Result<T, String> {
    Err(msg.into())
}

/// Writes the archive part (entries, table of contents, trailer) of a payload.
/// Returns the number of bytes written.
pub fn write_payload<W: Write>(out: &mut W, files: &[(String, Vec<u8>)]) -> Result<u64, String> {
    let io = |e: std::io::Error| e.to_string();
    let mut toc = Vec::new();
    toc.extend((files.len() as u32).to_le_bytes());
    let mut pos = 0u64;
    let mut seen = std::collections::HashSet::new();
    for (path, data) in files {
        if !safe_path(path) {
            return err(format!("unsafe payload path {path:?}"));
        }
        if !seen.insert(path) {
            return err(format!("duplicate payload path {path:?}"));
        }
        let (method, stored) = compress(data);
        out.write_all(&stored).map_err(io)?;
        toc.extend((path.len() as u16).to_le_bytes());
        toc.extend(path.as_bytes());
        toc.extend(pos.to_le_bytes());
        toc.extend((stored.len() as u64).to_le_bytes());
        toc.extend((data.len() as u64).to_le_bytes());
        toc.push(method);
        toc.extend(crc32fast::hash(data).to_le_bytes());
        pos += stored.len() as u64;
    }
    out.write_all(&toc).map_err(io)?;
    let payload_len = pos + toc.len() as u64 + TRAILER_LEN;
    out.write_all(MAGIC).map_err(io)?;
    out.write_all(&VERSION.to_le_bytes()).map_err(io)?;
    out.write_all(&0u32.to_le_bytes()).map_err(io)?;
    out.write_all(&payload_len.to_le_bytes()).map_err(io)?;
    out.write_all(&(toc.len() as u64).to_le_bytes())
        .map_err(io)?;
    Ok(payload_len)
}

fn compress(data: &[u8]) -> (u8, Vec<u8>) {
    if data.len() >= MIN_COMPRESS {
        let mut enc =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        if enc.write_all(data).is_ok()
            && let Ok(out) = enc.finish()
            && out.len() < data.len()
        {
            return (1, out);
        }
    }
    (0, data.to_vec())
}

/// A payload found at the end of a file or buffer. Reads entries on demand.
pub struct Archive<R> {
    reader: R,
    /// Where the payload starts in the file (the host's length).
    pub start: u64,
    pub entries: Vec<Entry>,
    toc_hash: u64,
}

impl Archive<std::fs::File> {
    /// Opens the payload at the end of the file `path`: `Ok(None)` when it has none.
    pub fn open(path: &Path) -> Result<Option<Self>, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::new(file)
    }
}

impl<R: Read + Seek> Archive<R> {
    /// Reads the trailer and table of contents; `Ok(None)` when there is no trailer, an
    /// error when there is one that is damaged or from another format version. The trailer
    /// is searched at the end of the file, and for a signed executable just before its
    /// Authenticode certificate table (which signing appends after the payload).
    pub fn new(mut reader: R) -> Result<Option<Self>, String> {
        let io = |e: std::io::Error| e.to_string();
        let len = reader.seek(SeekFrom::End(0)).map_err(io)?;
        let mut ends = vec![len];
        if let Some(cert) = pe_certificate_offset(&mut reader, len) {
            // The certificate table starts on an 8-byte boundary: up to 7 padding bytes.
            ends.extend((0..8).filter_map(|k| cert.checked_sub(k)));
        }
        for end in ends {
            if let Some((start, entries, toc_hash)) = Self::probe(&mut reader, end)? {
                return Ok(Some(Self {
                    reader,
                    start,
                    entries,
                    toc_hash,
                }));
            }
        }
        Ok(None)
    }

    /// The payload whose trailer ends at `end`: where it starts, its entries and TOC hash.
    #[allow(clippy::type_complexity)]
    fn probe(reader: &mut R, end: u64) -> Result<Option<(u64, Vec<Entry>, u64)>, String> {
        let io = |e: std::io::Error| e.to_string();
        if end < TRAILER_LEN {
            return Ok(None);
        }
        reader
            .seek(SeekFrom::Start(end - TRAILER_LEN))
            .map_err(io)?;
        let mut t = [0u8; TRAILER_LEN as usize];
        reader.read_exact(&mut t).map_err(io)?;
        if &t[..8] != MAGIC {
            return Ok(None);
        }
        let version = u32::from_le_bytes(t[8..12].try_into().unwrap());
        if version != VERSION {
            return err(format!(
                "payload format {version} (this program reads {VERSION})"
            ));
        }
        let payload_len = u64::from_le_bytes(t[16..24].try_into().unwrap());
        let toc_len = u64::from_le_bytes(t[24..32].try_into().unwrap());
        if payload_len > end || toc_len > MAX_TOC || toc_len + TRAILER_LEN > payload_len {
            return err("payload trailer is damaged");
        }
        reader
            .seek(SeekFrom::Start(end - TRAILER_LEN - toc_len))
            .map_err(io)?;
        let mut toc = vec![0u8; toc_len as usize];
        reader.read_exact(&mut toc).map_err(io)?;
        let entries = parse_toc(&toc, payload_len - TRAILER_LEN - toc_len)?;
        let mut hash = 0xcbf29ce484222325u64;
        for b in toc.iter().chain(payload_len.to_le_bytes().iter()) {
            hash = (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
        Ok(Some((end - payload_len, entries, hash)))
    }

    /// A hash of the table of contents (which holds every file's CRC), to name a cache
    /// folder: the same payload gives the same value.
    pub fn id(&self) -> u64 {
        self.toc_hash
    }

    pub fn find(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// The bytes of one entry, checked against its CRC.
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let e = self
            .find(path)
            .cloned()
            .ok_or_else(|| format!("{path:?} is not in the payload"))?;
        self.read_entry(&e)
    }

    pub fn read_entry(&mut self, e: &Entry) -> Result<Vec<u8>, String> {
        let io = |err: std::io::Error| format!("{}: {err}", e.path);
        self.reader
            .seek(SeekFrom::Start(self.start + e.offset))
            .map_err(io)?;
        let mut stored = vec![0u8; e.stored_len as usize];
        self.reader.read_exact(&mut stored).map_err(io)?;
        let raw = match e.method {
            0 => stored,
            _ => {
                let mut out = Vec::with_capacity(e.raw_len as usize);
                flate2::read::DeflateDecoder::new(&stored[..])
                    .take(e.raw_len + 1)
                    .read_to_end(&mut out)
                    .map_err(io)?;
                out
            }
        };
        if raw.len() as u64 != e.raw_len || crc32fast::hash(&raw) != e.crc {
            return err(format!("{}: payload data is damaged", e.path));
        }
        Ok(raw)
    }

    /// Writes every entry under `dest`, creating folders.
    pub fn extract(&mut self, dest: &Path) -> Result<(), String> {
        for e in self.entries.clone() {
            let data = self.read_entry(&e)?;
            let file = dest.join(&e.path);
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            std::fs::write(&file, data).map_err(|e| format!("{}: {e}", file.display()))?;
        }
        Ok(())
    }
}

/// The length of the host part of an executable that has a payload (where the payload
/// starts), or of the whole file when it has none: for stripping an old payload.
pub fn host_len(path: &Path) -> Result<u64, String> {
    Ok(match Archive::open(path)? {
        Some(a) => a.start,
        None => std::fs::metadata(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len(),
    })
}

/// The file offset of a PE file's Authenticode certificate table (data directory 4), if it
/// has one.
fn pe_certificate_offset<R: Read + Seek>(reader: &mut R, len: u64) -> Option<u64> {
    let mut at = |pos: u64, n: usize| -> Option<Vec<u8>> {
        reader.seek(SeekFrom::Start(pos)).ok()?;
        let mut b = vec![0u8; n];
        reader.read_exact(&mut b).ok()?;
        Some(b)
    };
    let dos = at(0, 64)?;
    if &dos[..2] != b"MZ" {
        return None;
    }
    let pe = u64::from(u32::from_le_bytes(dos[0x3c..0x40].try_into().ok()?));
    let head = at(pe, 26)?;
    if &head[..4] != b"PE\0\0" {
        return None;
    }
    let dirs = pe
        + 24
        + match u16::from_le_bytes([head[24], head[25]]) {
            0x10b => 96,
            0x20b => 112,
            _ => return None,
        };
    let entry = at(dirs + 4 * 8, 8)?;
    let off = u64::from(u32::from_le_bytes(entry[..4].try_into().ok()?));
    (off != 0 && off <= len).then_some(off)
}

fn parse_toc(toc: &[u8], data_len: u64) -> Result<Vec<Entry>, String> {
    let bad = || "payload table of contents is damaged".to_string();
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], String> {
        let s = toc.get(at..at + n).ok_or_else(bad)?;
        at += n;
        Ok(s)
    };
    let count = u32::from_le_bytes(take(4)?.try_into().unwrap());
    if count > MAX_ENTRIES {
        return Err(bad());
    }
    let mut out = Vec::new();
    for _ in 0..count {
        let n = u16::from_le_bytes(take(2)?.try_into().unwrap()) as usize;
        let path = std::str::from_utf8(take(n)?)
            .map_err(|_| bad())?
            .to_string();
        if !safe_path(&path) {
            return err(format!("unsafe payload path {path:?}"));
        }
        let offset = u64::from_le_bytes(take(8)?.try_into().unwrap());
        let stored_len = u64::from_le_bytes(take(8)?.try_into().unwrap());
        let raw_len = u64::from_le_bytes(take(8)?.try_into().unwrap());
        let method = take(1)?[0];
        let crc = u32::from_le_bytes(take(4)?.try_into().unwrap());
        if method > 1 || offset.checked_add(stored_len).is_none_or(|e| e > data_len) {
            return Err(bad());
        }
        out.push(Entry {
            path,
            offset,
            stored_len,
            raw_len,
            method,
            crc,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample() -> Vec<(String, Vec<u8>)> {
        vec![
            ("build/game.irbundle".into(), b"{\"format\":1}".repeat(50)),
            ("assets/tiny.bin".into(), vec![1, 2, 3]),
            ("assets/ünï.txt".into(), vec![7; 1000]),
        ]
    }

    // M9-API-1000
    #[test]
    fn payload_round_trips_after_a_host() {
        let mut file = b"MZ fake host".to_vec();
        let n = write_payload(&mut file, &sample()).unwrap();
        assert_eq!(file.len() as u64, 12 + n);
        let mut a = Archive::new(Cursor::new(file)).unwrap().unwrap();
        assert_eq!(a.start, 12);
        assert_eq!(a.entries.len(), 3);
        for (p, d) in sample() {
            assert_eq!(a.read(&p).unwrap(), d);
        }
        // Compressible data is stored deflated, tiny data as it is.
        assert_eq!(a.find("assets/tiny.bin").unwrap().method, 0);
        assert_eq!(a.find("assets/ünï.txt").unwrap().method, 1);
    }

    // M9-API-1001
    #[test]
    fn a_file_without_a_trailer_has_no_payload() {
        assert!(Archive::new(Cursor::new(vec![0u8; 500])).unwrap().is_none());
        assert!(Archive::new(Cursor::new(Vec::new())).unwrap().is_none());
    }

    // M9-API-1002
    #[test]
    fn unsafe_paths_are_refused() {
        for p in [
            "../x", "a/../b", "/abs", "C:/x", "a\\b", "", "a//b", "./a", "a/.",
        ] {
            assert!(!safe_path(p), "{p}");
            assert!(write_payload(&mut Vec::new(), &[(p.into(), vec![1])]).is_err());
        }
        assert!(safe_path("assets/a b.png"));
        // A hand-made archive with an unsafe path is refused when read.
        let mut file = Vec::new();
        write_payload(&mut file, &[("ok/a".into(), vec![1])]).unwrap();
        let at = file.windows(4).position(|w| w == b"ok/a").unwrap();
        file[at..at + 4].copy_from_slice(b"../a");
        assert!(Archive::new(Cursor::new(file)).is_err());
    }

    // M9-API-1003
    #[test]
    fn damage_and_duplicates_are_found() {
        let dup = [("a".to_string(), vec![]), ("a".to_string(), vec![])];
        assert!(write_payload(&mut Vec::new(), &dup).is_err());
        let mut file = Vec::new();
        write_payload(&mut file, &sample()).unwrap();
        let mut bad = file.clone();
        bad[3] ^= 0xff; // inside the first entry
        let mut a = Archive::new(Cursor::new(bad)).unwrap().unwrap();
        assert!(a.read("build/game.irbundle").is_err());
        let mut newer = file.clone();
        let n = newer.len();
        newer[n - 24] = 9; // version
        assert!(Archive::new(Cursor::new(newer)).is_err());
    }

    // M9-API-1005
    #[test]
    fn a_payload_before_a_certificate_table_is_found() {
        // A minimal PE32+ header whose certificate directory points at the file's tail.
        let mut file = vec![0u8; 0x200];
        file[..2].copy_from_slice(b"MZ");
        file[0x3c] = 0x80;
        file[0x80..0x84].copy_from_slice(b"PE\0\0");
        file[0x80 + 24..0x80 + 26].copy_from_slice(&0x20bu16.to_le_bytes());
        write_payload(&mut file, &sample()).unwrap();
        while !file.len().is_multiple_of(8) {
            file.push(0); // signtool pads to 8 before the certificate
        }
        let cert_at = file.len() as u32;
        file.extend(vec![0xAB; 300]); // the certificate
        let dir = 0x80 + 24 + 112 + 4 * 8;
        file[dir..dir + 4].copy_from_slice(&cert_at.to_le_bytes());
        file[dir + 4..dir + 8].copy_from_slice(&300u32.to_le_bytes());
        let mut a = Archive::new(Cursor::new(file)).unwrap().unwrap();
        assert_eq!(a.start, 0x200);
        assert_eq!(a.read("assets/tiny.bin").unwrap(), [1, 2, 3]);
    }

    // M9-API-1004
    #[test]
    fn extract_writes_the_files_and_id_follows_the_content() {
        let mut file = Vec::new();
        write_payload(&mut file, &sample()).unwrap();
        let mut a = Archive::new(Cursor::new(file.clone())).unwrap().unwrap();
        let dir = std::env::temp_dir().join(format!("iron_payload_test_{}", std::process::id()));
        a.extract(&dir).unwrap();
        assert_eq!(
            std::fs::read(dir.join("assets/tiny.bin")).unwrap(),
            [1, 2, 3]
        );
        let _ = std::fs::remove_dir_all(&dir);
        let mut other = sample();
        other[1].1 = vec![9];
        let mut f2 = Vec::new();
        write_payload(&mut f2, &other).unwrap();
        let b = Archive::new(Cursor::new(f2)).unwrap().unwrap();
        assert_ne!(a.id(), b.id());
        let a2 = Archive::new(Cursor::new(file)).unwrap().unwrap();
        assert_eq!(a.id(), a2.id());
    }
}
