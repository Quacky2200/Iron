// SPDX-License-Identifier: Apache-2.0

//! Reading a pack: the header and table of contents are checked once at open; entries are found
//! by binary search (GUID, or path hash then path) in the TOC as stored, without building maps.
//! Data is read chunk by chunk, so a 4 KiB read of a 2 GiB entry touches one chunk.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::format::{
    AssetKind, CHUNK_SIZE, Codec, ENTRY_SIZE, FLAG_CONTIGUOUS, FLAG_COOKED, FLAG_TOMBSTONE, Guid,
    HEADER_SIZE, Header, PATH_SLOT_SIZE, RawChunk, RawEntry, TOC_HEADER_SIZE, u32_at, u64_at,
};
use crate::hash::{content_hash, crc32, normalize_path, path_hash};
use crate::{Error, Result};

/// How [`Pack::open_with`] reads the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenMode {
    /// Memory-mapped: reads of uncompressed entries are slices of the map (no copy).
    Map,
    /// Positional reads through the file handle (the TOC is read into memory).
    Read,
}

enum Backing {
    Map(memmap2::Mmap),
    Owned(Vec<u8>),
    /// The file and where the pack starts in it.
    File(File, u64),
}

struct Inner {
    backing: Backing,
    /// The TOC, for `Backing::File` (the others slice it from the backing).
    toc_copy: Vec<u8>,
    header: Header,
    entry_count: usize,
    chunk_count: usize,
    origin: PathBuf,
    verify: AtomicBool,
}

impl Inner {
    /// The pack's bytes when they are all in memory (mapped or owned).
    fn all(&self) -> Option<&[u8]> {
        match &self.backing {
            Backing::Map(m) => Some(m),
            Backing::Owned(v) => Some(v),
            Backing::File(..) => None,
        }
    }

    fn toc(&self) -> &[u8] {
        match self.all() {
            Some(all) => {
                let start = self.header.toc_offset as usize;
                &all[start..start + self.header.toc_size as usize]
            }
            None => &self.toc_copy,
        }
    }
}

/// Bytes read from a pack: a slice of its memory map (kept alive by this value) or a copy.
/// Cheap to clone; usable as `AsRef<[u8]>` (for example `std::io::Cursor<Bytes>` for streaming
/// audio straight from the map).
#[derive(Clone)]
pub struct Bytes(Repr);

#[derive(Clone)]
enum Repr {
    Shared(Arc<Inner>, usize, usize),
    Owned(Arc<Vec<u8>>),
}

impl Bytes {
    /// True when the bytes are a view into a pack, not a copy.
    pub fn is_shared(&self) -> bool {
        matches!(self.0, Repr::Shared(..))
    }

    /// The bytes as a `Vec` (copied unless this value owned them alone).
    pub fn into_vec(self) -> Vec<u8> {
        match self.0 {
            Repr::Owned(v) => Arc::try_unwrap(v).unwrap_or_else(|v| v.as_ref().clone()),
            shared @ Repr::Shared(..) => Bytes(shared).as_ref().to_vec(),
        }
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(v: Vec<u8>) -> Self {
        Bytes(Repr::Owned(Arc::new(v)))
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Repr::Shared(inner, start, end) => {
                &inner.all().expect("shared bytes are mapped")[*start..*end]
            }
            Repr::Owned(v) => v,
        }
    }
}

impl std::ops::Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bytes({} bytes{})",
            self.len(),
            if self.is_shared() { ", shared" } else { "" }
        )
    }
}

/// One entry of a pack's TOC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub index: u32,
    pub guid: Guid,
    pub content_hash: [u8; 16],
    pub size: u64,
    pub kind: AssetKind,
    pub flags: u8,
    pub path: String,
    /// The `.meta` importer settings (JSON) the build kept, or empty.
    pub meta: String,
    first_chunk: u32,
    chunk_count: u32,
}

