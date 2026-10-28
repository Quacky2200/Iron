// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Writes `THIRD-PARTY-NOTICES.md`, the notices of everything the host (`platypus-app`) is built
//! from: the hand-written part (`hand-written.md`: native code, fonts, assets) and then, one
//! section per licence, every third-party crate of the host's dependency tree (Cargo.lock), with
//! its copyright lines and the licence text once. `platypus-app --licenses` prints the file and
//! `platypus-build package` writes it beside every exe. See `docs/platypus-build.md`.

pub mod metadata;
mod render;
pub mod spdx;
pub mod tables;
pub mod text;

use std::path::Path;

pub use render::{Notices, Problem, render};

/// The package whose dependency tree is listed.
pub const HOST: &str = "platypus_app";
/// The target it is listed for (the Windows host; other targets' crates are not shipped).
pub const TARGET: &str = "x86_64-pc-windows-msvc";
/// The file written at the repository root.
pub const FILE: &str = "THIRD-PARTY-NOTICES.md";
/// The hand-written part, included verbatim before the crates.
pub const HAND_WRITTEN: &str = include_str!("../hand-written.md");

/// A third-party crate with its licence files read.
#[derive(Debug, Clone, Default)]
pub struct Crate {
    pub name: String,
    pub version: String,
    /// The SPDX expression of its `Cargo.toml` (`None`: it has none).
    pub license: Option<String>,
    /// Its repository, else its homepage, else its crates.io page.
    pub link: String,
    /// Its `Cargo.toml` authors, without e-mail addresses.
    pub authors: Vec<String>,
    /// The licence and notice files at the top of its source, sorted by name.
    pub files: Vec<File>,
    /// Licence files below the top (code it bundles), sorted by path (`/`).
    pub nested: Vec<File>,
}

impl Crate {
    pub fn label(&self) -> String {
        format!("`{}` {}", self.name, self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub text: String,
}

/// Why the notices could not be made.
#[derive(Debug)]
pub enum Error {
    /// `cargo metadata --offline` did not run or gave no usable output.
    Cargo(String),
    /// The registry source of these crates is not on this machine.
    MissingSource(Vec<String>),
    /// Crates that break a rule (see `tables`).
    Problems(Vec<Problem>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Cargo(e) => write!(f, "cargo metadata: {e}"),
            Error::MissingSource(v) => {
                write!(f, "no registry source for: {}", v.join(", "))
            }
            Error::Problems(v) => {
                writeln!(f, "{} crate(s) need a look:", v.len())?;
                for p in v {
                    writeln!(f, "  {p}")?;
                }
                Ok(())
            }
        }
    }
}

/// Reads the host's crates under `root` (the repository) and renders the notices.
pub fn generate(root: &Path) -> Result<Notices, Error> {
    let json = metadata::run_cargo(root)?;
    let pkgs = metadata::third_party(&json, HOST)?;
    let crates = metadata::read(&pkgs)?;
    render(&crates, HAND_WRITTEN, &tables::REVIEWED).map_err(Error::Problems)
}

/// `None` when `current` is `new` (line ends aside), else a short summary of what differs.
pub fn diff(current: &str, new: &str) -> Option<String> {
    let current = current.replace("\r\n", "\n");
    if current == new {
        return None;
    }
    let old: Vec<&str> = current.lines().collect();
    let new: Vec<&str> = new.lines().collect();
    let first = old
        .iter()
        .zip(&new)
        .position(|(a, b)| a != b)
        .unwrap_or(old.len().min(new.len()));
    let only = |a: &[&str], b: &[&str]| -> Vec<String> {
        let set: std::collections::HashSet<&&str> = b.iter().collect();
        a.iter()
            .filter(|l| !l.is_empty() && !set.contains(l))
            .map(|l| l.to_string())
            .collect()
    };
    let (gone, added) = (only(&old, &new), only(&new, &old));
    let mut s = format!(
        "{FILE} is out of date: {} line(s) now, {} before; first difference at line {}\n",
        new.len(),
        old.len(),
        first + 1
    );
    for (sign, lines) in [("-", &gone), ("+", &added)] {
        for l in lines.iter().take(15) {
            s.push_str(&format!("{sign} {l}\n"));
        }
        if lines.len() > 15 {
            s.push_str(&format!("{sign} ... {} more\n", lines.len() - 15));
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // M9-LIC-06
    #[test]
    fn the_check_reports_what_changed_and_ignores_line_ends() {
        assert!(diff("a\r\nb\n", "a\nb\n").is_none());
        let d = diff("a\nb\n", "a\nc\n").unwrap();
        assert!(d.contains("first difference at line 2"), "{d}");
        assert!(d.contains("- b") && d.contains("+ c"), "{d}");
    }
}
