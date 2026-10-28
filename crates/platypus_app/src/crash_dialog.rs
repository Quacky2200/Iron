//! The crash dialog's process: `<exe> --crash-report <file> [--plain]`, started by a game that
//! crashed (`crash.rs`), so the dialog never runs in the broken process. It shows Platypus's own
//! dialog (`crash_dialog.tsx`, run as a small game by this same program with `--crash-dialog
//! <folder>`) and falls back to the system's plain dialog when that one can't run (no GPU, a
//! broken install, its own crash) or when `--plain` asks for it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The flag of the supervisor process (followed by the report's path).
pub const REPORT_FLAG: &str = "--crash-report";
/// Asks for the plain dialog only.
pub const PLAIN_FLAG: &str = "--plain";
/// The flag of the Platypus dialog's own process (followed by its game folder).
pub const DIALOG_FLAG: &str = "--crash-dialog";
/// Set in the Platypus dialog's process: a crash there must not open another dialog.
pub const CHILD_ENV: &str = "PLATYPUS_CRASH_DIALOG";
/// The Platypus dialog's exit code for "open the report's folder" (0 just closes).
const EXIT_OPEN_FOLDER: i32 = 10;

const DIALOG_SOURCE: &str = include_str!("crash_dialog.tsx");

/// What a report says, for the dialog.
struct Report {
    game: String,
    reason: String,
    text: String,
}

impl Report {
    fn read(path: &Path) -> Report {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| format!("The crash report could not be read ({e}).\n"));
        Report::parse(text)
    }

    fn parse(text: String) -> Report {
        let field = |name: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(name))
                .map(|v| v.trim().to_string())
        };
        Report {
            game: field("Game:").unwrap_or_else(|| "The game".into()),
            reason: field("Reason:").unwrap_or_else(|| "An unknown error".into()),
            text,
        }
    }
}

/// When this process was started as a crash dialog, shows it and returns the exit code.
pub fn main_if_asked() -> Option<i32> {
    let args: Vec<OsString> = std::env::args_os().collect();
    let at = args.iter().position(|a| a == REPORT_FLAG)?;
    let path = PathBuf::from(args.get(at + 1)?);
    let plain = args.iter().any(|a| a == PLAIN_FLAG);
    let report = Report::read(&path);
    if plain || !platypus_dialog(&report, &path) {
        plain_dialog(&report, &path);
    }
    Some(0)
}

/// Runs Platypus's dialog as a game of its own; false when it could not show (then the plain one
/// does). The folder is written to the temp folder and removed afterwards.
fn platypus_dialog(report: &Report, path: &Path) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let dir = std::env::temp_dir().join(format!("platypus-crash-dialog-{}", std::process::id()));
    if write_dialog_game(&dir, report, path).is_err() {
        return false;
    }
    let status = Command::new(exe)
        .arg(DIALOG_FLAG)
        .arg(&dir)
        .env(CHILD_ENV, "1")
        // Its log stays with it, not in a settings folder of its own.
        .env("PLATYPUS_LOG_FILE", dir.join("dialog.log"))
        .status();
    let shown = match status.ok().and_then(|s| s.code()) {
        Some(0) => true,
        Some(EXIT_OPEN_FOLDER) => {
            reveal(path);
            true
        }
        _ => false,
    };
    let _ = std::fs::remove_dir_all(&dir);
    shown
}

/// The dialog's game folder: `project.json` and `build/game.pbundle`, its source the dialog's
/// TSX after the report's text as constants.
fn write_dialog_game(dir: &Path, report: &Report, path: &Path) -> std::io::Result<()> {
    let s = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
    let source = format!(
        "const GAME = {};\nconst REASON = {};\nconst DETAILS = {};\nconst REPORT_PATH = {};\n{DIALOG_SOURCE}",
        s(&report.game),
        s(&report.reason),
        s(&report.text),
        s(&path.display().to_string()),
    );
    let title = format!("{} crashed", report.game);
    let project = serde_json::json!({
        "name": "crash-report",
        "title": title,
        "company": "Platypus",
        "identifier": "dev.platypus.crash-report",
        "display": { "mode": "Windowed", "width": 640, "height": 420, "resizable": false },
        "fps": false,
    });
    let bundle = serde_json::json!({
        "format": 1,
        "title": title,
        "company": "Platypus",
        "identifier": "dev.platypus.crash-report",
        "entry": "crash_dialog.tsx",
        "source": source,
        "files": [],
        "css": null,
        "assets": [],
        "patterns": [],
    });
    std::fs::create_dir_all(dir.join("build"))?;
    std::fs::write(dir.join("project.json"), project.to_string())?;
    std::fs::write(dir.join("build").join("game.pbundle"), bundle.to_string())
}

/// Shows the report's file in the system's file browser.
fn reveal(path: &Path) {
    #[cfg(windows)]
    let _ = Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open")
        .arg(path.parent().unwrap_or(Path::new(".")))
        .spawn();
}

/// The system's own message box: what still works when Platypus's dialog does not.
#[cfg(windows)]
fn plain_dialog(report: &Report, path: &Path) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_ICONERROR, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW,
    };
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let text = wide(&format!(
        "{} stopped unexpectedly. It ran into an error it could not recover from and had to close.\n\n{}\n\nA crash report was saved to:\n{}\n\nOpen the folder that has it?",
        report.game,
        report.reason,
        path.display()
    ));
    let caption = wide(&format!("{} crashed", report.game));
    // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call.
    let answer = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_YESNO | MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };
    if answer == IDYES {
        reveal(path);
    }
}

#[cfg(not(windows))]
fn plain_dialog(report: &Report, path: &Path) {
    eprintln!(
        "{} stopped unexpectedly: {}\nA crash report was saved to {}",
        report.game,
        report.reason,
        path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::Report;

    // M9-API-918
    #[test]
    fn a_report_gives_the_dialog_its_game_and_reason() {
        let r = Report::parse(
            "Platypus crash report\nGame: Zombie Horders\nReason: Heap corruption at 0x1\n\ndetails\n"
                .into(),
        );
        assert_eq!(r.game, "Zombie Horders");
        assert_eq!(r.reason, "Heap corruption at 0x1");
        assert!(r.text.ends_with("details\n"));
        let empty = Report::parse(String::new());
        assert_eq!(empty.game, "The game");
        assert_eq!(empty.reason, "An unknown error");
    }
}
