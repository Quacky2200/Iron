// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The crates to Markdown: one section per licence, the crates under it with their copyright
//! lines, then the licence text once (and other wordings of it, de-duplicated); then the
//! notices crates ship and the licences of code bundled inside them.

use crate::tables::{FIXED_TEXT, KNOWN, NestedUse, Rule, Tables};
use crate::{Crate, File, spdx, text};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// A crate that breaks a rule of the tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// No `license` in its `Cargo.toml`.
    NoLicence { krate: String },
    /// A `license` that is not an SPDX expression.
    BadExpression { krate: String, error: String },
    /// A licence the tool does not know.
    Unknown { krate: String, licence: String },
    /// GPL, LGPL or AGPL, with no other choice.
    Copyleft { krate: String, licence: String },
    /// No licence file for the licence it is listed under.
    NoLicenceFile { krate: String, licence: String },
    /// A licence file inside the crate that the nested table does not list.
    Unreviewed { krate: String, path: String },
    /// No crate ships the text of a licence the section needs.
    NoText { licence: String },
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::NoLicence { krate } => write!(f, "{krate}: no `license` in Cargo.toml"),
            Problem::BadExpression { krate, error } => write!(f, "{krate}: {error}"),
            Problem::Unknown { krate, licence } => {
                write!(f, "{krate}: unknown licence `{licence}`")
            }
            Problem::Copyleft { krate, licence } => {
                write!(f, "{krate}: copyleft licence `{licence}`")
            }
            Problem::NoLicenceFile { krate, licence } => {
                write!(f, "{krate}: no licence file for {licence}")
            }
            Problem::Unreviewed { krate, path } => {
                write!(
                    f,
                    "{krate}: unreviewed licence file `{path}` inside the crate"
                )
            }
            Problem::NoText { licence } => write!(f, "no crate ships the text of {licence}"),
        }
    }
}

/// The rendered file and what went into it.
#[derive(Debug, Clone)]
pub struct Notices {
    pub text: String,
    /// Third-party crates listed.
    pub crates: usize,
    /// Each licence section and its number of crates, in order.
    pub sections: Vec<(String, usize)>,
    /// Crates let through by the allow table: `name version: rule (reason)`.
    pub allowed: Vec<String>,
    /// Allow entries that matched no crate (stale).
    pub unused_allow: Vec<&'static str>,
}

struct Entry<'a> {
    krate: &'a Crate,
    copyright: Vec<String>,
    /// Its own texts for this licence; empty when it ships none (allowed).
    texts: Vec<&'a File>,
}

struct Extra {
    title: String,
    text: String,
}

const LLVM: &str = "Apache-2.0 WITH LLVM-exception";

fn rank(id: &str) -> usize {
    KNOWN.iter().position(|k| *k == id).unwrap_or(KNOWN.len())
}

fn copyleft(id: &str) -> bool {
    id.to_ascii_uppercase().contains("GPL")
}

/// A file classified as `class` holds the text of `id`.
fn holds(id: &str, class: &str) -> bool {
    id == class || (id == LLVM && class == "Apache-2.0") || (id == "Apache-2.0" && class == LLVM)
}

fn fenced(out: &mut String, body: &str) {
    let fence = text::fence(body);
    let _ = write!(out, "{fence}text\n{body}\n{fence}\n\n");
}

