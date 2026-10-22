//! The engine's script API beyond the scene, declared with `#[iron_api]`: `Color`, `Time`, `input`,
//! `Gizmos`, `Camera`, `Screen`, `QualitySettings` and `Window`. Each module keeps its state per thread and
//! has plain Rust functions for the host (the app) to feed or read; scripts, compiled or in
//! QuickJS, only see the generated API. The names and behaviour follow `js/engine`.
//!
//! Wiring this into `iron_app` is the app's job: call [`time::advance`] and the `input` event
//! functions each frame, run scripts, then read [`gizmos::take_frame`] and [`camera::cameras`].
//! Design: `docs/research/iron-ts-engine-boundary.md` and `docs/iron-api.md`.

pub use iron_math;

pub mod animation;
pub mod animator;
pub mod app;
pub mod assets;
pub mod audio;
pub mod audio_components;
pub mod camera;
pub mod clock;
pub mod color;
pub mod cursor;
pub mod datetime;
pub mod gizmos;
pub mod input;
pub mod layers;
pub mod log;
pub mod objects;
pub mod prefs;
pub mod random;
pub mod screen;
pub mod spline;
pub mod time;
pub mod ui_resources;
pub mod window;

/// The manifests (JSON) of every namespace this crate declares, for the compiler to check calls
/// against. The scene's (`iron_scene::api::scene_api`) is separate.
pub fn manifests() -> Vec<String> {
    let mut all = iron_math::manifests();
    all.extend([
        animation::keyframe_api::MANIFEST.to_json(),
        animation::animation_curve_api::MANIFEST.to_json(),
        animation::animation_clip_api::MANIFEST.to_json(),
        animation::animation_component_api::MANIFEST.to_json(),
        animation::animation_state_api::MANIFEST.to_json(),
        animation::animations_api::MANIFEST.to_json(),
        animator::animation_parameter_api::MANIFEST.to_json(),
        animator::animation_condition_api::MANIFEST.to_json(),
        animator::animation_transition_api::MANIFEST.to_json(),
        animator::animation_blend_tree_api::MANIFEST.to_json(),
        animator::animation_node_api::MANIFEST.to_json(),
        animator::animation_controller_api::MANIFEST.to_json(),
        animator::animation_builder_api::MANIFEST.to_json(),
        animator::animator_api::MANIFEST.to_json(),
        color::color_api::MANIFEST.to_json(),
        cursor::cursor_api::MANIFEST.to_json(),
        cursor::cursor_lock_mode_api::MANIFEST.to_json(),
        datetime::time_span_api::MANIFEST.to_json(),
        datetime::date_time_api::MANIFEST.to_json(),
        time::time_api::MANIFEST.to_json(),
        clock::time_since_api::MANIFEST.to_json(),
        clock::time_until_api::MANIFEST.to_json(),
        input::input_api::MANIFEST.to_json(),
        gizmos::gizmos_api::MANIFEST.to_json(),
        camera::camera_api::MANIFEST.to_json(),
        screen::screen_api::MANIFEST.to_json(),
        screen::quality_api::MANIFEST.to_json(),
        audio::audio_api::MANIFEST.to_json(),
        audio_components::audio_source_api::MANIFEST.to_json(),
        audio_components::audio_listener_api::MANIFEST.to_json(),
        random::rng_api::MANIFEST.to_json(),
        random::random_api::MANIFEST.to_json(),
        layers::layer_mask_api::MANIFEST.to_json(),
        objects::hide_flags_api::MANIFEST.to_json(),
        log::debug_api::MANIFEST.to_json(),
        log::log_api::MANIFEST.to_json(),
        log::console_api::MANIFEST.to_json(),
        prefs::prefs_api::MANIFEST.to_json(),
        assets::assets_api::MANIFEST.to_json(),
        app::application_api::MANIFEST.to_json(),
        app::system_info_api::MANIFEST.to_json(),
        ui_resources::ui_resources_api::MANIFEST.to_json(),
        window::window_api::MANIFEST.to_json(),
        spline::spline_api::MANIFEST.to_json(),
    ]);
    all
}

/// Installs every QuickJS-enabled namespace of the engine in a context: the math classes, the
/// scene (`Scene`, `GameObject`, `Transform`), and this crate's. Handles (`GameObject`, `Camera`,
/// `AnimationClip`...) are opaque objects there (see `iron_api::qjs`). Audio, `Rng` and `Cursor` are in it.
pub fn register_quickjs<'js>(ctx: &iron_api::rquickjs::Ctx<'js>) -> iron_api::rquickjs::Result<()> {
    iron_math::register_quickjs(ctx)?;
    iron_scene::api::scene_api::register_quickjs(ctx)?;
    animation::keyframe_api::register_quickjs(ctx)?;
    animation::animation_curve_api::register_quickjs(ctx)?;
    animation::animation_clip_api::register_quickjs(ctx)?;
    animation::animation_component_api::register_quickjs(ctx)?;
    animation::animation_state_api::register_quickjs(ctx)?;
    animation::animations_api::register_quickjs(ctx)?;
    animator::animation_parameter_api::register_quickjs(ctx)?;
    animator::animation_condition_api::register_quickjs(ctx)?;
    animator::animation_transition_api::register_quickjs(ctx)?;
    animator::animation_blend_tree_api::register_quickjs(ctx)?;
    animator::animation_node_api::register_quickjs(ctx)?;
    animator::animation_controller_api::register_quickjs(ctx)?;
    animator::animation_builder_api::register_quickjs(ctx)?;
    animator::animator_api::register_quickjs(ctx)?;
    color::color_api::register_quickjs(ctx)?;
    datetime::time_span_api::register_quickjs(ctx)?;
    datetime::date_time_api::register_quickjs(ctx)?;
    time::time_api::register_quickjs(ctx)?;
    clock::time_since_api::register_quickjs(ctx)?;
    clock::time_until_api::register_quickjs(ctx)?;
    input::input_api::register_quickjs(ctx)?;
    gizmos::gizmos_api::register_quickjs(ctx)?;
    camera::camera_api::register_quickjs(ctx)?;
    screen::screen_api::register_quickjs(ctx)?;
    screen::quality_api::register_quickjs(ctx)?;
    layers::layer_mask_api::register_quickjs(ctx)?;
    objects::hide_flags_api::register_quickjs(ctx)?;
    log::debug_api::register_quickjs(ctx)?;
    log::log_api::register_quickjs(ctx)?;
    log::console_api::register_quickjs(ctx)?;
    prefs::prefs_api::register_quickjs(ctx)?;
    assets::assets_api::register_quickjs(ctx)?;
    app::application_api::register_quickjs(ctx)?;
    app::system_info_api::register_quickjs(ctx)?;
    ui_resources::ui_resources_api::register_quickjs(ctx)?;
    random::random_api::register_quickjs(ctx)?;
    random::rng_api::register_quickjs(ctx)?;
    audio::audio_api::register_quickjs(ctx)?;
    audio_components::audio_source_api::register_quickjs(ctx)?;
    audio_components::audio_listener_api::register_quickjs(ctx)?;
    cursor::cursor_api::register_quickjs(ctx)?;
    cursor::cursor_lock_mode_api::register_quickjs(ctx)?;
    window::register_quickjs(ctx)?;
    spline::spline_api::register_quickjs(ctx)?;
    Ok(())
}
