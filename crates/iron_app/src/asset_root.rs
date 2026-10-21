//! Where a game's assets are read from, apart from where its code is.
//!
//! A game folder has both (`dist/game.irbundle` and `assets/`), so the asset root is the game
//! folder. A packaged exe (`iron-build package`) carries only the code, which `payload.rs`
//! extracts to a cache folder; its assets are the `data/` folder beside the exe (or another one
//! named by `--data` / `$IRON_DATA`). Every asset read goes through an [`AssetRoot`], the seam
//! where asset packs can mount later (`docs/research/iron-asset-packs.md`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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

    /// The file for `relative`, if it stays inside the root (`iron_core::resolve_asset`).
    #[allow(dead_code)] // the seam for packs: call sites still pass `path()` to the old helpers
    pub fn resolve(&self, relative: &str) -> Option<PathBuf> {
        iron_core::resolve_asset(&self.dir, relative)
    }
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

/// What `iron_app` needs to decide where a packaged game's assets are.
pub struct DataSearch<'a> {
    /// `--data <dir>`.
    pub flag: Option<&'a Path>,
    /// `$IRON_DATA`.
    pub env: Option<&'a OsStr>,
    /// The folder the exe is in.
    pub exe_dir: Option<&'a Path>,
    /// The extracted payload (the code folder), which holds the payload's `project.json`.
    pub code_dir: &'a Path,
}

/// The data folder of a packaged game. Order: `--data`, `$IRON_DATA`, `<exe dir>/<dataDir>`
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
            Err(format!("IRON_DATA {}: not a folder", p.display()))
        };
    }
    let user = iron_core::project::SystemUser;
    let project = iron_core::project::Project::load(
        s.code_dir,
        &iron_core::project::LoadOptions::new(iron_core::project::Platform::current(), &user),
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
         `iron-build package` wrote beside the program, or pass --data <folder> or set IRON_DATA.",
        beside
            .map(|b| format!("`{}`", b.display()))
            .unwrap_or_else(|| format!("`{name}` beside the program"))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("iron_app_assetroot_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn code_dir(d: &Path, required: bool, data_dir: Option<&str>) -> PathBuf {
        let code = d.join("code");
        std::fs::create_dir_all(&code).unwrap();
        let mut project = serde_json::json!({ "resolved": true, "dataRequired": required });
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
        assert!(e.contains("IRON_DATA") && e.contains("gone"), "{e}");
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
            e.contains("game-data") && e.contains("--data") && e.contains("IRON_DATA"),
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
        assert_eq!(css_base(code, &code.join("dist/game.css")), "dist");
        assert_eq!(css_base(code, Path::new("/elsewhere/game.css")), "");
        let d = temp("resolve");
        std::fs::write(d.join("a.png"), b"x").unwrap();
        let root = AssetRoot::new(&d);
        assert_eq!(root.path(), d);
        assert!(root.resolve("a.png").is_some());
        assert!(root.resolve("../a.png").is_none());
    }
}
