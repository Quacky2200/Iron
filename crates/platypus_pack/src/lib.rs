// SPDX-License-Identifier: Apache-2.0

//! Asset packs: how a release build ships its assets (design: `docs/research/platypus-asset-packs.md`).
//!
//! - [`format`]: the `.ppk` byte layout (header, table of contents, chunks), little-endian.
//! - [`PackWriter`]: streams entries into a pack (chunked, optionally compressed, deduplicated by
//!   content), then writes the table of contents. The output depends only on what was added.
//! - [`Pack`]: opens a pack (memory-mapped, read through a file handle, or from bytes) and reads
//!   entries by GUID or path without reading the rest of the file.
//! - [`Vfs`]: loose folders ([`DirSource`]) and packs ([`PackSource`]) stacked, the last mounted
//!   winning, so the dev loop and release builds load through the same calls.

pub mod format;
pub mod hash;
pub mod lz4;
mod reader;
mod vfs;
mod writer;

pub use format::{AssetKind, Codec, Guid};
pub use reader::{Bytes, Entry, OpenMode, Pack};
pub use vfs::{AssetInfo, AssetSource, DirSource, Found, PackSource, Vfs};
pub use writer::{EntrySpec, PackSummary, PackWriter, WriteOptions};

/// Why reading or writing a pack failed.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Not a pack, a newer version, or damaged (the text says which).
    Format(String),
    /// A chunk or entry whose checksum or hash does not match.
    Corrupt(String),
    /// Bad input to the writer (duplicate GUID or path, a path that is not a game path).
    Invalid(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Format(s) => write!(f, "not a readable pack: {s}"),
            Error::Corrupt(s) => write!(f, "damaged pack: {s}"),
            Error::Invalid(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Io(e) => e,
            other => std::io::Error::new(std::io::ErrorKind::InvalidData, other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
