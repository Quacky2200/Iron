// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The host executable carries the Platypus icon and version info (linked in by `build.rs`).

// M9-ICO-04
#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn the_host_exe_has_the_platypus_icon_and_product_name() {
    let exe = std::fs::read(env!("CARGO_BIN_EXE_platypus")).unwrap();
    let utf16 = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let has = |needle: &[u8]| exe.windows(needle.len()).any(|w| w == needle);
    assert!(has(&utf16("MAINICON")), "no icon group");
    assert!(has(&utf16("ProductName")) && has(&utf16("Platypus")));
    // The icon images are PNGs.
    assert!(has(b"\x89PNG\r\n\x1a\n"));
}
