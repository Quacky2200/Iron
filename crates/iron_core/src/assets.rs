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

/// Resolves `url` against the game folder `root`, refusing anything that could
/// leave it: absolute paths, drive letters, URL schemes and `..` components.
/// Game scripts can't use this to read arbitrary files.
pub fn resolve_asset(root: &Path, url: &str) -> Option<PathBuf> {
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
}
