// SPDX-License-Identifier: Apache-2.0

//! `SceneManager`, `Scene` and `Prefab` from QuickJS: handles are opaque objects. IDs are in
//! TESTS.md (M9-API-*).
use iron_scene::{Scene, api::enter};
use rquickjs::Ctx;

fn run<R>(f: impl FnOnce(&Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        iron_engine_api::register_quickjs(&ctx).unwrap();
        iron_scenes::register_quickjs(&ctx).unwrap();
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

// M9-API-111
#[test]
fn scenes_and_prefabs_are_handle_classes_in_quickjs() {
    let mut world = Scene::new();
    let _scope = enter(&mut world);
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const level = SceneManager.createScene('level');
            const go = GameObject.create();
            go.name = 'crate';
            go.transform.position = new Vector3(1, 2, 3);
            SceneManager.moveGameObjectToScene(go, level);
            const prefab = Prefab.fromObject(go);
            const copy = prefab.instantiate();
            const noScene = SceneManager.getSceneByName('missing');
            JSON.stringify([
                level.name, level.isLoaded, level.rootCount, level.rootAt(0).name,
                SceneManager.sceneOf(go).name, SceneManager.getSceneByName('level').name,
                String(level), String(prefab), JSON.stringify(level), JSON.parse(JSON.stringify(prefab)).includes('iron.prefab'),
                prefab.name, copy !== null && copy.name, level instanceof Scene,
                noScene === null || noScene.isLoaded === false,
                SceneManager.setActiveScene(level), SceneManager.getActiveScene().name,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["level",true,1,"crate","level","level","[object Scene]","[object Prefab]","{}",true,"crate","crate",true,true,true,"level"]"#
        );
    });
}
