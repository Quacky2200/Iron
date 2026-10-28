// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Which program this is. The same source is built as three binaries, told apart by their name
//! at compile time (as `editor::ENABLED` does): `platypus-game` (games: the full engine), `platypus-app`
//! (apps that are only UI: a lighter renderer, no 3D world) and `platypus_editor`.

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether this program is `platypus-app`.
pub(crate) const IS_UI_APP: bool =
    super::editor::same(env!("CARGO_BIN_NAME").as_bytes(), b"platypus-app");

/// Set for a run of `platypus-game` that is only UI (the video-settings prompt).
static UI_ONLY: AtomicBool = AtomicBool::new(false);

/// Whether this run takes `platypus-app`'s light path: the GLES renderer (else wgpu with small
/// GPU blocks) and no world renderer unless something asks for it.
pub(crate) fn ui_only() -> bool {
    IS_UI_APP || UI_ONLY.load(Ordering::Relaxed)
}

/// Makes this run UI only, whichever program it is.
pub(crate) fn set_ui_only() {
    UI_ONLY.store(true, Ordering::Relaxed);
}
