// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Licence files: which ones a crate has, which licence each holds, their copyright lines, and a
//! key that is the same for two wordings that differ only in those lines and layout.

/// A licence or notice file at the top of a crate (or, for bundled code, below it).
pub fn is_licence_file(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    ["LICENSE", "LICENCE", "COPYING", "NOTICE", "UNLICENSE"]
        .iter()
        .any(|p| n.starts_with(p))
}

/// A notice rather than a licence: `NOTICE`, `NOTICES.md`, `LICENSE-THIRD-PARTY`.
pub fn is_notice_file(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    n.starts_with("NOTICE") || n.contains("THIRD-PARTY") || n.contains("THIRD_PARTY")
}

/// The file as text: UTF-8 (lossy, without a BOM), `\n` line ends, no trailing spaces, no blank
/// lines at either end.
pub fn clean(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    let s = s.strip_prefix('\u{feff}').unwrap_or(&s);
    let lines: Vec<&str> = s.lines().map(str::trim_end).collect();
    let start = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(start, |e| e + 1);
    lines[start..end].join("\n")
}

/// One line, lower case, single spaces, without comment markers (a licence kept as `// ...`).
fn flat(text: &str) -> String {
    text.lines()
        .map(|l| l.trim_start().trim_start_matches(['/', '#', '*', ';']))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The licences a file holds, from its name (`LICENSE-MIT`) or else its text.
pub fn classify(name: &str, text: &str) -> Vec<&'static str> {
    let n = name.to_ascii_uppercase();
    let t = flat(text);
    let apache = || {
        if t.contains("llvm exception") {
            "Apache-2.0 WITH LLVM-exception"
        } else {
            "Apache-2.0"
        }
    };
    let bsd = || {
        if t.contains("endorse or promote") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        }
    };
    // By name first; the order matters (`UNLICENSE` holds `LICENSE`, `0BSD` holds `BSD`).
    let by_name: Option<&'static str> = if n.starts_with("UNLICENSE") {
        Some("Unlicense")
    } else if n.contains("0BSD") {
        Some("0BSD")
    } else if n.contains("APACHE") || n.contains("LLVM") {
        Some(apache())
    } else if n.contains("MIT") {
        Some("MIT")
    } else if n.contains("ZLIB") {
        Some("Zlib")
    } else if n.contains("BSD") {
        Some(bsd())
    } else if n.contains("UNICODE") {
        Some("Unicode-3.0")
    } else if n.contains("CC0") {
        Some("CC0-1.0")
    } else if n.contains("BOOST") || n.contains("BSL") {
        Some("BSL-1.0")
    } else {
        None
    };
    if let Some(l) = by_name {
        return vec![l];
    }
    let mut found = Vec::new();
    if t.contains("apache license") && t.contains("version 2.0") {
        found.push(apache());
    }
    if t.contains("permission is hereby granted, free of charge") {
        found.push("MIT");
    }
    if t.contains("mozilla public license") && t.contains("2.0") {
        found.push("MPL-2.0");
    }
    if t.contains("free and unencumbered software released into the public domain") {
        found.push("Unlicense");
    }
    if t.contains("redistribution and use in source and binary forms") {
        found.push(bsd());
    }
    if t.contains("altered source versions must be plainly marked") {
        found.push("Zlib");
    }
    if t.contains("unicode license v3") || t.contains("unicode, inc. license agreement") {
        found.push("Unicode-3.0");
    }
    if t.contains("boost software license") {
        found.push("BSL-1.0");
    }
    if t.contains(
        "permission to use, copy, modify, and/or distribute this software for any purpose",
    ) {
        found.push(if t.contains("provided that the above copyright notice") {
            "ISC"
        } else {
            "0BSD"
        });
    }
    if t.contains("cc0") && t.contains("creative commons") {
        found.push("CC0-1.0");
    }
    found
}

/// A line naming who holds the copyright (`Copyright (c) 2014 Alex Crichton`), not a line of the
/// licence's own wording that starts with the word (`copyright notice ...`, `COPYRIGHT HOLDERS
/// AND CONTRIBUTORS ...`) or a template's placeholder (`Copyright [yyyy] [name of owner]`).
fn copyright_line(line: &str) -> Option<&str> {
    let t = line
        .trim()
        .trim_start_matches(['#', '*', '/', '-', '>'])
        .trim();
    let low = t.to_lowercase();
    let rest = low
        .strip_prefix("copyright")
        .filter(|r| r.starts_with([' ', '(', '\u{a9}', ':']))
        // `(c)` also opens list items ("(c) You must retain ..."): only with a year.
        .or_else(|| {
            low.strip_prefix("(c) ")
                .filter(|r| r.chars().any(|c| c.is_ascii_digit()))
        })
        .or_else(|| low.strip_prefix('\u{a9}'))?;
    const WORDING: &[&str] = &[
        "notice",
        "holder",
        "owner",
        "liable",
        "licen",
        " law",
        "[yyyy]",
        "{yyyy}",
        "<year>",
        "[year]",
        "related rights",
        "statement",
        "the above",
        "protection",
    ];
    (!rest.trim().is_empty() && !WORDING.iter().any(|w| low.contains(w))).then_some(t)
}

