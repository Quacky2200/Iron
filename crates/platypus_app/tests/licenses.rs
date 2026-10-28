// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! `platypus-app --licenses` prints the third-party notices built into the program and exits 0,
//! without a window (so this needs no GPU).

use std::process::Command;

// M9-LIC-15
#[test]
fn licenses_prints_the_embedded_notices_and_exits_zero() {
    let out = Command::new(env!("CARGO_BIN_EXE_platypus-app"))
        .args(["some-game", "--licenses"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let file = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../THIRD-PARTY-NOTICES.md"
    ))
    .unwrap();
    let printed = String::from_utf8(out.stdout).unwrap();
    assert_eq!(printed.replace("\r\n", "\n"), file.replace("\r\n", "\n"));
}