impl Entry {
    pub fn is_tombstone(&self) -> bool {
        self.flags & FLAG_TOMBSTONE != 0
    }
    pub fn is_cooked(&self) -> bool {
        self.flags & FLAG_COOKED != 0
    }
    pub fn is_contiguous(&self) -> bool {
        self.flags & FLAG_CONTIGUOUS != 0
    }
    pub fn chunk_count(&self) -> u32 {
        self.chunk_count
    }
}

/// An open pack. Cheap to clone (shared); `Send + Sync`, so loader threads can read it.
#[derive(Clone)]
pub struct Pack {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Pack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pack({}, {} entries)",
            self.inner.origin.display(),
            self.len()
        )
    }
}

impl Pack {
    /// Opens a pack file, memory-mapped.
    pub fn open(path: &Path) -> Result<Pack> {
        Pack::open_with(path, OpenMode::Map)
    }

    pub fn open_with(path: &Path, mode: OpenMode) -> Result<Pack> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Pack::open_at(file, 0, len, mode, path)
    }

    /// Opens a pack stored at `offset..offset + len` of `file` (an APK's uncompressed asset, a
    /// pack appended to something else). `origin` names it in messages.
    pub fn open_at(
        file: File,
        offset: u64,
        len: u64,
        mode: OpenMode,
        origin: &Path,
    ) -> Result<Pack> {
        let backing = match mode {
            OpenMode::Map => {
                // SAFETY: packs are read-only release files; the engine never writes them while
                // mapped. Another program truncating one under us is outside what we defend.
                let map = unsafe {
                    memmap2::MmapOptions::new()
                        .offset(offset)
                        .len(
                            usize::try_from(len)
                                .map_err(|_| Error::Format("too big to map".into()))?,
                        )
                        .map(&file)?
                };
                Backing::Map(map)
            }
            OpenMode::Read => Backing::File(file, offset),
        };
        Pack::from_backing(backing, len, origin.to_path_buf())
    }

    /// A pack held in memory (tests, packs fetched over the network, platform asset APIs that
    /// only hand out bytes).
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Pack> {
        let len = bytes.len() as u64;
        Pack::from_backing(Backing::Owned(bytes), len, PathBuf::from("<memory>"))
    }

    fn from_backing(backing: Backing, len: u64, origin: PathBuf) -> Result<Pack> {
        let mut head = [0u8; HEADER_SIZE];
        match &backing {
            Backing::Map(m) => head.copy_from_slice(m.get(..HEADER_SIZE).ok_or_else(short)?),
            Backing::Owned(v) => head.copy_from_slice(v.get(..HEADER_SIZE).ok_or_else(short)?),
            Backing::File(f, base) => read_at(f, *base, &mut head)?,
        }
        let header = Header::decode(&head)?;
        if header.file_size > len
            || header.toc_offset < HEADER_SIZE as u64
            || header.toc_offset.checked_add(header.toc_size) != Some(header.file_size)
        {
            return Err(Error::Format(format!(
                "sizes do not fit (file {len} bytes, pack {} bytes): truncated?",
                header.file_size
            )));
        }
        let toc_copy = match &backing {
            Backing::File(f, base) => {
                let mut toc = vec![0u8; header.toc_size as usize];
                read_at(f, base + header.toc_offset, &mut toc)?;
                toc
            }
            _ => Vec::new(),
        };
        let mut inner = Inner {
            backing,
            toc_copy,
            header,
            entry_count: 0,
            chunk_count: 0,
            origin,
            verify: AtomicBool::new(true),
        };
        let toc = inner.toc();
        if crc32(toc) != inner.header.toc_crc {
            return Err(Error::Corrupt("table of contents checksum".into()));
        }
        if toc.len() < TOC_HEADER_SIZE {
            return Err(Error::Format("table of contents too short".into()));
        }
        let (entries, chunks, strings) = (
            u32_at(toc, 0) as usize,
            u32_at(toc, 4) as usize,
            u32_at(toc, 8) as usize,
        );
        let expected = TOC_HEADER_SIZE as u64
            + (ENTRY_SIZE + PATH_SLOT_SIZE) as u64 * entries as u64
            + CHUNK_SIZE as u64 * chunks as u64
            + strings as u64;
        if expected != toc.len() as u64 {
            return Err(Error::Format("table of contents sizes disagree".into()));
        }
        // Every entry's chunks and strings inside their tables, every chunk inside the data.
        let chunk_base = TOC_HEADER_SIZE + (ENTRY_SIZE + PATH_SLOT_SIZE) * entries;
        let data_end = inner.header.toc_offset;
        for c in 0..chunks {
            let at = chunk_base + c * CHUNK_SIZE;
            let ch = RawChunk::decode(&toc[at..at + CHUNK_SIZE]);
            if ch.offset < HEADER_SIZE as u64
                || ch.offset + u64::from(ch.stored_size) > data_end
                || Codec::from_u8(ch.codec).is_none()
            {
                return Err(Error::Format(format!("chunk {c} out of range")));
            }
        }
        for i in 0..entries {
            let at = TOC_HEADER_SIZE + i * ENTRY_SIZE;
            let e = RawEntry::decode(&toc[at..at + ENTRY_SIZE]);
            let path_end = e.path_offset as usize + usize::from(e.path_len);
            let meta_end = e.meta_offset as usize + e.meta_len as usize;
            if e.first_chunk as usize + e.chunk_count as usize > chunks
                || path_end > strings
                || meta_end > strings
            {
                return Err(Error::Format(format!("entry {i} out of range")));
            }
            let slot = TOC_HEADER_SIZE + ENTRY_SIZE * entries + i * PATH_SLOT_SIZE;
            if u32_at(toc, slot + 8) as usize >= entries {
                return Err(Error::Format(format!("path slot {i} out of range")));
            }
        }
        inner.entry_count = entries;
        inner.chunk_count = chunks;
        Ok(Pack {
            inner: Arc::new(inner),
        })
    }

    pub fn header(&self) -> &Header {
        &self.inner.header
    }

    /// The pack's id: the content hash of its TOC (so of everything in it).
    pub fn id(&self) -> [u8; 16] {
        self.inner.header.pack_id
    }

    pub fn origin(&self) -> &Path {
        &self.inner.origin
    }

    pub fn len(&self) -> usize {
        self.inner.entry_count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether reads check chunk checksums (on by default; compressed chunks are always
    /// checked).
    pub fn set_verify(&self, on: bool) {
        self.inner.verify.store(on, Ordering::Relaxed);
    }

    fn raw_entry(&self, i: usize) -> RawEntry {
        let at = TOC_HEADER_SIZE + i * ENTRY_SIZE;
        RawEntry::decode(&self.inner.toc()[at..at + ENTRY_SIZE])
    }

    fn chunk(&self, i: usize) -> RawChunk {
        let at = TOC_HEADER_SIZE
            + (ENTRY_SIZE + PATH_SLOT_SIZE) * self.inner.entry_count
            + i * CHUNK_SIZE;
        RawChunk::decode(&self.inner.toc()[at..at + CHUNK_SIZE])
    }

    fn strings(&self) -> &[u8] {
        let toc = self.inner.toc();
        let at = TOC_HEADER_SIZE
            + (ENTRY_SIZE + PATH_SLOT_SIZE) * self.inner.entry_count
            + CHUNK_SIZE * self.inner.chunk_count;
        &toc[at..]
    }

    fn path_of(&self, e: &RawEntry) -> &[u8] {
        let s = e.path_offset as usize;
        &self.strings()[s..s + usize::from(e.path_len)]
    }

    /// The entry at a TOC position (entries are sorted by GUID).
    pub fn entry(&self, i: usize) -> Entry {
        let e = self.raw_entry(i);
        let strings = self.strings();
        let meta_start = e.meta_offset as usize;
        let meta = &strings[meta_start..meta_start + e.meta_len as usize];
        Entry {
            index: i as u32,
            guid: e.guid,
            content_hash: e.content_hash,
            size: e.size,
            kind: AssetKind::from_u8(e.kind),
            flags: e.flags,
            path: String::from_utf8_lossy(self.path_of(&e)).into_owned(),
            meta: String::from_utf8_lossy(meta).into_owned(),
            first_chunk: e.first_chunk,
            chunk_count: e.chunk_count,
        }
    }

    /// How an entry's chunks are stored: (codec, stored bytes, raw bytes) per chunk (for
    /// inspection tools; a chunk of an unknown codec is reported as `None`).
    pub fn chunks_of(&self, e: &Entry) -> Vec<(Option<Codec>, u32, u32)> {
        (e.first_chunk..e.first_chunk + e.chunk_count)
            .map(|i| {
                let c = self.chunk(i as usize);
                (Codec::from_u8(c.codec), c.stored_size, c.raw_size)
            })
            .collect()
    }

    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        (0..self.len()).map(|i| self.entry(i))
    }

    pub fn find_guid(&self, guid: &Guid) -> Option<Entry> {
        let toc = self.inner.toc();
        let (mut lo, mut hi) = (0, self.inner.entry_count);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let at = TOC_HEADER_SIZE + mid * ENTRY_SIZE;
            match toc[at..at + 16].cmp(&guid.0[..]) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(self.entry(mid)),
            }
        }
        None
    }

    /// The entry for a game path: its source file, or with `cooked` its cooked form.
    pub fn find_path(&self, path: &str, cooked: bool) -> Option<Entry> {
        let path = normalize_path(path)?;
        let hash = path_hash(&path);
        let toc = self.inner.toc();
        let slots = TOC_HEADER_SIZE + ENTRY_SIZE * self.inner.entry_count;
        let slot_hash = |i: usize| u64_at(toc, slots + i * PATH_SLOT_SIZE);
        // First slot with this hash, then the few that share it.
        let (mut lo, mut hi) = (0, self.inner.entry_count);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if slot_hash(mid) < hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut i = lo;
        while i < self.inner.entry_count && slot_hash(i) == hash {
            let at = slots + i * PATH_SLOT_SIZE;
            let index = u32_at(toc, at + 8) as usize;
            let slot_cooked = u32_at(toc, at + 12) != 0;
            if slot_cooked == cooked && self.path_of(&self.raw_entry(index)) == path.as_bytes() {
                return Some(self.entry(index));
            }
            i += 1;
        }
        None
    }

    /// An entry's bytes. Uncompressed entries of a mapped pack are not copied.
    pub fn read(&self, e: &Entry) -> Result<Bytes> {
        self.read_range(e, 0, e.size)
    }

    /// `len` bytes of an entry from `offset` (clamped to its end). Only the chunks covering the
    /// range are read and decompressed.
    pub fn read_range(&self, e: &Entry, offset: u64, len: u64) -> Result<Bytes> {
        let end = offset.saturating_add(len).min(e.size);
        let offset = offset.min(end);
        if offset == end {
            return Ok(Bytes::from(Vec::new()));
        }
        let log2 = self.inner.header.chunk_log2;
        let first = (offset >> log2) as usize;
        let last = ((end - 1) >> log2) as usize;
        let verify = self.inner.verify.load(Ordering::Relaxed);
        let chunk_at = |k: usize| -> Result<RawChunk> {
            if k >= e.chunk_count as usize {
                return Err(Error::Corrupt(format!("{}: missing chunk", e.path)));
            }
            Ok(self.chunk(e.first_chunk as usize + k))
        };
        if e.is_contiguous()
            && let Some(all) = self.inner.all()
        {
            let start_chunk = chunk_at(first)?;
            if verify {
                for k in first..=last {
                    let c = chunk_at(k)?;
                    let at = c.offset as usize;
                    if crc32(&all[at..at + c.stored_size as usize]) != c.crc {
                        return Err(Error::Corrupt(format!("{}: chunk {k} checksum", e.path)));
                    }
                }
            }
            let start = start_chunk.offset as usize + (offset - ((first as u64) << log2)) as usize;
            return Ok(Bytes(Repr::Shared(
                self.inner.clone(),
                start,
                start + (end - offset) as usize,
            )));
        }
        let mut out = Vec::with_capacity((end - offset) as usize);
        let mut stored = Vec::new();
        for k in first..=last {
            let c = chunk_at(k)?;
            let raw_start = (k as u64) << log2;
            let bytes: &[u8] = match self.inner.all() {
                Some(all) => {
                    &all[c.offset as usize..(c.offset + u64::from(c.stored_size)) as usize]
                }
                None => {
                    let Backing::File(f, base) = &self.inner.backing else {
                        unreachable!("not in memory means a file");
                    };
                    stored.resize(c.stored_size as usize, 0);
                    read_at(f, base + c.offset, &mut stored)?;
                    &stored
                }
            };
            let codec = Codec::from_u8(c.codec).expect("checked at open");
            if (verify || codec != Codec::None) && crc32(bytes) != c.crc {
                return Err(Error::Corrupt(format!("{}: chunk {k} checksum", e.path)));
            }
            let lo = (offset.max(raw_start) - raw_start) as usize;
            let hi = (end.min(raw_start + u64::from(c.raw_size)) - raw_start) as usize;
            match codec {
                Codec::None => out.extend_from_slice(bytes.get(lo..hi).ok_or_else(short)?),
                Codec::Lz4 => {
                    let raw = crate::lz4::decompress(bytes, c.raw_size as usize)
                        .map_err(|m| Error::Corrupt(format!("{}: {m}", e.path)))?;
                    out.extend_from_slice(raw.get(lo..hi).ok_or_else(short)?);
                }
                Codec::Deflate => {
                    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(
                        bytes,
                        c.raw_size as usize,
                    )
                    .map_err(|m| Error::Corrupt(format!("{}: deflate {m:?}", e.path)))?;
                    if raw.len() != c.raw_size as usize {
                        return Err(Error::Corrupt(format!("{}: chunk {k} size", e.path)));
                    }
                    out.extend_from_slice(raw.get(lo..hi).ok_or_else(short)?);
                }
                Codec::Zstd => {
                    let raw = zstd::bulk::decompress(bytes, c.raw_size as usize)
                        .map_err(|m| Error::Corrupt(format!("{}: zstd {m}", e.path)))?;
                    if raw.len() != c.raw_size as usize {
                        return Err(Error::Corrupt(format!("{}: chunk {k} size", e.path)));
                    }
                    out.extend_from_slice(raw.get(lo..hi).ok_or_else(short)?);
                }
            }
        }
        Ok(Bytes::from(out))
    }

    /// Reads every entry and checks chunk checksums and content hashes (`iron-build pack --verify`,
    /// tests). Returns the number of entries checked.
    pub fn verify(&self) -> Result<usize> {
        let mut n = 0;
        for e in self.entries() {
            if e.is_tombstone() {
                continue;
            }
            let bytes = self.read(&e)?;
            if content_hash(&bytes) != e.content_hash {
                return Err(Error::Corrupt(format!("{}: content hash", e.path)));
            }
            n += 1;
        }
        Ok(n)
    }
}

