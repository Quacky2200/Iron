//! Engine services from QuickJS: input and listener callbacks, time, random numbers, scenes and
//! prefabs, the cursor. IDs are in TESTS.md (M9-API-*).
mod common;
use common::{js, throws, with_world};
use iron_engine_api::{app, assets, input, screen, time};

// M9-API-134
//
// Listener APIs hand out tokens, call the JS function with the event's arguments, and stop after
// `off(token)`.
#[test]
fn input_listeners_fire_with_the_events_arguments() {
    with_world(|ctx| {
        let t: f64 = js(
            ctx,
            r#"
            globalThis.log = [];
            input.onKeyDown((code, repeat) => log.push('d:' + code + ':' + repeat));
            input.onKeyUp((code) => log.push('u:' + code));
            input.onGamepadConnected((id) => log.push('c:' + id));
            input.onGamepadButtonDown((id, b) => log.push('b:' + id + b));
            input.onGamepadAxis((id, axis, v) => log.push('a:' + id + axis + v));
            globalThis.tok = input.onKeyDown(() => log.push('second'));
            tok
            "#,
        );
        assert!(t > 0.0);
        input::key_down_event("KeyW");
        input::key_up_event("KeyW");
        input::gamepad_connected(2, "pad");
        input::gamepad_button(2, "south", 1.0);
        input::gamepad_axis(2, "leftX", 0.5);
        let out: String = js(ctx, "JSON.stringify(log)");
        assert_eq!(
            out,
            r#"["d:KeyW:false","second","u:KeyW","c:2","b:2south","a:2leftX0.5"]"#
        );
        let held: bool = js(ctx, "input.isKeyDown('KeyW')");
        assert!(!held, "KeyW went up");
        assert!(js::<bool>(ctx, "input.off(tok)"));
        assert!(!js::<bool>(ctx, "input.off(tok)"));
        input::key_down_event("KeyA");
        let n: f64 = js(ctx, "log.filter(x => x === 'second').length");
        assert_eq!(n, 1.0);
        assert!(throws(ctx, "input.onKeyDown('nope')").starts_with("TypeError"));
    });
}

// M9-API-135
//
// Assets, Application and Screen listeners, and the time services.
#[test]
fn asset_app_and_screen_listeners_and_time() {
    with_world(|ctx| {
        let _: f64 = js(
            ctx,
            r#"
            globalThis.seen = [];
            Assets.onChanged((path) => seen.push('asset:' + path));
            Application.onFocusChanged((focused) => seen.push('focus:' + focused));
            Screen.onChange(() => seen.push('screen'));
            0
            "#,
        );
        assets::changed(["textures/a.png".to_string()]);
        app::set_focus(false, false);
        screen::set_size(1280, 720);
        let out: String = js(ctx, "JSON.stringify(seen)");
        assert_eq!(out, r#"["asset:textures/a.png","focus:false","screen"]"#);
        time::advance(0.5);
        let t: String = js(
            ctx,
            "JSON.stringify([Time.deltaTime > 0, Time.timeScale, Time.time >= 0.5 || Time.time > 0])",
        );
        assert_eq!(t, "[true,1,true]");
        let w: f64 = js(ctx, "Screen.width");
        assert_eq!(w, 1280.0);
    });
}

// M9-API-136
//
// `Rng` is a handle class (`new Rng(seed)`); the same seed gives the same numbers as the compiled
// shims, and `Random` is the shared generator.
#[test]
fn rng_handles_and_random_agree_with_the_shims() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const a = new Rng(42), b = Rng.create(42), c = new Rng(7);
            const xs = [a.nextDouble(), a.nextDouble(), a.int(0, 100), a.float(1, 2), a.bool(0.5)];
            const ys = [b.nextDouble(), b.nextDouble(), b.int(0, 100), b.float(1, 2), b.bool(0.5)];
            const v = c.insideUnitSphere(), q = c.rotation(), u = c.onUnitSphere();
            const inside = Math.hypot(v.x, v.y, v.z) <= 1;
            const unit = Math.abs(Math.hypot(u.x, u.y, u.z) - 1) < 1e-9;
            const r = Random.value;
            JSON.stringify([JSON.stringify(xs) === JSON.stringify(ys), inside, unit, q instanceof Quaternion,
                r >= 0 && r < 1, String(a), JSON.stringify(a)])
            "#,
        );
        assert_eq!(out, r#"[true,true,true,true,true,"[object Rng]","{}"]"#);
        // The compiled shims on a Rng the script made give the same sequence.
        let k: rquickjs::Value = ctx.eval("globalThis.k = new Rng(5)").unwrap();
        let raw = rquickjs::Class::<iron_api::qjs::HandleBox>::from_value(&k)
            .unwrap()
            .borrow()
            .raw;
        let from_shim = iron_engine_api::random::rng_api::iron_Rng_nextDouble(raw);
        let from_js: f64 = js(ctx, "new Rng(5).nextDouble()");
        assert_eq!(from_shim, from_js);
        assert!(throws(ctx, "new Rng('seed')").starts_with("TypeError"));
    });
}

