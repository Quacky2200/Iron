//! Writing a pack: entries stream out as they are added (only the table of contents is kept in
//! memory), then [`PackWriter::finish`] writes the TOC and fills in the header. Nothing but the
//! added entries and the options affects the bytes (no times, no machine names), so building the
//! same assets twice gives the same file.

use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use crate::format::{
    AssetKind, Codec, FLAG_CONTIGUOUS, FLAG_COOKED, FLAG_TOMBSTONE, Guid, HEADER_SIZE, Header,
    RawChunk, RawEntry, SMALL_ALIGN, TOC_HEADER_SIZE, VERSION,
};
use crate::hash::{content_hash, crc32, normalize_path, path_hash};
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct WriteOptions {
    /// The codec tried for entries that ask for compression.
    pub codec: Codec,
    /// Raw bytes per chunk, as a power of two (18: 256 KiB).
    pub chunk_log2: u8,
    /// Entries at least this big start on this boundary (12: 4 KiB, a page).
    pub align_log2: u8,
    /// Deflate level (1..=9) when `codec` is deflate.
    pub level: u8,
}

impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions {
            codec: Codec::Lz4,
            chunk_log2: 18,
            align_log2: 12,
            level: 6,
        }
    }
}

/// One asset to add.
#[derive(Clone, Debug)]
pub struct EntrySpec<'a> {
    pub guid: Guid,
    /// A game path (normalized on add).
    pub path: &'a str,
    pub kind: AssetKind,
    /// The payload is the cooked form (see `FLAG_COOKED`).
    pub cooked: bool,
    /// Text kept with the entry: the `.meta` importer settings as JSON, or empty.
    pub meta: &'a str,
    pub data: &'a [u8],
    /// Try the pack's codec on this entry (off for data that is compressed already: mp3, ogg, jpg).
    pub compress: bool,
    /// A patch entry that deletes this GUID and path from packs mounted earlier (`data` ignored).
    pub tombstone: bool,
}

impl<'a> EntrySpec<'a> {
    /// A source file at `path`, with a GUID derived from the path and the kind from its name.
    pub fn new(path: &'a str, data: &'a [u8]) -> EntrySpec<'a> {
        EntrySpec {
            guid: Guid::from_path(&normalize_path(path).unwrap_or_default()),
            path,
            kind: AssetKind::of_path(path),
            cooked: false,
            meta: "",
            data,
            compress: true,
            tombstone: false,
        }
    }
}

/// What a written pack holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackSummary {
    pub entries: usize,
    pub chunks: usize,
    /// Entries that reuse another entry's chunks (same bytes).
    pub shared: usize,
    pub raw_bytes: u64,
    pub stored_bytes: u64,
    pub file_size: u64,
    pub pack_id: [u8; 16],
}

struct Pending {
    raw: RawEntry,
    path: String,
    meta: String,
}

/// (content hash, size, compress): what makes two entries' chunks interchangeable.
type ContentKey = ([u8; 16], u64, bool);
/// (first chunk, chunk count, contiguous).
type ChunkRun = (u32, u32, bool);

pub struct PackWriter<W: Write + Seek> {
    out: W,
    /// Where the pack starts in `out`.
    base: u64,
    /// The write position, from the start of the pack.
    pos: u64,
    opts: WriteOptions,
    entries: Vec<Pending>,
    chunks: Vec<RawChunk>,
    by_content: HashMap<ContentKey, ChunkRun>,
    guids: HashSet<Guid>,
    paths: HashSet<(String, bool)>,
    summary: PackSummary,
}

impl<W: Write + Seek> PackWriter<W> {
    /// Starts a pack at the writer's current position (usually the start of a file).
    pub fn new(mut out: W, opts: WriteOptions) -> Result<PackWriter<W>> {
        if !(10..=30).contains(&opts.chunk_log2) || !(4..=24).contains(&opts.align_log2) {
            return Err(Error::Invalid(
                "chunk_log2 or align_log2 out of range".into(),
            ));
        }
        let base = out.stream_position()?;
        out.write_all(&[0u8; HEADER_SIZE])?;
        Ok(PackWriter {
            out,
            base,
            pos: HEADER_SIZE as u64,
            opts,
            entries: Vec::new(),
            chunks: Vec::new(),
            by_content: HashMap::new(),
            guids: HashSet::new(),
            paths: HashSet::new(),
            summary: PackSummary::default(),
        })
    }

