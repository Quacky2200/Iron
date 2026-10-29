// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Asset `.meta` files: every asset in a game folder gets `<file>.meta` next to it holding a GUID
//! and its importer settings. `platypus-build assets` writes them for a whole folder; the editor
//! writes one for each file it makes or copies in ([`ensure`]).

use std::path::Path;

/// Asset kind by extension (lowercase), `None` for files that are not assets.
pub fn kind(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "png" | "jpg" | "jpeg" | "svg" | "hdr" => "texture",
        "glb" | "gltf" | "bin" | "fbx" => "model",
        "wav" | "ogg" | "mp3" | "flac" => "audio",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "json" => "data",
        _ => return None,
    })
}

/// Files that look like assets but are a project's own settings.
const NOT_ASSETS: &[&str] = &[
    "package.json",
    "project.json",
    "tsconfig.json",
    "bun.lock",
    "bun.lockb",
];

/// Whether a file (a path or a name) is an asset that gets a `.meta`.
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

/// The GUID in a `.meta` file.
pub fn read_guid(meta: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(meta).ok()?).ok()?;
    v["guid"].as_str().map(String::from)
}

/// Gives the asset at `path` a `.meta` file if it has none (and is an asset). Returns its GUID
/// ("" for a file that isn't an asset).
pub fn ensure(path: &Path) -> std::io::Result<String> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if !is_asset(&name) {
        return Ok(String::new());
    }
    let mut meta_name = path.file_name().unwrap_or_default().to_os_string();
    meta_name.push(".meta");
    let meta = path.with_file_name(meta_name);
    if let Some(guid) = read_guid(&meta) {
        return Ok(guid);
    }
    let guid = new_guid();
    std::fs::write(&meta, default_meta(&name, &guid))?;
    Ok(guid)
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-SCN-12
    #[test]
    fn an_asset_gets_one_meta_and_keeps_its_guid() {
        let dir = std::env::temp_dir().join(format!("platypus-meta-{}", new_guid()));
        std::fs::create_dir_all(&dir).unwrap();
        let scene = dir.join("level.scene.json");
        std::fs::write(&scene, "{}").unwrap();
        let guid = ensure(&scene).unwrap();
        assert_eq!(guid.len(), 32);
        assert_eq!(ensure(&scene).unwrap(), guid, "kept");
        assert!(dir.join("level.scene.json.meta").is_file());
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        assert_eq!(ensure(&dir.join("notes.txt")).unwrap(), "", "not an asset");
        assert!(is_asset("tree.FBX") && kind("fbx") == Some("model"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
