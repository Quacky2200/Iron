//! Shared setup: a QuickJS context with the whole native API and a current scene.
#![allow(dead_code)]
use iron_scene::{Scene, api::enter};
use rquickjs::{Ctx, FromJs};

/// Runs `f` with every native API installed, a fresh world on this thread and a current scene.
/// Engine state that holds script callbacks is cleared before the runtime is dropped.
pub fn with_world<R>(f: impl FnOnce(&Ctx<'_>) -> R) -> R {
    iron_physics_api::reset();
    iron_render_api::reset();
    iron_engine_api::camera::reset();
    iron_engine_api::input::reset();
    iron_engine_api::audio::reset();
    iron_engine_api::audio_components::reset();
    iron_engine_api::gizmos::reset();
    iron_engine_api::assets::reset();
    iron_engine_api::app::reset();
    iron_engine_api::screen::reset();
    iron_engine_api::animation::reset();
    iron_engine_api::window::reset();
    iron_scenes::reset();
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    // Callbacks hold script functions: release them before the runtime goes (also on a panic).
    let _release = Release;
    ctx.with(|ctx| {
        iron_native::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

struct Release;

impl Drop for Release {
    fn drop(&mut self) {
        iron_engine_api::input::reset();
        iron_engine_api::assets::reset();
        iron_engine_api::app::reset();
        iron_engine_api::screen::reset();
        iron_engine_api::audio::reset();
        iron_engine_api::audio_components::reset();
        iron_engine_api::window::reset();
        iron_render_api::reset();
        iron_scenes::reset();
    }
}

/// Evaluates script text; an exception panics with its message.
pub fn js<T: for<'a> FromJs<'a>>(ctx: &Ctx<'_>, code: &str) -> T {
    match ctx.eval::<T, _>(code) {
        Ok(v) => v,
        Err(rquickjs::Error::Exception) => {
            let e = ctx.catch();
            let msg = e
                .as_object()
                .and_then(|o| o.get::<_, String>("message").ok())
                .unwrap_or_else(|| format!("{e:?}"));
            panic!("{msg}\n--- in ---\n{code}")
        }
        Err(e) => panic!("{e}"),
    }
}

/// The message of the error `code` throws (the test fails when it does not throw).
pub fn throws(ctx: &Ctx<'_>, code: &str) -> String {
    js::<String>(
        ctx,
        &format!(
            "(() => {{ try {{ {code}; return 'NO ERROR'; }} catch (e) {{ return e.name + ': ' + e.message; }} }})()"
        ),
    )
}
