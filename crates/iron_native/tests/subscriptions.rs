//! Functions that return an unsubscribe function (`#[subscription(off = ..)]`) from QuickJS: the
//! same native code compiled games call (`tools/iron_ts_tests/tests/all/facade.rs`, M9-TS-3351).
//! IDs are in TESTS.md (M9-API-1636).
mod common;
use common::{js, with_world};
use iron_engine_api::{app, assets, screen};

// M9-API-1636
#[test]
fn listeners_return_a_function_that_stops_them() {
    with_world(|ctx| {
        let _: f64 = js(
            ctx,
            r#"
            globalThis.seen = [];
            globalThis.offs = [
              Assets.onChanged((path) => seen.push('asset:' + path)),
              Application.onFocusChanged((focused) => seen.push('focus:' + focused)),
              Screen.onChange(() => seen.push('screen')),
              QualitySettings.onLevelChanged((level) => seen.push('level:' + level)),
              SceneManager.onSceneLoaded((scene, mode) => seen.push('loaded:' + mode)),
              SceneManager.onSceneUnloaded((scene) => seen.push('unloaded')),
              SceneManager.onActiveSceneChanged((from, to) => seen.push('active')),
            ];
            0
            "#,
        );
        let kinds: String = js(ctx, "JSON.stringify(offs.map(f => typeof f))");
        assert_eq!(
            kinds,
            r#"["function","function","function","function","function","function","function"]"#
        );
        assets::changed(["a.png".to_string()]);
        app::set_focus(false, false);
        screen::set_size(1280, 720);
        js::<f64>(ctx, "QualitySettings.setQualityLevel(1); 0");
        let before: String = js(ctx, "JSON.stringify(seen)");
        assert_eq!(
            before,
            r#"["asset:a.png","focus:false","screen","level:1"]"#
        );
        // Stopped listeners hear nothing more; stopping twice is harmless.
        let _: f64 = js(
            ctx,
            "for (const off of offs) off(); for (const off of offs) off(); 0",
        );
        assets::changed(["b.png".to_string()]);
        app::set_focus(true, true);
        screen::set_size(640, 480);
        js::<f64>(ctx, "QualitySettings.setQualityLevel(2); 0");
        let after: String = js(ctx, "JSON.stringify(seen)");
        assert_eq!(after, before);
    });
}
