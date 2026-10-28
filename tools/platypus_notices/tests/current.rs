// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The committed `THIRD-PARTY-NOTICES.md` is what the generator writes from Cargo.lock now, so a
//! dependency change cannot land without its notices. Skipped where `cargo metadata --offline`
//! cannot run or a crate's registry source is not on this machine.

use platypus_notices::{Error, FILE, diff, generate};
use std::path::Path;

// M9-LIC-02
#[test]
fn notices_are_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let notices = match generate(&root) {
        Ok(n) => n,
        Err(Error::Cargo(e)) => {
            eprintln!("cargo metadata --offline did not run ({e}): skipped");
            return;
        }
        Err(Error::MissingSource(v)) => {
            eprintln!(
                "no registry source for {} crate(s) ({}...): skipped",
                v.len(),
                v[0]
            );
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let committed = std::fs::read_to_string(root.join(FILE)).unwrap_or_default();
    if let Some(d) = diff(&committed, &notices.text) {
        panic!("{d}run `cargo run -p platypus_notices` in tools/ and commit {FILE}");
    }
}
