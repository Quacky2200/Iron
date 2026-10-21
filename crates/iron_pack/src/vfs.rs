//! The asset file system: sources stacked in mount order, the last mounted answering first.
//! A development run mounts the game folder ([`DirSource`]); a release run mounts the packs the
//! manifest lists ([`PackSource`]), patches and DLC after them, mods last. Loaders ask the
//! [`Vfs`] and never see which one answered.
//!
//! Paths are game paths (`assets/x.png`, checked like [`iron_core::resolve_asset`]); `data:`
//! URLs decode in place, `guid:<32 hex>` finds the asset by GUID.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::format::{AssetKind, Guid};
use crate::hash::normalize_path;
use crate::reader::{Bytes, Entry, Pack};

/// What a source knows about one asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetInfo {
    pub path: String,
    pub guid: Option<Guid>,
    pub kind: AssetKind,
    pub size: u64,
    /// The bytes are the cooked form (`IRTX` texture, `IRMD` model), not the source file.
    pub cooked: bool,
    pub content_hash: Option<[u8; 16]>,
    /// `.meta` importer settings (JSON) when known.
    pub meta: String,
    /// The name of the source that has it.
    pub source: String,
}

/// A lookup's answer: the asset, or a patch's tombstone that hides it from sources below.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Asset(AssetInfo),
    Deleted,
}

/// One layer of the file system.
pub trait AssetSource: Send + Sync {
    fn name(&self) -> &str;
    /// A game path's source file or, with `cooked`, its cooked form.
    fn find(&self, path: &str, cooked: bool) -> Option<Found>;
    fn find_guid(&self, guid: &Guid) -> Option<Found>;
    /// The bytes of an asset this source found.
    fn read(&self, info: &AssetInfo) -> std::io::Result<Bytes>;
    /// Part of an asset (streaming audio, texture mips): `len` bytes from `offset`, clamped.
    fn read_range(&self, info: &AssetInfo, offset: u64, len: u64) -> std::io::Result<Bytes>;
    /// Every asset path this source has (sorted, deduplicated).
    fn paths(&self) -> Vec<String>;
    /// The folder behind a loose-file source.
    fn root(&self) -> Option<&Path> {
        None
    }
}

fn not_found(path: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("`{path}`: no such asset"),
    )
}

// ---- Loose files ----

/// A game folder on disk (the dev loop). `.meta` files give GUIDs and importer settings.
pub struct DirSource {
    name: String,
    root: PathBuf,
    /// GUID -> path, from the `.meta` files at creation (`rescan` refreshes it).
    guids: std::sync::RwLock<std::collections::HashMap<Guid, String>>,
}

/// Folders that hold no assets (the same list as `scripts/assets.ts`).
const SKIP: &[&str] = &["dist", "node_modules", "src", "target", "types"];

impl DirSource {
    pub fn new(root: &Path) -> DirSource {
        let s = DirSource {
            name: format!("dir:{}", root.display()),
            root: root.to_path_buf(),
            guids: Default::default(),
        };
        s.rescan();
        s
    }

