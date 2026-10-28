// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Where a game's assets are read from, apart from where its code is.
//!
//! A game folder has both (`build/game.pbundle` and `assets/`), so the asset root is the game
//! folder. A packaged exe (`platypus-build package`) carries only the code, which `payload.rs`
//! extracts to a cache folder; its assets are the `data/` folder beside the exe (or another one
//! named by `--data` / `$PLATYPUS_DATA`). Every asset read goes through the [`Vfs`] that [`mount`]
//! builds over that folder: its packs (`index.json`, `assets.dat`, `scene-*.dat`) when it has
//! them, else the folder itself (`docs/research/platypus-virtual-fs-plan.md`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use platypus_pack::{DirSource, PackSource, Vfs};

/// A folder asset paths (`assets/a.png`) are resolved against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRoot {
    dir: PathBuf,
}

impl AssetRoot {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// The file for `relative`, if it stays inside the root (`platypus_core::resolve_asset`).
    #[allow(dead_code)] // only the tests ask for a file name
    pub fn resolve(&self, relative: &str) -> Option<PathBuf> {
        platypus_core::resolve_asset(&self.dir, relative)
    }
}

/// The packs a data folder holds, in mount order (bottom first): `assets.dat`, then the scene
/// packs. From `index.json` when it is there, else any `assets.dat` / `scene-*.dat` found.
fn pack_files(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let index = dir.join("index.json");
    if let Ok(text) = std::fs::read_to_string(&index) {
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", index.display()))?;
        for p in json["packs"].as_array().into_iter().flatten() {
            let (Some(name), Some(file)) = (p["name"].as_str(), p["file"].as_str()) else {
                continue;
            };
            // A pack named by the manifest stays inside the data folder.
            if file.contains(['/', '\\']) || file.starts_with('.') {
                return Err(format!(
                    "{}: pack file {file:?} is not a plain name",
                    index.display()
                ));
            }
            found.push((name.to_owned(), dir.join(file)));
        }
    } else if let Ok(read) = std::fs::read_dir(dir) {
        for e in read.flatten() {
            let file = e.file_name().to_string_lossy().into_owned();
            if let Some(name) = file.strip_suffix(".dat")
                && (name == "assets" || name.starts_with("scene-"))
            {
                found.push((name.to_owned(), e.path()));
            }
        }
        found.sort();
    }
    // `assets.dat` at the bottom, whatever the manifest's order.
    found.sort_by_key(|(name, _)| name != "assets");
    Ok(found)
}

