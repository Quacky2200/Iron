//! Resolving asset paths (`url(...)` images, fonts) that scripts give the engine.

use std::path::{Component, Path, PathBuf};

/// Decodes a `data:` URL (`data:[<type>][;base64],<data>`), as Bun emits for small
/// assets referenced from CSS. Returns `None` if `url` isn't a valid data URL.
pub fn decode_data_url(url: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let rest = url.trim().strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    if meta.ends_with(";base64") {
        let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        base64::engine::general_purpose::STANDARD
            .decode(cleaned)
            .ok()
    } else {
        Some(percent_decode(data))
    }
}

fn percent_decode(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..(i + 3).min(s.len())], 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

static PLAN: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
static WARNED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Dev loop: the game paths a packaged build would hold (`dist/assets.plan`, written by
/// `iron-build build`). Afterwards [`resolve_asset`] warns once for a file that exists but is not
/// in it, since a packaged game would not find it.
pub fn set_pack_plan(keys: impl IntoIterator<Item = String>) {
    let _ = PLAN.set(keys.into_iter().collect());
}

/// The warning for a loaded `key` that a packaged build would not include, if any.
pub fn plan_warning(plan: &std::collections::HashSet<String>, key: &str) -> Option<String> {
    let skip = key.ends_with(".meta") || key.starts_with("dist/") || key.starts_with(".iron/");
    (!skip && !plan.contains(key)).then(|| {
        format!(
            "asset \"{key}\" loaded, but a packaged build would not include it (add it to \"include\" in project.json)"
        )
    })
}

fn check_plan(url: &str, path: &Path) {
    let Some(plan) = PLAN.get() else { return };
    let key = url
        .split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .collect::<Vec<_>>()
        .join("/");
    if let Some(w) = plan_warning(plan, &key)
        && path.is_file()
        && let Ok(mut seen) = WARNED.lock()
        && !seen.contains(&key)
    {
        seen.push(key);
        tracing::warn!("{w}");
    }
}

/// Resolves `url` against the game folder `root`, refusing anything that could
/// leave it: absolute paths, drive letters, URL schemes and `..` components.
/// Game scripts can't use this to read arbitrary files.
pub fn resolve_asset(root: &Path, url: &str) -> Option<PathBuf> {
    let path = resolve_unchecked(root, url)?;
    check_plan(url.trim(), &path);
    Some(path)
}

fn resolve_unchecked(root: &Path, url: &str) -> Option<PathBuf> {
    let url = url.trim();
    if url.is_empty() || url.contains(':') || url.starts_with(['/', '\\']) {
        return None;
    }
    let relative = Path::new(url);
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    Some(root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_data_urls() {
        assert_eq!(
            decode_data_url("data:text/plain;base64,aGk=").as_deref(),
            Some(&b"hi"[..])
        );
        assert_eq!(decode_data_url("data:,a%20b").as_deref(), Some(&b"a b"[..]));
        assert_eq!(decode_data_url("data:;base64,!!!"), None);
        assert_eq!(decode_data_url("assets/x.png"), None);
    }

    #[test]
    fn urls_cannot_escape_the_game_folder() {
        let root = Path::new("game");
        assert_eq!(
            resolve_asset(root, "ui/panel.png"),
            Some(root.join("ui/panel.png"))
        );
        assert_eq!(resolve_asset(root, "./a.png"), Some(root.join("./a.png")));
        for bad in [
            "",
            "../secret.png",
            "ui/../../x.png",
            "/etc/passwd",
            "\\\\server\\x.png",
            "C:/x.png",
            "https://example.com/x.png",
        ] {
            assert_eq!(resolve_asset(root, bad), None, "{bad:?}");
        }
    }

    // M9-API-1143
    #[test]
    fn a_loaded_file_outside_the_plan_gets_one_warning_text() {
        let plan: std::collections::HashSet<String> =
            ["assets/a.png".to_owned()].into_iter().collect();
        assert_eq!(plan_warning(&plan, "assets/a.png"), None);
        assert_eq!(plan_warning(&plan, "assets/a.png.meta"), None);
        assert_eq!(plan_warning(&plan, "dist/big-1.ttf"), None);
        let w = plan_warning(&plan, "assets/b.wav").unwrap();
        assert!(w.contains("assets/b.wav") && w.contains("include"), "{w}");
    }
}
