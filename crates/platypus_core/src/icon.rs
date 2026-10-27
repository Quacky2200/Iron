// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The Platypus icon (`resources/icon/icon.png`), used by every game that has none of its own,
//! and the dependency-free half of Windows icons: the `.ico` container and the resources of a PE
//! file (`.res`). Decoding and scaling the PNG is left to the users of this module (the `image`
//! crate in `platypus_app` and `platypus-build`). The host's build script includes this file by
//! path, so it uses only `std`.

/// The Platypus icon: a 1254 x 1254 PNG with transparent corners.
pub const DEFAULT_PNG: &[u8] = include_bytes!("../../../resources/icon/icon.png");

/// The sizes of a Windows icon made from a PNG (Explorer's views and the taskbar at 100-200%).
pub const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// The product name the host and packaged games without their own metadata carry.
pub const PRODUCT_NAME: &str = "Platypus";

/// An `.ico` file holding PNG-compressed images (Windows Vista and later read them at every
/// size), from `(size, png bytes)` pairs.
pub fn write_ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = vec![0, 0, 1, 0];
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        out.extend(dir_entry(*size, png.len()));
        out.extend(offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend(png);
    }
    out
}

/// The 12 bytes an icon directory entry starts with (both in `.ico` files and `RT_GROUP_ICON`).
fn dir_entry(size: u32, len: usize) -> Vec<u8> {
    let dim = if size >= 256 { 0 } else { size as u8 };
    let mut e = vec![dim, dim, 0, 0];
    e.extend(1u16.to_le_bytes()); // planes
    e.extend(32u16.to_le_bytes()); // bits per pixel
    e.extend((len as u32).to_le_bytes());
    e
}

pub fn is_ico(bytes: &[u8]) -> bool {
    bytes.len() >= 22 && bytes[..4] == [0, 0, 1, 0] && u16::from_le_bytes([bytes[4], bytes[5]]) > 0
}

/// The (width, height) of every image in an `.ico` (256 is stored as 0).
pub fn ico_sizes(bytes: &[u8]) -> Vec<(u32, u32)> {
    if !is_ico(bytes) {
        return Vec::new();
    }
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..n)
        .filter_map(|i| bytes.get(6 + i * 16..6 + i * 16 + 2))
        .map(|e| {
            let d = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            (d(e[0]), d(e[1]))
        })
        .collect()
}

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// US English.
const LANGUAGE: u16 = 0x0409;

/// A Windows resource file (`.res`, which the MSVC linker and lld-link take as an input) with
/// the icon `images` (`(size, png bytes)`, as for [`write_ico`]) as the program's icon and a
/// version block of `strings` (`ProductName`, ...) and `version`.
pub fn windows_res(
    images: &[(u32, Vec<u8>)],
    strings: &[(&str, &str)],
    version: [u16; 4],
) -> Vec<u8> {
    // A .res file starts with an empty entry.
    let mut out = res_entry(0, Name::Id(0), &[]);
    let mut group = vec![0, 0, 1, 0];
    group.extend((images.len() as u16).to_le_bytes());
    for (i, (size, png)) in images.iter().enumerate() {
        let id = i as u16 + 1;
        out.extend(res_entry(RT_ICON, Name::Id(id), png));
        group.extend(dir_entry(*size, png.len()));
        group.extend(id.to_le_bytes());
    }
    // `MAINICON`, the group `platypus-build package` replaces with a game's own icon.
    out.extend(res_entry(RT_GROUP_ICON, Name::Text("MAINICON"), &group));
    let info = version_info(strings, version);
    out.extend(res_entry(RT_VERSION, Name::Id(1), &info));
    out
}

enum Name<'a> {
    Id(u16),
    Text(&'a str),
}

/// One resource: a header (numeric type, numeric or text name, flags, language), the data,
/// padding to 4 bytes.
fn res_entry(kind: u16, name: Name<'_>, data: &[u8]) -> Vec<u8> {
    let mut head = vec![0xff, 0xff];
    head.extend(kind.to_le_bytes());
    match name {
        Name::Id(id) => {
            head.extend([0xff, 0xff]);
            head.extend(id.to_le_bytes());
        }
        Name::Text(s) => head.extend(utf16z(s)),
    }
    pad4(&mut head);
    let mut e = Vec::with_capacity(24 + head.len() + data.len());
    e.extend((data.len() as u32).to_le_bytes());
    e.extend((8 + head.len() as u32 + 16).to_le_bytes()); // header size
    e.extend(head);
    e.extend(0u32.to_le_bytes()); // data version
    let flags: u16 = if kind == 0 { 0 } else { 0x1030 }; // moveable, pure, discardable
    e.extend(flags.to_le_bytes());
    e.extend(if kind == 0 { 0 } else { LANGUAGE }.to_le_bytes());
    e.extend(0u32.to_le_bytes()); // version
    e.extend(0u32.to_le_bytes()); // characteristics
    e.extend(data);
    pad4(&mut e);
    e
}

