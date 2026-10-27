// SPDX-License-Identifier: Apache-2.0

//! Path globs for `include` / `exclude` in `project.json` (and later `Assets.list`).
//!
//! Paths are game paths: relative, `/`-separated. `*` matches any characters inside one path
//! part, `?` one character inside a part, `**` as a whole part matches any number of parts
//! (including none). Matching is case-sensitive, like packs.

/// True when `s` has a glob character.
pub fn is_glob(s: &str) -> bool {
    s.contains(['*', '?'])
}

/// Whether `path` matches `pattern`. A pattern with no glob characters matches itself, and also
/// everything below it when `as_folder` is true (`assets` = `assets/**`).
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern
        .trim_start_matches("./")
        .trim_end_matches('/')
        .split('/')
        .collect();
    let parts: Vec<&str> = path.trim_start_matches("./").split('/').collect();
    parts_match(&pat, &parts)
}

/// Like [`glob_match`], and a pattern with no glob characters also matches the files below it.
pub fn include_match(pattern: &str, path: &str) -> bool {
    if is_glob(pattern) {
        glob_match(pattern, path)
    } else {
        let p = pattern.trim_start_matches("./").trim_end_matches('/');
        path == p || path.strip_prefix(p).is_some_and(|r| r.starts_with('/'))
    }
}

fn parts_match(pat: &[&str], parts: &[&str]) -> bool {
    match pat.split_first() {
        None => parts.is_empty(),
        Some((&"**", rest)) => (0..=parts.len()).any(|i| parts_match(rest, &parts[i..])),
        Some((p, rest)) => parts
            .split_first()
            .is_some_and(|(first, tail)| part_match(p, first) && parts_match(rest, tail)),
    }
}

/// `*` and `?` inside one path part.
fn part_match(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) && p[pi] != '*' {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-TS-2550
    #[test]
    fn star_stays_in_a_part_and_double_star_crosses_parts() {
        assert!(glob_match("assets/sfx/*.wav", "assets/sfx/a.wav"));
        assert!(!glob_match("assets/sfx/*.wav", "assets/sfx/deep/a.wav"));
        assert!(!glob_match("assets/sfx/*.wav", "assets/sfx/a.mp3"));
        assert!(glob_match("assets/**", "assets/a/b/c.png"));
        assert!(glob_match("assets/**", "assets/c.png"));
        assert!(glob_match("**/*.psd", "assets/wip/x.psd"));
        assert!(glob_match("**/*.psd", "x.psd"));
        assert!(glob_match("assets/*/x.png", "assets/a/x.png"));
        assert!(!glob_match("assets/*/x.png", "assets/x.png"));
        assert!(glob_match("a/?.png", "a/b.png"));
        assert!(!glob_match("a/?.png", "a/bc.png"));
        assert!(glob_match("assets/sfx/*.*", "assets/sfx/a.b.wav"));
    }

    // M9-TS-2550
    #[test]
    fn matching_is_case_sensitive_and_a_plain_path_means_the_folder_below_it() {
        assert!(!glob_match("assets/*.PNG", "assets/a.png"));
        assert!(include_match("assets", "assets/a/b.png"));
        assert!(include_match("assets/", "assets/a.png"));
        assert!(include_match("./assets/a.png", "assets/a.png"));
        assert!(!include_match("assets", "assets2/a.png"));
        assert!(!include_match("assets/Sfx", "assets/sfx/a.wav"));
        assert!(include_match("assets/*.png", "assets/a.png"));
    }
}
