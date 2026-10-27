// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Asset `.meta` files (a port of `scripts/assets.ts`): every asset in a game folder gets
//! `<file>.meta` next to it holding a GUID and its importer settings. Existing files are kept; a file
//! moved with its `.meta` left behind keeps its GUID when the move is unambiguous.

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Asset kind by extension (lowercase), `None` for files that are not assets.
pub fn kind(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "png" | "jpg" | "jpeg" | "svg" | "hdr" => "texture",
        "glb" | "gltf" | "bin" => "model",
        "wav" | "ogg" | "mp3" | "flac" => "audio",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "json" => "data",
        _ => return None,
    })
}

/// Folders that hold no assets.
const SKIP: &[&str] = &["dist", "node_modules", "src", "target", "types"];
const NOT_ASSETS: &[&str] = &[
    "package.json",
    "project.json",
    "tsconfig.json",
    "bun.lock",
    "bun.lockb",
];

pub fn is_asset(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    if name.ends_with(".meta") || NOT_ASSETS.contains(&name) {
        return false;
    }
    name.rsplit_once('.')
        .is_some_and(|(_, e)| kind(&e.to_lowercase()).is_some())
}

/// The default `.meta` text of an asset (the JSON `scripts/assets.ts` wrote, key order included).
pub fn default_meta(file: &str, guid: &str) -> String {
    let ext = file.rsplit('.').next().unwrap_or("").to_lowercase();
    let importer = match kind(&ext).unwrap_or("data") {
        "texture" => {
            "{\n    \"type\": \"texture\",\n    \"filterMode\": \"bilinear\",\n    \"wrapMode\": \"repeat\",\n    \"mipmaps\": true,\n    \"anisoLevel\": 1,\n    \"linear\": false\n  }"
        }
        "model" => "{\n    \"type\": \"model\",\n    \"colliders\": \"none\"\n  }",
        "audio" => "{\n    \"type\": \"audio\",\n    \"stream\": false\n  }",
        "font" => "{\n    \"type\": \"font\"\n  }",
        _ => "{\n    \"type\": \"data\"\n  }",
    };
    format!("{{\n  \"guid\": \"{guid}\",\n  \"importer\": {importer}\n}}\n")
}

/// 32 hex digits, random (a UUID v4 without dashes).
pub fn new_guid() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let mut bytes = [0u8; 16];
    for half in 0..2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(N.fetch_add(1, Ordering::Relaxed));
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        bytes[half * 8..half * 8 + 8].copy_from_slice(&h.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every asset file and `.meta` file under `root` (relative, `/`), sorted.
pub fn scan(root: &Path) -> (Vec<String>, Vec<String>) {
    let (mut assets, mut metas) = (Vec::new(), Vec::new());
    fn walk(root: &Path, dir: &Path, assets: &mut Vec<String>, metas: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            if path.is_dir() {
                if !SKIP.contains(&name.as_str()) {
                    walk(root, &path, assets, metas);
                }
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if name.ends_with(".meta") {
                metas.push(rel);
            } else if is_asset(&name) {
                assets.push(rel);
            }
        }
    }
    walk(root, root, &mut assets, &mut metas);
    assets.sort();
    metas.sort();
    (assets, metas)
}

#[derive(Debug, Default, PartialEq)]
pub struct Report {
    pub created: Vec<String>,
    pub moved: Vec<(String, String)>,
    pub orphans: Vec<String>,
}

/// Makes sure every asset under `root` has a `.meta`; returns what changed.
pub fn update(root: &Path, prune: bool) -> std::io::Result<Report> {
    let (assets, metas) = scan(root);
    let asset_set: HashSet<&String> = assets.iter().collect();
    let orphans: Vec<String> = metas
        .iter()
        .filter(|m| !asset_set.contains(&m[..m.len() - ".meta".len()].to_string()))
        .cloned()
        .collect();
    let missing: Vec<&String> = assets
        .iter()
        .filter(|a| !root.join(format!("{a}.meta")).exists())
        .collect();
    let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    let mut report = Report::default();
    let mut by_orphan: HashMap<String, Vec<String>> = HashMap::new();
    for o in &orphans {
        let a = o[..o.len() - ".meta".len()].to_string();
        by_orphan.entry(base(&a)).or_default().push(a);
    }
    let mut by_missing: HashMap<String, Vec<&String>> = HashMap::new();
    for a in &missing {
        by_missing.entry(base(a)).or_default().push(a);
    }
    let mut moved: HashSet<String> = HashSet::new();
    let mut names: Vec<&String> = by_missing.keys().collect();
    names.sort();
    for name in names {
        let files = &by_missing[name];
        if let Some(from) = by_orphan.get(name)
            && files.len() == 1
            && from.len() == 1
        {
            std::fs::rename(
                root.join(format!("{}.meta", from[0])),
                root.join(format!("{}.meta", files[0])),
            )?;
            report.moved.push((from[0].clone(), files[0].clone()));
            moved.insert(files[0].clone());
            moved.insert(format!("{}.meta", from[0]));
        }
    }
    for a in missing {
        if moved.contains(a) {
            continue;
        }
        std::fs::write(root.join(format!("{a}.meta")), default_meta(a, &new_guid()))?;
        report.created.push(a.clone());
    }
    for o in orphans {
        if moved.contains(&o) {
            continue;
        }
        if prune {
            std::fs::remove_file(root.join(&o))?;
        }
        report.orphans.push(o);
    }
    Ok(report)
}

/// Reads the GUID of a `.meta` file (None when it is not one).
pub fn read_guid(path: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    v["guid"].as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("platypus_build_{name}_{}", new_guid()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // M9-TS-1830
    #[test]
    fn metas_are_made_for_new_assets_and_kept_for_old_ones() {
        let d = temp("meta");
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/a.png"), b"x").unwrap();
        std::fs::write(d.join("assets/s.wav"), b"x").unwrap();
        std::fs::write(d.join("package.json"), b"{}").unwrap();
        let r = update(&d, false).unwrap();
        assert_eq!(r.created, ["assets/a.png", "assets/s.wav"]);
        let guid = read_guid(&d.join("assets/a.png.meta")).unwrap();
        assert_eq!(guid.len(), 32);
        assert!(!d.join("package.json.meta").exists());
        assert!(update(&d, false).unwrap().created.is_empty());
        assert_eq!(read_guid(&d.join("assets/a.png.meta")).unwrap(), guid);
        let text = std::fs::read_to_string(d.join("assets/a.png.meta")).unwrap();
        assert!(text.contains("\"filterMode\": \"bilinear\"") && text.ends_with("}\n"));
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-1831
    #[test]
    fn a_moved_asset_keeps_its_guid_and_a_gone_one_is_an_orphan() {
        let d = temp("move");
        std::fs::write(d.join("a.png"), b"x").unwrap();
        std::fs::write(d.join("gone.wav"), b"x").unwrap();
        update(&d, false).unwrap();
        let guid = read_guid(&d.join("a.png.meta")).unwrap();
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::rename(d.join("a.png"), d.join("sub/a.png")).unwrap();
        std::fs::remove_file(d.join("gone.wav")).unwrap();
        let r = update(&d, false).unwrap();
        assert_eq!(r.moved, [("a.png".to_string(), "sub/a.png".to_string())]);
        assert_eq!(r.orphans, ["gone.wav.meta"]);
        assert_eq!(read_guid(&d.join("sub/a.png.meta")).unwrap(), guid);
        update(&d, true).unwrap();
        assert!(!d.join("gone.wav.meta").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
