//! Windows resources of the packaged executable: the icon and the VERSIONINFO block, written
//! into a copy of the host's PE file with the `editpe` crate (no `rc.exe`, no external tool).
//! Must run before the payload is appended and before any signing (`docs/research/iron-code-signing.md`).

use crate::package::Metadata;
use editpe::constants::RT_GROUP_ICON;
use editpe::types::{VersionU16, VersionU32};
use editpe::{Image, ResourceEntryName, VersionInfo, VersionStringTable};

/// US English, Unicode: the key of the string table and the `Translation` value.
const LANG_KEY: &str = "040904b0";

/// `"1.2.3-beta"` as the four numbers of a Windows file version (`[1, 2, 3, 0]`).
pub fn version_quad(version: &str) -> [u16; 4] {
    let mut out = [0u16; 4];
    let numeric = version.split(['-', '+']).next().unwrap_or("");
    for (slot, part) in out.iter_mut().zip(numeric.split('.')) {
        *slot = part.trim().parse().unwrap_or(0);
    }
    out
}

/// The VERSIONINFO block for `meta`.
pub fn version_info(meta: &Metadata) -> VersionInfo {
    let q = version_quad(&meta.version);
    let packed = VersionU32 {
        major: (u32::from(q[0]) << 16) | u32::from(q[1]),
        minor: (u32::from(q[2]) << 16) | u32::from(q[3]),
    };
    let mut table = VersionStringTable {
        key: LANG_KEY.into(),
        ..Default::default()
    };
    let file = meta.file_name();
    let fields = [
        ("CompanyName", meta.company.as_str()),
        ("FileDescription", meta.description.as_str()),
        ("FileVersion", meta.version.as_str()),
        ("InternalName", meta.exe.as_str()),
        ("LegalCopyright", meta.copyright.as_str()),
        ("OriginalFilename", file.as_str()),
        ("ProductName", meta.title.as_str()),
        ("ProductVersion", meta.version.as_str()),
    ];
    for (k, v) in fields {
        table.strings.insert(k.into(), v.into());
    }
    if meta.license != "private" {
        table
            .strings
            .insert("Comments".into(), format!("License: {}", meta.license));
    }
    let mut info = VersionInfo::default();
    info.info.file_version = packed;
    info.info.product_version = packed;
    info.strings = vec![table];
    info.vars = vec![VersionU16 {
        major: 0x0409,
        minor: 0x04b0,
    }];
    info
}

/// `host` (a PE file) with the icon (`.ico` bytes) and version info of `meta` set.
pub fn set_resources(
    host: Vec<u8>,
    meta: &Metadata,
    ico: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut image = Image::parse(host).map_err(|e| format!("host is not a PE file: {e}"))?;
    let mut res = image.resource_directory().cloned().unwrap_or_default();
    if let Some(ico) = ico {
        // Replace the host's own icon rather than add to it.
        res.remove_main_icon().map_err(|e| format!("icon: {e}"))?;
        res.set_main_icon(ico.to_vec())
            .map_err(|e| format!("icon: {e}"))?;
    }
    res.set_version_info(&version_info(meta))
        .map_err(|e| format!("version info: {e}"))?;
    image
        .set_resource_directory(res)
        .map_err(|e| format!("resources: {e}"))?;
    let mut out = Vec::new();
    image
        .write_writer(&mut out)
        .map_err(|e| format!("writing the executable: {e}"))?;
    Ok(out)
}

/// What a PE file's resources say, for checks: the string table of the version info and the
/// sizes (as `ico_sizes`) of the main icon group.
pub struct Resources {
    pub strings: Vec<(String, String)>,
    pub file_version: [u16; 4],
    pub product_version: [u16; 4],
    pub icon_sizes: Vec<(u32, u32)>,
}

pub fn read_resources(pe: &[u8]) -> Result<Resources, String> {
    let image = Image::parse(pe).map_err(|e| format!("not a PE file: {e}"))?;
    let res = image
        .resource_directory()
        .ok_or("no resource directory")?
        .clone();
    let vi = res
        .get_version_info()
        .map_err(|e| e.to_string())?
        .ok_or("no version info")?;
    let strings = vi
        .strings
        .first()
        .map(|t| {
            t.strings
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    let split = |v: VersionU32| {
        let (a, b) = (v.major, v.minor);
        [(a >> 16) as u16, a as u16, (b >> 16) as u16, b as u16]
    };
    let mut icon_sizes = Vec::new();
    let group = res
        .root()
        .get(ResourceEntryName::ID(RT_GROUP_ICON as u32))
        .and_then(|e| e.as_table())
        .and_then(|t| t.get(ResourceEntryName::from_string("MAINICON")))
        .and_then(|e| e.as_table());
    if let Some(table) = group
        && let Some(name) = table.entries().first()
        && let Some(data) = table.get(*name).and_then(|e| e.as_data())
    {
        // GRPICONDIR: reserved, type, count; then 14-byte entries starting with width, height.
        let d = data.data();
        let n = d
            .get(4..6)
            .map_or(0, |c| u16::from_le_bytes([c[0], c[1]]) as usize);
        for i in 0..n {
            if let Some(e) = d.get(6 + i * 14..6 + i * 14 + 2) {
                let dim = |b: u8| if b == 0 { 256 } else { u32::from(b) };
                icon_sizes.push((dim(e[0]), dim(e[1])));
            }
        }
    }
    Ok(Resources {
        strings,
        file_version: split(vi.info.file_version),
        product_version: split(vi.info.product_version),
        icon_sizes,
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use iron_core::project::Project;
    use serde_json::json;

    fn meta() -> Metadata {
        let pkg = json!({"name": "demo", "version": "1.2.3", "iron": {
            "title": "Demo Game", "company": "Acme Ltd", "copyright": "(c) 2026 Acme",
            "description": "A demo", "exe": "demo"}});
        Metadata::from_project(&Project::from_package_json(&pkg, "demo"))
    }

    // M9-TS-2430
    #[test]
    fn version_info_and_icon_round_trip_through_a_pe_file() {
        // This test program is itself a PE file.
        let host = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        let png = {
            let img = image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 200, 30, 255]));
            let mut out = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                .unwrap();
            out
        };
        let ico = crate::icon::to_ico(&png).unwrap();
        let out = set_resources(host.clone(), &meta(), Some(&ico)).unwrap();
        let r = read_resources(&out).unwrap();
        let get = |k: &str| {
            r.strings
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("ProductName"), Some("Demo Game"));
        assert_eq!(get("CompanyName"), Some("Acme Ltd"));
        assert_eq!(get("FileDescription"), Some("A demo"));
        assert_eq!(get("LegalCopyright"), Some("(c) 2026 Acme"));
        assert_eq!(get("FileVersion"), Some("1.2.3"));
        assert_eq!(get("ProductVersion"), Some("1.2.3"));
        assert_eq!(get("OriginalFilename"), Some("demo.exe"));
        assert_eq!(r.file_version, [1, 2, 3, 0]);
        assert_eq!(r.product_version, [1, 2, 3, 0]);
        assert_eq!(r.icon_sizes, [(16, 16), (32, 32), (48, 48), (256, 256)]);
        // A payload appended after the edit is found again.
        let mut with = out;
        iron_core::payload::write_payload(&mut with, &[("a".into(), vec![1])]).unwrap();
        let found = iron_core::payload::Archive::new(std::io::Cursor::new(with)).unwrap();
        assert!(found.is_some());
    }
}
