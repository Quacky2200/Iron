// SPDX-License-Identifier: Apache-2.0

//! Every tracked source file we wrote starts with `SPDX-License-Identifier: Apache-2.0`
//! (README.md, "Licence"). New files need the line in their comment syntax, after a `#!` line.

use std::path::Path;
use std::process::Command;

const ID: &str = "SPDX-License-Identifier: Apache-2.0";
const EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "mts", "wgsl", "css", "py", "toml", "sh",
];

/// Paths (or prefixes) without the header, and why.
const EXCLUDED: &[&str] = &[
    // Generated: rewritten by their generators, which would drop the line.
    "crates/iron_render_api/src/generated.rs",
    "tools/iron_ts_rt/src/intl_pad.rs",
    "tools/iron_ts_tests/bench/datefmt.ts",
    "tools/iron_ts_tests/bench/gen_datefmt_expected.ts",
    "tools/iron_ts_tests/bench/intl4.ts",
    "tools/iron_ts_tests/bench/promises.ts",
    "tools/iron_ts_tests/bench/typed.ts",
    // Third-party ports (MiniGameEngine, the zombie game) and copies of them.
    "examples/sine-circle-demo/",
    "examples/raindrop-demo/",
    "tools/iron_ts_tests/bench/sine.ts",
    "tools/iron_ts_tests/bench/sine_demo.ts",
    "tools/iron_ts_tests/bench/sine_demo_full.ts",
    "tools/iron_ts_tests/fixtures/sim.ts",
    "tools/iron_ts_tests/fixtures/sim.test.ts",
];

fn excluded(path: &str) -> bool {
    // Built fixtures (`bun run build:fixtures`).
    let built_fixture = path
        .strip_prefix("tests/fixtures/")
        .is_some_and(|rest| rest.ends_with(".js") && !rest.contains('/'));
    built_fixture
        || EXCLUDED
            .iter()
            .any(|e| path == *e || (e.ends_with('/') && path.starts_with(e)))
}

// M9-LIC-01
#[test]
fn every_tracked_source_file_has_the_spdx_header() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(out) = Command::new("git")
        .arg("ls-files")
        .current_dir(&root)
        .output()
    else {
        eprintln!("git is not available: skipped");
        return;
    };
    if !out.status.success() {
        eprintln!("not a git checkout: skipped");
        return;
    }
    let listing = String::from_utf8_lossy(&out.stdout);
    let mut missing = Vec::new();
    let mut checked = 0;
    for path in listing.lines() {
        let ext = path.rsplit_once('.').map_or("", |(_, e)| e);
        if !EXTENSIONS.contains(&ext) || excluded(path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(path)) else {
            continue; // deleted in the working tree
        };
        checked += 1;
        if !text.lines().take(3).any(|l| l.contains(ID)) {
            missing.push(path);
        }
    }
    assert!(checked > 500, "only {checked} source files found");
    assert!(
        missing.is_empty(),
        "{} file(s) lack `{ID}` in their first lines:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}