    /// Reads the `.meta` GUIDs again (after `iron-build assets` or a file move).
    pub fn rescan(&self) {
        let mut map = std::collections::HashMap::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let p = e.path();
                if p.is_dir() {
                    if !SKIP.contains(&name.as_str()) {
                        stack.push(p);
                    }
                } else if name.ends_with(".meta")
                    && let Some(guid) = std::fs::read_to_string(&p).ok().and_then(|t| meta_guid(&t))
                    && let Ok(rel) = p.with_extension("").strip_prefix(&self.root)
                {
                    map.insert(guid, rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        *self.guids.write().expect("not poisoned") = map;
    }

    fn file(&self, path: &str) -> Option<PathBuf> {
        iron_core::resolve_asset(&self.root, path)
    }

    fn info(&self, path: &str) -> Option<AssetInfo> {
        let path = normalize_path(path)?;
        let file = self.file(&path)?;
        let size = std::fs::metadata(&file).ok().filter(|m| m.is_file())?.len();
        let meta = std::fs::read_to_string(
            file.with_file_name(format!("{}.meta", file.file_name()?.to_string_lossy())),
        )
        .unwrap_or_default();
        Some(AssetInfo {
            guid: meta_guid(&meta),
            kind: AssetKind::of_path(&path),
            size,
            cooked: false,
            content_hash: None,
            meta,
            source: self.name.clone(),
            path,
        })
    }
}

/// The `"guid"` of a `.meta` file's JSON (a small scan: this crate has no JSON parser).
fn meta_guid(text: &str) -> Option<Guid> {
    let at = text.find("\"guid\"")?;
    let rest = &text[at + 6..];
    let rest = rest[rest.find(':')? + 1..].trim_start().strip_prefix('"')?;
    Guid::parse(&rest[..rest.find('"')?])
}

impl AssetSource for DirSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn find(&self, path: &str, cooked: bool) -> Option<Found> {
        // Cooked files of the dev loop (`.iron/cooked`) are the app's business: it checks they
        // are fresh and made with the settings asked for.
        if cooked {
            return None;
        }
        self.info(path).map(Found::Asset)
    }

    fn find_guid(&self, guid: &Guid) -> Option<Found> {
        let path = self
            .guids
            .read()
            .expect("not poisoned")
            .get(guid)
            .cloned()?;
        self.info(&path).map(Found::Asset)
    }

    fn read(&self, info: &AssetInfo) -> std::io::Result<Bytes> {
        let file = self.file(&info.path).ok_or_else(|| not_found(&info.path))?;
        std::fs::read(file).map(Bytes::from)
    }

    fn read_range(&self, info: &AssetInfo, offset: u64, len: u64) -> std::io::Result<Bytes> {
        let file = self.file(&info.path).ok_or_else(|| not_found(&info.path))?;
        let mut f = std::fs::File::open(file)?;
        f.seek(SeekFrom::Start(offset))?;
        let mut out = Vec::new();
        f.take(len).read_to_end(&mut out)?;
        Ok(Bytes::from(out))
    }

    fn paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || name.ends_with(".meta") {
                    continue;
                }
                let p = e.path();
                if p.is_dir() {
                    if !SKIP.contains(&name.as_str()) {
                        stack.push(p);
                    }
                } else if let Ok(rel) = p.strip_prefix(&self.root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        out.sort();
        out
    }

    fn root(&self) -> Option<&Path> {
        Some(&self.root)
    }
}

// ---- Packs ----

/// A mounted pack.
pub struct PackSource {
    name: String,
    pack: Pack,
}

impl PackSource {
    pub fn new(name: &str, pack: Pack) -> PackSource {
        PackSource {
            name: name.to_owned(),
            pack,
        }
    }

    pub fn open(name: &str, path: &Path) -> crate::Result<PackSource> {
        Ok(PackSource::new(name, Pack::open(path)?))
    }

    pub fn pack(&self) -> &Pack {
        &self.pack
    }

    fn found(&self, e: Entry) -> Found {
        if e.is_tombstone() {
            return Found::Deleted;
        }
        Found::Asset(AssetInfo {
            cooked: e.is_cooked(),
            guid: Some(e.guid),
            kind: e.kind,
            size: e.size,
            content_hash: Some(e.content_hash),
            meta: e.meta,
            source: self.name.clone(),
            path: e.path,
        })
    }

    fn entry(&self, info: &AssetInfo) -> std::io::Result<Entry> {
        info.guid
            .and_then(|g| self.pack.find_guid(&g))
            .or_else(|| self.pack.find_path(&info.path, info.cooked))
            .ok_or_else(|| not_found(&info.path))
    }
}

impl AssetSource for PackSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn find(&self, path: &str, cooked: bool) -> Option<Found> {
        self.pack.find_path(path, cooked).map(|e| self.found(e))
    }

    fn find_guid(&self, guid: &Guid) -> Option<Found> {
        self.pack.find_guid(guid).map(|e| self.found(e))
    }

    fn read(&self, info: &AssetInfo) -> std::io::Result<Bytes> {
        Ok(self.pack.read(&self.entry(info)?)?)
    }

    fn read_range(&self, info: &AssetInfo, offset: u64, len: u64) -> std::io::Result<Bytes> {
        Ok(self.pack.read_range(&self.entry(info)?, offset, len)?)
    }

    fn paths(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .pack
            .entries()
            .filter(|e| !e.is_tombstone())
            .map(|e| e.path)
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

// ---- The stack ----

/// Sources in mount order; lookups go from the last mounted down and stop at the first answer
/// (a tombstone answers "deleted"). Cheap to clone.
#[derive(Clone, Default)]
pub struct Vfs {
    layers: Vec<Arc<dyn AssetSource>>,
}

impl std::fmt::Debug for Vfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.layers.iter().map(|l| l.name()))
            .finish()
    }
}

