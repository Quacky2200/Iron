// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The editor's file operations on the project folder: list a folder, copy files in
//! from outside (what was dropped on the window), move within. Paths from scripts are
//! relative to the project and checked like asset paths (`platypus_core::resolve_asset`:
//! no absolute paths, drives or `..`), and symlinks can't lead out of it either.

use std::path::{Path, PathBuf};

use platypus_core::resolve_asset;

/// The most files one copy may create (a dropped folder is copied whole).
const COPY_LIMIT: usize = 10_000;

/// One entry of a folder listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
    /// Bytes (0 for folders).
    pub size: u64,
}

/// A project-relative path (`/` or `\` separated; empty or `.` is the root) as a path
/// inside `root`, or why it isn't one.
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().replace('\\', "/");
    let rel = rel.trim_end_matches('/');
    let path = if rel.is_empty() || rel == "." {
        root.to_path_buf()
    } else {
        resolve_asset(root, rel).ok_or_else(|| format!("{rel:?} is outside the project"))?
    };
    // Through symlinks, the real place must still be inside.
    if let (Ok(real), Ok(real_root)) = (path.canonicalize(), root.canonicalize())
        && !real.starts_with(&real_root)
    {
        return Err(format!("{rel:?} leads outside the project"));
    }
    Ok(path)
}

