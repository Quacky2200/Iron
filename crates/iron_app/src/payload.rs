// SPDX-License-Identifier: Apache-2.0

//! A single-file game: when this executable carries a payload (`iron-build package`, format in
//! `iron_core::payload`), the game's *code* (bundle, CSS, `project.json`, icon, and a few tiny
//! essentials) is extracted once to a per-user cache folder and the game runs from there.
//! The assets are not in the payload: they are the `data/` folder beside the exe
//! (`asset_root.rs` finds it).
//!
//! Why a folder and not an in-memory file system: textures, models, sounds, fonts, scenes
//! and CSS are read from disk in over a hundred places (and by the asset watcher and the
//! cooker); one real folder keeps every one of them unchanged. The folder is keyed by the
//! payload's content hash, so a new build of the game extracts again and an unchanged one
//! starts at once. Older folders of the same game are removed.

use std::path::{Path, PathBuf};

use iron_core::payload::Archive;

/// A file in the cache folder that says the extraction finished.
const DONE: &str = ".iron-extracted";

/// The folder of the embedded game, extracting it first if needed; `None` when this
/// executable has no payload (it is the plain engine host).
pub fn embedded_game() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    match prepare(&exe, &cache_root()?) {
        Ok(dir) => dir,
        Err(e) => {
            // No window yet and no log: stderr is all there is.
            eprintln!("iron: the embedded game could not be unpacked: {e}");
            None
        }
    }
}

/// `exe`'s payload extracted under `cache`: `Ok(None)` without one.
pub fn prepare(exe: &Path, cache: &Path) -> Result<Option<PathBuf>, String> {
    let Some(mut archive) = Archive::open(exe)? else {
        return Ok(None);
    };
    let stem = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game".into());
    let prefix = format!("{}-", folder_safe(&stem));
    let dir = cache.join(format!("{prefix}{:016x}", archive.id()));
    if !dir.join(DONE).is_file() {
        // Extract beside, then rename: a crash or a second instance never sees half a game.
        let tmp = cache.join(format!("{prefix}tmp{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        archive.extract(&tmp)?;
        std::fs::write(tmp.join(DONE), b"").map_err(|e| e.to_string())?;
        if std::fs::rename(&tmp, &dir).is_err() && !dir.join(DONE).is_file() {
            return Err(format!("could not create {}", dir.display()));
        }
        let _ = std::fs::remove_dir_all(&tmp);
        remove_old(cache, &prefix, &dir);
    }
    Ok(Some(dir))
}

/// Other folders of the same game (an older build), not the one in use.
fn remove_old(cache: &Path, prefix: &str, keep: &Path) {
    let Ok(read) = std::fs::read_dir(cache) else {
        return;
    };
    for e in read.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        // `prefix` + 16 hex digits only: another game named `a-b` has the prefix `a-b-`.
        let hash = name.strip_prefix(prefix).unwrap_or("");
        if p != keep && hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            let _ = std::fs::remove_dir_all(&p);
        }
    }
}

fn folder_safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_ .".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `<per-user cache>/Iron/games`.
fn cache_root() -> Option<PathBuf> {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let base = if cfg!(target_os = "windows") {
        env("LOCALAPPDATA")?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Caches")
    } else {
        env("XDG_CACHE_HOME").or_else(|| Some(env("HOME")?.join(".cache")))?
    };
    let dir = base.join("Iron").join("games");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::payload::write_payload;

    fn exe_with(dir: &Path, name: &str, files: &[(String, Vec<u8>)]) -> PathBuf {
        let path = dir.join(name);
        let mut bytes = b"MZ not a real exe".to_vec();
        write_payload(&mut bytes, files).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_app_payload_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // M9-API-1010
    #[test]
    fn an_exe_without_a_payload_is_the_plain_host() {
        let d = temp("plain");
        let exe = d.join("iron_app.exe");
        std::fs::write(&exe, vec![0u8; 4096]).unwrap();
        assert_eq!(prepare(&exe, &d).unwrap(), None);
    }

    // M9-API-1011
    #[test]
    fn the_payload_is_extracted_once_and_old_builds_are_removed() {
        let d = temp("extract");
        let cache = d.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let v1 = [
            ("dist/game.irbundle".to_string(), b"one".to_vec()),
            ("assets/a.png".to_string(), vec![1; 100]),
        ];
        let exe = exe_with(&d, "game.exe", &v1);
        let dir = prepare(&exe, &cache).unwrap().unwrap();
        assert_eq!(
            std::fs::read(dir.join("dist/game.irbundle")).unwrap(),
            b"one"
        );
        assert_eq!(
            std::fs::read(dir.join("assets/a.png")).unwrap(),
            vec![1; 100]
        );
        // Again: the same folder, untouched.
        std::fs::write(dir.join("marker"), b"x").unwrap();
        assert_eq!(prepare(&exe, &cache).unwrap().unwrap(), dir);
        assert!(dir.join("marker").exists());
        // A new build: a new folder, and the old one is gone. Another game's folder stays.
        let other = cache.join("game-extra-0123456789abcdef");
        std::fs::create_dir_all(&other).unwrap();
        let v2 = [("dist/game.irbundle".to_string(), b"two".to_vec())];
        let exe2 = exe_with(&d, "game.exe", &v2);
        let dir2 = prepare(&exe2, &cache).unwrap().unwrap();
        assert_ne!(dir, dir2);
        assert!(!dir.exists());
        assert!(other.exists());
    }
}