/// Renders the notices: `hand_written` verbatim, then the crates. Fails with every crate that
/// breaks a rule of `tables`.
pub fn render(
    crates: &[Crate],
    hand_written: &str,
    tables: &Tables,
) -> Result<Notices, Vec<Problem>> {
    let mut crates: Vec<&Crate> = crates.iter().collect();
    crates.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    let mut problems = Vec::new();
    let mut sections: BTreeMap<(usize, String), Vec<Entry>> = BTreeMap::new();
    let mut extras: Vec<Extra> = Vec::new();
    let mut allowed = Vec::new();
    let mut used_allow = BTreeSet::new();
    for c in crates {
        let label = c.label();
        let krate = format!("{} {}", c.name, c.version);
        let mut allow = |rule: Rule| {
            let a = tables
                .allow
                .iter()
                .find(|a| a.rule == rule && a.matches(&c.name))?;
            used_allow.insert(a.krate);
            allowed.push(format!("{krate}: {rule:?} ({})", a.reason));
            Some(())
        };
        let Some(expr) = &c.license else {
            problems.push(Problem::NoLicence {
                krate: krate.clone(),
            });
            continue;
        };
        let alts = match spdx::alternatives(expr) {
            Ok(a) => a,
            Err(error) => {
                problems.push(Problem::BadExpression {
                    krate: krate.clone(),
                    error,
                });
                continue;
            }
        };
        let licence_files: Vec<(&File, Vec<&str>)> = c
            .files
            .iter()
            .filter(|f| !text::is_notice_file(&f.path))
            .map(|f| (f, text::classify(&f.path, &f.text)))
            .collect();
        let known = |id: &str| KNOWN.contains(&id);
        let files_of = |id: &str| -> Vec<&File> {
            licence_files
                .iter()
                .filter(|(_, cls)| !known(id) || cls.iter().any(|cl| holds(id, cl)))
                .map(|(f, _)| *f)
                .collect()
        };
        let usable_id = |id: &str, allow: &mut dyn FnMut(Rule) -> Option<()>| {
            if copyleft(id) {
                allow(Rule::Copyleft).is_some()
            } else {
                known(id) || allow(Rule::UnknownLicence).is_some()
            }
        };
        // Allow entries are only recorded for the alternative that is used, so probe first.
        let mut probe = |_: Rule| None;
        let plain: Vec<&Vec<String>> = alts
            .iter()
            .filter(|alt| alt.iter().all(|id| usable_id(id, &mut probe)))
            .collect();
        let usable: Vec<&Vec<String>> = if plain.is_empty() {
            alts.iter()
                .filter(|alt| alt.iter().all(|id| usable_id(id, &mut allow)))
                .collect()
        } else {
            plain
        };
        let Some(best) = usable.into_iter().min_by_key(|alt| {
            let missing = alt.iter().filter(|id| files_of(id).is_empty()).count();
            let mut ranks: Vec<usize> = alt.iter().map(|id| rank(id)).collect();
            ranks.sort();
            (missing, ranks)
        }) else {
            let mut seen = BTreeSet::new();
            for id in alts.iter().flatten().filter(|id| !known(id)) {
                if seen.insert(id) {
                    let licence = id.clone();
                    let krate = krate.clone();
                    problems.push(if copyleft(id) {
                        Problem::Copyleft { krate, licence }
                    } else {
                        Problem::Unknown { krate, licence }
                    });
                }
            }
            continue;
        };
        for id in best {
            let texts = files_of(id);
            if texts.is_empty() && allow(Rule::NoLicenceFile).is_none() {
                problems.push(Problem::NoLicenceFile {
                    krate: krate.clone(),
                    licence: id.clone(),
                });
                continue;
            }
            let mut copyright: Vec<String> = Vec::new();
            for l in texts.iter().flat_map(|f| text::copyright_lines(&f.text)) {
                if !copyright.contains(&l) {
                    copyright.push(l);
                }
            }
            sections
                .entry((rank(id), id.clone()))
                .or_default()
                .push(Entry {
                    krate: c,
                    copyright,
                    texts,
                });
        }
        for f in c.files.iter().filter(|f| text::is_notice_file(&f.path)) {
            extras.push(Extra {
                title: format!("Notice of {label} (`{}`)", f.path),
                text: f.text.clone(),
            });
        }
        for f in &c.nested {
            let entry = tables
                .nested
                .iter()
                .find(|n| n.krate == c.name && n.path == f.path);
            match entry.map(|n| n.what) {
                Some(NestedUse::Include(what)) => extras.push(Extra {
                    title: format!("{what} ({label}, `{}`)", f.path),
                    text: f.text.clone(),
                }),
                Some(NestedUse::Skip(_)) => {}
                None => problems.push(Problem::Unreviewed {
                    krate: krate.clone(),
                    path: f.path.clone(),
                }),
            }
        }
    }
    for ((_, id), entries) in &sections {
        if entries.iter().all(|e| e.texts.is_empty()) {
            problems.push(Problem::NoText {
                licence: id.clone(),
            });
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    let total: BTreeSet<(&str, &str)> = sections
        .values()
        .flatten()
        .map(|e| (e.krate.name.as_str(), e.krate.version.as_str()))
        .collect();
    let mut out = String::new();
    out.push_str("<!-- Generated by tools/platypus_notices (`cargo run -p platypus_notices` in tools/) from Cargo.lock and tools/platypus_notices/hand-written.md: do not edit by hand. -->\n\n");
    out.push_str(text::clean(hand_written.as_bytes()).as_str());
    out.push_str("\n\n# Rust crates\n\n");
    let _ = write!(
        out,
        "The host is built from {} third-party Rust crates from crates.io: the normal and build \
         dependencies of `{}` for `{}`, as Cargo.lock pins them. Each is listed under its licence \
         with the copyright lines of its licence file (or, where the published crate has no \
         licence file, the authors in its Cargo.toml), and the licence text follows once. A crate \
         offered under a choice of licences (`MIT OR Apache-2.0`) is listed under the one Platypus \
         uses: the first of {} that it ships a licence file for. A crate under several licences at \
         once (`AND`) is listed under each. Crates that word a licence differently from its usual \
         text have their own text after it.\n\n",
        total.len(),
        crate::HOST,
        crate::TARGET,
        KNOWN.join(", ")
    );
    let summary: Vec<String> = sections
        .iter()
        .map(|((_, id), e)| format!("{id} ({})", e.len()))
        .collect();
    let _ = write!(out, "Licences: {}.\n\n", summary.join(", "));
    for ((_, id), entries) in &sections {
        render_section(&mut out, id, entries);
    }
    if !extras.is_empty() {
        out.push_str("## Notices and bundled code\n\n");
        out.push_str(
            "The notices some crates ship, and the licences of code and data bundled inside \
             crates (their own licences are above).\n\n",
        );
        // The same text once, under every title that carries it.
        let mut by_text: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut order: Vec<&str> = Vec::new();
        for e in &extras {
            let titles = by_text.entry(e.text.as_str()).or_default();
            if titles.is_empty() {
                order.push(e.text.as_str());
            }
            titles.push(e.title.as_str());
        }
        for t in order {
            let _ = write!(out, "### {}\n\n", by_text[t].join("; "));
            fenced(&mut out, t);
        }
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    allowed.sort();
    allowed.dedup();
    let unused_allow = tables
        .allow
        .iter()
        .map(|a| a.krate)
        .filter(|k| !used_allow.contains(k))
        .collect();
    Ok(Notices {
        text: out,
        crates: total.len(),
        sections: sections
            .iter()
            .map(|((_, id), e)| (id.clone(), e.len()))
            .collect(),
        allowed,
        unused_allow,
    })
}

fn render_section(out: &mut String, id: &str, entries: &[Entry]) {
    let mpl = id == "MPL-2.0";
    let _ = write!(out, "## {id}\n\n");
    let _ = write!(
        out,
        "{} crate{}.\n\n",
        entries.len(),
        if entries.len() == 1 { "" } else { "s" },
    );
    for e in entries {
        let c = e.krate;
        if mpl {
            let _ = writeln!(
                out,
                "- {}: used unmodified from crates.io; source at <{}>",
                c.label(),
                c.link
            );
        } else {
            let _ = writeln!(out, "- {}: <{}>", c.label(), c.link);
        }
        for l in &e.copyright {
            let _ = writeln!(out, "  - {l}");
        }
        let authors = c.authors.join(", ");
        if e.texts.is_empty() {
            let _ = writeln!(
                out,
                "  - Published without a licence file{}",
                if authors.is_empty() {
                    String::new()
                } else {
                    format!("; authors: {authors}")
                }
            );
        } else if e.copyright.is_empty() && !authors.is_empty() {
            let _ = writeln!(out, "  - Authors: {authors}");
        }
    }
    out.push('\n');
    // Wordings by key, each with its first text (in crate order) and the crates using it.
    let mut wordings: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();
    for e in entries {
        for f in &e.texts {
            let w = wordings
                .entry(text::key(&f.text))
                .or_insert_with(|| (text::body(&f.text), Vec::new()));
            let label = e.krate.label();
            if !w.1.contains(&label) {
                w.1.push(label);
            }
        }
    }
    let Some(standard) = wordings
        .iter()
        .max_by(|a, b| a.1.1.len().cmp(&b.1.1.len()).then(b.0.cmp(a.0)))
        .map(|(k, _)| k.clone())
    else {
        return;
    };
    out.push_str("### Licence text\n\n");
    fenced(out, &wordings[&standard].0);
    if FIXED_TEXT.contains(&id) {
        return;
    }
    let mut others: Vec<&(String, Vec<String>)> = wordings
        .iter()
        .filter(|(k, _)| **k != standard)
        .map(|(_, w)| w)
        .collect();
    if others.is_empty() {
        return;
    }
    others.sort_by(|a, b| a.1.cmp(&b.1));
    let _ = write!(
        out,
        "### Other wordings of {id}\n\nThese crates word the licence in their own way; their text:\n\n"
    );
    for (body, labels) in others {
        let _ = write!(out, "#### {}\n\n", labels.join(", "));
        fenced(out, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{Allow, Nested};

    const MIT_BODY: &str = "Permission is hereby granted, free of charge, to any person obtaining a copy\nof this software.\n\nThe above copyright notice and this permission notice shall be included.";
    const APACHE: &str = "Apache License\nVersion 2.0, January 2004\n\nTERMS AND CONDITIONS";
    const MPL: &str = "Mozilla Public License Version 2.0\n\n1. Definitions";

    const TABLES: Tables = Tables {
        allow: &[
            Allow {
                krate: "bare-*",
                rule: Rule::NoLicenceFile,
                reason: "test",
            },
            Allow {
                krate: "odd",
                rule: Rule::UnknownLicence,
                reason: "test",
            },
        ],
        nested: &[
            Nested {
                krate: "bundler",
                path: "vendor/LICENSE",
                what: NestedUse::Include("Vendored code"),
            },
            Nested {
                krate: "bundler",
                path: "tests/LICENSE",
                what: NestedUse::Skip("test data"),
            },
        ],
    };

    fn mit(holder: &str) -> String {
        format!("MIT License\n\nCopyright (c) 2020 {holder}\n\n{MIT_BODY}")
    }

    fn krate(name: &str, license: &str, files: &[(&str, &str)]) -> Crate {
        Crate {
            name: name.into(),
            version: "1.0.0".into(),
            license: Some(license.into()),
            link: format!("https://example.com/{name}"),
            authors: vec!["Some Author".into()],
            files: files
                .iter()
                .map(|(p, t)| File {
                    path: p.to_string(),
                    text: t.to_string(),
                })
                .collect(),
            nested: Vec::new(),
        }
    }

    fn ok(crates: &[Crate]) -> Notices {
        render(crates, "# Notices\n\nHand-written.", &TABLES).unwrap()
    }

    /// The text of section `id` (up to the next `## `).
    fn section<'a>(n: &'a Notices, id: &str) -> &'a str {
        let start = n
            .text
            .find(&format!("\n## {id}\n"))
            .unwrap_or_else(|| panic!("no section {id}:\n{}", n.text));
        let rest = &n.text[start + 1..];
        let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 4);
        &rest[..end]
    }

    // M9-LIC-08
    #[test]
    fn a_dual_licensed_crate_is_listed_once_under_the_preferred_licence() {
        let a = mit("Alice");
        let n = ok(&[krate(
            "dual",
            "Apache-2.0 OR MIT",
            &[("LICENSE-APACHE", APACHE), ("LICENSE-MIT", &a)],
        )]);
        assert_eq!(n.sections, [("MIT".to_string(), 1)]);
        assert!(section(&n, "MIT").contains("- `dual` 1.0.0: <https://example.com/dual>"));
        assert!(n.text.starts_with("<!-- Generated"));
        assert!(
            n.text
                .contains("# Notices\n\nHand-written.\n\n# Rust crates")
        );
        // Without an MIT file it is listed under the licence it does ship.
        let n = ok(&[krate("dual", "MIT OR Apache-2.0", &[("LICENSE", APACHE)])]);
        assert_eq!(n.sections, [("Apache-2.0".to_string(), 1)]);
    }

    // M9-LIC-09
    #[test]
    fn an_and_expression_lists_the_crate_under_each_licence() {
        let a = mit("Alice");
        let n = ok(&[krate(
            "both",
            "(MIT OR Apache-2.0) AND Unicode-3.0",
            &[
                ("LICENSE-MIT", &a),
                (
                    "LICENSE-UNICODE",
                    "UNICODE LICENSE V3\n\nCopyright \u{a9} 2020 Unicode, Inc.",
                ),
            ],
        )]);
        assert_eq!(
            n.sections,
            [("MIT".to_string(), 1), ("Unicode-3.0".to_string(), 1)]
        );
        assert!(section(&n, "Unicode-3.0").contains("Copyright \u{a9} 2020 Unicode, Inc."));
        assert_eq!(n.crates, 1);
    }

    // M9-LIC-10
    #[test]
    fn mpl_crates_say_they_are_used_unmodified_with_their_source() {
        let n = ok(&[krate("css", "MPL-2.0", &[("LICENSE", MPL)])]);
        let s = section(&n, "MPL-2.0");
        assert!(
            s.contains(
                "- `css` 1.0.0: used unmodified from crates.io; source at <https://example.com/css>"
            ),
            "{s}"
        );
        assert!(s.contains("Mozilla Public License"));
    }

    // M9-LIC-11
    #[test]
    fn unknown_copyleft_and_unlicensed_crates_fail_unless_allowed() {
        let a = mit("Alice");
        let errs = render(
            &[
                krate("weird", "WTFPL", &[("LICENSE", "x")]),
                krate("gpl", "GPL-3.0-only", &[("COPYING", "x")]),
                krate("bare", "MIT", &[]),
                krate("ok", "MIT OR GPL-2.0", &[("LICENSE", &a)]),
            ],
            "",
            &TABLES,
        )
        .unwrap_err();
        assert_eq!(
            errs,
            [
                Problem::NoLicenceFile {
                    krate: "bare 1.0.0".into(),
                    licence: "MIT".into()
                },
                Problem::Copyleft {
                    krate: "gpl 1.0.0".into(),
                    licence: "GPL-3.0-only".into()
                },
                Problem::Unknown {
                    krate: "weird 1.0.0".into(),
                    licence: "WTFPL".into()
                },
            ]
        );
        // Allowed: no file uses the standard text and the authors; an unknown licence gets its
        // own section with the crate's text.
        let n = ok(&[
            krate("bare-one", "MIT", &[]),
            krate("odd", "WTFPL", &[("COPYING", "DO WHAT YOU WANT")]),
            krate("ok", "MIT", &[("LICENSE", &a)]),
        ]);
        let s = section(&n, "MIT");
        assert!(
            s.contains("Published without a licence file; authors: Some Author"),
            "{s}"
        );
        assert!(s.contains("Permission is hereby granted"));
        assert!(section(&n, "WTFPL").contains("DO WHAT YOU WANT"));
        assert_eq!(n.allowed.len(), 2, "{:?}", n.allowed);
        assert!(n.unused_allow.is_empty());
    }

    // M9-LIC-12
    #[test]
    fn identical_texts_are_printed_once_and_copyright_lines_stay_with_their_crate() {
        let (a, b) = (mit("Alice"), mit("Bob"));
        let reworded = format!("Copyright (c) 2021 Carol\n\n{MIT_BODY}\n\nExcept for the logo.");
        let n = ok(&[
            krate("a", "MIT", &[("LICENSE", &a)]),
            krate("b", "MIT", &[("LICENSE-MIT", &b), ("LICENSE", &a)]),
            krate("c", "MIT", &[("LICENSE", &reworded)]),
        ]);
        let s = section(&n, "MIT");
        assert_eq!(s.matches("Permission is hereby granted").count(), 2, "{s}");
        assert!(s.contains("- `a` 1.0.0: <https://example.com/a>\n  - Copyright (c) 2020 Alice\n"));
        assert!(s.contains(
            "- `b` 1.0.0: <https://example.com/b>\n  - Copyright (c) 2020 Bob\n  - Copyright (c) 2020 Alice\n"
        ));
        assert!(s.contains("### Other wordings of MIT"));
        assert!(s.contains("#### `c` 1.0.0\n\n```text\nPermission"), "{s}");
        assert!(!s.contains("Copyright (c) 2021 Carol\n\nPermission"));
        // A fixed-text licence never prints a second wording.
        let n = ok(&[
            krate("x", "Apache-2.0", &[("LICENSE", APACHE)]),
            krate(
                "y",
                "Apache-2.0",
                &[("LICENSE", &format!("{APACHE}\n\nAPPENDIX"))],
            ),
        ]);
        assert_eq!(
            section(&n, "Apache-2.0")
                .matches("TERMS AND CONDITIONS")
                .count(),
            1
        );
    }

    // M9-LIC-13
    #[test]
    fn notice_files_and_reviewed_bundled_licences_are_included() {
        let mut c = krate(
            "bundler",
            "Apache-2.0",
            &[
                ("LICENSE", APACHE),
                ("NOTICE", "Bundler\nCopyright 2020 The Bundler Authors"),
            ],
        );
        c.nested = vec![
            File {
                path: "vendor/LICENSE".into(),
                text: "Vendored licence".into(),
            },
            File {
                path: "tests/LICENSE".into(),
                text: "Test data licence".into(),
            },
        ];
        let n = ok(&[c.clone()]);
        let s = section(&n, "Notices and bundled code");
        assert!(s.contains("### Notice of `bundler` 1.0.0 (`NOTICE`)\n\n```text\nBundler\nCopyright 2020 The Bundler Authors\n```"), "{s}");
        assert!(s.contains("### Vendored code (`bundler` 1.0.0, `vendor/LICENSE`)"));
        assert!(!s.contains("Test data licence"));
        // The NOTICE is not taken for a licence text.
        assert!(!section(&n, "Apache-2.0").contains("Bundler Authors"));
        // A licence file inside a crate that nobody reviewed fails.
        c.nested.push(File {
            path: "third/COPYING".into(),
            text: "?".into(),
        });
        assert_eq!(
            render(&[c], "", &TABLES).unwrap_err(),
            [Problem::Unreviewed {
                krate: "bundler 1.0.0".into(),
                path: "third/COPYING".into()
            }]
        );
    }
}
