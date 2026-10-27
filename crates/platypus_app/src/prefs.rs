// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The player's saved settings (Unity's `PlayerPrefs`): one JSON file per game in the
//! user's settings folder, read at startup (scripts get it in `__platypus_info.prefs`) and
//! written on `prefs.save` and when the game closes. The engine keeps its own entries
//! under `platypus.` keys (the window mode and size, `platypus.display`).
//!
//! Where it lives (per game, named from the manifest's company and title):
//! - Windows: `%APPDATA%\<company>\<title>\prefs.json`
//! - macOS: `~/Library/Application Support/<identifier or company.title>/prefs.json`
//! - Linux and others: `$XDG_CONFIG_HOME/<company>/<title>/prefs.json` (or `~/.config`)
//!
//! `PLATYPUS_PREFS_DIR` puts it somewhere else (tests, portable installs).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::info::Manifest;

pub struct Prefs {
    /// `None`: kept in memory only (screenshot runs, no settings folder).
    path: Option<PathBuf>,
    values: Map<String, Value>,
    dirty: bool,
}

impl Prefs {
    /// Prefs that are never written (screenshot runs stay the same every time).
    pub fn in_memory() -> Self {
        Self {
            path: None,
            values: Map::new(),
            dirty: false,
        }
    }

    /// The game's prefs file, read if it exists.
    pub fn load(manifest: &Manifest) -> Self {
        let dir = std::env::var_os("PLATYPUS_PREFS_DIR")
            .map(PathBuf::from)
            .or_else(|| settings_dir(manifest));
        let Some(dir) = dir else {
            tracing::warn!("no settings folder: player prefs won't be saved");
            return Self::in_memory();
        };
        Self::open(dir.join("prefs.json"))
    }

    pub fn open(path: PathBuf) -> Self {
        let values = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(Value::Object(map)) => map,
                _ => {
                    tracing::warn!("{}: not a JSON object, starting afresh", path.display());
                    Map::new()
                }
            },
            Err(_) => Map::new(),
        };
        Self {
            path: Some(path),
            values,
            dirty: false,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn all(&self) -> &Map<String, Value> {
        &self.values
    }

    pub fn set(&mut self, key: &str, value: Value) {
        if self.values.get(key) != Some(&value) {
            self.values.insert(key.to_owned(), value);
            self.dirty = true;
        }
    }

    pub fn delete(&mut self, key: &str) {
        self.dirty |= self.values.remove(key).is_some();
    }

    /// Every key, the engine's too (Unity's `DeleteAll`).
    pub fn delete_all(&mut self) {
        self.dirty |= !self.values.is_empty();
        self.values.clear();
    }

    /// Writes the file if anything changed (via a temporary file, so a crash mid-write
    /// can't leave half a file).
    pub fn save(&mut self) {
        let Some(path) = &self.path else { return };
        if !self.dirty {
            return;
        }
        let result = (|| {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            let text = serde_json::to_string_pretty(&self.values).map_err(std::io::Error::other)?;
            std::fs::write(&tmp, text)?;
            std::fs::rename(&tmp, path)
        })();
        match result {
            Ok(()) => self.dirty = false,
            Err(e) => tracing::warn!("saving {}: {e}", path.display()),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// A script value as JSON (prefs hold numbers, strings and the odd list).
pub fn json_of(v: &platypus_core::Value) -> Value {
    match v {
        platypus_core::Value::Null => Value::Null,
        platypus_core::Value::Bool(b) => Value::Bool(*b),
        platypus_core::Value::Number(n) => {
            serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number)
        }
        platypus_core::Value::String(s) => Value::String(s.clone()),
        platypus_core::Value::Array(items) => Value::Array(items.iter().map(json_of).collect()),
    }
}

/// A file-name-safe piece of a path (no separators, dots at the ends or control
/// characters; "Platypus" if nothing's left).
fn safe(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    if cleaned.is_empty() {
        "Platypus".into()
    } else {
        cleaned
    }
}

/// The folder for this game's settings on this OS (see the module docs).
pub fn settings_dir(manifest: &Manifest) -> Option<PathBuf> {
    if !manifest.company.is_empty() {
        return settings_dir_of(manifest, &manifest.company);
    }
    let dir = settings_dir_of(manifest, "Platypus")?;
    // A game without a company kept its settings under "Iron" before the rename to Platypus.
    match settings_dir_of(manifest, "Iron") {
        Some(old) if !dir.exists() && old.is_dir() => {
            tracing::info!(
                "settings from before the rename to Platypus: {}",
                old.display()
            );
            Some(old)
        }
        _ => Some(dir),
    }
}

fn settings_dir_of(manifest: &Manifest, company: &str) -> Option<PathBuf> {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let company = safe(company);
    let title = safe(&manifest.title);
    if cfg!(target_os = "windows") {
        Some(env("APPDATA")?.join(company).join(title))
    } else if cfg!(target_os = "macos") {
        let id = if manifest.identifier.is_empty() {
            format!("{company}.{title}")
        } else {
            safe(&manifest.identifier)
        };
        Some(env("HOME")?.join("Library/Application Support").join(id))
    } else {
        let base = env("XDG_CONFIG_HOME").or_else(|| Some(env("HOME")?.join(".config")))?;
        Some(base.join(company).join(title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-PRF-01
    #[test]
    fn prefs_save_and_load_back_and_skip_unchanged_writes() {
        let dir = std::env::temp_dir().join(format!("platypus-prefs-test-{}", std::process::id()));
        let path = dir.join("nested").join("prefs.json");
        let _ = std::fs::remove_dir_all(&dir);
        let mut p = Prefs::open(path.clone());
        assert!(p.get("volume").is_none());
        p.set("volume", Value::from(0.5));
        p.set("name", Value::from("Ada"));
        p.save();
        assert!(path.is_file(), "folders made on the way");
        let mut again = Prefs::open(path.clone());
        assert_eq!(again.get("volume"), Some(&Value::from(0.5)));
        assert_eq!(again.get("name"), Some(&Value::from("Ada")));
        // Setting the same value isn't a change.
        again.set("volume", Value::from(0.5));
        assert!(!again.dirty);
        again.delete("name");
        again.save();
        assert!(Prefs::open(path.clone()).get("name").is_none());
        // A broken file starts afresh rather than failing.
        std::fs::write(&path, "not json").unwrap();
        assert!(Prefs::open(path).all().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn folder_names_are_made_safe() {
        assert_eq!(safe("Platypus - Arcade"), "Platypus - Arcade");
        assert_eq!(safe("a/b\\\\c:d"), "a_b__c_d");
        assert_eq!(safe(" .. "), "Platypus");
    }
}
