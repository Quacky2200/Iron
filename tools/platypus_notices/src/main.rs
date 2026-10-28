// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `platypus_notices [--check] [--root <repo>]`: writes `THIRD-PARTY-NOTICES.md` at the repository
//! root, or with `--check` exits 1 when the committed file is not what it would write. Run it
//! (`cargo run -p platypus_notices` in tools/) whenever Cargo.lock or `hand-written.md` changes.

use platypus_notices::{FILE, diff, generate};
use std::path::PathBuf;

const USAGE: &str = "usage: platypus_notices [--check] [--root <repository>]";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut check = false;
    // tools/platypus_notices -> the repository.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut root = manifest
        .ancestors()
        .nth(2)
        .unwrap_or(&manifest)
        .to_path_buf();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--root" => match args.next() {
                Some(r) => root = PathBuf::from(r),
                None => {
                    eprintln!("{USAGE}");
                    return 2;
                }
            },
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let notices = match generate(&root) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    for a in &notices.allowed {
        println!("allowed: {a}");
    }
    for k in &notices.unused_allow {
        eprintln!("warning: allow entry `{k}` matches no crate (remove it)");
    }
    let sections: Vec<String> = notices
        .sections
        .iter()
        .map(|(id, n)| format!("{id} {n}"))
        .collect();
    println!(
        "{} crates, {} licences: {}",
        notices.crates,
        sections.len(),
        sections.join(", ")
    );
    let path = root.join(FILE);
    if check {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        return match diff(&current, &notices.text) {
            None => {
                println!("{} is current", path.display());
                0
            }
            Some(d) => {
                eprint!("{d}");
                eprintln!("run `cargo run -p platypus_notices` in tools/ to rewrite it");
                1
            }
        };
    }
    match std::fs::write(&path, &notices.text) {
        Ok(()) => {
            println!("wrote {} ({} bytes)", path.display(), notices.text.len());
            0
        }
        Err(e) => {
            eprintln!("error: {}: {e}", path.display());
            1
        }
    }
}
