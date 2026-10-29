// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `Editor`: what the editor (`editor/`, a game like any other) asks of the host. The log for its
//! console (`logSince`), opening a file in the user's code editor (`openFile`) and file operations
//! on the project folder (`listDir`, `copyIn`, `move`) that answer at once, with `lastError` saying
//! why one failed.
//!
//! It works only when the host was started with a project (`platypus-app editor --project <folder>`,
//! [`enable`]); in any other game the calls do nothing and fail, so games can't use them. The host
//! feeds the log ([`push_log`] from its tracing layer while [`capturing`]) and says which outside
//! paths were dropped on the window ([`allow_sources`]), the only ones `copyIn` copies.

pub mod files;
mod open;
pub mod project;

use platypus_api::platypus_api;
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

/// The most log records kept; older ones drop off.
pub const LOG_LIMIT: usize = 5000;

/// One log record as the editor's console shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct LogRecord {
    /// Counts up from 1, so readers can ask for what they haven't seen.
    pub seq: f64,
    /// 0 trace, 1 debug, 2 info, 3 warn, 4 error.
    pub level: f64,
    pub target: String,
    pub message: String,
    /// Milliseconds since the log started.
    pub time_ms: f64,
}

/// What a project path is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileStat {
    pub exists: bool,
    pub dir: bool,
    pub size: f64,
    /// Last modified, milliseconds since 1970 (0 when unknown).
    pub modified_ms: f64,
}

/// One entry of a folder listing.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectEntry {
    pub name: String,
    pub dir: bool,
    /// Bytes (0 for folders).
    pub size: f64,
}

struct State {
    /// The project folder (`--project`): set, the editor calls work.
    project: Option<PathBuf>,
    /// The editor's own folder, for relative paths in `openFile`.
    ui_root: PathBuf,
    /// Files and folders dropped on the window: the only outside paths `copyIn` copies.
    dropped: BTreeSet<PathBuf>,
    log: VecDeque<LogRecord>,
    next_seq: u64,
    started: Option<Instant>,
    /// Why the last file operation failed ("" when it worked).
    error: String,
}

// A static, not per thread: the host's tracing layer pushes records from any thread.
static STATE: Mutex<State> = Mutex::new(State {
    project: None,
    ui_root: PathBuf::new(),
    dropped: BTreeSet::new(),
    log: VecDeque::new(),
    next_seq: 1,
    started: None,
    error: String::new(),
});

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Turns the editor calls on for a project folder (made absolute); `ui_root` is the editor's own
/// folder. The log is kept from now on.
pub fn enable(project: &Path, ui_root: &Path) {
    let project = std::path::absolute(project).unwrap_or_else(|_| project.to_path_buf());
    st(|s| {
        s.project = Some(project);
        s.ui_root = ui_root.to_path_buf();
        s.started.get_or_insert_with(Instant::now);
    });
}

/// Whether the host runs the editor ([`enable`] was called).
pub fn enabled() -> bool {
    st(|s| s.project.is_some())
}

/// Whether log records are kept (the host's tracing layer asks before formatting one).
pub fn capturing() -> bool {
    enabled()
}

/// Paths dropped on the window from the system, which `copyIn` may copy.
pub fn allow_sources(files: &[String]) {
    st(|s| s.dropped.extend(files.iter().map(PathBuf::from)));
}

/// Keeps one log record (while [`capturing`]).
pub fn push_log(level: u8, target: &str, message: String) {
    st(|s| {
        if s.project.is_none() {
            return;
        }
        let started = *s.started.get_or_insert_with(Instant::now);
        let seq = s.next_seq;
        s.next_seq += 1;
        if s.log.len() >= LOG_LIMIT {
            s.log.pop_front();
        }
        s.log.push_back(LogRecord {
            seq: seq as f64,
            level: f64::from(level),
            target: target.to_string(),
            message,
            time_ms: started.elapsed().as_secs_f64() * 1000.0,
        });
    });
}

/// For tests: forgets the project, the log and the dropped paths.
pub fn reset() {
    st(|s| {
        s.project = None;
        s.dropped.clear();
        s.log.clear();
        s.next_seq = 1;
        s.started = None;
        s.error.clear();
    });
}

