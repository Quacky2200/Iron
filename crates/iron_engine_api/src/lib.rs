//! The engine's script API beyond the scene, declared with `#[iron_api]`: `Color`, `Time`, `input`,
//! `Gizmos`, `Camera`, `Screen` and `QualitySettings`. Each module keeps its state per thread and
//! has plain Rust functions for the host (the app) to feed or read; scripts, compiled or in
//! QuickJS, only see the generated API. The names and behaviour follow `js/engine`.
//!
//! Wiring this into `iron_app` is the app's job: call [`time::advance`] and the `input` event
//! functions each frame, run scripts, then read [`gizmos::take_frame`] and [`camera::cameras`].
//! Design: `docs/research/iron-ts-engine-boundary.md` and `docs/iron-api.md`.

pub mod audio;
pub mod camera;
pub mod color;
pub mod gizmos;
pub mod input;
pub mod screen;
pub mod time;