/// `path` relative to `root`, with `/` separators.
pub fn relative(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A folder's entries: folders first, then files, by name (case-insensitive). Names
/// starting with `.` are left out.
pub fn list(root: &Path, rel: &str) -> Result<Vec<Entry>, String> {
    let dir = resolve(root, rel)?;
    let read = std::fs::read_dir(&dir).map_err(|e| format!("can't list {rel:?}: {e}"))?;
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            // Dot files, and `.meta` files: they go with their asset (moves, renames, deletes).
            if name.starts_with('.') || name.ends_with(".meta") {
                return None;
            }
            let meta = e.metadata().ok()?;
            Some(Entry {
                name,
                dir: meta.is_dir(),
                size: if meta.is_dir() { 0 } else { meta.len() },
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.dir
            .cmp(&a.dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// A name in `dir` that isn't taken: `name`, else `stem (2).ext`, `stem (3).ext`, ...
pub(super) fn free_name(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rfind('.').filter(|&i| i > 0) {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// Copies files (or whole folders) from outside into the project folder `rel_dir`,
/// renaming instead of overwriting. Returns the new project-relative paths.
pub fn copy_in(root: &Path, sources: &[PathBuf], rel_dir: &str) -> Result<Vec<String>, String> {
    let dir = resolve(root, rel_dir)?;
    if !dir.is_dir() {
        return Err(format!("{rel_dir:?} is not a folder"));
    }
    let mut created = Vec::new();
    let mut budget = COPY_LIMIT;
    for source in sources {
        let name = source
            .file_name()
            .ok_or_else(|| format!("{} has no name", source.display()))?;
        if source.is_dir()
            && let (Ok(src), Ok(dst)) = (source.canonicalize(), dir.canonicalize())
            && dst.starts_with(&src)
        {
            return Err(format!("can't copy {} into itself", source.display()));
        }
        let target = free_name(&dir, &name.to_string_lossy());
        copy_any(source, &target, &mut budget)
            .map_err(|e| format!("copying {}: {e}", source.display()))?;
        created.push(relative(root, &target));
    }
    Ok(created)
}

fn copy_any(from: &Path, to: &Path, budget: &mut usize) -> std::io::Result<()> {
    if *budget == 0 {
        return Err(std::io::Error::other("too many files"));
    }
    *budget -= 1;
    let meta = std::fs::symlink_metadata(from)?;
    if meta.is_dir() {
        std::fs::create_dir(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_any(&entry.path(), &to.join(entry.file_name()), budget)?;
        }
        Ok(())
    } else if meta.is_file() {
        std::fs::copy(from, to)?;
        // A new asset gets its GUID now (a copied-in `.meta` is kept).
        platypus_core::meta::ensure(to).map(|_| ())
    } else {
        // Symlinks and other special files are skipped.
        Ok(())
    }
}

/// Moves a project file or folder into the project folder `rel_dir`. Returns its new
/// project-relative path. Refuses to overwrite, or to move a folder into itself.
pub fn move_into(root: &Path, rel_from: &str, rel_dir: &str) -> Result<String, String> {
    let from = resolve(root, rel_from)?;
    let dir = resolve(root, rel_dir)?;
    if from == root || !from.exists() {
        return Err(format!("{rel_from:?} can't be moved"));
    }
    if !dir.is_dir() {
        return Err(format!("{rel_dir:?} is not a folder"));
    }
    if dir.starts_with(&from) {
        return Err(format!("can't move {rel_from:?} into itself"));
    }
    let name = from.file_name().ok_or("no name")?;
    let to = dir.join(name);
    if to == from {
        return Ok(relative(root, &to));
    }
    if to.exists() {
        return Err(format!("{} already exists", relative(root, &to)));
    }
    super::files::move_with_meta(&from, &to).map_err(|e| format!("moving {rel_from:?}: {e}"))?;
    Ok(relative(root, &to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("platypus-project-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // M5-DND-12
    #[test]
    fn paths_stay_inside_the_project() {
        let root = temp("resolve");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        assert_eq!(resolve(&root, "").unwrap(), root);
        assert_eq!(resolve(&root, "assets/").unwrap(), root.join("assets"));
        assert_eq!(
            resolve(&root, "assets\\x.png").unwrap(),
            root.join("assets/x.png")
        );
        for bad in ["../x", "/etc", "C:/Windows", "assets/../../x", "a:b"] {
            assert!(resolve(&root, bad).is_err(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    // M5-DND-12
    #[test]
    fn listings_put_folders_first_and_skip_dot_files() {
        let root = temp("list");
        std::fs::create_dir_all(root.join("b-dir")).unwrap();
        std::fs::write(root.join("A.txt"), "hello").unwrap();
        std::fs::write(root.join("c.png"), "").unwrap();
        std::fs::write(root.join(".hidden"), "").unwrap();
        std::fs::write(root.join("c.png.meta"), "").unwrap();
        let names: Vec<_> = list(&root, "")
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.dir, e.size))
            .collect();
        assert_eq!(
            names,
            [
                ("b-dir".to_string(), true, 0),
                ("A.txt".to_string(), false, 5),
                ("c.png".to_string(), false, 0)
            ]
        );
        assert!(list(&root, "missing").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    // M5-DND-12
    #[test]
    fn copies_come_in_renamed_and_moves_stay_inside() {
        let root = temp("copy");
        let outside = temp("copy-src");
        std::fs::write(outside.join("a.png"), "png").unwrap();
        std::fs::create_dir_all(outside.join("pack/sub")).unwrap();
        std::fs::write(outside.join("pack/sub/b.txt"), "b").unwrap();
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets/a.png"), "old").unwrap();

        let made = copy_in(
            &root,
            &[outside.join("a.png"), outside.join("pack")],
            "assets",
        )
        .unwrap();
        assert_eq!(made, ["assets/a (2).png", "assets/pack"]);
        assert_eq!(
            std::fs::read_to_string(root.join("assets/a.png")).unwrap(),
            "old"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("assets/pack/sub/b.txt")).unwrap(),
            "b"
        );
        assert!(copy_in(&root, &[outside.join("a.png")], "../x").is_err());

        assert_eq!(
            move_into(&root, "assets/a (2).png", "").unwrap(),
            "a (2).png"
        );
        assert!(root.join("a (2).png").is_file());
        // An asset's `.meta` moves with it.
        std::fs::write(root.join("a (2).png.meta"), "guid").unwrap();
        assert_eq!(
            move_into(&root, "a (2).png", "assets").unwrap(),
            "assets/a (2).png"
        );
        assert!(root.join("assets/a (2).png.meta").is_file());
        assert_eq!(
            move_into(&root, "assets/a (2).png", "").unwrap(),
            "a (2).png"
        );
        assert!(
            move_into(&root, "assets", "assets/pack").is_err(),
            "into itself"
        );
        std::fs::write(root.join("assets/pack/a (2).png"), "").unwrap();
        assert!(
            move_into(&root, "a (2).png", "assets/pack").is_err(),
            "no overwrite"
        );
        assert!(move_into(&root, "", "assets").is_err(), "not the root");
        assert!(move_into(&root, "../a", "assets").is_err());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
