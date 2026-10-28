// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `screenshots [name ...] [--no-host-build] [--no-build]`: builds the release host and every example
//! (or the ones named), then renders each with `platypus-app <example> --screenshot` to
//! `tests/output/examples-<name>.png` (git-ignored), for checking by eye. Each run opens a window for
//! a moment.

use std::process::Command;

const USAGE: &str = "usage: screenshots [example ...] [--no-host-build] [--no-build]
(cd tools && cargo run -p platypus_shots --bin screenshots -- [args])";

fn main() {
    let args = platypus_shots::Args::parse(USAGE);
    let root = platypus_shots::repo_root();
    let host = match platypus_shots::release_host(&root, args.build_host) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let output = root.join("tests").join("output");
    if let Err(e) = std::fs::create_dir_all(&output) {
        eprintln!("error: {}: {e}", output.display());
        std::process::exit(1);
    }
    let mut failed = 0;
    for (name, game) in args.games(&root, &[]) {
        if let Err(e) = platypus_shots::build_game(&game, args.build_games) {
            println!("failed to build {name}: {e}");
            failed += 1;
            continue;
        }
        let out = output.join(format!("examples-{name}.png"));
        let ok = Command::new(&host)
            .arg(&game)
            .arg("--screenshot")
            .arg(&out)
            .args(["--frames", "30"])
            .current_dir(&root)
            .status()
            .is_ok_and(|s| s.success());
        if ok && out.is_file() {
            println!("wrote {}", out.display());
        } else {
            println!("failed: {name}");
            failed += 1;
        }
    }
    std::process::exit(i32::from(failed > 0));
}