fn short() -> Error {
    Error::Format("file too short".into())
}

#[cfg(unix)]
fn read_at(f: &File, at: u64, buf: &mut [u8]) -> Result<()> {
    use std::os::unix::fs::FileExt;
    f.read_exact_at(buf, at).map_err(Error::Io)
}

#[cfg(windows)]
fn read_at(f: &File, mut at: u64, mut buf: &mut [u8]) -> Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match f.seek_read(buf, at) {
            Ok(0) => return Err(short()),
            Ok(n) => {
                at += n as u64;
                buf = &mut buf[n..];
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(Error::Io(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::{EntrySpec, PackWriter, WriteOptions};
    use std::io::Cursor;

    fn noise(n: usize, seed: u32) -> Vec<u8> {
        let mut x = seed | 1;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x as u8
            })
            .collect()
    }

    fn build(opts: WriteOptions, specs: &[EntrySpec<'_>]) -> Vec<u8> {
        let mut w = PackWriter::new(Cursor::new(Vec::new()), opts).unwrap();
        for s in specs {
            w.add(s).unwrap();
        }
        w.finish().unwrap().0.into_inner()
    }

    fn small_chunks() -> WriteOptions {
        WriteOptions {
            chunk_log2: 10,
            ..WriteOptions::default()
        }
    }

    // M9-API-1103
    #[test]
    fn entries_round_trip_by_guid_and_path_in_every_open_mode() {
        let text = "hello pack ".repeat(500);
        let big = noise(5000, 7);
        let mut tex = EntrySpec::new("assets/a.png", text.as_bytes());
        tex.guid = Guid::parse("aa000000000000000000000000000001").unwrap();
        tex.meta = r#"{"type":"texture"}"#;
        let mut cooked = EntrySpec::new("assets/a.png", &big);
        cooked.guid = Guid::parse("aa000000000000000000000000000002").unwrap();
        cooked.cooked = true;
        let mut mp3 = EntrySpec::new("./sfx\\hit.mp3", &big[..3000]);
        mp3.compress = false;
        let empty = EntrySpec::new("empty.json", b"");
        let bytes = build(small_chunks(), &[tex, cooked, mp3, empty]);

        let dir = std::env::temp_dir().join(format!("iron_pack_rt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.irpk");
        std::fs::write(&file, &bytes).unwrap();
        for pack in [
            Pack::from_bytes(bytes.clone()).unwrap(),
            Pack::open(&file).unwrap(),
            Pack::open_with(&file, OpenMode::Read).unwrap(),
        ] {
            assert_eq!(pack.len(), 4);
            let e = pack.find_path("assets/a.png", false).unwrap();
            assert_eq!(e.kind, AssetKind::Texture);
            assert_eq!(e.meta, r#"{"type":"texture"}"#);
            assert!(!e.is_contiguous(), "text compresses");
            assert_eq!(&*pack.read(&e).unwrap(), text.as_bytes());
            let c = pack.find_path("./assets/a.png", true).unwrap();
            assert!(c.is_cooked() && c.is_contiguous(), "noise is stored raw");
            assert_eq!(c.chunk_count(), 5);
            assert_eq!(&*pack.read(&c).unwrap(), &big[..]);
            let by_guid = pack.find_guid(&c.guid).unwrap();
            assert_eq!(by_guid, c);
            let s = pack.find_path("sfx/hit.mp3", false).unwrap();
            assert_eq!(s.kind, AssetKind::Audio);
            assert_eq!(&*pack.read(&s).unwrap(), &big[..3000]);
            assert_eq!(
                pack.read(&pack.find_path("empty.json", false).unwrap())
                    .unwrap()
                    .len(),
                0
            );
            assert!(pack.find_path("assets/b.png", false).is_none());
            assert!(pack.find_path("sfx/hit.mp3", true).is_none());
            assert!(pack.find_guid(&Guid([9; 16])).is_none());
            assert_eq!(pack.verify().unwrap(), 4);
        }
        // Mapped raw entries are views, not copies.
        let pack = Pack::open(&file).unwrap();
        let c = pack.find_path("assets/a.png", true).unwrap();
        assert!(pack.read(&c).unwrap().is_shared());
        drop(pack);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M9-API-1104
    #[test]
    fn ranges_read_only_the_chunks_they_cover() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let raw = noise(10_000, 3);
        let mut a = EntrySpec::new("a.bin", &data);
        a.guid = Guid([1; 16]);
        let mut b = EntrySpec::new("b.bin", &raw);
        b.guid = Guid([2; 16]);
        b.compress = false;
        let pack = Pack::from_bytes(build(small_chunks(), &[a, b])).unwrap();
        for e in [
            pack.find_guid(&Guid([1; 16])).unwrap(),
            pack.find_guid(&Guid([2; 16])).unwrap(),
        ] {
            let src = if e.path == "a.bin" { &data } else { &raw };
            for (off, len) in [
                (0, 10),
                (1000, 2000),
                (1023, 2),
                (9990, 100),
                (20_000, 5),
                (0, 10_000),
            ] {
                let got = pack.read_range(&e, off, len).unwrap();
                let lo = (off as usize).min(src.len());
                let hi = (off as usize + len as usize).min(src.len());
                assert_eq!(&*got, &src[lo..hi], "{} {off} {len}", e.path);
            }
        }
    }

    // M9-API-1105
    #[test]
    fn identical_bytes_share_chunks_and_builds_are_reproducible() {
        let data = noise(50_000, 11);
        let make = || {
            let mut a = EntrySpec::new("one/coin.png", &data);
            a.cooked = true;
            let mut b = EntrySpec::new("two/coin.png", &data);
            b.cooked = true;
            let c = EntrySpec::new("three.txt", b"three");
            build(WriteOptions::default(), &[a, b, c])
        };
        let first = make();
        assert_eq!(first, make(), "same input, same bytes");
        let pack = Pack::from_bytes(first.clone()).unwrap();
        let a = pack.find_path("one/coin.png", true).unwrap();
        let b = pack.find_path("two/coin.png", true).unwrap();
        assert_eq!(a.content_hash, b.content_hash);
        assert!(
            first.len() < data.len() + 8192,
            "stored once: {}",
            first.len()
        );
        assert_eq!(&*pack.read(&b).unwrap(), &data[..]);
        // The id follows the content.
        let other = build(WriteOptions::default(), &[EntrySpec::new("x.txt", b"x")]);
        assert_ne!(pack.id(), Pack::from_bytes(other).unwrap().id());
    }

    // M9-API-1106
    #[test]
    fn damaged_or_truncated_packs_are_errors_not_panics() {
        let text = "abcdefgh".repeat(4000);
        let bytes = build(small_chunks(), &[EntrySpec::new("t.txt", text.as_bytes())]);
        assert!(matches!(
            Pack::from_bytes(bytes[..bytes.len() - 1].to_vec()),
            Err(Error::Format(_))
        ));
        assert!(Pack::from_bytes(b"IRPK".to_vec()).is_err());
        assert!(Pack::from_bytes(vec![0; 200]).is_err());
        // A flipped byte in the data: the chunk checksum catches it on read.
        let mut bad = bytes.clone();
        bad[4096 + 3] ^= 0x40;
        let pack = Pack::from_bytes(bad).unwrap();
        let e = pack.find_path("t.txt", false).unwrap();
        assert!(matches!(pack.read(&e), Err(Error::Corrupt(_))));
        // A flipped byte in the TOC fails the open.
        let mut bad = bytes.clone();
        let toc = Header::decode(&bytes).unwrap().toc_offset as usize;
        bad[toc + 20] ^= 1;
        assert!(matches!(Pack::from_bytes(bad), Err(Error::Corrupt(_))));
        // Every single-byte truncation and a sweep of flipped bytes: never a panic.
        for cut in (0..bytes.len()).step_by(97) {
            let _ = Pack::from_bytes(bytes[..cut].to_vec());
        }
        for at in (0..bytes.len()).step_by(13) {
            let mut b = bytes.clone();
            b[at] ^= 0xff;
            if let Ok(p) = Pack::from_bytes(b) {
                let _ = p.verify();
            }
        }
    }

    // M9-API-1107
    #[test]
    fn deflate_and_stored_packs_read_back_the_same_bytes() {
        let text = "deflate me ".repeat(3000);
        for codec in [Codec::None, Codec::Lz4, Codec::Deflate, Codec::Zstd] {
            let opts = WriteOptions {
                codec,
                chunk_log2: 12,
                ..WriteOptions::default()
            };
            let bytes = build(opts, &[EntrySpec::new("t.txt", text.as_bytes())]);
            let pack = Pack::from_bytes(bytes.clone()).unwrap();
            let e = pack.find_path("t.txt", false).unwrap();
            assert_eq!(&*pack.read(&e).unwrap(), text.as_bytes(), "{codec:?}");
            assert_eq!(e.is_contiguous(), codec == Codec::None);
            if codec != Codec::None {
                assert!(bytes.len() < text.len() / 4, "{codec:?}: {}", bytes.len());
            }
        }
    }

    // M9-API-1144
    #[test]
    fn zstd_chunks_round_trip_ranges_and_the_codec_is_recorded() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let mut a = EntrySpec::new("a.bin", &data);
        a.guid = Guid([1; 16]);
        let raw = noise(3000, 5);
        let mut b = EntrySpec::new("b.mp3", &raw);
        b.compress = false;
        let opts = WriteOptions {
            codec: Codec::Zstd,
            chunk_log2: 10,
            ..WriteOptions::default()
        };
        let pack = Pack::from_bytes(build(opts, &[a, b])).unwrap();
        let e = pack.find_guid(&Guid([1; 16])).unwrap();
        assert_eq!(e.chunk_count(), 10);
        assert!(
            pack.chunks_of(&e)
                .iter()
                .all(|(c, stored, raw)| *c == Some(Codec::Zstd) && stored < raw)
        );
        assert_eq!(&*pack.read(&e).unwrap(), &data[..]);
        for (off, len) in [(0, 10), (1000, 2000), (1023, 2), (9990, 100), (0, 10_000)] {
            let got = pack.read_range(&e, off, len).unwrap();
            let hi = (off as usize + len as usize).min(data.len());
            assert_eq!(&*got, &data[off as usize..hi], "{off} {len}");
        }
        // Data that does not shrink stays stored.
        let m = pack.find_path("b.mp3", false).unwrap();
        assert!(m.is_contiguous());
        assert_eq!(pack.verify().unwrap(), 2);
    }

    // M9-API-1108
    #[test]
    fn a_pack_inside_a_larger_file_opens_at_its_offset() {
        let bytes = build(
            WriteOptions::default(),
            &[EntrySpec::new("x/y.txt", b"inside")],
        );
        let dir = std::env::temp_dir().join(format!("iron_pack_at_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("bundle.bin");
        let mut all = vec![0xeeu8; 12_345];
        all.extend_from_slice(&bytes);
        all.extend_from_slice(&[0xdd; 100]);
        std::fs::write(&file, &all).unwrap();
        for mode in [OpenMode::Map, OpenMode::Read] {
            let pack = Pack::open_at(
                File::open(&file).unwrap(),
                12_345,
                bytes.len() as u64,
                mode,
                &file,
            )
            .unwrap();
            let e = pack.find_path("x/y.txt", false).unwrap();
            assert_eq!(&*pack.read(&e).unwrap(), b"inside");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M9-API-1109
    #[test]
    fn the_writer_refuses_duplicates_and_paths_outside_the_game() {
        let mut w = PackWriter::new(Cursor::new(Vec::new()), WriteOptions::default()).unwrap();
        w.add(&EntrySpec::new("a.txt", b"a")).unwrap();
        assert!(
            w.add(&EntrySpec::new("./a.txt", b"b")).is_err(),
            "same path"
        );
        let mut other = EntrySpec::new("b.txt", b"b");
        other.guid = Guid::from_path("a.txt");
        assert!(w.add(&other).is_err(), "same GUID");
        assert!(w.add(&EntrySpec::new("../b.txt", b"b")).is_err());
        let mut cooked = EntrySpec::new("a.txt", b"cooked");
        cooked.guid = Guid([5; 16]);
        cooked.cooked = true;
        w.add(&cooked).unwrap();
    }
}
