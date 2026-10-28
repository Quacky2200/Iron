// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `platypus-app --licenses`: prints the third-party notices built into the program and exits,
//! before anything else starts (a packaged game's payload is not even unpacked). The text is
//! `THIRD-PARTY-NOTICES.md`, written by `tools/platypus_notices` from Cargo.lock, so every exe
//! carries the notices of what it is built from.

use std::ffi::OsString;
use std::io::Write;

/// The flag.
pub const FLAG: &str = "--licenses";

/// The notices, as committed at the repository root.
pub const NOTICES: &str = include_str!("../../../THIRD-PARTY-NOTICES.md");

/// `--licenses` is among the arguments (before `--`, after which they are the game's).
pub fn asked(args: impl IntoIterator<Item = OsString>) -> bool {
    args.into_iter()
        .take_while(|a| a != "--")
        .any(|a| a == FLAG)
}

/// Writes the notices to stdout (a closed pipe, as with `| head`, is not an error).
pub fn print() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE,
        };
        // SAFETY: plain Win32 calls with no pointers; failure just means no console. A windowed
        // program started from a terminal has no stdout until it joins the terminal's console.
        unsafe {
            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            if out.is_null() || out == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(NOTICES.as_bytes());
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<OsString> {
        a.iter().map(OsString::from).collect()
    }

    // M9-LIC-14
    #[test]
    fn licenses_is_asked_for_anywhere_before_the_script_arguments() {
        assert!(asked(args(&["--licenses"])));
        assert!(asked(args(&["game", "--fps", "--licenses"])));
        assert!(!asked(args(&["game"])));
        assert!(!asked(args(&["game", "--", "--licenses"])));
        let file = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../THIRD-PARTY-NOTICES.md"
        ))
        .unwrap();
        let notices = NOTICES.replace("\r\n", "\n");
        assert_eq!(notices, file.replace("\r\n", "\n"));
        assert!(notices.contains("## ANGLE (BSD-3-Clause)") && notices.contains("\n## MIT\n"));
    }
}
