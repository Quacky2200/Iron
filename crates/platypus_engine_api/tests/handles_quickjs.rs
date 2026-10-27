// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The engine's handle classes from QuickJS: `GameObject`, `Transform`, `Camera`, `AnimationCurve`.
//! The same state is read through the compiled-code shims. IDs are in TESTS.md (M9-API-*).
use platypus_api::qjs::HandleBox;
use platypus_engine_api::{animation, camera};
use platypus_scene::{Scene, api::enter};
use rquickjs::{Class, Ctx, Value};

fn run<R>(f: impl FnOnce(&Ctx<'_>) -> R) -> R {
    camera::reset();
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        platypus_engine_api::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

fn js<T: for<'a> rquickjs::FromJs<'a>>(ctx: &Ctx<'_>, code: &str) -> T {
    match ctx.eval::<T, _>(code) {
        Ok(v) => v,
        Err(rquickjs::Error::Exception) => panic!("{:?}", ctx.catch()),
        Err(e) => panic!("{e}"),
    }
}

/// The id inside a handle object.
fn raw_of(v: &Value<'_>) -> u64 {
    Class::<HandleBox>::from_value(v).unwrap().borrow().raw
}

// M9-API-108
#[test]
fn game_objects_transforms_and_cameras_are_opaque_handles() {
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const go = GameObject.create();
            go.name = 'hero';
            go.transform.position = new Vector3(1, 2, 3);
            const cam = Camera.attach(go);
            cam.fieldOfView = 90;
            JSON.stringify([
                go.name, go.transform.position.y, JSON.stringify(go), JSON.stringify(go.transform),
                cam.gameObject.name, Camera.findOn(go) !== null, cam.fieldOfView,
                String(go), String(go.transform), String(cam),
                go.transform.gameObject.name, GameObject.find('hero') !== null, GameObject.find('nobody'),
                go.parent, go instanceof GameObject, go.transform instanceof Transform,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["hero",2,"{}","{}","hero",true,90,"[object GameObject]","[object Transform]","[object Camera]","hero",true,null,null,true,true]"#
        );
        // The two classes share an id but are not interchangeable.
        let err: String = js(
            ctx,
            "const g2 = GameObject.create(); try { Scene.destroy(g2.transform); 'no error' } catch (e) { e.message }",
        );
        assert_eq!(err, "expected a GameObject, got a Transform");
    });
}

// M9-API-109
#[test]
fn handles_made_by_script_are_the_same_state_compiled_code_sees() {
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    run(|ctx| {
        let go: Value = ctx.eval("globalThis.g = GameObject.create(); g").unwrap();
        let cam: Value = ctx
            .eval("globalThis.c = Camera.attach(g); c.fieldOfView = 75; c")
            .unwrap();
        // Compiled code reads through the shims with the raw ids.
        assert_eq!(
            camera::camera_api::platypus_Camera_fieldOfView(raw_of(&cam)),
            75.0
        );
        camera::camera_api::platypus_Camera_setFieldOfView(raw_of(&cam), 40.0);
        let fov: f64 = js(ctx, "c.fieldOfView");
        assert_eq!(fov, 40.0);
        let count: f64 = js(ctx, "Cameras.count()");
        assert_eq!(
            count,
            f64::from(camera::camera_api::platypus_Cameras_count())
        );
        // A handle the host made is usable from the script.
        ctx.globals()
            .set(
                "fromHost",
                platypus_api::qjs::handle_new(ctx, "GameObject", raw_of(&go)).unwrap(),
            )
            .unwrap();
        let same: bool = js(
            ctx,
            "fromHost.transform.position.x === g.transform.position.x",
        );
        assert!(same);
    });
}

// M9-API-110
#[test]
fn animation_curves_take_arrays_and_return_handles() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const c = AnimationCurve.linear(0, 0, 2, 10);
            const d = AnimationCurve.fromPoints([new Vector2(0, 0), new Vector2(1, 4), [2, 0]], 'linear');
            JSON.stringify([c.evaluate(1), c.length, d.evaluate(0.5), d.evaluate(1.5), JSON.stringify(c),
                String(d), c.addKey(3, 20) >= 0, c.evaluate(3)])
            "#,
        );
        assert_eq!(out, r#"[5,2,2,2,"{}","[object AnimationCurve]",true,20]"#);
        let _ = animation::animation_curve_api::MANIFEST;
    });
}
