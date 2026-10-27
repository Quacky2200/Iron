// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The hashes a pack uses: a 128-bit content hash (deduplication, incremental builds, pack ids),
//! a 64-bit path hash (the path index) and CRC-32 (per-chunk integrity). All fixed functions of
//! the bytes, so builds are reproducible and readers on any machine agree.

use std::hash::Hasher;

use siphasher::sip128::{Hasher128, SipHasher13};

/// Fixed keys: the hash must be the same in every build ("iron" / "pack").
const KEYS: (u64, u64) = (0x6972_6f6e, 0x7061_636b);

/// SipHash-1-3, 128 bits, of `bytes`. Not a defence against crafted collisions (packs are not a
/// security boundary); accidental ones among a game's assets are out of reach at 128 bits.
pub fn content_hash(bytes: &[u8]) -> [u8; 16] {
    let mut h = SipHasher13::new_with_keys(KEYS.0, KEYS.1);
    h.write(bytes);
    h.finish128().as_bytes()
}

/// FNV-1a, 64 bits, of a normalized path (see [`normalize_path`]).
pub fn path_hash(path: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub fn crc32(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

/// A game path as packs store it: forward slashes, no leading `./`, no empty or `.` parts.
/// `None` for anything [`platypus_core::resolve_asset`] refuses (absolute, `..`, schemes, drives).
/// Case is kept: paths are case-sensitive, as on Linux and Android.
pub fn normalize_path(path: &str) -> Option<String> {
    let p = path.trim().replace('\\', "/");
    platypus_core::resolve_asset_quiet(std::path::Path::new(""), &p)?;
    let parts: Vec<&str> = p
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-API-1100
    #[test]
    fn hashes_are_fixed_functions_of_the_bytes() {
        assert_eq!(content_hash(b"iron"), content_hash(b"iron"));
        assert_ne!(content_hash(b"iron"), content_hash(b"irom"));
        // FNV-1a test vectors.
        assert_eq!(path_hash(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(path_hash("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(
            normalize_path("./assets\\tex//a.png").as_deref(),
            Some("assets/tex/a.png")
        );
        for bad in [
            "",
            "../x.png",
            "/etc/x",
            "C:/x",
            "a/../../b",
            "https://x/y.png",
        ] {
            assert_eq!(normalize_path(bad), None, "{bad:?}");
        }
    }
}
