// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! A screenshot run with `--video-settings-prompt` renders the startup prompt (the dialog's own
//! UI-only window, not the game) and saves it (needs a GPU, like the other rendering tests).

use std::path::PathBuf;
use std::process::Command;

// M9-VSP-09
#[test]
fn a_forced_prompt_in_a_screenshot_run_saves_the_dialogs_picture() {
    let dir = std::env::temp_dir().join(format!("platypus-vsp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hello.js");
    let png = dir.join("out.png");
    let log = dir.join("run.log");
    let status = Command::new(env!("CARGO_BIN_EXE_platypus-game"))
        .arg(&game)
        .arg("--screenshot")
        .arg(&png)
        .args(["--frames", "5", "--force", "--video-settings-prompt"])
        .env("PLATYPUS_LOG_FILE", &log)
        .status()
        .unwrap();
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(status.success(), "{text}");
    assert!(png.is_file(), "no screenshot; log:\n{text}");
    // The dialog's window (600 x 500 logical pixels, times the display's scale), not the game's.
    let (w, h) = image::image_dimensions(&png).unwrap();
    assert_eq!(w * 500, h * 600, "{w}x{h}\n{text}");
    assert!(!text.contains("did not run"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
