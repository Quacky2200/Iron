// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! A `--screenshot --fps` run saves the file and logs no errors (needs a GPU, like the
//! other rendering tests).

use std::path::PathBuf;
use std::process::Command;

fn run(name: &str, flag: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("platypus-fps-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hello.js");
    let png = dir.join("out.png");
    let log = dir.join("run.log");
    let status = Command::new(env!("CARGO_BIN_EXE_platypus-game"))
        .arg(&game)
        .arg("--screenshot")
        .arg(&png)
        .args(["--frames", "3", flag, "--force"]) // hello.js is a QuickJS bundle
        .env("PLATYPUS_LOG_FILE", &log)
        .status()
        .unwrap();
    assert!(status.success());
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    (png, text)
}

// M9-API-911
#[test]
fn a_screenshot_run_with_fps_saves_the_file_without_errors() {
    let (png, log) = run("on", "--fps");
    assert!(png.is_file(), "no screenshot; log:\n{log}");
    assert!(!log.contains("ERROR"), "{log}");
}

// M9-API-912
#[test]
fn a_screenshot_run_with_no_fps_saves_the_file_without_errors() {
    let (png, log) = run("off", "--no-fps");
    assert!(png.is_file(), "no screenshot; log:\n{log}");
    assert!(!log.contains("ERROR"), "{log}");
}
