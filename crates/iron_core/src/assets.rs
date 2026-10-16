//! Resolving asset paths (`url(...)` images, fonts) that scripts give the engine.

use std::path::{Component, Path, PathBuf};

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
