// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Functions that return an unsubscribe function (`#[subscription(off = ..)]`) from QuickJS: the
//! same native code compiled games call (`tools/platypus_ts_tests/tests/engine_api/facade.rs`, M9-TS-3351).
//! IDs are in TESTS.md (M9-API-1636).
mod common;
use common::{js, with_world};
use platypus_engine_api::{app, assets, screen};

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

// M9-API-1637
//
// `#[promise]` functions give a real promise that the engine settles through two callbacks:
// `SceneManager.loadSceneAsync` and `Prefab.load` resolve in the tick the host loads the file and
// reject with an `Error` for a file that cannot be read or parsed.
#[test]
fn load_functions_return_promises_the_host_settles() {
    use platypus_scenes as manager;
    with_world(|ctx| {
        platypus_scenes::register_engine_kinds();
        let text: String = js(
            ctx,
            r#"
            const root = GameObject.create();
            root.name = 'crate';
            const level = SceneManager.createScene('level');
            SceneManager.moveGameObjectToScene(root, level);
            level.toJson()
            "#,
        );
        let _: f64 = js(
            ctx,
            r#"
            globalThis.log = [];
            globalThis.p1 = SceneManager.loadSceneAsync('lvl.scene.json', 'additive');
            p1.then(s => log.push('ok:' + s.path), e => log.push('err:' + e.message));
            SceneManager.loadSceneAsync('bad.scene.json').then(s => log.push('bad ok'), e => log.push(e instanceof Error ? 'bad:' + e.message.slice(0, 30) : 'not an error'));
            Prefab.load('p.prefab.json').then(p => log.push('prefab'), e => log.push('prefab err'));
            0
            "#,
        );
        assert_eq!(js::<String>(ctx, "typeof p1.then"), "function");
        let requests = manager::take_load_requests();
        assert_eq!(requests.len(), 2);
        for (id, path) in requests {
            if path == "lvl.scene.json" {
                manager::finish_load(id, &text).unwrap();
            } else {
                assert!(manager::finish_load(id, "not json").is_err());
            }
        }
        let (pid, _) = manager::take_prefab_requests()[0].clone();
        assert!(manager::finish_prefab(pid, Err("missing")).is_err());
        platypus_scenes::dispatch();
        while ctx.execute_pending_job() {}
        let log: String = js(ctx, "JSON.stringify(log)");
        assert_eq!(
            log,
            r#"["ok:lvl.scene.json","bad:could not load scene bad.scene","prefab err"]"#
        );
    });
}

// M9-API-1638
//
// `Texture.whenLoaded()` resolves in the call that reports the load (at once for a texture that
// has loaded) and rejects with an `Error` when the load failed.
#[test]
fn when_loaded_promises_settle_when_the_host_reports_the_texture() {
    use platypus_render_api::{TextureRef, texture_loaded};
    with_world(|ctx| {
        let _: f64 = js(
            ctx,
            r#"
            globalThis.log = [];
            globalThis.a = Texture.load('a.png');
            globalThis.b = Texture.load('b.png');
            a.whenLoaded().then(t => log.push('a:' + (t.src === a.src) + ':' + t.width), e => log.push('a err'));
            b.whenLoaded().then(t => log.push('b ok'), e => log.push(e instanceof Error ? 'b:' + e.message : '?'));
            0
            "#,
        );
        while ctx.execute_pending_job() {}
        assert_eq!(js::<String>(ctx, "JSON.stringify(log)"), "[]");
        texture_loaded(TextureRef(1), 8, 4);
        texture_loaded(TextureRef(2), 0, 0);
        while ctx.execute_pending_job() {}
        assert_eq!(
            js::<String>(ctx, "JSON.stringify(log)"),
            r#"["a:true:8","b:texture failed to load"]"#
        );
        // Already loaded: settled without waiting.
        let _: f64 = js(ctx, "a.whenLoaded().then(() => log.push('again')); 0");
        while ctx.execute_pending_job() {}
        assert_eq!(js::<String>(ctx, "log[2]"), "again");
    });
}
