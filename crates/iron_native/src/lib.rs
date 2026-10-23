// SPDX-License-Identifier: Apache-2.0

//! The whole native script API in one place: what the app installs to run scripts in QuickJS.
//!
//! `register_quickjs(ctx)` installs every `#[iron_api(.., quickjs)]` namespace of the engine
//! (math, scene, engine services, scenes, physics, rendering data) in a QuickJS context, and
//! [`install`] does it for a [`QuickJsRuntime`], again after each `reset()`. Handles are opaque
//! objects there (see `iron_api::qjs`); the state they refer to lives in Rust, per thread.

use iron_api::rquickjs::{Ctx, Result};
use iron_script::{QuickJsRuntime, ScriptError};

/// Installs every native namespace in a context, in dependency order (the math classes and
/// `GameObject` first).
pub fn register_quickjs<'js>(ctx: &Ctx<'js>) -> Result<()> {
    iron_engine_api::register_quickjs(ctx)?;
    iron_scenes::register_quickjs(ctx)?;
    iron_physics_api::register_quickjs(ctx)?;
    iron_render_api::register_quickjs(ctx)?;
    Ok(())
}

/// Installs the native API in a runtime, now and after every `reset()`.
pub fn install(rt: &mut QuickJsRuntime) -> std::result::Result<(), ScriptError> {
    rt.add_native(register_quickjs)
}

/// The manifests (JSON) of every namespace, for the compiler to check calls against.
pub fn manifests() -> Vec<String> {
    let mut all = iron_engine_api::manifests();
    all.push(iron_scene::api::scene_api::MANIFEST.to_json());
    all.extend(iron_scenes::manifests());
    all.extend(iron_physics_api::manifests());
    all.extend(iron_render_api::manifests());
    all
}
