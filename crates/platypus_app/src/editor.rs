// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! What only the editor (`platypus_editor`) does: where its UI lives, the project folder,
//! feeding the log to its console (`__platypus_log`) and the `editor.*` ops (opening code,
//! project files). In `platypus-app` the ops are refused, so games can't use them.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use platypus_core::Value;
use platypus_script::ScriptRuntime;

use super::{logging, project};

static PROJECT: OnceLock<PathBuf> = OnceLock::new();

/// Sets the project folder the editor works on (`platypus_editor [project]`), once.
pub fn set_project(dir: &Path) {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let _ = PROJECT.set(dir);
}

/// The project folder (`None` outside the editor).
pub fn project() -> Option<&'static Path> {
    PROJECT.get().map(PathBuf::as_path)
}

/// Whether this program is `platypus_editor` (the same source is built as both binaries).
pub const ENABLED: bool = same(env!("CARGO_BIN_NAME").as_bytes(), b"platypus_editor");

pub(crate) const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The editor's UI (a game folder with `build/game.js`): `PLATYPUS_EDITOR_UI`, else an
/// `editor` folder next to the program (a packaged editor), else the workspace's.
pub fn ui_folder() -> PathBuf {
    if let Some(dir) = std::env::var_os("PLATYPUS_EDITOR_UI").filter(|v| !v.is_empty()) {
        return dir.into();
    }
    let packaged = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("editor")))
        .filter(|dir| dir.join("build").join("game.js").is_file());
    packaged.unwrap_or_else(|| workspace_root().join("editor"))
}

/// The source tree the program was built from (for engine paths in log lines).
fn workspace_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(crate_dir)
        .to_path_buf()
}

/// The editor's side of the app: which log records its console has seen, files it
/// may copy in, and answers to file ops waiting for the scripts.
#[derive(Default)]
pub struct EditorHost {
    /// Set by `editor.logHistory`: the scripts want the log.
    subscribed: bool,
    /// The last record sent to `__platypus_log`.
    sent: u64,
    /// Files and folders dropped on the window: the only outside paths
    /// `editor.copyIn` copies from.
    dropped: HashSet<PathBuf>,
    /// Answers for `__platypus_editor_fs`: `[id, ok, json or error, ...]`.
    results: Vec<Value>,
}

impl EditorHost {
    /// Paths dropped on the window from the system, which `editor.copyIn` may copy.
    pub fn allow_sources(&mut self, files: &[String]) {
        self.dropped.extend(files.iter().map(PathBuf::from));
    }

    /// Once per frame: answers to file ops go to `__platypus_editor_fs([id, ok, data,
    /// ...])`, and log records the scripts haven't seen to `__platypus_log([seq, level,
    /// target, message, timeMs, ...])`.
    pub fn pump(&mut self, scripts: &mut dyn ScriptRuntime) {
        if !self.results.is_empty() {
            let results = std::mem::take(&mut self.results);
            if let Err(e) = scripts.call_global("__platypus_editor_fs", &[Value::Array(results)]) {
                tracing::error!("script error in __platypus_editor_fs: {e}");
            }
        }
        if !self.subscribed {
            return;
        }
        let records = logging::records_since(self.sent);
        let Some(last) = records.last() else {
            return;
        };
        self.sent = last.seq;
        let flat = records
            .into_iter()
            .flat_map(|r| {
                [
                    Value::Number(r.seq as f64),
                    Value::from(u32::from(r.level)),
                    Value::String(r.target),
                    Value::String(r.message),
                    Value::Number(r.time_ms),
                ]
            })
            .collect();
        if let Err(e) = scripts.call_global("__platypus_log", &[Value::Array(flat)]) {
            // Not logged as an error: that would come straight back here.
            eprintln!("script error in __platypus_log: {e}");
        }
    }

