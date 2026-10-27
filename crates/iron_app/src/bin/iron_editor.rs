// SPDX-License-Identifier: Apache-2.0

// A windowed program, like `iron`.
#![cfg_attr(not(test), windows_subsystem = "windows")]
//! `iron_editor`: the Iron editor. The same program as `iron` (main.rs, included as a
//! module), which sees `editor::ENABLED` and loads the editor's UI (`editor/dist`) with
//! the editor-only ops (`editor.*`) turned on.

// main.rs's crate-level attributes are for when it is the crate root.
#[allow(unused_attributes)]
#[path = "../main.rs"]
mod app;

// The shared modules reach each other through `crate::`.
use app::{
    anim_cull, asset_db, bone_cache, cook, info, models, particles, postfx, reload, sync_profile,
    world_view,
};

fn main() -> anyhow::Result<()> {
    app::main()
}
