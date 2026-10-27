// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The host executables carry the Platypus icon and version info (linked in by `build.rs`).

// M9-ICO-04
#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn both_host_exes_have_the_platypus_icon_and_product_name() {
    let utf16 = |s: &str| -> Vec<u8> { s.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    for path in [
        env!("CARGO_BIN_EXE_platypus-game"),
        env!("CARGO_BIN_EXE_platypus-app"),
    ] {
        let exe = std::fs::read(path).unwrap();
        let has = |needle: &[u8]| exe.windows(needle.len()).any(|w| w == needle);
        assert!(has(&utf16("MAINICON")), "no icon group in {path}");
        assert!(
            has(&utf16("ProductName")) && has(&utf16("Platypus")),
            "{path}"
        );
        // The icon images are PNGs.
        assert!(has(b"\x89PNG\r\n\x1a\n"), "{path}");
    }
}
