// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The main thread runs the game's compiled code and the UI reconciler, which recurse over the
// element tree (a long Markdown page in `examples/md-reader` nests a few hundred calls deep).
// Windows gives a program's main thread 1 MB unless the executable asks for more; a debug build's
// frames are large, so ask for 16 MB (other platforms default to 8 MB).
fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target == "windows" {
        if env == "msvc" {
            println!("cargo:rustc-link-arg-bins=/STACK:16777216");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,16777216");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