    fn pad_to(&mut self, align: u64) -> Result<()> {
        let pad = self.pos.next_multiple_of(align) - self.pos;
        if pad > 0 {
            self.out.write_all(&vec![0u8; pad as usize])?;
            self.pos += pad;
        }
        Ok(())
    }

    /// Adds an entry. A GUID, or a path with the same `cooked` flag, may appear once per pack.
    pub fn add(&mut self, spec: &EntrySpec<'_>) -> Result<()> {
        let path = normalize_path(spec.path)
            .ok_or_else(|| Error::Invalid(format!("{:?} is not a game path", spec.path)))?;
        if path.len() > usize::from(u16::MAX) {
            return Err(Error::Invalid(format!("path too long: {path}")));
        }
        if !self.guids.insert(spec.guid) {
            return Err(Error::Invalid(format!("GUID {} added twice", spec.guid)));
        }
        if !self.paths.insert((path.clone(), spec.cooked)) {
            return Err(Error::Invalid(format!("{path} added twice")));
        }
        let mut flags = if spec.cooked { FLAG_COOKED } else { 0 };
        let data: &[u8] = if spec.tombstone { &[] } else { spec.data };
        let hash = content_hash(data);
        let size = data.len() as u64;
        let (first_chunk, chunk_count) = if spec.tombstone {
            flags |= FLAG_TOMBSTONE;
            (self.chunks.len() as u32, 0)
        } else if let Some(&(first, count, contiguous)) =
            self.by_content.get(&(hash, size, spec.compress))
        {
            self.summary.shared += 1;
            if contiguous {
                flags |= FLAG_CONTIGUOUS;
            }
            (first, count)
        } else {
            let first = self.chunks.len() as u32;
            let align = if size >= 1 << self.opts.align_log2 {
                1u64 << self.opts.align_log2
            } else {
                SMALL_ALIGN
            };
            self.pad_to(align)?;
            let mut contiguous = true;
            for raw in data.chunks(1 << self.opts.chunk_log2) {
                let packed = if spec.compress {
                    self.compress(raw)
                } else {
                    None
                };
                let (codec, stored) = match &packed {
                    Some((codec, bytes)) => (*codec, bytes.as_slice()),
                    None => (Codec::None, raw),
                };
                contiguous &= codec == Codec::None;
                self.chunks.push(RawChunk {
                    offset: self.pos,
                    stored_size: stored.len() as u32,
                    raw_size: raw.len() as u32,
                    codec: codec as u8,
                    crc: crc32(stored),
                });
                self.out.write_all(stored)?;
                self.pos += stored.len() as u64;
                self.summary.stored_bytes += stored.len() as u64;
            }
            let count = self.chunks.len() as u32 - first;
            if contiguous {
                flags |= FLAG_CONTIGUOUS;
            }
            self.by_content
                .insert((hash, size, spec.compress), (first, count, contiguous));
            (first, count)
        };
        self.summary.raw_bytes += size;
        self.entries.push(Pending {
            raw: RawEntry {
                guid: spec.guid,
                content_hash: hash,
                size,
                first_chunk,
                chunk_count,
                path_offset: 0,
                path_len: path.len() as u16,
                kind: spec.kind as u8,
                flags,
                meta_offset: 0,
                meta_len: spec.meta.len() as u32,
            },
            path,
            meta: spec.meta.to_owned(),
        });
        Ok(())
    }

