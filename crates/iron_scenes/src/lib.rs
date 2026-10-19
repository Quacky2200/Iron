//! Scenes and scene files in Rust: `scenes.ts`'s `SceneManager`, `Scene`, `Prefab` and
//! `GameObject.instantiate`, with the same JSON format (`iron.scene` version 1). Design:
//! `docs/research/iron-native-scenes.md`.
//!
//! The loader is generic over the component kinds registered in [`iron_kinds`]: this crate
//! registers `Camera`, `Animation` and `Animator`; a compiled program's `NativeHost` registers
//! one per class it has. A component of a kind that is not registered is skipped with a warning.

mod api;
mod data;
mod kinds;
mod manager;

pub use api::{manifests, scene_manager_api};
pub use data::{take_warnings, warn};
pub use kinds::register_engine_kinds;
pub use manager::*;

use std::cell::Cell;

thread_local! {
    static KINDS_READY: Cell<bool> = const { Cell::new(false) };
}

/// Registers the engine's kinds once per thread (every entry point calls this).
pub(crate) fn ensure_kinds() {
    if !KINDS_READY.with(|k| k.replace(true)) {
        register_engine_kinds();
    }
}
