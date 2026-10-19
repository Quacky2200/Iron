//! `Assets`: the asset database (each asset `.meta` file gives it a GUID and importer settings),
//! `"guid:<guid>"` sources, and the paths of files that changed on disk (hot reload). The host
//! fills it ([`set_assets`], [`changed`]); scripts read it. `js/engine/src/assets.ts` had
//! `onChanged(fn)`; until closures cross the boundary here, scripts poll `takeChanges()`.

use iron_api::iron_api;
use std::cell::RefCell;

/// One asset's database entry.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetEntry {
    /// Game-relative, forward slashes.
    pub path: String,
    pub guid: String,
    /// Its importer settings as a JSON object (`{}` when there are none).
    pub importer_json: String,
}

#[derive(Default)]
struct State {
    entries: Vec<AssetEntry>,
    changed: Vec<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn set_assets(entries: Vec<AssetEntry>) {
    STATE.with(|s| s.borrow_mut().entries = entries);
}

/// Files that changed on disk: scripts read them with `takeChanges()`.
pub fn changed(paths: impl IntoIterator<Item = String>) {
    STATE.with(|s| s.borrow_mut().changed.extend(paths));
}

pub fn reset() {
    STATE.with(|s| *s.borrow_mut() = State::default());
}

fn clean(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.strip_prefix("./").unwrap_or(&p).to_string()
}

fn guid_to_path(guid: &str) -> Option<String> {
    STATE.with(|s| {
        s.borrow()
            .entries
            .iter()
            .find(|e| e.guid == guid)
            .map(|e| e.path.clone())
    })
}

/// A path for `"guid:<guid>"` sources (others unchanged; an unknown GUID stays as given).
pub fn resolve(src: &str) -> String {
    match src.strip_prefix("guid:") {
        Some(guid) => guid_to_path(guid).unwrap_or_else(|| src.to_string()),
        None => src.to_string(),
    }
}

#[iron_api(namespace = "Assets", quickjs)]
pub mod assets_api {
    use super::{STATE, clean};

    /// How many assets have a `.meta` file.
    pub fn count() -> f64 {
        STATE.with(|s| s.borrow().entries.len() as f64)
    }

    /// The path of the i-th asset ("" past the end).
    pub fn path_at(index: f64) -> String {
        STATE.with(|s| {
            s.borrow()
                .entries
                .get(index as usize)
                .map(|e| e.path.clone())
                .unwrap_or_default()
        })
    }

    /// The GUID of the i-th asset ("" past the end).
    pub fn guid_at(index: f64) -> String {
        STATE.with(|s| {
            s.borrow()
                .entries
                .get(index as usize)
                .map(|e| e.guid.clone())
                .unwrap_or_default()
        })
    }

    /// The game path of the asset with this GUID ("" when there is none).
    pub fn guid_to_path(guid: &str) -> String {
        super::guid_to_path(guid).unwrap_or_default()
    }

    /// The GUID of the asset at a game path ("" when it has no `.meta`).
    pub fn path_to_guid(path: &str) -> String {
        let p = clean(path);
        STATE.with(|s| {
            s.borrow()
                .entries
                .iter()
                .find(|e| e.path == p)
                .map(|e| e.guid.clone())
                .unwrap_or_default()
        })
    }

    /// An asset's importer settings from its `.meta`, as a JSON object (`{}` without one).
    pub fn importer(path: &str) -> String {
        let p = clean(&super::resolve(path));
        STATE.with(|s| {
            s.borrow()
                .entries
                .iter()
                .find(|e| e.path == p)
                .map_or_else(|| "{}".to_string(), |e| e.importer_json.clone())
        })
    }

    /// A path for `"guid:<guid>"` sources (others unchanged). Unknown GUIDs stay as given.
    pub fn resolve(src: &str) -> String {
        super::resolve(src)
    }

    /// The paths of asset files that changed since the last call, one per line.
    pub fn take_changes() -> String {
        STATE.with(|s| std::mem::take(&mut s.borrow_mut().changed).join("\n"))
    }
}