fn pad4(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// A VERSIONINFO node: length, value length, type, key, then the value and the children, each
/// starting on a 4-byte boundary.
fn node(key: &str, value: &[u8], value_len: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut n = vec![0, 0];
    n.extend(value_len.to_le_bytes());
    n.extend(u16::from(text).to_le_bytes());
    n.extend(utf16z(key));
    pad4(&mut n);
    n.extend(value);
    for c in children {
        pad4(&mut n);
        n.extend(c);
    }
    let len = n.len() as u16;
    n[..2].copy_from_slice(&len.to_le_bytes());
    n
}

/// The `VS_VERSIONINFO` block: the fixed part (an application for Windows NT) with `version`
/// as both file and product version, a US English string table and the translation.
fn version_info(strings: &[(&str, &str)], version: [u16; 4]) -> Vec<u8> {
    let ms = (u32::from(version[0]) << 16) | u32::from(version[1]);
    let ls = (u32::from(version[2]) << 16) | u32::from(version[3]);
    let fixed: Vec<u8> = [
        0xFEEF_04BD, // signature
        0x0001_0000, // structure version
        ms,
        ls,
        ms,
        ls,
        0x3F,        // flags mask
        0,           // flags
        0x0004_0004, // VOS_NT_WINDOWS32
        1,           // VFT_APP
        0,
        0,
        0,
    ]
    .iter()
    .flat_map(|d: &u32| d.to_le_bytes())
    .collect();
    let entries: Vec<Vec<u8>> = strings
        .iter()
        .map(|(k, v)| {
            let value = utf16z(v);
            node(k, &value, (value.len() / 2) as u16, true, &[])
        })
        .collect();
    let table = node("040904b0", &[], 0, true, &entries);
    let string_info = node("StringFileInfo", &[], 0, true, &[table]);
    let translation = [LANGUAGE.to_le_bytes(), 0x04b0u16.to_le_bytes()].concat();
    let var = node("Translation", &translation, 4, false, &[]);
    let var_info = node("VarFileInfo", &[], 0, true, &[var]);
    node(
        "VS_VERSION_INFO",
        &fixed,
        fixed.len() as u16,
        false,
        &[string_info, var_info],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-ICO-01
    #[test]
    fn the_default_icon_is_a_square_png() {
        assert_eq!(&DEFAULT_PNG[..8], b"\x89PNG\r\n\x1a\n");
        let w = u32::from_be_bytes(DEFAULT_PNG[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(DEFAULT_PNG[20..24].try_into().unwrap());
        assert_eq!((w, h), (1254, 1254));
    }

    // M9-ICO-02
    #[test]
    fn an_ico_lists_its_sizes_and_a_res_file_holds_icons_and_version_strings() {
        let images: Vec<(u32, Vec<u8>)> =
            ICO_SIZES.iter().map(|&s| (s, vec![s as u8; 10])).collect();
        let ico = write_ico(&images);
        assert!(is_ico(&ico));
        assert_eq!(
            ico_sizes(&ico),
            ICO_SIZES.iter().map(|&s| (s, s)).collect::<Vec<_>>()
        );
        let res = windows_res(&images, &[("ProductName", PRODUCT_NAME)], [1, 2, 3, 0]);
        // The leading empty entry, then every entry 4-byte aligned.
        assert_eq!(&res[..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
        assert_eq!(res.len() % 4, 0);
        let utf16 = |s: &str| {
            s.encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<u8>>()
        };
        let has = |needle: &[u8]| res.windows(needle.len()).any(|w| w == needle);
        assert!(has(&utf16("ProductName")) && has(&utf16(PRODUCT_NAME)));
        assert!(has(&utf16("VS_VERSION_INFO")));
        // Walk the entries: 7 icons, one group, one version block.
        let mut kinds = Vec::new();
        let mut at = 0;
        while at < res.len() {
            let word = |i: usize| u32::from_le_bytes(res[at + i..at + i + 4].try_into().unwrap());
            let (size, header) = (word(0) as usize, word(4) as usize);
            assert_eq!(header % 4, 0);
            kinds.push(u16::from_le_bytes([res[at + 10], res[at + 11]]));
            at += (header + size).div_ceil(4) * 4;
        }
        assert_eq!(at, res.len());
        assert_eq!(kinds, [0, 3, 3, 3, 3, 3, 3, 3, 14, 16]);
        assert!(has(&utf16("MAINICON")));
    }
}
