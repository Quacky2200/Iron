// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Where the engine's log goes. `platypus-app` is a windowed (GUI) program, so there's no
//! console of its own: the log is written to `Player.log` in the game's settings folder
//! (next to `prefs.json`; the last run's is kept as `Player-prev.log`, like Unity's),
//! with the same ANSI colours a terminal shows (`less -R`, or `cat` in a terminal, shows
//! them). Started from a terminal, it also writes there: on Windows it attaches to the
//! parent's console, and piped output (scripts, tests) gets it as before.
//!
//! `PLATYPUS_LOG_FILE` names another file; `RUST_LOG` filters as usual.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const DEFAULT_FILTER: &str =
    "info,wgpu_core=warn,wgpu_hal=error,cranelift_jit=warn,cranelift_codegen=warn";

/// On Windows, a GUI program started from a terminal writes to that terminal's console
/// (when its output isn't already piped somewhere).
pub fn attach_console() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
        };
        // SAFETY: plain Win32 calls with no pointers; failure just means no console.
        unsafe {
            let err = GetStdHandle(STD_ERROR_HANDLE);
            if err.is_null() || err == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
}

/// Whether anything receives stderr (a terminal, a pipe or a file).
fn has_stderr() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE};
        // SAFETY: no pointers involved.
        let h = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
        !h.is_null() && h != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
    }
    #[cfg(not(windows))]
    {
        true
    }
}

/// The log file for a settings folder: `PLATYPUS_LOG_FILE`, else `<dir>/Player.log`.
pub fn log_path(settings_dir: Option<&Path>) -> Option<PathBuf> {
    std::env::var_os("PLATYPUS_LOG_FILE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| settings_dir.map(|d| d.join("Player.log")))
}

/// Opens the log file, keeping the previous one as `<name>-prev.<ext>`.
fn open(path: &Path) -> std::io::Result<std::fs::File> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if path.exists() {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let prev = match path.extension() {
            Some(ext) => format!("{stem}-prev.{}", ext.to_string_lossy()),
            None => format!("{stem}-prev"),
        };
        let _ = std::fs::rename(path, path.with_file_name(prev));
    }
    std::fs::File::create(path)
}

/// Starts logging to `file` (if given and it opens) and to stderr (if there is one).
/// Returns the file actually used.
pub fn init(file: Option<PathBuf>) -> Option<PathBuf> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());
    let opened = file.and_then(|p| match open(&p) {
        Ok(f) => Some((p, Arc::new(f))),
        Err(e) => {
            eprintln!("platypus: can't write the log to {}: {e}", p.display());
            None
        }
    });
    let file_layer = opened.as_ref().map(|(_, f)| {
        tracing_subscriber::fmt::layer()
            .with_writer(f.clone())
            .with_ansi(true)
            .with_filter(filter())
    });
    let console_layer = has_stderr().then(|| {
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(filter())
    });
    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();
    // Panics go to the log too (a GUI program's stderr is often nowhere).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        default_hook(info);
    }));
    opened.map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-APP-04
    #[test]
    fn the_log_goes_next_to_prefs_and_keeps_the_last_one() {
        let dir = std::env::temp_dir().join(format!("platypus-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("Player.log");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "last run").unwrap();
        let mut f = open(&path).unwrap();
        std::io::Write::write_all(&mut f, b"this run").unwrap();
        drop(f);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "this run");
        assert_eq!(
            std::fs::read_to_string(dir.join("Player-prev.log")).unwrap(),
            "last run"
        );
        let _ = std::fs::remove_dir_all(&dir);
        if std::env::var_os("PLATYPUS_LOG_FILE").is_none() {
            assert_eq!(log_path(Some(&dir)), Some(dir.join("Player.log")));
        }
    }
}
