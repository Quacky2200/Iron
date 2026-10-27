// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The QuickJS backend is deprecated: `platypus_app` refuses a QuickJS bundle unless `--force` is
//! given, and leaves native bundles alone (headless `--screenshot` runs; they need a GPU, like
//! the other rendering tests).

use std::path::PathBuf;
use std::process::Command;

struct Run {
    ok: bool,
    png: PathBuf,
    log: String,
}

fn run(name: &str, game: PathBuf, extra: &[&str]) -> Run {
    let dir = std::env::temp_dir().join(format!("platypus-qjs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("out.png");
    let log = dir.join("run.log");
    let output = Command::new(env!("CARGO_BIN_EXE_platypus-game"))
        .arg(&game)
        .arg("--screenshot")
        .arg(&png)
        .args(["--frames", "3", "--no-fps"])
        .args(extra)
        .env("PLATYPUS_LOG_FILE", &log)
        .output()
        .unwrap();
    Run {
        ok: output.status.success(),
        png,
        log: std::fs::read_to_string(&log).unwrap_or_default(),
    }
}

fn quickjs_game() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hello.js")
}

// M9-APP-03
#[test]
fn a_quickjs_game_without_force_exits_non_zero_with_the_deprecation_message() {
    let r = run("refused", quickjs_game(), &[]);
    assert!(!r.ok, "ran without --force; log:\n{}", r.log);
    assert!(!r.png.exists());
    assert!(r.log.contains("QuickJS backend is deprecated"), "{}", r.log);
    assert!(r.log.contains("--force"), "{}", r.log);
    assert!(r.log.contains("platypus-build build"), "{}", r.log);
}

// M9-APP-04
#[test]
fn a_quickjs_game_with_force_runs_and_warns_once() {
    let r = run("forced", quickjs_game(), &["--force"]);
    assert!(r.ok, "log:\n{}", r.log);
    assert!(r.png.is_file());
    assert_eq!(
        r.log.matches("QuickJS backend is deprecated").count(),
        1,
        "{}",
        r.log
    );
}

// M9-APP-05
#[test]
fn a_native_game_is_unaffected_by_the_gate() {
    let dir = std::env::temp_dir().join(format!("platypus-qjs-native-src-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("game.ts");
    std::fs::write(&src, "console.log(\"native hello\");\n").unwrap();
    let r = run("native", src, &[]);
    assert!(r.ok, "log:\n{}", r.log);
    assert!(r.png.is_file());
    assert!(!r.log.contains("deprecated"), "{}", r.log);
}