    /// Runs an `editor.*` op (`name` without the prefix). `root` is the UI's folder.
    pub fn op(&mut self, name: &str, args: &[Value], root: &Path) {
        if !ENABLED {
            tracing::warn!("editor.{name}: editor ops only work in platypus_editor");
            return;
        }
        match name {
            // The console (re)starts: everything kept so far, then new records.
            "logHistory" => {
                logging::capture(true);
                self.subscribed = true;
                self.sent = 0;
            }
            "openFile" => {
                let Some(file) = args.first().and_then(Value::as_str) else {
                    tracing::warn!("editor.openFile needs a path");
                    return;
                };
                let num = |i: usize| args.get(i).and_then(Value::as_f64).map_or(1, |n| n as u32);
                open_file(file, num(1).max(1), num(2).max(1), root);
            }
            "listDir" | "copyIn" | "move" => {
                let id = args.first().and_then(Value::as_f64).unwrap_or(0.0);
                let result = self.fs_op(name, &args[args.len().min(1)..]);
                if let Err(e) = &result {
                    tracing::warn!("editor.{name}: {e}");
                }
                self.results.push(Value::Number(id));
                self.results.push(Value::Bool(result.is_ok()));
                self.results
                    .push(Value::String(result.unwrap_or_else(|e| e)));
            }
            other => tracing::warn!("unknown editor op `editor.{other}`"),
        }
    }

    /// Project file ops; answers are JSON:
    /// - `listDir path` -> `{ path, entries: [{ name, dir, size }] }`
    /// - `copyIn [sources] dir` (sources must have been dropped on the window) ->
    ///   `{ created: [paths] }`
    /// - `move path dir` -> `{ path }`
    fn fs_op(&self, name: &str, args: &[Value]) -> Result<String, String> {
        let root = project().ok_or("no project folder")?;
        let text = |i: usize| args.get(i).and_then(Value::as_str).unwrap_or("");
        let json = match name {
            "listDir" => {
                let entries = project::list(root, text(0))?;
                let entries: Vec<_> = entries
                    .into_iter()
                    .map(|e| serde_json::json!({ "name": e.name, "dir": e.dir, "size": e.size }))
                    .collect();
                serde_json::json!({ "path": text(0), "entries": entries })
            }
            "copyIn" => {
                let sources: Vec<PathBuf> = match args.first() {
                    Some(Value::Array(list)) => list
                        .iter()
                        .filter_map(Value::as_str)
                        .map(PathBuf::from)
                        .collect(),
                    _ => return Err("copyIn needs a list of files".into()),
                };
                if let Some(bad) = sources.iter().find(|s| !self.dropped.contains(*s)) {
                    return Err(format!("{} wasn't dropped on the editor", bad.display()));
                }
                let created = project::copy_in(root, &sources, text(1))?;
                serde_json::json!({ "created": created })
            }
            _ => serde_json::json!({ "path": project::move_into(root, text(0), text(1))? }),
        };
        Ok(json.to_string())
    }
}

/// Opens `file` at a line and column in the user's code editor.
fn open_file(file: &str, line: u32, col: u32, root: &Path) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let path = resolve_source(file, &[cwd, root.to_path_buf(), workspace_root()]);
    let template = std::env::var("PLATYPUS_EDITOR_OPEN").ok();
    let Some((program, args)) = open_command(
        template.as_deref(),
        &path.to_string_lossy(),
        line,
        col,
        which,
    ) else {
        tracing::info!(
            "can't open {}:{line}:{col}: no `code` (VS Code) on PATH; set PLATYPUS_EDITOR_OPEN, e.g. \"code -g {{file}}:{{line}}:{{col}}\"",
            path.display()
        );
        return;
    };
    let mut command = std::process::Command::new(&program);
    command.args(&args);
    #[cfg(windows)]
    {
        // `code` is a .cmd script: don't flash a console window for it.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    match command.spawn() {
        Ok(_) => tracing::debug!("opened {}:{line}:{col}", path.display()),
        Err(e) => tracing::warn!("can't run {}: {e}", program.display()),
    }
}

