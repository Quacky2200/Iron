// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Scenes and scene files in Rust: `scenes.ts`'s `SceneManager`, `Scene`, `Prefab` and
//! `GameObject.instantiate`, with the same JSON format (`platypus.scene` version 1). Design:
//! `docs/research/platypus-native-scenes.md`.
//!
//! The loader is generic over the component kinds registered in [`platypus_kinds`]: this crate
//! registers `Camera`, `Animation` and `Animator`; a compiled program's `NativeHost` registers
//! one per class it has. A component of a kind that is not registered is kept as data (an
//! [`Opaque`] component, with a warning) and written back when the scene is saved.

mod api;
mod data;
#[cfg(test)]
mod data_tests;
mod editor_api;
#[cfg(test)]
mod editor_tests;
mod kinds;
#[cfg(test)]
mod kinds_tests;
mod manager;

pub use api::{manifests, scene_manager_api};
pub use data::{Opaque, opaque_components, remove_opaque, take_warnings, warn};
pub use editor_api::editor_scene_api;
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
