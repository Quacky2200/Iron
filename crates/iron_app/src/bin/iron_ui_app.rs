// A windowed program, like `iron-game`.
#![cfg_attr(not(test), windows_subsystem = "windows")]
//! `iron-app`: runs Iron projects of type `app` (only UI). The same program as `iron-game`
//! (main.rs, included as a module), which sees `host::IS_UI_APP` and takes the light path:
//! no 3D world renderer, small GPU allocations.

// main.rs's crate-level attributes are for when it is the crate root.
#[allow(unused_attributes)]
#[path = "../main.rs"]
mod app;

// The shared modules reach each other through `crate::`.
use app::{
    anim_cull, asset_db, bone_cache, cook, info, models, particles, reload, sync_profile,
    world_view,
};

fn main() -> anyhow::Result<()> {
    app::main()
}
