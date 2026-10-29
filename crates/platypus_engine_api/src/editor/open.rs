// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Opening a source file at a line in the user's code editor (`Editor.openFile`, the console's
//! code links).

use std::path::{Path, PathBuf};

/// The source tree the program was built from (for engine paths in log lines).
fn workspace_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(crate_dir)
        .to_path_buf()
}

/// Opens `file` at a line and column in the user's code editor; `root` is the editor's folder.
pub(super) fn open_file(file: &str, line: u32, col: u32, root: &Path) -> Result<(), String> {
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
        return Err(format!(
            "can't open {}:{line}:{col}: no `code` (VS Code) on PATH; set PLATYPUS_EDITOR_OPEN, e.g. \"code -g {{file}}:{{line}}:{{col}}\"",
            path.display()
        ));
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
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("can't run {}: {e}", program.display()))
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
pub fn split_args(line: &str) -> Vec<String> {
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

    // M5-EDT-26
    #[test]
    fn code_links_open_with_vs_code_by_default() {
        let (program, args) = open_command(None, "src/main.ts", 12, 5, found).unwrap();
        assert_eq!(program, PathBuf::from("/bin/code"));
        assert_eq!(args, ["-g", "src/main.ts:12:5"]);
        // No `code` and no template: nothing to run (the app logs a hint).
        assert_eq!(open_command(None, "a.ts", 1, 1, |_| None), None);
    }

    // M5-EDT-26
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

    // M5-EDT-26
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
}
