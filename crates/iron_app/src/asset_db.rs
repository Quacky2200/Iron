//! The game's asset database: every `<file>.meta` in the game folder (written by
//! `scripts/assets.ts`), giving assets GUIDs and importer settings. Scripts get it as
//! `__iron_info.assets` (`[{ path, guid, importer }]`) for `Assets.guidToPath` and
//! importer defaults.

use std::path::Path;

use serde_json::{Value, json};

/// Every asset with a `.meta` under `root`, sorted by path: `[{ path, guid, importer }]`.
/// Builds, packages, code and hidden folders are skipped, as are unreadable metas.
pub fn scan(root: &Path) -> Value {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a: &Value, b: &Value| a["path"].as_str().cmp(&b["path"].as_str()));
    Value::Array(out)
}

/// [`scan`] for a game folder plus the packs mounted: every pack entry whose GUID came from a
/// `.meta` file (the others have a GUID made from the path, which scripts have no use for).
pub fn scan_vfs(vfs: &iron_pack::Vfs, root: &Path) -> Value {
    let mut out: Vec<Value> = match vfs.root() {
        Some(_) => scan(root).as_array().cloned().unwrap_or_default(),
        None => Vec::new(),
    };
    for path in vfs.paths() {
        let Some(info) = vfs.info(&path) else {
            continue;
        };
        let Some(guid) = info.guid else { continue };
        if info.source.starts_with("dir:")
            || guid == iron_pack::Guid::from_path(&path)
            || out
                .iter()
                .any(|a| a["path"].as_str() == Some(path.as_str()))
        {
            continue;
        }
        let importer = serde_json::from_str::<Value>(&info.meta).unwrap_or(json!({}));
        out.push(json!({
            "path": path,
            "guid": guid.to_hex(),
            "importer": if importer.is_object() { importer } else { json!({}) },
        }));
    }
    out.sort_by(|a: &Value, b: &Value| a["path"].as_str().cmp(&b["path"].as_str()));
    Value::Array(out)
}

/// Every asset the game can load, as `[[path, pack]]` (`""` for a loose file): what `Assets.exists`,
/// `list` and `packOf` answer from.
pub fn index(vfs: &iron_pack::Vfs) -> Value {
    Value::Array(
        vfs.paths()
            .into_iter()
            .map(|p| {
                let pack = vfs.pack_of(&p);
                json!([p, pack])
            })
            .collect(),
    )
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Value>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            if !matches!(name.as_str(), "dist" | "node_modules" | "src" | "target") {
                walk(root, &path, out);
            }
            continue;
        }
        let Some(asset) = name.strip_suffix(".meta") else {
            continue;
        };
        let Some(meta) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        else {
            tracing::warn!("{}: not a readable .meta file", path.display());
            continue;
        };
        let Some(guid) = meta["guid"].as_str() else {
            continue;
        };
        let relative = path
            .with_file_name(asset)
            .strip_prefix(root)
            .map(|p| crate::reload::asset_key(&p.to_string_lossy()))
            .unwrap_or_default();
        out.push(json!({
            "path": relative,
            "guid": guid,
            "importer": meta.get("importer").cloned().unwrap_or(json!({})),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-AST-04
    #[test]
    fn metas_give_paths_guids_and_importers() {
        let dir = std::env::temp_dir().join(format!("iron-assetdb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets/tex")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::write(
            dir.join("assets/tex/a.png.meta"),
            r#"{"guid":"aa11","importer":{"type":"texture","filterMode":"point"}}"#,
        )
        .unwrap();
        std::fs::write(dir.join("assets/b.glb.meta"), r#"{"guid":"bb22"}"#).unwrap();
        std::fs::write(dir.join("assets/broken.png.meta"), "not json").unwrap();
        std::fs::write(dir.join("node_modules/x/c.png.meta"), r#"{"guid":"cc"}"#).unwrap();
        let db = scan(&dir);
        assert_eq!(
            db,
            json!([
                { "path": "assets/b.glb", "guid": "bb22", "importer": {} },
                { "path": "assets/tex/a.png", "guid": "aa11", "importer": { "type": "texture", "filterMode": "point" } },
            ])
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
