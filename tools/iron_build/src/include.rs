//! Which files of a game folder a packaged build takes: `include` / `exclude` of `project.json`
//! (paths, folders, globs), the dynamic path families the code builds, and what the code names.
//! Shared by `iron-build build` (the report and `dist/assets.plan`) and `package`.
//! Plan: `docs/research/iron-virtual-fs-plan.md` sections 2.5 and 2.6.

use crate::assets;
use iron_core::glob::{glob_match, include_match, is_glob};
use std::path::Path;

/// Why a file is taken into a packaged build, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// Named by the code (a whole literal, a CSS `url()`), or the start scene.
    Code,
    /// Reached by a dynamic path the code builds.
    Pattern,
    /// Selected by `include` (the default is `assets/**`).
    Include,
}

/// The default `include`: the whole assets folder.
pub const DEFAULT_INCLUDE: &str = "assets/**";

/// Every file under the game folder (relative, `/`-separated, sorted), without dot names and the
/// top-level `dist`, `node_modules` and `target`.
pub fn game_files(game: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, top: bool, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.')
                || (top && matches!(name.as_str(), "dist" | "node_modules" | "target"))
            {
                continue;
            }
            let path = e.path();
            if path.is_dir() {
                walk(base, &path, false, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(
                    rel.components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(game, game, true, &mut out);
    out.sort();
    out
}

/// What `include` / `exclude` select.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Sorted, distinct, no `.meta` files.
    pub files: Vec<String>,
    /// `include` entries that matched no file (empty when `include` is the default).
    pub unmatched: Vec<String>,
}

/// The files `include` (default [`DEFAULT_INCLUDE`]) selects among `all`, minus `exclude`.
/// An entry that names a folder or a file and not a glob keeps its meaning (the folder's files).
pub fn select(all: &[String], include: Option<&[String]>, exclude: Option<&[String]>) -> Selection {
    let default = [DEFAULT_INCLUDE.to_owned()];
    let explicit = include.is_some();
    let include = include.unwrap_or(&default);
    let mut files = Vec::new();
    let mut unmatched = Vec::new();
    for pattern in include {
        let before = files.len();
        for f in all {
            if !f.ends_with(".meta") && include_match(pattern, f) {
                files.push(f.clone());
            }
        }
        if files.len() == before && explicit {
            unmatched.push(pattern.clone());
        }
    }
    let ex = exclude.unwrap_or(&[]);
    files.retain(|f| !ex.iter().any(|x| include_match(x, f)));
    files.sort();
    files.dedup();
    Selection { files, unmatched }
}

/// A dynamic path family found in the code and the asset files it can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    pub glob: String,
    /// `src/file.ts:line`.
    pub at: String,
    pub matches: Vec<String>,
}

/// Whether a glob ends in a file name that may be an asset (`*`, or an asset extension).
pub fn asset_looking_glob(glob: &str) -> bool {
    let last = glob.rsplit('/').next().unwrap_or("");
    last.rsplit_once('.').is_some_and(|(_, e)| {
        e == "*" || assets::kind(&e.to_lowercase()).is_some() && !e.contains('*')
    })
}

/// The asset files of `all` that `glob` reaches.
pub fn match_pattern(all: &[String], glob: &str) -> Vec<String> {
    all.iter()
        .filter(|f| assets::is_asset(f) && glob_match(glob, f))
        .cloned()
        .collect()
}

/// A whole string literal that looks like an asset path (a folder, an asset extension, no glob).
pub fn asset_looking_literal(lit: &str) -> bool {
    !is_glob(lit)
        && lit.contains('/')
        && !lit.contains("://")
        && !lit.starts_with(['/', '\\', '.'])
        && !lit.ends_with(".meta")
        && lit
            .rsplit_once('.')
            .is_some_and(|(_, e)| e != "json" && assets::kind(&e.to_lowercase()).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    // M9-TS-2552
    #[test]
    fn include_takes_folders_files_and_globs_and_exclude_removes() {
        let all = v(&[
            "assets/a.png",
            "assets/a.png.meta",
            "assets/sfx/x.wav",
            "assets/sfx/y.mp3",
            "assets/wip/z.psd",
            "levels/l1.scene.json",
            "extra.txt",
        ]);
        let s = select(&all, None, None);
        assert_eq!(s.files.len(), 4);
        assert!(s.unmatched.is_empty());
        let inc = v(&[
            "assets/sfx/*.wav",
            "levels",
            "extra.txt",
            "assets/nope/*.png",
        ]);
        let s = select(&all, Some(&inc), None);
        assert_eq!(
            s.files,
            ["assets/sfx/x.wav", "extra.txt", "levels/l1.scene.json"]
        );
        assert_eq!(s.unmatched, ["assets/nope/*.png"]);
        let ex = v(&["**/*.psd", "assets/sfx"]);
        let s = select(&all, None, Some(&ex));
        assert_eq!(s.files, ["assets/a.png"]);
    }

    // M9-TS-2552
    #[test]
    fn globs_and_literals_are_judged_by_their_file_names() {
        assert!(asset_looking_glob("assets/emoji/*.svg"));
        assert!(asset_looking_glob("assets/sfx/*.*"));
        assert!(!asset_looking_glob("src/*.ts"));
        assert!(asset_looking_literal("assets/a.png"));
        assert!(!asset_looking_literal("assets/*.png"));
        assert!(!asset_looking_literal("https://x.com/a.png"));
        assert!(!asset_looking_literal("a.png"));
        assert!(!asset_looking_literal("data/save.json"));
        let all = v(&["assets/s/a.wav", "assets/s/a.wav.meta", "assets/s/b.txt"]);
        assert_eq!(match_pattern(&all, "assets/s/*.*"), ["assets/s/a.wav"]);
    }
}