impl Vfs {
    pub fn new() -> Vfs {
        Vfs::default()
    }

    /// A game folder as the only source (the dev loop).
    pub fn dir(root: &Path) -> Vfs {
        let mut v = Vfs::new();
        v.mount(Arc::new(DirSource::new(root)));
        v
    }

    /// Puts a source on top.
    pub fn mount(&mut self, source: Arc<dyn AssetSource>) {
        self.layers.push(source);
    }

    /// Removes the topmost source with this name; false when there is none.
    pub fn unmount(&mut self, name: &str) -> bool {
        match self.layers.iter().rposition(|l| l.name() == name) {
            Some(i) => {
                self.layers.remove(i);
                true
            }
            None => false,
        }
    }

    pub fn layers(&self) -> &[Arc<dyn AssetSource>] {
        &self.layers
    }

    /// The folder of the topmost loose-file source (dev loop: the game folder).
    pub fn root(&self) -> Option<&Path> {
        self.layers.iter().rev().find_map(|l| l.root())
    }

    fn lookup(
        &self,
        ask: impl Fn(&dyn AssetSource) -> Option<Found>,
    ) -> Option<(Arc<dyn AssetSource>, AssetInfo)> {
        for layer in self.layers.iter().rev() {
            match ask(layer.as_ref()) {
                Some(Found::Asset(info)) => return Some((layer.clone(), info)),
                Some(Found::Deleted) => return None,
                None => {}
            }
        }
        None
    }

    /// Where an asset is (source form; `guid:<hex>` by GUID).
    pub fn info(&self, path: &str) -> Option<AssetInfo> {
        self.find(path, false).map(|(_, i)| i)
    }

    fn find(&self, path: &str, cooked: bool) -> Option<(Arc<dyn AssetSource>, AssetInfo)> {
        if let Some(hex) = path.strip_prefix("guid:") {
            let guid = Guid::parse(hex)?;
            return self.lookup(|l| l.find_guid(&guid));
        }
        self.lookup(|l| l.find(path, cooked))
    }

    pub fn exists(&self, path: &str) -> bool {
        path.trim_start().starts_with("data:") || self.find(path, false).is_some()
    }