/// A source path from a log line: absolute paths as they are, relative ones against the
/// first root they exist in (else the first root).
pub fn resolve_source(file: &str, roots: &[PathBuf]) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    roots
        .iter()
        .map(|root| root.join(path))
        .find(|p| p.exists())
        .or_else(|| roots.first().map(|r| r.join(path)))
        .unwrap_or_else(|| path.to_path_buf())
}

/// The program and arguments that open `file` at `line:col`. A template (from
/// `PLATYPUS_EDITOR_OPEN`) is split like a shell would (double quotes group) and gets
/// `{file}`, `{line}` and `{col}` filled in; without one, VS Code's `code -g` if `find`
/// finds it on PATH.
pub fn open_command(
    template: Option<&str>,
    file: &str,
    line: u32,
    col: u32,
    find: impl Fn(&str) -> Option<PathBuf>,
) -> Option<(PathBuf, Vec<String>)> {
    let words = match template.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => split_args(t),
        None => vec!["code".into(), "-g".into(), "{file}:{line}:{col}".into()],
    };
    let fill = |w: &str| {
        w.replace("{file}", file)
            .replace("{line}", &line.to_string())
            .replace("{col}", &col.to_string())
    };
    let (program, rest) = words.split_first()?;
    let program = find(program)?;
    Some((program, rest.iter().map(|w| fill(w)).collect()))
}

/// Splits a command line on spaces, keeping double-quoted parts together.
fn split_args(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut any) = (false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    words.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            c => {
                word.push(c);
                any = true;
            }
        }
    }
    if any {
        words.push(word);
    }
    words
}

/// Finds a program: a path that exists, or a name on PATH (with PATHEXT's extensions
/// on Windows, where `code` is `code.cmd`).
pub fn which(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .filter(|e| !e.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };
    let dirs = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&dirs) {
        for ext in &exts {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(name: &str) -> Option<PathBuf> {
        Some(PathBuf::from(format!("/bin/{name}")))
    }

    // M5-EDT-16
    #[test]
    fn code_links_open_with_vs_code_by_default() {
        let (program, args) = open_command(None, "src/main.ts", 12, 5, found).unwrap();
        assert_eq!(program, PathBuf::from("/bin/code"));
        assert_eq!(args, ["-g", "src/main.ts:12:5"]);
        // No `code` and no template: nothing to run (the app logs a hint).
        assert_eq!(open_command(None, "a.ts", 1, 1, |_| None), None);
    }

    // M5-EDT-16
    #[test]
    fn the_open_template_fills_file_line_and_column() {
        let (program, args) = open_command(
            Some(r#"subl "{file}:{line}:{col}" --wait"#),
            "C:/My Game/a.ts",
            3,
            9,
            found,
        )
        .unwrap();
        assert_eq!(program, PathBuf::from("/bin/subl"));
        assert_eq!(args, ["C:/My Game/a.ts:3:9", "--wait"]);
        assert_eq!(split_args(r#"  a  "b c"  "" "#), ["a", "b c", ""]);
        // A blank template is the same as none.
        assert!(open_command(Some("  "), "a", 1, 1, found).is_some());
    }

    // M5-EDT-16
    #[test]
    fn relative_sources_resolve_against_the_first_root_that_has_them() {
        let dir = std::env::temp_dir().join(format!("platypus-editor-src-{}", std::process::id()));
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(b.join("src")).unwrap();
        std::fs::create_dir_all(&a).unwrap();
        std::fs::write(b.join("src").join("x.ts"), "").unwrap();
        let roots = [a.clone(), b.clone()];
        assert_eq!(resolve_source("src/x.ts", &roots), b.join("src/x.ts"));
        assert_eq!(resolve_source("missing.ts", &roots), a.join("missing.ts"));
        let abs = dir.join("abs.ts");
        assert_eq!(resolve_source(&abs.to_string_lossy(), &roots), abs);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M5-EDT-17
    #[test]
    fn editor_ops_are_refused_outside_the_editor() {
        // Tests run as the `platypus-app` binary, where `ENABLED` is false.
        let mut host = EditorHost::default();
        host.op("logHistory", &[], Path::new("."));
        assert_eq!(host.subscribed, ENABLED);
    }
}