/// The file system of a game: the packs of `dir` (all of them, at start, so a script can load a
/// path only another scene's pack holds), and the folder itself when it has no packs (a game
/// folder in the dev loop, or a loose `data/`). Lookups go from the last mounted down: scene
/// packs, then `assets.dat`, then the folder.
pub fn mount(dir: &Path) -> Result<Vfs, String> {
    let packs = pack_files(dir)?;
    let mut vfs = Vfs::new();
    if packs.is_empty() {
        vfs.mount(Arc::new(DirSource::new(dir)));
        return Ok(vfs);
    }
    for (name, path) in packs {
        let source =
            PackSource::open(&name, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        tracing::info!(
            "mounted {} ({} assets)",
            path.display(),
            source.pack().len()
        );
        vfs.mount(Arc::new(source));
    }
    Ok(vfs)
}

/// The folder `css_path`'s directory is, relative to the code folder, `/`-separated (what `url()`s in the
/// CSS are rebased onto); empty when it is not inside it.
pub fn css_base(code_root: &Path, css_path: &Path) -> String {
    css_path
        .parent()
        .and_then(|dir| dir.strip_prefix(code_root).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

/// What `platypus_app` needs to decide where a packaged game's assets are.
pub struct DataSearch<'a> {
    /// `--data <dir>`.
    pub flag: Option<&'a Path>,
    /// `$PLATYPUS_DATA`.
    pub env: Option<&'a OsStr>,
    /// The folder the exe is in.
    pub exe_dir: Option<&'a Path>,
    /// The extracted payload (the code folder), which holds the payload's `project.json`.
    pub code_dir: &'a Path,
}

/// The data folder of a packaged game. Order: `--data`, `$PLATYPUS_DATA`, `<exe dir>/<dataDir>`
/// (`dataDir` of the payload's `project.json`, default `data`). A flag or variable that names
/// a missing folder is an error (it is a request, not a guess). When none is found, a game that
/// does not need data (`dataRequired` false: no assets, or only the embedded essentials)
/// runs from the extracted folder; any other is an error that names every folder looked in.
pub fn locate_data(s: &DataSearch<'_>) -> Result<PathBuf, String> {
    if let Some(p) = s.flag {
        return if p.is_dir() {
            Ok(p.to_path_buf())
        } else {
            Err(format!("--data {}: not a folder", p.display()))
        };
    }
    if let Some(p) = s.env.filter(|v| !v.is_empty()) {
        let p = PathBuf::from(p);
        return if p.is_dir() {
            Ok(p)
        } else {
            Err(format!("PLATYPUS_DATA {}: not a folder", p.display()))
        };
    }
    let user = platypus_core::project::SystemUser;
    let project = platypus_core::project::Project::load(
        s.code_dir,
        &platypus_core::project::LoadOptions::new(
            platypus_core::project::Platform::current(),
            &user,
        ),
    )
    .ok();
    let name = project
        .as_ref()
        .and_then(|p| p.data_dir.as_deref())
        .filter(|n| !n.is_empty() && !n.contains(['/', '\\']) && !n.starts_with('.'))
        .unwrap_or("data");
    let beside = s.exe_dir.map(|d| d.join(name));
    if let Some(b) = beside.as_ref().filter(|b| b.is_dir()) {
        return Ok(b.clone());
    }
    if project.as_ref().and_then(|p| p.data_required) == Some(false) {
        return Ok(s.code_dir.to_path_buf());
    }
    Err(format!(
        "the game's data folder was not found (looked for {}). Put the `{name}` folder that \
         `platypus-build package` wrote beside the program, or pass --data <folder> or set PLATYPUS_DATA.",
        beside
            .map(|b| format!("`{}`", b.display()))
            .unwrap_or_else(|| format!("`{name}` beside the program"))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "platypus_app_assetroot_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn code_dir(d: &Path, required: bool, data_dir: Option<&str>) -> PathBuf {
        let code = d.join("code");
        std::fs::create_dir_all(&code).unwrap();
        let mut project =
            serde_json::json!({ "resolved": true, "name": "g", "dataRequired": required });
        if let Some(n) = data_dir {
            project["dataDir"] = n.into();
        }
        std::fs::write(code.join("project.json"), project.to_string()).unwrap();
        code
    }

    // M9-API-1200
    #[test]
    fn the_flag_then_the_variable_then_the_folder_beside_the_exe() {
        let d = temp("order");
        let code = code_dir(&d, true, None);
        let exe_dir = d.join("bin");
        let (flag, env, beside) = (d.join("flag"), d.join("env"), exe_dir.join("data"));
        for p in [&flag, &env, &beside] {
            std::fs::create_dir_all(p).unwrap();
        }
        let find = |flag: Option<&Path>, env: Option<&OsStr>| {
            locate_data(&DataSearch {
                flag,
                env,
                exe_dir: Some(&exe_dir),
                code_dir: &code,
            })
        };
        assert_eq!(find(Some(&flag), Some(env.as_os_str())).unwrap(), flag);
        assert_eq!(find(None, Some(env.as_os_str())).unwrap(), env);
        assert_eq!(find(None, None).unwrap(), beside);
        // An empty variable is ignored.
        assert_eq!(find(None, Some(OsStr::new(""))).unwrap(), beside);
    }

    // M9-API-1201
    #[test]
    fn a_missing_flag_or_variable_folder_is_an_error_not_a_fallback() {
        let d = temp("missing");
        let code = code_dir(&d, false, None);
        let exe_dir = d.join("bin");
        std::fs::create_dir_all(exe_dir.join("data")).unwrap();
        let gone = d.join("gone");
        let s = |flag, env| DataSearch {
            flag,
            env,
            exe_dir: Some(&exe_dir),
            code_dir: &code,
        };
        let e = locate_data(&s(Some(&gone), None)).unwrap_err();
        assert!(e.contains("--data") && e.contains("gone"), "{e}");
        let e = locate_data(&s(None, Some(gone.as_os_str()))).unwrap_err();
        assert!(e.contains("PLATYPUS_DATA") && e.contains("gone"), "{e}");
    }

    // M9-API-1202
    #[test]
    fn without_a_data_folder_a_game_that_needs_one_gets_a_clear_error() {
        let d = temp("error");
        let code = code_dir(&d, true, Some("game-data"));
        let exe_dir = d.join("bin");
        std::fs::create_dir_all(exe_dir.join("data")).unwrap(); // the wrong name does not count
        let e = locate_data(&DataSearch {
            flag: None,
            env: None,
            exe_dir: Some(&exe_dir),
            code_dir: &code,
        })
        .unwrap_err();
        assert!(
            e.contains("game-data") && e.contains("--data") && e.contains("PLATYPUS_DATA"),
            "{e}"
        );
        assert!(e.contains(&*exe_dir.display().to_string()), "{e}");
    }

    // M9-API-1203
    #[test]
    fn a_game_that_needs_no_data_runs_from_the_extracted_folder() {
        let d = temp("nodata");
        let code = code_dir(&d, false, None);
        let got = locate_data(&DataSearch {
            flag: None,
            env: None,
            exe_dir: Some(&d.join("bin")),
            code_dir: &code,
        })
        .unwrap();
        assert_eq!(got, code);
        // A payload without metadata (an older exe) needs the folder.
        let old = d.join("old");
        std::fs::create_dir_all(&old).unwrap();
        assert!(
            locate_data(&DataSearch {
                flag: None,
                env: None,
                exe_dir: None,
                code_dir: &old,
            })
            .is_err()
        );
    }

    // M9-API-1204
    #[test]
    fn css_urls_rebase_onto_the_code_folder_and_assets_resolve_inside_the_root() {
        let code = Path::new("/c/code");
        assert_eq!(css_base(code, &code.join("build/game.css")), "build");
        assert_eq!(css_base(code, Path::new("/elsewhere/game.css")), "");
        let d = temp("resolve");
        std::fs::write(d.join("a.png"), b"x").unwrap();
        let root = AssetRoot::new(&d);
        assert_eq!(root.path(), d);
        assert!(root.resolve("a.png").is_some());
        assert!(root.resolve("../a.png").is_none());
    }

    // M9-API-1205
    #[test]
    fn a_data_folder_mounts_its_packs_assets_first_or_falls_back_to_the_folder() {
        use platypus_pack::{EntrySpec, PackWriter, WriteOptions};
        let d = temp("mount");
        let write = |file: &str, entries: &[(&str, &[u8])]| {
            PackWriter::write_file(&d.join(file), WriteOptions::default(), |w| {
                for (p, b) in entries {
                    w.add(&EntrySpec::new(p, b))?;
                }
                Ok(())
            })
            .unwrap();
        };
        // Loose files only: the folder answers.
        std::fs::write(d.join("a.txt"), b"loose").unwrap();
        let vfs = mount(&d).unwrap();
        assert_eq!(&*vfs.read("a.txt").unwrap(), b"loose");
        assert_eq!(vfs.pack_of("a.txt"), "");
        // With packs, the manifest's order does not matter: scene packs sit above assets.dat.
        write(
            "assets.dat",
            &[("assets/x.txt", b"assets"), ("assets/y.txt", b"y")],
        );
        write("scene-one.dat", &[("assets/x.txt", b"scene one")]);
        std::fs::write(
            d.join("index.json"),
            r#"{"format":1,"packs":[
                {"name":"scene-one","file":"scene-one.dat","role":"scene"},
                {"name":"assets","file":"assets.dat","role":"global"}]}"#,
        )
        .unwrap();
        let vfs = mount(&d).unwrap();
        assert_eq!(&*vfs.read("assets/x.txt").unwrap(), b"scene one");
        assert_eq!(&*vfs.read("assets/y.txt").unwrap(), b"y");
        assert_eq!(vfs.pack_of("assets/y.txt"), "assets");
        // The loose file beside the packs is not an asset any more.
        assert!(vfs.read("a.txt").is_err());
        // No manifest: the packs are found by name.
        std::fs::remove_file(d.join("index.json")).unwrap();
        let vfs = mount(&d).unwrap();
        assert_eq!(&*vfs.read("assets/x.txt").unwrap(), b"scene one");
        // A manifest that names a path outside the folder, or a pack that is not there, fails.
        std::fs::write(
            d.join("index.json"),
            r#"{"packs":[{"name":"assets","file":"../assets.dat"}]}"#,
        )
        .unwrap();
        assert!(mount(&d).is_err());
        std::fs::write(
            d.join("index.json"),
            r#"{"packs":[{"name":"assets","file":"gone.dat"}]}"#,
        )
        .unwrap();
        assert!(mount(&d).unwrap_err().contains("gone.dat"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