    /// An asset's source bytes: a `data:` URL decoded, a `guid:` or a game path found.
    pub fn read(&self, path: &str) -> std::io::Result<Bytes> {
        if path.trim_start().starts_with("data:") {
            return iron_core::decode_data_url(path)
                .map(Bytes::from)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad data URL")
                });
        }
        let (layer, info) = self.find(path, false).ok_or_else(|| not_found(path))?;
        layer.read(&info)
    }

    pub fn read_to_string(&self, path: &str) -> std::io::Result<String> {
        String::from_utf8(self.read(path)?.into_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// The cooked form of a game path when a source has one (packs do; loose folders leave it to
    /// the app's `.iron/cooked` check).
    pub fn read_cooked(&self, path: &str) -> Option<std::io::Result<(AssetInfo, Bytes)>> {
        let (layer, info) = self.find(path, true)?;
        Some(layer.read(&info).map(|b| (info, b)))
    }

    pub fn read_range(&self, path: &str, offset: u64, len: u64) -> std::io::Result<Bytes> {
        let (layer, info) = self.find(path, false).ok_or_else(|| not_found(path))?;
        layer.read_range(&info, offset, len)
    }

    /// The path of the asset with this GUID.
    pub fn guid_to_path(&self, guid: &Guid) -> Option<String> {
        self.lookup(|l| l.find_guid(guid)).map(|(_, i)| i.path)
    }

    /// Every visible asset path (hidden ones under a tombstone left out), sorted.
    pub fn paths(&self) -> Vec<String> {
        let mut all: Vec<String> = self.layers.iter().flat_map(|l| l.paths()).collect();
        all.sort();
        all.dedup();
        all.retain(|p| self.find(p, false).is_some() || self.find(p, true).is_some());
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::{EntrySpec, PackWriter, WriteOptions};
    use std::io::Cursor;

    fn pack(specs: &[EntrySpec<'_>]) -> Pack {
        let mut w = PackWriter::new(Cursor::new(Vec::new()), WriteOptions::default()).unwrap();
        for s in specs {
            w.add(s).unwrap();
        }
        Pack::from_bytes(w.finish().unwrap().0.into_inner()).unwrap()
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_vfs_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // M9-API-1110
    #[test]
    fn later_sources_override_earlier_ones_and_tombstones_hide_them() {
        let base = pack(&[
            EntrySpec::new("assets/a.txt", b"base a"),
            EntrySpec::new("assets/b.txt", b"base b"),
            EntrySpec::new("assets/c.txt", b"base c"),
        ]);
        let mut gone = EntrySpec::new("assets/c.txt", b"");
        gone.tombstone = true;
        let patch = pack(&[EntrySpec::new("assets/a.txt", b"patched a"), gone]);
        let mut vfs = Vfs::new();
        vfs.mount(Arc::new(PackSource::new("base", base)));
        vfs.mount(Arc::new(PackSource::new("patch-1", patch)));
        assert_eq!(&*vfs.read("assets/a.txt").unwrap(), b"patched a");
        assert_eq!(&*vfs.read("./assets/b.txt").unwrap(), b"base b");
        assert_eq!(
            vfs.read("assets/c.txt").unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        assert_eq!(vfs.info("assets/b.txt").unwrap().source, "base");
        assert_eq!(vfs.paths(), ["assets/a.txt", "assets/b.txt"]);
        let guid = Guid::from_path("assets/b.txt");
        assert_eq!(&*vfs.read(&format!("guid:{guid}")).unwrap(), b"base b");
        assert_eq!(vfs.guid_to_path(&guid).as_deref(), Some("assets/b.txt"));
        assert_eq!(&*vfs.read("data:,inline").unwrap(), b"inline");
        assert!(vfs.unmount("patch-1"));
        assert_eq!(&*vfs.read("assets/c.txt").unwrap(), b"base c");
    }

    // M9-API-1111
    #[test]
    fn a_folder_and_a_pack_answer_the_same_calls() {
        let d = temp("dir");
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::create_dir_all(d.join("dist")).unwrap();
        std::fs::write(d.join("assets/hit.wav"), b"RIFF....").unwrap();
        std::fs::write(
            d.join("assets/hit.wav.meta"),
            "{\n  \"guid\": \"0123456789abcdef0123456789abcdef\",\n  \"importer\": {}\n}\n",
        )
        .unwrap();
        std::fs::write(d.join("dist/game.irbundle"), b"{}").unwrap();
        let dir = Vfs::dir(&d);
        let mut spec = EntrySpec::new("assets/hit.wav", b"RIFF....");
        spec.guid = Guid::parse("0123456789abcdef0123456789abcdef").unwrap();
        let mut packed = Vfs::new();
        packed.mount(Arc::new(PackSource::new("p", pack(&[spec]))));
        for vfs in [&dir, &packed] {
            assert_eq!(&*vfs.read("assets/hit.wav").unwrap(), b"RIFF....");
            assert_eq!(&*vfs.read_range("assets/hit.wav", 4, 100).unwrap(), b"....");
            let info = vfs.info("assets/hit.wav").unwrap();
            assert_eq!((info.kind, info.size), (AssetKind::Audio, 8));
            assert_eq!(
                info.guid.unwrap().to_hex(),
                "0123456789abcdef0123456789abcdef"
            );
            assert_eq!(
                &*vfs.read("guid:0123456789abcdef0123456789abcdef").unwrap(),
                b"RIFF...."
            );
            assert_eq!(vfs.paths(), ["assets/hit.wav"]);
            assert!(vfs.read("../outside.txt").is_err());
            assert!(vfs.read_cooked("assets/hit.wav").is_none());
        }
        assert_eq!(dir.root(), Some(d.as_path()));
        assert_eq!(packed.root(), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-API-1112
    #[test]
    fn cooked_forms_are_found_beside_the_source() {
        let mut cooked = EntrySpec::new("assets/a.png", b"IRTX cooked");
        cooked.guid = Guid([3; 16]);
        cooked.cooked = true;
        let mut vfs = Vfs::new();
        vfs.mount(Arc::new(PackSource::new(
            "p",
            pack(&[EntrySpec::new("assets/a.png", b"\x89PNG"), cooked]),
        )));
        let (info, bytes) = vfs.read_cooked("assets/a.png").unwrap().unwrap();
        assert!(info.cooked);
        assert_eq!(&*bytes, b"IRTX cooked");
        assert_eq!(&*vfs.read("assets/a.png").unwrap(), b"\x89PNG");
    }
}