    /// The chunk compressed, when that saves at least 1/16 of it.
    fn compress(&self, raw: &[u8]) -> Option<(Codec, Vec<u8>)> {
        let packed = match self.opts.codec {
            Codec::None => return None,
            Codec::Lz4 => crate::lz4::compress(raw),
            Codec::Deflate => miniz_oxide::deflate::compress_to_vec(raw, self.opts.level),
        };
        (packed.len() + raw.len() / 16 <= raw.len()).then_some((self.opts.codec, packed))
    }

    /// Writes the table of contents and the header; returns the writer and what was written.
    pub fn finish(mut self) -> Result<(W, PackSummary)> {
        self.entries.sort_by_key(|e| e.raw.guid);
        // Each distinct text stored once: importer settings repeat across most assets, and a
        // path's source and cooked entries share it.
        let mut strings = Vec::new();
        let mut interned: HashMap<String, u32> = HashMap::new();
        let mut intern = |text: &str, strings: &mut Vec<u8>| -> u32 {
            *interned.entry(text.to_owned()).or_insert_with(|| {
                let at = strings.len() as u32;
                strings.extend_from_slice(text.as_bytes());
                at
            })
        };
        for e in &mut self.entries {
            e.raw.path_offset = intern(&e.path, &mut strings);
            e.raw.meta_offset = intern(&e.meta, &mut strings);
        }
        if strings.len() > u32::MAX as usize {
            return Err(Error::Invalid("TOC strings over 4 GiB".into()));
        }
        let mut slots: Vec<(u64, bool, &str, u32)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                (
                    path_hash(&e.path),
                    e.raw.flags & FLAG_COOKED != 0,
                    e.path.as_str(),
                    i as u32,
                )
            })
            .collect();
        slots.sort();
        let mut toc = Vec::new();
        toc.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        toc.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
        toc.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        toc.extend_from_slice(&0u32.to_le_bytes());
        debug_assert_eq!(toc.len(), TOC_HEADER_SIZE);
        for e in &self.entries {
            e.raw.encode(&mut toc);
        }
        for (hash, cooked, _, index) in &slots {
            toc.extend_from_slice(&hash.to_le_bytes());
            toc.extend_from_slice(&index.to_le_bytes());
            toc.extend_from_slice(&u32::from(*cooked).to_le_bytes());
        }
        for c in &self.chunks {
            c.encode(&mut toc);
        }
        toc.extend_from_slice(&strings);

        self.pad_to(SMALL_ALIGN)?;
        let toc_offset = self.pos;
        self.out.write_all(&toc)?;
        self.pos += toc.len() as u64;
        let header = Header {
            version: VERSION,
            flags: 0,
            align_log2: self.opts.align_log2,
            chunk_log2: self.opts.chunk_log2,
            toc_offset,
            toc_size: toc.len() as u64,
            file_size: self.pos,
            pack_id: content_hash(&toc),
            toc_crc: crc32(&toc),
        };
        let end = self.out.stream_position()?;
        self.out.seek(SeekFrom::Start(self.base))?;
        self.out.write_all(&header.encode())?;
        self.out.seek(SeekFrom::Start(end))?;
        self.out.flush()?;
        let summary = PackSummary {
            entries: self.entries.len(),
            chunks: self.chunks.len(),
            file_size: self.pos,
            pack_id: header.pack_id,
            ..self.summary
        };
        Ok((self.out, summary))
    }
}

impl PackWriter<BufWriter<std::fs::File>> {
    /// Writes a pack file through a temporary file and a rename, so a reader never sees half a
    /// pack. `fill` adds the entries.
    pub fn write_file(
        path: &Path,
        opts: WriteOptions,
        fill: impl FnOnce(&mut PackWriter<BufWriter<std::fs::File>>) -> Result<()>,
    ) -> Result<PackSummary> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("irpk-tmp");
        let file = std::fs::File::create(&tmp)?;
        let mut w = PackWriter::new(BufWriter::new(file), opts)?;
        let result = fill(&mut w).and_then(|_| w.finish());
        match result {
            Ok((out, summary)) => {
                out.into_inner()
                    .map_err(|e| Error::Io(e.into_error()))?
                    .sync_all()?;
                std::fs::rename(&tmp, path)?;
                Ok(summary)
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}
