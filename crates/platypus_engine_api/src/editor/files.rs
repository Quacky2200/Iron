// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The editor's other file operations on the project folder: read and write text (writes go to a
//! temporary file first, then replace the old one, so a crash never leaves half a file), look a
//! path up, make folders, rename in place, and delete into the project's own trash
//! (`.platypus/trash`, from where an undo restores it). Paths are checked like the others in
//! [`super::project`]. An asset's `.meta` file (its GUID) moves, renames and goes to the trash with it.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::project::{relative, resolve};

/// The biggest text file `read_text` reads.
const READ_LIMIT: u64 = 64 << 20;

/// Where deleted items go, inside the project.
pub const TRASH: &str = ".platypus/trash";

/// What [`stat`] says about a path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stat {
    pub exists: bool,
    pub dir: bool,
    /// Bytes (0 for folders).
    pub size: u64,
    /// Last modified, milliseconds since 1970 (0 when unknown).
    pub modified_ms: f64,
}

/// The `.meta` file beside an asset.
fn meta_of(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".meta");
    path.with_file_name(name)
}

/// Moves `from` to `to`, and its `.meta` file with it when it has one.
pub(super) fn move_with_meta(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)?;
    let meta = meta_of(from);
    if meta.is_file() {
        std::fs::rename(&meta, meta_of(to))?;
    }
    Ok(())
}

/// A project file's text.
pub fn read_text(root: &Path, rel: &str) -> Result<String, String> {
    let path = resolve(root, rel)?;
    let meta = std::fs::metadata(&path).map_err(|e| format!("can't read {rel:?}: {e}"))?;
    if meta.is_dir() {
        return Err(format!("{rel:?} is a folder"));
    }
    if meta.len() > READ_LIMIT {
        return Err(format!("{rel:?} is too big to read as text"));
    }
    std::fs::read_to_string(&path).map_err(|e| format!("can't read {rel:?}: {e}"))
}

/// Writes a project file (making its folders): the text goes to a temporary file beside it,
/// which then replaces the old file in one step.
pub fn write_text(root: &Path, rel: &str, text: &str) -> Result<(), String> {
    let path = resolve(root, rel)?;
    if path == root || path.is_dir() {
        return Err(format!("{rel:?} is a folder"));
    }
    let dir = path.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("can't make the folder of {rel:?}: {e}"))?;
    // The folders made may not lead outside either (a symlinked parent).
    resolve(root, rel)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, text).map_err(|e| format!("can't write {rel:?}: {e}"))?;
    std::fs::rename(&temp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("can't write {rel:?}: {e}")
    })
}

/// Whether a project path exists (false for paths outside the project).
pub fn exists(root: &Path, rel: &str) -> bool {
    resolve(root, rel).is_ok_and(|p| p.exists())
}

/// What a project path is: whether it exists, is a folder, its size and modification time.
pub fn stat(root: &Path, rel: &str) -> Result<Stat, String> {
    let path = resolve(root, rel)?;
    let Ok(meta) = std::fs::metadata(&path) else {
        return Ok(Stat::default());
    };
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0);
    Ok(Stat {
        exists: true,
        dir: meta.is_dir(),
        size: if meta.is_dir() { 0 } else { meta.len() },
        modified_ms,
    })
}

/// A path in project folder `dir` that isn't taken: `dir/name`, else `dir/stem (2).ext`, ...
pub fn free_name(root: &Path, dir: &str, name: &str) -> Result<String, String> {
    check_name(name)?;
    let folder = resolve(root, dir)?;
    Ok(relative(root, &super::project::free_name(&folder, name)))
}

/// Makes a project folder (and the folders above it); refuses one that exists.
pub fn create_dir(root: &Path, rel: &str) -> Result<String, String> {
    let path = resolve(root, rel)?;
    if path.exists() {
        return Err(format!("{rel:?} already exists"));
    }
    std::fs::create_dir_all(&path).map_err(|e| format!("can't make {rel:?}: {e}"))?;
    Ok(relative(root, &path))
}

