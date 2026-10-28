// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `platypus-app <game> --bench` runs a native game without a window, sends its scripted input and
//! writes the timings (needs a GPU, like the other rendering tests; no window opens).

use std::path::PathBuf;
use std::process::Command;

const GAME: &str = r#"
class Counter extends Component {
  frames = 0;
  override onUpdate(_dt: number) {
    this.frames++;
    if (input.getKeyDown("Space")) console.log("pressed-space");
  }
}
register({ Counter });
"#;

const SPEC: &str = r#"{ "warmup": 3, "segments": [
  { "name": "idle", "frames": 10 },
  { "name": "keys", "warmup": 2, "frames": 8,
    "input": [{ "frame": 0, "kind": "key", "code": "Space", "count": 3, "every": 3 }] }
] }"#;

// M9-BEN-03
#[test]
fn a_bench_run_times_a_native_game_without_a_window() {
    let dir = std::env::temp_dir().join(format!("platypus-bench-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let game = dir.join("main.ts");
    std::fs::write(&game, GAME).unwrap();
    let spec = dir.join("bench.json");
    std::fs::write(&spec, SPEC).unwrap();
    let out = dir.join("out.json");
    let log = dir.join("run.log");
    let status = Command::new(env!("CARGO_BIN_EXE_platypus-app"))
        .arg(&game)
        .args(["--bench", "--fixed-dt", "0.02", "--size", "320x200"])
        .arg("--bench-input")
        .arg(&spec)
        .arg("--bench-out")
        .arg(&out)
        .env("PLATYPUS_LOG_FILE", &log)
        .status()
        .unwrap();
    let log_text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(status.success(), "{log_text}");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(report["backend"], "native");
    assert_eq!(report["size"], serde_json::json!([320, 200]));
    assert_eq!(report["fixedDt"], 0.02);
    let segments = report["segments"].as_array().unwrap();
    let names: Vec<&str> = segments
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["idle", "keys"]);
    assert_eq!(segments[1]["frames"], 8);
    assert_eq!(report["total"]["frames"], 18);
    assert!(report["total"]["fps"].as_f64().unwrap() > 0.0);
    assert!(report["startupMs"].as_f64().is_some());
    // The keys reached the game: three presses, three lines.
    assert_eq!(log_text.matches("pressed-space").count(), 3, "{log_text}");
    assert!(!log_text.contains("ERROR"), "{log_text}");
    let _ = std::fs::remove_dir_all(PathBuf::from(&dir));
}