/// The copyright lines of a text, in order, each once.
pub fn copyright_lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in text.lines().filter_map(copyright_line) {
        if !out.iter().any(|o| o == l) {
            out.push(l.to_string());
        }
    }
    out
}

/// A heading such as "The MIT License (MIT)" or "MIT License".
fn title(line: &str) -> bool {
    let f = flat(line);
    f.contains("license") && f.split(' ').count() <= 6
}

/// The text without its copyright lines (nor a lone "All rights reserved."), blank runs closed
/// up: what is printed once for every crate that words its licence this way.
pub fn body(text: &str) -> String {
    body_of(text, false)
}

fn body_of(text: &str, untitled: bool) -> String {
    let mut out: Vec<&str> = Vec::new();
    for l in text.lines() {
        let skip = copyright_line(l).is_some()
            || flat(l).trim_end_matches('.') == "all rights reserved"
            || (untitled && out.iter().all(|o| o.is_empty()) && title(l));
        if skip {
            continue;
        }
        if l.is_empty() && out.last().is_none_or(|p| p.is_empty()) {
            continue;
        }
        out.push(l);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// The same for two wordings that differ only in copyright lines, a title ("MIT License"),
/// layout, case or punctuation.
pub fn key(text: &str) -> String {
    let b = body_of(text, true).to_lowercase();
    let words: String = b
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A Markdown code fence longer than any run of backticks in `text`.
pub fn fence(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    "`".repeat((longest + 1).max(3))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIT: &str = "MIT License\n\nCopyright (c) 2014 Alex Crichton\n\nPermission is hereby granted, free of charge, to any person\nThe above copyright notice and this permission notice shall be included\n";

    // M9-LIC-04
    #[test]
    fn files_are_classified_by_name_then_text() {
        assert_eq!(classify("LICENSE-MIT", ""), ["MIT"]);
        assert_eq!(
            classify("license-apache-2.0", "Apache License"),
            ["Apache-2.0"]
        );
        assert_eq!(
            classify(
                "LICENSE",
                "Apache License Version 2.0 ... LLVM Exceptions to the Apache 2.0 License ... LLVM Exception"
            ),
            ["Apache-2.0 WITH LLVM-exception"]
        );
        assert_eq!(classify("UNLICENSE", ""), ["Unlicense"]);
        assert_eq!(classify("LICENSE", MIT), ["MIT"]);
        assert_eq!(
            classify(
                "COPYING",
                "Redistribution and use in source and binary forms ... endorse or promote"
            ),
            ["BSD-3-Clause"]
        );
        // A pointer to the real files holds no licence.
        assert!(
            classify(
                "COPYING",
                "This project is dual-licensed under the Unlicense and MIT licenses."
            )
            .is_empty()
        );
        assert!(is_notice_file("NOTICE") && is_notice_file("LICENSE-THIRD-PARTY"));
        assert!(is_licence_file("license-mit") && !is_licence_file("README.md"));
    }

    // M9-LIC-05
    #[test]
    fn copyright_lines_are_found_but_not_the_licence_wording() {
        let text = "Copyright (c) 2014 Alex Crichton\n  Copyright 2020 The Rust Project Developers\nCopyright [yyyy] [name of copyright owner]\nThe above copyright notice\nCOPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\"\ncopyright license to reproduce\n(c) You must retain\n(c) 2015 Andrew Gallant\n\u{a9} 2016 Unicode, Inc.\nCopyright (c) 2014 Alex Crichton";
        assert_eq!(
            copyright_lines(text),
            [
                "Copyright (c) 2014 Alex Crichton",
                "Copyright 2020 The Rust Project Developers",
                "(c) 2015 Andrew Gallant",
                "\u{a9} 2016 Unicode, Inc."
            ]
        );
    }

    #[test]
    fn wordings_that_differ_in_copyright_and_layout_share_a_key() {
        let other = "MIT License\n\nCopyright (c) 2019 Someone Else\nAll rights reserved.\n\nPermission is hereby granted, free of charge,\nto any person\nThe above copyright notice and this permission notice shall be included";
        assert_eq!(key(MIT), key(other));
        assert_ne!(key(MIT), key("Permission is granted"));
        assert!(!body(MIT).contains("Alex"));
        assert_eq!(clean(b"\xef\xbb\xbf\r\n a \r\nb  \r\n\r\n"), " a\nb");
        assert_eq!(fence("x ``` y"), "````");
    }
}