/// Runs an editor operation on the project folder (other crates' editor calls too), keeping its
/// error for `Editor.lastError` and warning with it; `T::default()` when it failed.
pub fn project_op<T: Default>(name: &str, f: impl FnOnce(&Path) -> Result<T, String>) -> T {
    file_op(name, |root, _| f(root))
}

/// The project folder, when the editor runs.
pub fn project_root() -> Option<PathBuf> {
    st(|s| s.project.clone())
}

/// Runs a file operation on the project folder, keeping its error for `lastError`.
fn file_op<T: Default>(
    name: &str,
    f: impl FnOnce(&Path, &BTreeSet<PathBuf>) -> Result<T, String>,
) -> T {
    let (root, dropped) = st(|s| (s.project.clone(), s.dropped.clone()));
    let result = match root {
        Some(root) => f(&root, &dropped),
        None => Err("no project folder (start the editor with --project <folder>)".into()),
    };
    match result {
        Ok(v) => {
            st(|s| s.error.clear());
            v
        }
        Err(e) => {
            warn(&format!("Editor.{name}: {e}"));
            st(|s| s.error = e);
            T::default()
        }
    }
}

/// A warning in the log (through the log sink: this crate has no tracing of its own).
fn warn(text: &str) {
    crate::log::emit(crate::log::Level::Warn, text);
}

// The shim of a call returning one record (`stat`) writes through the caller's pointer.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[platypus_api(
    namespace = "Editor",
    records(
        /// One record of the engine's log: engine lines and the scripts' `console.*` (target `script`) in order.
        LogRecord = LogRecord [seq: f64, level: f64, target: string, message: string, time_ms: f64],
        /// A file or folder of the project.
        ProjectEntry = ProjectEntry [name: string, dir: bool, size: f64],
        /// What a project path is (`Editor.stat`).
        FileStat = FileStat [exists: bool, dir: bool, size: f64, modified_ms: f64]
    )
)]
pub mod editor_api {
    use super::*;

    /// Whether the host runs the editor on a project (`platypus-app editor --project <folder>`).
    #[property]
    pub fn is_available() -> bool {
        enabled()
    }