/// A file name on its own: no separators, not `.`/`..`, not empty.
fn check_name(name: &str) -> Result<(), String> {
    let bad = name.trim().is_empty()
        || name.contains(['/', '\\', ':'])
        || name == "."
        || name == ".."
        || name.trim() != name;
    if bad {
        Err(format!("{name:?} isn't a usable name"))
    } else {
        Ok(())
    }
}

/// Renames a project file or folder in its folder (with its `.meta`); returns the new path.
/// Refuses a name that is taken.
pub fn rename(root: &Path, rel: &str, name: &str) -> Result<String, String> {
    check_name(name)?;
    let from = resolve(root, rel)?;
    if from == root || !from.exists() {
        return Err(format!("{rel:?} can't be renamed"));
    }
    let to = from.with_file_name(name);
    if to == from {
        return Ok(relative(root, &to));
    }
    // A change of case only is allowed (the same file on Windows and macOS).
    let same_file =
        from.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some(name.to_lowercase());
    if to.exists() && !same_file {
        return Err(format!("{} already exists", relative(root, &to)));
    }
    move_with_meta(&from, &to).map_err(|e| format!("renaming {rel:?}: {e}"))?;
    Ok(relative(root, &to))
}

/// Moves a project file or folder (with its `.meta`) into the project's trash
/// (`.platypus/trash/<id>/`, beside a note of where it was). Returns the id [`restore`] takes.
pub fn trash(root: &Path, rel: &str) -> Result<String, String> {
    let from = resolve(root, rel)?;
    if from == root || !from.exists() {
        return Err(format!("{rel:?} can't be deleted"));
    }
    let bin = root.join(TRASH);
    if from.starts_with(&bin) || bin.starts_with(&from) {
        return Err(format!("{rel:?} holds the trash"));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let (id, slot) = (0..)
        .map(|n| {
            let id = format!("{stamp}-{n}");
            let slot = bin.join(&id);
            (id, slot)
        })
        .find(|(_, slot)| !slot.exists())
        .expect("a free slot");
    std::fs::create_dir_all(&slot).map_err(|e| format!("can't make the trash: {e}"))?;
    let name = from.file_name().ok_or("no name")?;
    std::fs::write(slot.join(".origin"), relative(root, &from))
        .map_err(|e| format!("can't write to the trash: {e}"))?;
    move_with_meta(&from, &slot.join(name)).map_err(|e| {
        let _ = std::fs::remove_dir_all(&slot);
        format!("deleting {rel:?}: {e}")
    })?;
    Ok(id)
}

/// Puts an item back from the trash where it was; returns its path. Refuses when something
/// else is there now.
pub fn restore(root: &Path, id: &str) -> Result<String, String> {
    check_name(id)?;
    let slot = root.join(TRASH).join(id);
    let origin = std::fs::read_to_string(slot.join(".origin"))
        .map_err(|_| format!("nothing in the trash under {id:?}"))?;
    let to = resolve(root, &origin)?;
    if to.exists() {
        return Err(format!("{origin} exists again; not restored"));
    }
    let name = to.file_name().ok_or("no name")?;
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("can't make {}: {e}", dir.display()))?;
    }
    move_with_meta(&slot.join(name), &to).map_err(|e| format!("restoring {origin}: {e}"))?;
    let _ = std::fs::remove_dir_all(&slot);
    Ok(origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("platypus-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // M5-EFS-02
    #[test]
    fn text_files_are_written_whole_and_read_back() {
        let root = temp("text");
        write_text(&root, "scenes/main.scene.json", "{}").unwrap();
        assert_eq!(read_text(&root, "scenes/main.scene.json").unwrap(), "{}");
        write_text(&root, "scenes/main.scene.json", "{\"a\":1}").unwrap();
        assert_eq!(
            read_text(&root, "scenes/main.scene.json").unwrap(),
            "{\"a\":1}"
        );
        // No temporary file is left beside it.
        let left: Vec<_> = std::fs::read_dir(root.join("scenes"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 1, "{left:?}");
        assert!(write_text(&root, "scenes", "x").is_err(), "a folder");
        assert!(write_text(&root, "../x.txt", "x").is_err(), "outside");
        assert!(read_text(&root, "missing.txt").is_err());
        assert!(read_text(&root, "scenes").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    // M5-EFS-03
    #[test]
    fn paths_are_looked_up_and_folders_made() {
        let root = temp("stat");
        assert!(!exists(&root, "a"));
        assert!(!exists(&root, "../a"));
        assert_eq!(create_dir(&root, "a/b").unwrap(), "a/b");
        assert!(create_dir(&root, "a/b").is_err(), "exists");
        std::fs::write(root.join("a/x.txt"), "12345").unwrap();
        let s = stat(&root, "a/x.txt").unwrap();
        assert!(s.exists && !s.dir && s.size == 5 && s.modified_ms > 0.0);
        assert!(stat(&root, "a").unwrap().dir);
        assert!(!stat(&root, "nope").unwrap().exists);
        assert!(stat(&root, "C:/x").is_err());
        assert_eq!(free_name(&root, "a", "x.txt").unwrap(), "a/x (2).txt");
        assert_eq!(free_name(&root, "a", "y.txt").unwrap(), "a/y.txt");
        assert!(free_name(&root, "a", "../y").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    // M5-EFS-04
    #[test]
    fn renames_stay_in_their_folder_and_take_the_meta_along() {
        let root = temp("rename");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets/a.png"), "png").unwrap();
        std::fs::write(root.join("assets/a.png.meta"), "guid").unwrap();
        std::fs::write(root.join("assets/b.png"), "").unwrap();
        assert_eq!(
            rename(&root, "assets/a.png", "c.png").unwrap(),
            "assets/c.png"
        );
        assert!(root.join("assets/c.png.meta").is_file());
        assert!(!root.join("assets/a.png.meta").exists());
        assert!(rename(&root, "assets/c.png", "b.png").is_err(), "taken");
        for bad in ["", "x/y", "..", "a\\b", " c.png"] {
            assert!(rename(&root, "assets/c.png", bad).is_err(), "{bad:?}");
        }
        assert_eq!(
            rename(&root, "assets/c.png", "C.png").unwrap(),
            "assets/C.png"
        );
        assert!(rename(&root, "", "x").is_err(), "the root");
        let _ = std::fs::remove_dir_all(&root);
    }

    // M5-EFS-05
    #[test]
    fn deleting_goes_to_the_project_trash_and_restores() {
        let root = temp("trash");
        std::fs::create_dir_all(root.join("assets/sub")).unwrap();
        std::fs::write(root.join("assets/a.png"), "png").unwrap();
        std::fs::write(root.join("assets/a.png.meta"), "guid").unwrap();
        std::fs::write(root.join("assets/sub/b.txt"), "b").unwrap();
        let id = trash(&root, "assets/a.png").unwrap();
        assert!(!root.join("assets/a.png").exists());
        assert!(!root.join("assets/a.png.meta").exists());
        let slot = root.join(TRASH).join(&id);
        assert!(slot.join("a.png").is_file() && slot.join("a.png.meta").is_file());
        let folder = trash(&root, "assets/sub").unwrap();
        assert_ne!(id, folder);
        assert_eq!(restore(&root, &id).unwrap(), "assets/a.png");
        assert_eq!(
            std::fs::read_to_string(root.join("assets/a.png")).unwrap(),
            "png"
        );
        assert!(root.join("assets/a.png.meta").is_file());
        assert!(!slot.exists(), "the slot goes");
        assert!(restore(&root, &id).is_err(), "only once");
        // Something new in its place: not overwritten.
        std::fs::create_dir_all(root.join("assets/sub")).unwrap();
        assert!(restore(&root, &folder).is_err());
        assert!(trash(&root, ".platypus").is_err(), "not the trash itself");
        assert!(trash(&root, "").is_err() && trash(&root, "../x").is_err());
        assert!(restore(&root, "../../x").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
