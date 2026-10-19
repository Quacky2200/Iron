//! The engine's script API beyond the scene, declared with `#[iron_api]`: `Color`, `Time`, `input`,
//! `Gizmos`, `Camera`, `Screen` and `QualitySettings`. Each module keeps its state per thread and
//! has plain Rust functions for the host (the app) to feed or read; scripts, compiled or in
//! QuickJS, only see the generated API. The names and behaviour follow `js/engine`.
//!
//! Wiring this into `iron_app` is the app's job: call [`time::advance`] and the `input` event
//! functions each frame, run scripts, then read [`gizmos::take_frame`] and [`camera::cameras`].
//! Design: `docs/research/iron-ts-engine-boundary.md` and `docs/iron-api.md`.

pub mod app;
pub mod assets;
pub mod audio;
pub mod camera;
pub mod clock;
pub mod color;
pub mod gizmos;
pub mod input;
pub mod layers;
pub mod log;
pub mod prefs;
pub mod random;
pub mod screen;
pub mod time;

/// The manifests (JSON) of every namespace this crate declares, for the compiler to check calls
/// against. The scene's (`iron_scene::api::scene_api`) is separate.
pub fn manifests() -> Vec<String> {
    vec![
        color::color_api::MANIFEST.to_json(),
        time::time_api::MANIFEST.to_json(),
        clock::time_since_api::MANIFEST.to_json(),
        clock::time_until_api::MANIFEST.to_json(),
        input::input_api::MANIFEST.to_json(),
        gizmos::gizmos_api::MANIFEST.to_json(),
        camera::camera_api::MANIFEST.to_json(),
        screen::screen_api::MANIFEST.to_json(),
        screen::quality_api::MANIFEST.to_json(),
        audio::audio_api::MANIFEST.to_json(),
        random::rng_api::MANIFEST.to_json(),
        random::random_api::MANIFEST.to_json(),
        layers::layer_mask_api::MANIFEST.to_json(),
        log::debug_api::MANIFEST.to_json(),
        log::log_api::MANIFEST.to_json(),
        log::console_api::MANIFEST.to_json(),
        prefs::prefs_api::MANIFEST.to_json(),
        assets::assets_api::MANIFEST.to_json(),
        app::application_api::MANIFEST.to_json(),
        app::system_info_api::MANIFEST.to_json(),
    ]
}
