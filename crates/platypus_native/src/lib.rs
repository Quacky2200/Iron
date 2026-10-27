// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The whole native script API in one place: what the app installs to run scripts in QuickJS.
//!
//! `register_quickjs(ctx)` installs every `#[platypus_api(.., quickjs)]` namespace of the engine
//! (math, scene, engine services, scenes, physics, rendering data) in a QuickJS context, and
//! [`install`] does it for a [`QuickJsRuntime`], again after each `reset()`. Handles are opaque
//! objects there (see `platypus_api::qjs`); the state they refer to lives in Rust, per thread.

use platypus_api::rquickjs::{Ctx, Result};
use platypus_script::{QuickJsRuntime, ScriptError};

/// Installs every native namespace in a context, in dependency order (the math classes and
/// `GameObject` first).
pub fn register_quickjs<'js>(ctx: &Ctx<'js>) -> Result<()> {
    platypus_engine_api::register_quickjs(ctx)?;
    platypus_scenes::register_quickjs(ctx)?;
    platypus_physics_api::register_quickjs(ctx)?;
    platypus_render_api::register_quickjs(ctx)?;
    Ok(())
}

/// Installs the native API in a runtime, now and after every `reset()`.
pub fn install(rt: &mut QuickJsRuntime) -> std::result::Result<(), ScriptError> {
    rt.add_native(register_quickjs)
}

/// The manifests (JSON) of every namespace, for the compiler to check calls against.
pub fn manifests() -> Vec<String> {
    let mut all = platypus_engine_api::manifests();
    all.push(platypus_scene::api::scene_api::MANIFEST.to_json());
    all.extend(platypus_scenes::manifests());
    all.extend(platypus_physics_api::manifests());
    all.extend(platypus_render_api::manifests());
    all
}