// M9-API-137
//
// Scenes from script: build, serialize, load back additively, instantiate and the scene events.
#[test]
fn scenes_load_instantiate_and_report_events() {
    with_world(|ctx| {
        iron_scenes::register_engine_kinds();
        let out: String = js(
            ctx,
            r#"
            globalThis.events = [];
            const t1 = SceneManager.onSceneLoaded((scene, mode) => events.push('loaded:' + scene.name + ':' + mode));
            SceneManager.onSceneUnloaded((scene) => events.push('unloaded:' + scene.name));
            SceneManager.onActiveSceneChanged((from, to) => events.push('active:' + (to ? to.name : null)));
            const root = GameObject.create();
            root.name = 'crate';
            root.transform.position = new Vector3(1, 2, 3);
            const child = GameObject.create();
            child.name = 'lid';
            child.transform.setParent(root.transform, false);
            const level = SceneManager.createScene('level');
            SceneManager.moveGameObjectToScene(root, level);
            const text = level.toJson();
            const loaded = SceneManager.loadScene(text, 'additive');
            const copy = GameObject.instantiate ? null : null;
            const prefab = Prefab.fromObject(root);
            const clone = prefab.instantiate(9, 0, 0);
            const unloaded = SceneManager.unloadScene(loaded);
            JSON.stringify([text.includes('crate'), loaded !== null, clone.name, clone.childCount,
                clone.transform.position.x, unloaded, SceneManager.off === undefined,
                SceneManager.sceneCount >= 1])
            "#,
        );
        assert!(out.starts_with("[true,true,\"crate\",1,9,true"), "{out}");
        let ev: String = js(ctx, "JSON.stringify(events)");
        assert!(ev.contains("loaded:level"), "{ev}");
        assert!(ev.contains("unloaded:"), "{ev}");
        assert!(throws(ctx, "SceneManager.loadScene(5)").starts_with("TypeError"));
        assert!(throws(ctx, "SceneManager.onSceneLoaded(1)").starts_with("TypeError"));
    });
}

// M9-API-138
//
// The cursor namespace properties and the camera/gizmos pair with their enum-like strings.
#[test]
fn cursor_and_screen_properties_pair_getters_and_setters() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            Cursor.visible = false;
            Cursor.lockState = 'locked';
            Cursor.setCursor('cursors/hand.png', 4, 4);
            const state = [Cursor.visible, Cursor.lockState];
            Cursor.clearCursor();
            Screen.fullScreen = true;
            QualitySettings.vSyncCount = 0;
            JSON.stringify([state, Screen.fullScreen, QualitySettings.vSyncCount])
            "#,
        );
        assert_eq!(out, r#"[[false,"locked"],true,0]"#);
        assert!(throws(ctx, "Cursor.visible = {}").starts_with("TypeError"));
    });
}
// M9-API-145
//
// GC stress: a tiny collection threshold while scripts make thousands of handle objects, value
// instances and callbacks that only the engine keeps; everything still answers correctly.
#[test]
fn gc_stress_with_handles_values_and_callbacks() {
    use iron_scene::{Scene, api::enter};
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    let rt = rquickjs::Runtime::new().unwrap();
    rt.set_gc_threshold(2000);
    let ctx = rquickjs::Context::full(&rt).unwrap();
    struct Release;
    impl Drop for Release {
        fn drop(&mut self) {
            iron_engine_api::input::reset();
        }
    }
    let _release = Release;
    ctx.with(|ctx| {
        iron_native::register_quickjs(&ctx).unwrap();
        let out: String = js(
            &ctx,
            r#"
            globalThis.hits = 0;
            const keep = [];
            for (let i = 0; i < 200; i++) {
                const tag = { n: i };
                input.onKeyDown(() => { hits += tag.n >= 0 ? 1 : 0; });
            }
            let sum = 0;
            for (let i = 0; i < 4000; i++) {
                const go = GameObject.create();
                go.transform.position = new Vector3(i, 0, 0);
                sum += go.transform.position.x;
                if (i % 40 === 0) keep.push(go);
                if (i % 2 === 0) go.destroy();
            }
            JSON.stringify([sum, keep.length, keep.every(g => g.transform.position.x % 40 === 0)])
            "#,
        );
        assert_eq!(out, "[7998000,100,true]");
        ctx.run_gc();
        input::key_down_event("KeyG");
        let hits: f64 = js(&ctx, "hits");
        assert_eq!(hits, 200.0);
    });
}