    /// The project folder's full path ("" without one).
    #[property]
    pub fn project() -> String {
        st(|s| {
            s.project
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    }

    /// The kept log records with `seq` above `after`, oldest first (the last 5000 at most).
    pub fn log_since(after: f64) -> Vec<LogRecord> {
        st(|s| s.log.iter().filter(|r| r.seq > after).cloned().collect())
    }

    /// Opens a file at a line and column in the user's code editor: `PLATYPUS_EDITOR_OPEN` (with
    /// `{file}`, `{line}`, `{col}`), else VS Code's `code -g` when it is on PATH. A relative path is
    /// looked up in the current folder, the editor's folder and the engine's source tree.
    #[optional(line = 1.0, col = 1.0)]
    pub fn open_file(path: &str, line: f64, col: f64) {
        let Some(ui_root) = st(|s| s.project.is_some().then(|| s.ui_root.clone())) else {
            warn("Editor.openFile: only the editor opens files");
            return;
        };
        if let Err(e) = open::open_file(path, line.max(1.0) as u32, col.max(1.0) as u32, &ui_root) {
            warn(&e);
        }
    }

    /// A project folder's entries ("" is the root): folders first, by name, without dot files.
    pub fn list_dir(path: &str) -> Vec<ProjectEntry> {
        file_op("listDir", |root, _| {
            Ok(project::list(root, path)?
                .into_iter()
                .map(|e| ProjectEntry {
                    name: e.name,
                    dir: e.dir,
                    size: e.size as f64,
                })
                .collect())
        })
    }

    /// Copies files or folders dropped on the window into a project folder, renaming instead of
    /// overwriting (`a (2).png`). Returns the new project paths.
    pub fn copy_in(sources: &[String], dir: &str) -> Vec<String> {
        file_op("copyIn", |root, dropped| {
            let sources: Vec<PathBuf> = sources.iter().map(PathBuf::from).collect();
            if let Some(bad) = sources.iter().find(|s| !dropped.contains(*s)) {
                return Err(format!("{} wasn't dropped on the editor", bad.display()));
            }
            project::copy_in(root, &sources, dir)
        })
    }

    /// Moves a project file or folder into a project folder; returns its new path. Refuses
    /// overwriting and moving a folder into itself.
    #[js_name = "move"]
    pub fn move_into(path: &str, dir: &str) -> String {
        file_op("move", |root, _| project::move_into(root, path, dir))
    }

    /// A project file's text ("" when it can't be read: see `lastError`).
    pub fn read_text(path: &str) -> String {
        file_op("readText", |root, _| files::read_text(root, path))
    }

    /// Writes a project file, making its folders; the old file is replaced in one step (the text
    /// goes to a temporary file first). False when it failed.
    pub fn write_text(path: &str, text: &str) -> bool {
        file_op("writeText", |root, _| {
            files::write_text(root, path, text).map(|()| true)
        })
    }

    /// Whether a project path exists.
    pub fn exists(path: &str) -> bool {
        project_root().is_some_and(|r| files::exists(&r, path))
    }

    /// Whether a project path exists, is a folder, its size and when it last changed.
    pub fn stat(path: &str) -> FileStat {
        file_op("stat", |root, _| {
            let s = files::stat(root, path)?;
            Ok(FileStat {
                exists: s.exists,
                dir: s.dir,
                size: s.size as f64,
                modified_ms: s.modified_ms,
            })
        })
    }

    /// A path in project folder `dir` that isn't taken: `dir/name`, else `dir/name (2).ext`, ...
    pub fn free_name(dir: &str, name: &str) -> String {
        file_op("freeName", |root, _| files::free_name(root, dir, name))
    }

    /// Makes a project folder (and those above it); returns its path ("" if it exists or failed).
    pub fn create_dir(path: &str) -> String {
        file_op("createDir", |root, _| files::create_dir(root, path))
    }

    /// Renames a project file or folder in its folder (its `.meta` too); returns the new path.
    pub fn rename(path: &str, name: &str) -> String {
        file_op("rename", |root, _| files::rename(root, path, name))
    }

    /// Moves a project file or folder (and its `.meta`) into the project's trash
    /// (`.platypus/trash`); returns the id `restore` puts it back with.
    pub fn trash(path: &str) -> String {
        file_op("trash", |root, _| files::trash(root, path))
    }

    /// Puts an item back from the trash where it was; returns its path.
    pub fn restore(id: &str) -> String {
        file_op("restore", |root, _| files::restore(root, id))
    }

    /// Why the last file operation failed ("" when it worked).
    #[property]
    pub fn last_error() -> String {
        st(|s| s.error.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-EDT-27
    #[test]
    fn editor_calls_fail_without_a_project_and_work_with_one() {
        reset();
        assert!(!editor_api::is_available());
        push_log(2, "script", "not kept".into());
        assert!(editor_api::log_since(0.0).is_empty());
        assert!(editor_api::list_dir("").is_empty());
        assert!(editor_api::last_error().contains("--project"));

        let dir = std::env::temp_dir().join(format!("platypus-editor-api-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        enable(&dir, &dir);
        assert!(editor_api::is_available());
        push_log(3, "script", "careful".into());
        push_log(2, "platypus_app", "loaded".into());
        let got: Vec<_> = editor_api::log_since(0.0)
            .into_iter()
            .map(|r| (r.seq, r.level, r.target, r.message))
            .collect();
        assert_eq!(
            got,
            [
                (1.0, 3.0, "script".to_string(), "careful".to_string()),
                (2.0, 2.0, "platypus_app".to_string(), "loaded".to_string())
            ]
        );
        assert_eq!(editor_api::log_since(1.0).len(), 1);
        let names: Vec<_> = editor_api::list_dir("")
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["assets"]);
        assert_eq!(editor_api::last_error(), "");
        // Only what was dropped on the window comes in from outside.
        assert!(editor_api::copy_in(&["C:/elsewhere/x.png".into()], "assets").is_empty());
        assert!(editor_api::last_error().contains("wasn't dropped"));
        reset();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
