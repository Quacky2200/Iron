// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! What `screenshots` and `progress-shots` share: the checkout, the arguments, the release host and
//! building the examples with `platypus_build`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The checkout this tool was built from.
pub fn repo_root() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    std::path::absolute(&dir).unwrap_or(dir)
}

/// The command line: example names (none: every example) and the switches both tools take.
pub struct Args {
    pub names: Vec<String>,
    /// `--no-host-build`: use `target/release/platypus-app` as it is.
    pub build_host: bool,
    /// `--no-build`: run the examples' `build/` as it is.
    pub build_games: bool,
}

impl Args {
    pub fn parse(usage: &str) -> Args {
        let mut args = Args {
            names: Vec::new(),
            build_host: true,
            build_games: true,
        };
        for a in std::env::args().skip(1) {
            match a.as_str() {
                "--no-host-build" => args.build_host = false,
                "--no-build" => args.build_games = false,
                "--help" | "-h" => {
                    println!("{usage}");
                    std::process::exit(0);
                }
                s if s.starts_with("--") => {
                    eprintln!("unexpected argument {s}\n{usage}");
                    std::process::exit(2);
                }
                s => args.names.push(s.to_owned()),
            }
        }
        args
    }

    /// The examples to run: the names given, else every game in `examples/` but `skip`.
    pub fn games(&self, root: &Path, skip: &[&str]) -> Vec<(String, PathBuf)> {
        let examples = root.join("examples");
        if !self.names.is_empty() {
            return self
                .names
                .iter()
                .map(|n| (n.clone(), examples.join(n)))
                .collect();
        }
        platypus_build::find_games(&examples)
            .into_iter()
            .filter_map(|p| Some((p.file_name()?.to_string_lossy().into_owned(), p)))
            .filter(|(n, _)| !skip.contains(&n.as_str()))
            .collect()
    }
}

/// The release host, built first unless `--no-host-build` (the screenshots are of what ships).
pub fn release_host(root: &Path, build: bool) -> Result<PathBuf, String> {
    if build {
        eprintln!("cargo build --release -p platypus_app --bin platypus-app");
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "-p",
                "platypus_app",
                "--bin",
                "platypus-app",
            ])
            .current_dir(root)
            .status()
            .map_err(|e| format!("cargo: {e}"))?;
        if !status.success() {
            return Err("building the release host failed".into());
        }
    }
    let host = root
        .join("target")
        .join("release")
        .join(format!("platypus-app{}", std::env::consts::EXE_SUFFIX));
    if host.is_file() {
        Ok(host)
    } else {
        Err(format!(
            "no {}: cargo build --release -p platypus_app --bin platypus-app",
            host.display()
        ))
    }
}

/// `platypus-build build <game>` (no `tsc`), unless `--no-build`.
pub fn build_game(game: &Path, build: bool) -> Result<(), String> {
    if !build {
        return Ok(());
    }
    platypus_build::build(
        game,
        &platypus_build::Options {
            no_tsc: true,
            ..Default::default()
        },
    )
    .map(|_| ())
}
