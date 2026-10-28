// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The host's third-party crates from `cargo metadata`, and their licence files from the local
//! cargo registry (the folder of each crate's `Cargo.toml`).

use crate::{Crate, Error, File, TARGET, text};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A third-party package of the tree, before its files are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub link: String,
    pub authors: Vec<String>,
    /// Its source folder.
    pub dir: PathBuf,
}

/// `cargo metadata --format-version 1 --offline --locked` of the workspace at `root`, for
/// [`TARGET`].
pub fn run_cargo(root: &Path) -> Result<Value, Error> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline", "--locked"])
        .args(["--filter-platform", TARGET, "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .map_err(|e| Error::Cargo(e.to_string()))?;
    if !out.status.success() {
        return Err(Error::Cargo(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| Error::Cargo(e.to_string()))
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}

/// The third-party packages (from a registry, not a path) that `root` reaches through normal and
/// build dependencies, sorted by name and version.
pub fn third_party(json: &Value, root: &str) -> Result<Vec<Package>, Error> {
    let bad = |what: &str| Error::Cargo(format!("unexpected metadata: {what}"));
    let packages = json["packages"].as_array().ok_or_else(|| bad("packages"))?;
    let by_id: BTreeMap<&str, &Value> = packages
        .iter()
        .filter_map(|p| Some((p["id"].as_str()?, p)))
        .collect();
    let nodes: BTreeMap<&str, &Value> = json["resolve"]["nodes"]
        .as_array()
        .ok_or_else(|| bad("resolve.nodes"))?
        .iter()
        .filter_map(|n| Some((n["id"].as_str()?, n)))
        .collect();
    let start = packages
        .iter()
        .find(|p| p["name"] == root && p["source"].is_null())
        .and_then(|p| p["id"].as_str())
        .ok_or_else(|| bad(&format!("no package {root}")))?;
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let deps = nodes.get(id).and_then(|n| n["deps"].as_array());
        for d in deps.into_iter().flatten() {
            let shipped = d["dep_kinds"].as_array().is_some_and(|kinds| {
                kinds
                    .iter()
                    .any(|k| k["kind"].is_null() || k["kind"] == "build")
            });
            if let (true, Some(pkg)) = (shipped, d["pkg"].as_str()) {
                stack.push(pkg);
            }
        }
    }
    let mut out: Vec<Package> = seen
        .iter()
        .filter_map(|id| by_id.get(id))
        .filter(|p| !p["source"].is_null())
        .map(|p| {
            let name = str_of(p, "name").unwrap_or_default();
            let link = str_of(p, "repository")
                .or_else(|| str_of(p, "homepage"))
                .unwrap_or_else(|| format!("https://crates.io/crates/{name}"));
            let authors = p["authors"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|a| a.split('<').next().unwrap_or(a).trim().to_string())
                .filter(|a| !a.is_empty())
                .collect();
            let manifest = PathBuf::from(p["manifest_path"].as_str().unwrap_or_default());
            Package {
                version: str_of(p, "version").unwrap_or_default(),
                license: str_of(p, "license"),
                link,
                authors,
                dir: manifest.parent().map(Path::to_path_buf).unwrap_or_default(),
                name,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(out)
}

/// Every licence-like file below `dir`'s top, as `/` paths relative to it.
fn nested_files(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        match e.file_type() {
            Ok(t) if t.is_dir() => nested_files(&e.path(), &path, out),
            Ok(_) if !rel.is_empty() && text::is_licence_file(&name) => out.push(path),
            _ => {}
        }
    }
}

/// Reads each package's licence files. Fails with every package whose source is missing.
pub fn read(pkgs: &[Package]) -> Result<Vec<Crate>, Error> {
    let missing: Vec<String> = pkgs
        .iter()
        .filter(|p| !p.dir.join("Cargo.toml").is_file())
        .map(|p| format!("{} {}", p.name, p.version))
        .collect();
    if !missing.is_empty() {
        return Err(Error::MissingSource(missing));
    }
    let read_file = |dir: &Path, rel: &str| File {
        path: rel.to_string(),
        text: text::clean(&std::fs::read(dir.join(rel)).unwrap_or_default()),
    };
    Ok(pkgs
        .iter()
        .map(|p| {
            let mut top: Vec<String> = std::fs::read_dir(&p.dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| text::is_licence_file(n))
                .collect();
            top.sort();
            let mut nested = Vec::new();
            nested_files(&p.dir, "", &mut nested);
            nested.sort();
            Crate {
                name: p.name.clone(),
                version: p.version.clone(),
                license: p.license.clone(),
                link: p.link.clone(),
                authors: p.authors.clone(),
                files: top.iter().map(|n| read_file(&p.dir, n)).collect(),
                nested: nested.iter().map(|n| read_file(&p.dir, n)).collect(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // M9-LIC-07
    #[test]
    fn the_tree_follows_normal_and_build_dependencies_of_third_party_crates() {
        let reg = "registry+https://github.com/rust-lang/crates.io-index";
        let pkg = |id: &str, source: Option<&str>| {
            json!({
                "id": id, "name": id, "version": "1.0.0", "source": source,
                "license": "MIT", "repository": null, "homepage": null,
                "authors": ["A Person <a@example.com>"],
                "manifest_path": format!("/reg/{id}/Cargo.toml"),
            })
        };
        let dep =
            |pkg: &str, kind: Option<&str>| json!({ "pkg": pkg, "dep_kinds": [{ "kind": kind }] });
        let meta = json!({
            "packages": [pkg("app", None), pkg("ours", None), pkg("normal", Some(reg)),
                         pkg("build", Some(reg)), pkg("dev", Some(reg)), pkg("deep", Some(reg))],
            "resolve": { "nodes": [
                { "id": "app", "deps": [dep("ours", None), dep("build", Some("build")), dep("dev", Some("dev"))] },
                { "id": "ours", "deps": [dep("normal", None)] },
                { "id": "normal", "deps": [dep("deep", None)] },
                { "id": "build", "deps": [] }, { "id": "dev", "deps": [] }, { "id": "deep", "deps": [] },
            ]},
        });
        let pkgs = third_party(&meta, "app").unwrap();
        let names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["build", "deep", "normal"]);
        assert_eq!(pkgs[0].authors, ["A Person"]);
        assert_eq!(pkgs[0].link, "https://crates.io/crates/build");
        assert_eq!(pkgs[0].dir, PathBuf::from("/reg/build"));
        // A missing source is its own error (the test of the committed file skips on it).
        assert!(matches!(read(&pkgs), Err(Error::MissingSource(v)) if v.len() == 3));
    }
}
