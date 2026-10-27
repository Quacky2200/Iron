// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! CSS bundling, what `bun build` did for the examples: the stylesheets a program imports are joined in
//! import order (`@import` is inlined), and every `url()` is rewritten: a small file becomes a `data:`
//! URL, a larger one is copied into `dist/` under a hashed name and the URL is relative to
//! `dist/game.css` (which is where `platypus_app` reads it from, see `load_game_css`).

use base64::Engine as _;
use std::path::{Path, PathBuf};

/// Files up to this many bytes are inlined as `data:` URLs.
pub const INLINE_LIMIT: usize = 32 * 1024;

#[derive(Debug, Default)]
pub struct CssOut {
    pub css: String,
    /// Files copied into `dist/` (names relative to it).
    pub copied: Vec<String>,
    pub warnings: Vec<String>,
}

fn mime(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn fnv(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in bytes {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Bundles `files` (paths on disk, in order); copied assets go to `dist`.
pub fn bundle(files: &[PathBuf], dist: &Path) -> std::io::Result<CssOut> {
    let mut out = CssOut::default();
    let mut seen = Vec::new();
    for f in files {
        let text = file(f, dist, &mut out, &mut seen, 0)?;
        out.css.push_str(&text);
        if !out.css.ends_with('\n') {
            out.css.push('\n');
        }
    }
    Ok(out)
}

fn file(
    path: &Path,
    dist: &Path,
    out: &mut CssOut,
    seen: &mut Vec<PathBuf>,
    depth: usize,
) -> std::io::Result<String> {
    if depth > 16 || seen.contains(&path.to_path_buf()) {
        return Ok(String::new());
    }
    seen.push(path.to_path_buf());
    let text = std::fs::read_to_string(path)
        .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut result = String::new();
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let i_url = find_ci(rest, "url(");
        let i_imp = find_ci(rest, "@import");
        match (i_url, i_imp) {
            (_, Some(i)) if i_url.is_none_or(|u| i < u) => {
                result.push_str(&rest[..i]);
                let after = &rest[i + "@import".len()..];
                let end = after.find(';').map_or(after.len(), |e| e + 1);
                let stmt = &after[..end];
                match import_target(stmt) {
                    Some(t) if !is_external(&t) => {
                        let p = dir.join(&t);
                        if p.exists() {
                            result.push_str(&file(&p, dist, out, seen, depth + 1)?);
                        } else {
                            out.warnings
                                .push(format!("{}: @import {t} not found", path.display()));
                            result.push_str(&rest[i..i + "@import".len() + end]);
                        }
                    }
                    _ => result.push_str(&rest[i..i + "@import".len() + end]),
                }
                rest = &rest[i + "@import".len() + end..];
            }
            (Some(i), _) => {
                result.push_str(&rest[..i + 4]);
                let after = &rest[i + 4..];
                let Some(close) = after.find(')') else {
                    result.push_str(after);
                    break;
                };
                let raw = after[..close].trim();
                let target = raw.trim_matches(['"', '\'']);
                if is_external(target) || target.is_empty() {
                    result.push_str(&after[..close]);
                } else {
                    result.push_str(&rewrite(dir, target, dist, out, path)?);
                }
                rest = &after[close..];
            }
            _ => {
                result.push_str(rest);
                break;
            }
        }
    }
    Ok(result)
}

fn find_ci(s: &str, needle: &str) -> Option<usize> {
    s.to_ascii_lowercase().find(needle)
}

fn is_external(t: &str) -> bool {
    t.starts_with("data:")
        || t.starts_with("http:")
        || t.starts_with("https:")
        || t.starts_with("//")
        || t.starts_with('#')
        || t.starts_with("var(")
}

fn import_target(stmt: &str) -> Option<String> {
    let s = stmt.trim().trim_end_matches(';').trim();
    let s = s
        .strip_prefix("url(")
        .map_or(s, |r| r.split(')').next().unwrap_or(r));
    let t = s.split_whitespace().next()?.trim_matches(['"', '\'']);
    Some(t.to_string())
}

/// The replacement text of one `url(...)` argument (without the parentheses).
fn rewrite(
    dir: &Path,
    target: &str,
    dist: &Path,
    out: &mut CssOut,
    css: &Path,
) -> std::io::Result<String> {
    let clean = target.split(['?', '#']).next().unwrap_or(target);
    let p = dir.join(clean);
    let Ok(bytes) = std::fs::read(&p) else {
        out.warnings
            .push(format!("{}: url({target}) not found", css.display()));
        return Ok(format!("\"{target}\""));
    };
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if bytes.len() <= INLINE_LIMIT {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Ok(format!("\"data:{};base64,{b64}\"", mime(&ext)));
    }
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
    let name = format!("{stem}-{:08x}.{ext}", fnv(&bytes));
    std::fs::create_dir_all(dist)?;
    std::fs::write(dist.join(&name), &bytes)?;
    if !out.copied.contains(&name) {
        out.copied.push(name.clone());
    }
    Ok(format!("\"{name}\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "platypus_build_css_{name}_{}",
            crate::assets::new_guid()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // M9-TS-1840
    #[test]
    fn small_urls_are_inlined_large_ones_copied_with_a_hash_and_imports_are_joined() {
        let d = temp("css");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/small.png"), b"PNG").unwrap();
        std::fs::write(d.join("assets/big.ttf"), vec![7u8; INLINE_LIMIT + 1]).unwrap();
        std::fs::write(d.join("src/b.css"), ".b { color: red }").unwrap();
        std::fs::write(
            d.join("src/a.css"),
            "@import \"./b.css\";\n.a { background: URL('../assets/small.png') }\n@font-face { src: url(\"../assets/big.ttf\"); }\n.c { background: url(data:x) url(../nope.png) }",
        )
        .unwrap();
        std::fs::write(d.join("src/z.css"), ".z{}").unwrap();
        let dist = d.join("dist");
        let r = bundle(&[d.join("src/a.css"), d.join("src/z.css")], &dist).unwrap();
        assert!(r.css.starts_with(".b { color: red }"), "{}", r.css);
        assert!(r.css.contains("data:image/png;base64,UE5H"), "{}", r.css);
        assert_eq!(r.copied.len(), 1);
        assert!(r.copied[0].starts_with("big-") && r.copied[0].ends_with(".ttf"));
        assert!(dist.join(&r.copied[0]).exists());
        assert!(
            r.css.contains(&format!("url(\"{}\")", r.copied[0])),
            "{}",
            r.css
        );
        assert!(r.css.contains("url(data:x)"));
        assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
        assert!(r.css.trim_end().ends_with(".z{}"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
