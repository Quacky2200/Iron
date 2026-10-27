// SPDX-License-Identifier: Apache-2.0

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
    /// How lookups name this source in a not-found message (`folder`, `assets.dat`).
    fn label(&self) -> String;
    /// The file's real spelling when `path` differs from it only in case.
    fn real_case(&self, _path: &str) -> Option<String> {
        None
    }
    /// The pack this source is (`assets`, `scene-level1`), `""` for a folder.
    fn pack_name(&self) -> &str {
        ""
    }
}

fn not_found(path: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("asset \"{path}\" not found"),
    )
}

// ---- Loose files ----

/// A game folder on disk (the dev loop). `.meta` files give GUIDs and importer settings.
pub struct DirSource {
    name: String,
    root: PathBuf,
    /// GUID -> path, from the `.meta` files at creation (`rescan` refreshes it).
    guids: std::sync::RwLock<std::collections::HashMap<Guid, String>>,
    /// Paths already checked for their spelling (the check reads folders, so once per path).
    spelled: std::sync::Mutex<std::collections::HashSet<String>>,
}

/// Folders that hold no assets (the same list as `scripts/assets.ts`).
const SKIP: &[&str] = &["dist", "node_modules", "src", "target", "types"];

impl DirSource {
    pub fn new(root: &Path) -> DirSource {
        let s = DirSource {
            name: format!("dir:{}", root.display()),
            root: root.to_path_buf(),
            guids: Default::default(),
            spelled: Default::default(),
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

    /// The file to read (warns in the dev loop when a packaged build would not hold it).
    fn file(&self, path: &str) -> Option<PathBuf> {
        iron_core::resolve_asset(&self.root, path)
    }

    /// The file for a lookup, which warns about nothing: only loads do.
    fn file_quiet(&self, path: &str) -> Option<PathBuf> {
        iron_core::resolve_asset_quiet(&self.root, path)
    }

    /// Dev loop: the OS may find `assets/SFX/Boom.wav` where only `assets/sfx/boom.wav` exists
    /// (Windows, macOS), which a pack or Linux would not. Says so once per path.
    fn check_spelling(&self, path: &str) {
        if !self
            .spelled
            .lock()
            .is_ok_and(|mut seen| seen.insert(path.to_owned()))
        {
            return;
        }
        if let Some(real) = self.real_case(path) {
            tracing::warn!(
                "asset \"{path}\": the file on disk is \"{real}\" (packs are case-sensitive)"
            );
        }
    }

    fn info(&self, path: &str) -> Option<AssetInfo> {
        let path = normalize_path(path)?;
        let file = self.file_quiet(&path)?;
        let size = std::fs::metadata(&file).ok().filter(|m| m.is_file())?.len();
        self.check_spelling(&path);
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

    fn label(&self) -> String {
        "folder".into()
    }

    fn real_case(&self, path: &str) -> Option<String> {
        let path = normalize_path(path)?;
        let mut dir = self.root.clone();
        let mut real = Vec::new();
        for part in path.split('/') {
            let mut found = None;
            for e in std::fs::read_dir(&dir).ok()?.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name == part {
                    found = Some(name);
                    break;
                }
                if found.is_none() && name.eq_ignore_ascii_case(part) {
                    found = Some(name);
                }
            }
            let name = found?;
            dir.push(&name);
            real.push(name);
        }
        let real = real.join("/");
        (real != path).then_some(real)
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

    fn label(&self) -> String {
        if self.name.starts_with("scene-") {
            "scene data".into()
        } else {
            format!("{}.dat", self.name)
        }
    }

    fn pack_name(&self) -> &str {
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

// ---- Reading in pieces ----

/// Bytes fetched per read: a pack chunk, so a compressed chunk is inflated once for the many
/// small reads a decoder makes.
const WINDOW: u64 = 256 * 1024;

struct RangeSource {
    layer: Arc<dyn AssetSource>,
    info: AssetInfo,
    size: u64,
    /// The window held: its start and bytes.
    window: std::sync::Mutex<(u64, Bytes)>,
}

impl iron_core::RangeRead for RangeSource {
    fn len(&self) -> u64 {
        self.size
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        if offset >= self.size || buf.is_empty() {
            return Ok(0);
        }
        let mut w = self.window.lock().expect("not poisoned");
        let held = w.1.len() as u64;
        if offset < w.0 || offset >= w.0 + held {
            let start = offset / WINDOW * WINDOW;
            *w = (start, self.layer.read_range(&self.info, start, WINDOW)?);
        }
        let from = (offset - w.0) as usize;
        let n = buf.len().min(w.1.len().saturating_sub(from));
        buf[..n].copy_from_slice(&w.1[from..from + n]);
        Ok(n)
    }
}

impl iron_core::AssetFs for Vfs {
    fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        Ok(Vfs::read(self, path)?.into_vec())
    }

    fn open_range(&self, path: &str) -> std::io::Result<Arc<dyn iron_core::RangeRead>> {
        Vfs::open_range(self, path)
    }

    fn exists(&self, path: &str) -> bool {
        Vfs::exists(self, path)
    }

    fn list(&self) -> Vec<String> {
        self.paths()
    }

    fn pack_of(&self, path: &str) -> String {
        Vfs::pack_of(self, path)
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
        let (layer, info) = self.find(path, false).ok_or_else(|| self.missing(path))?;
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
        let (layer, info) = self.find(path, false).ok_or_else(|| self.missing(path))?;
        layer.read_range(&info, offset, len)
    }

    /// Where lookups go, top to bottom, as the not-found message names them: `scene data`
    /// (every scene pack together), `assets.dat`, `folder`.
    pub fn searched(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for l in self.layers.iter().rev() {
            let label = l.label();
            if !out.contains(&label) {
                out.push(label);
            }
        }
        out
    }

    /// The error for a path nothing has: `asset "x" not found in scene data, assets.dat or
    /// folder`, plus the real spelling when only the case differs.
    pub fn missing(&self, path: &str) -> std::io::Error {
        let places = self.searched();
        let place = match places.as_slice() {
            [] => "an empty file system".to_owned(),
            [one] => one.clone(),
            [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
        };
        let mut msg = format!("asset \"{path}\" not found in {place}");
        if let Some(real) = self.layers.iter().rev().find_map(|l| l.real_case(path)) {
            msg.push_str(&format!(
                " (the file on disk is \"{real}\"; asset paths are case-sensitive)"
            ));
        }
        std::io::Error::new(std::io::ErrorKind::NotFound, msg)
    }

    /// The pack that holds a visible asset (`assets`, `scene-level1`), `""` for a loose file or
    /// when there is none.
    pub fn pack_of(&self, path: &str) -> String {
        self.find(path, false)
            .map(|(layer, _)| layer.pack_name().to_owned())
            .unwrap_or_default()
    }

    /// A handle that reads an asset in pieces (streamed sounds): a window of it is read at a
    /// time, so a big compressed sound is never inflated whole.
    pub fn open_range(&self, path: &str) -> std::io::Result<Arc<dyn iron_core::RangeRead>> {
        let (layer, info) = self.find(path, false).ok_or_else(|| self.missing(path))?;
        Ok(Arc::new(RangeSource {
            size: info.size,
            layer,
            info,
            window: std::sync::Mutex::new((0, Bytes::from(Vec::new()))),
        }))
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

    // M9-API-1145
    #[test]
    fn scene_packs_answer_before_assets_dat_before_the_folder_and_a_miss_names_them() {
        let d = temp("order");
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/loose.txt"), b"loose").unwrap();
        std::fs::write(d.join("assets/both.txt"), b"folder").unwrap();
        let assets = pack(&[
            EntrySpec::new("assets/both.txt", b"assets.dat"),
            EntrySpec::new("assets/shared.txt", b"shared"),
        ]);
        let scene = pack(&[
            EntrySpec::new("assets/both.txt", b"scene"),
            EntrySpec::new("scenes/l1.scene.json", b"{}"),
        ]);
        // Mount order is bottom to top: the folder, assets.dat, then the scene packs.
        let mut vfs = Vfs::new();
        vfs.mount(Arc::new(DirSource::new(&d)));
        vfs.mount(Arc::new(PackSource::new("assets", assets.clone())));
        vfs.mount(Arc::new(PackSource::new("scene-l1", scene.clone())));
        assert_eq!(&*vfs.read("assets/both.txt").unwrap(), b"scene");
        assert_eq!(&*vfs.read("assets/shared.txt").unwrap(), b"shared");
        assert_eq!(&*vfs.read("assets/loose.txt").unwrap(), b"loose");
        assert_eq!(vfs.pack_of("assets/both.txt"), "scene-l1");
        assert_eq!(vfs.pack_of("assets/shared.txt"), "assets");
        assert_eq!(vfs.pack_of("assets/loose.txt"), "");
        assert_eq!(vfs.pack_of("assets/none.txt"), "");
        // A patch above everything wins, and a tombstone hides the scene pack's answer too.
        let mut gone = EntrySpec::new("assets/both.txt", b"");
        gone.tombstone = true;
        let mut with_patch = vfs.clone();
        with_patch.mount(Arc::new(PackSource::new(
            "patch-1",
            pack(&[EntrySpec::new("assets/shared.txt", b"patched"), gone]),
        )));
        assert_eq!(&*with_patch.read("assets/shared.txt").unwrap(), b"patched");
        assert!(with_patch.read("assets/both.txt").is_err());
        // The message names everything that was searched, top to bottom.
        let e = vfs.read("assets/x.wav").unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            e.to_string(),
            "asset \"assets/x.wav\" not found in scene data, assets.dat or folder"
        );
        // Packs only: no folder in the list.
        let mut release = Vfs::new();
        release.mount(Arc::new(PackSource::new("assets", assets)));
        release.mount(Arc::new(PackSource::new("scene-l1", scene)));
        assert_eq!(
            release.read("a.png").unwrap_err().to_string(),
            "asset \"a.png\" not found in scene data or assets.dat"
        );
        assert_eq!(
            Vfs::dir(&d).read("a.png").unwrap_err().to_string(),
            "asset \"a.png\" not found in folder"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-API-1146
    #[test]
    fn a_path_that_differs_only_in_case_gets_the_real_spelling() {
        let d = temp("case");
        std::fs::create_dir_all(d.join("assets/sfx")).unwrap();
        std::fs::write(d.join("assets/sfx/boom.wav"), b"x").unwrap();
        let dir = DirSource::new(&d);
        assert_eq!(
            dir.real_case("assets/SFX/Boom.wav").as_deref(),
            Some("assets/sfx/boom.wav")
        );
        assert_eq!(dir.real_case("assets/sfx/boom.wav"), None);
        assert_eq!(dir.real_case("assets/sfx/none.wav"), None);
        // Where the OS ignores case the load works (and warns once); where it does not, the
        // miss says what the file is called.
        let vfs = Vfs::dir(&d);
        match vfs.read("assets/SFX/Boom.wav") {
            Ok(b) => assert_eq!(&*b, b"x"),
            Err(e) => assert!(e.to_string().contains("assets/sfx/boom.wav"), "{e}"),
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-API-1147
    #[test]
    fn an_asset_reads_in_pieces_across_windows_without_the_whole_file() {
        let data: Vec<u8> = (0..700_000u32).map(|i| (i % 253) as u8).collect();
        let mut w = PackWriter::new(Cursor::new(Vec::new()), WriteOptions::default()).unwrap();
        w.add(&EntrySpec::new("assets/long.wav", &data)).unwrap();
        let bytes = w.finish().unwrap().0.into_inner();
        let mut vfs = Vfs::new();
        vfs.mount(Arc::new(PackSource::new(
            "assets",
            Pack::from_bytes(bytes).unwrap(),
        )));
        let r = vfs.open_range("assets/long.wav").unwrap();
        assert_eq!(r.len(), data.len() as u64);
        let mut got = Vec::new();
        let mut at = 0u64;
        let mut buf = [0u8; 4096];
        // Small reads, then one that starts in the middle of a window.
        while at < 300_000 {
            let n = r.read_at(at, &mut buf).unwrap();
            assert!(n > 0);
            got.extend_from_slice(&buf[..n]);
            at += n as u64;
        }
        assert_eq!(got, data[..got.len()]);
        let mut tail = vec![0u8; 100];
        assert_eq!(r.read_at(699_950, &mut tail).unwrap(), 50);
        assert_eq!(&tail[..50], &data[699_950..]);
        assert_eq!(r.read_at(700_000, &mut tail).unwrap(), 0);
        assert!(vfs.open_range("assets/none.wav").is_err());
    }
}
