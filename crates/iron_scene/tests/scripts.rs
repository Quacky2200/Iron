//! GameObjects and components driven by a real script bundle in QuickJS, the way the
//! app does it.

use iron_core::{ComponentId, Value};
use iron_scene::{SceneEvent, ScriptScene};
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};

const SCENE: &str = include_str!("../../../tests/fixtures/scene.js");

fn tick(rt: &mut QuickJsRuntime, host: &mut ComponentHost, scene: &mut ScriptScene) {
    let errors = host.update_active(rt, 1.0 / 60.0, &|o| scene.active(o));
    assert!(errors.is_empty(), "{errors:?}");
    for op in rt.drain_ops() {
        // The engine's other ops (`render.quality` on load) aren't the scene's business.
        let Some(name) = op.name.strip_prefix("scene.") else {
            continue;
        };
        match scene
            .apply(name, &op.args)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
        {
            None => {}
            Some(SceneEvent::ComponentAdded {
                object,
                component,
                class,
            }) => host.adopt(ComponentId(component), &class, Some(object)),
            Some(SceneEvent::ComponentRemoved(id)) => host.remove(rt, ComponentId(id)),
            Some(SceneEvent::Destroyed(objects)) => host.remove_on_objects(rt, &objects),
        }
    }
}

// M5-OBJ-06
#[test]
fn script_components_live_on_game_objects() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("scene.js", SCENE).unwrap();
    let mut host = ComponentHost::new();
    let mut scene = ScriptScene::new();
    let game = scene.create_engine_object("Game");
    host.spawn_on(&mut rt, "Game", Some(game), "Game").unwrap();

    for _ in 0..31 {
        tick(&mut rt, &mut host, &mut scene);
    }
    // Turret, Barrel and the engine's Game object.
    assert_eq!(scene.scene.len(), 3);
    let barrel = scene.scene.find("Barrel").unwrap();
    // The spinner ran ticks 2-31 at 90°/s: 45° of yaw, swinging the barrel (1 m
    // forward of the turret) round.
    let p = scene.scene.world(barrel).position;
    let s = std::f32::consts::FRAC_1_SQRT_2;
    assert!((p - glam::Vec3::new(-s, 2.0, -s)).length() < 1e-3, "{p}");
    let turret = scene.scene.find("Turret").unwrap();
    assert!(
        !scene.scene.active(barrel) && !scene.scene.active(turret),
        "turret disabled on tick 31"
    );

    // Disabled: the spinner stops; then the turret is destroyed with its children.
    for _ in 0..30 {
        tick(&mut rt, &mut host, &mut scene);
    }
    assert_eq!(scene.scene.len(), 1);
    assert_eq!(host.len(), 1);
    let log = rt.eval("__scene_log()").unwrap();
    assert_eq!(
        log,
        Value::String(
            "game start on Game, spinner start on Turret, spinner destroyed at 45°".into()
        )
    );
}

// M5-OBJ-08
#[test]
fn lifecycle_hooks_run_in_unity_order() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("scene.js", SCENE).unwrap();
    let mut host = ComponentHost::new();
    let id = host.spawn(&mut rt, "Lifecycle").unwrap();
    let all = |_: u32| true;
    // A frame: fixed ticks first, then update, late update, pre-render.
    host.fixed_update(&mut rt, 1.0 / 60.0, &all);
    host.fixed_update(&mut rt, 1.0 / 60.0, &all);
    host.update_active(&mut rt, 1.0 / 30.0, &all);
    host.pre_render(&mut rt, &all);
    host.remove(&mut rt, id);
    assert_eq!(
        rt.eval("__hooks_log()").unwrap(),
        Value::String("start fixed fixed update late prerender destroy".into())
    );
}

// M5-OBJ-07
#[test]
fn engine_moves_reach_scripts_and_bad_ops_fail() {
    let mut s = ScriptScene::new();
    let num = |v: f64| Value::Number(v);
    let arr = |v: &[f64]| Value::Array(v.iter().map(|x| Value::Number(*x)).collect());
    s.apply(
        "create",
        &[num(1.0), "a".into(), Value::Null, arr(&[1.0, 0.0, 0.0])],
    )
    .unwrap();
    s.apply("create", &[num(2.0), "b".into(), num(1.0)])
        .unwrap();
    for (name, args) in [
        ("create", vec![num(3.0), "c".into(), num(99.0)]),
        ("local", vec![num(42.0)]),
        ("parent", vec![num(1.0), num(2.0)]),
        (
            "local",
            vec![num(1.0), Value::Null, arr(&[0.0, 0.0, 0.0, 0.0])],
        ),
        ("warp", vec![]),
    ] {
        assert!(s.apply(name, &args).is_err(), "{name} {args:?} should fail");
    }

    // Script changes aren't echoed back; engine moves are, as local transforms.
    assert!(s.take_batch().is_empty());
    s.set_world_from_engine(2, glam::Vec3::new(5.0, 0.0, 0.0), glam::Quat::IDENTITY);
    let batch = s.take_batch();
    assert_eq!(batch.len(), 11);
    assert_eq!(batch[0], num(2.0));
    assert_eq!(batch[1], num(4.0), "local x under the parent at x = 1");

    // Ids past 2^24 (script components start there) stay exact.
    let added = s
        .apply("addComponent", &[num(2.0), num(16_777_217.0), "C".into()])
        .unwrap();
    assert_eq!(
        added,
        Some(SceneEvent::ComponentAdded {
            object: 2,
            component: 16_777_217,
            class: "C".into()
        })
    );

    let destroyed = s.apply("destroy", &[num(1.0)]).unwrap();
    assert_eq!(destroyed, Some(SceneEvent::Destroyed(vec![1, 2])));
    assert!(s.object(2).is_none());
}

// M5-LAY-01
#[test]
fn objects_keep_the_layer_scripts_give_them() {
    let mut s = ScriptScene::new();
    let num = |v: f64| Value::Number(v);
    s.apply("create", &[num(1.0), "a".into(), Value::Null])
        .unwrap();
    assert_eq!(s.layer(1), 0, "Default to start with");
    s.apply("layer", &[num(1.0), num(5.0)]).unwrap();
    assert_eq!(s.layer(1), 5);
    for bad in [num(32.0), num(-1.0), Value::Null] {
        assert!(s.apply("layer", &[num(1.0), bad]).is_err());
    }
    assert_eq!(s.layer(1), 5);
    assert!(s.apply("layer", &[num(9.0), num(1.0)]).is_err());
    assert_eq!(s.layer(9), 0, "unknown objects are on Default");
}

// M5-SCN-02
#[test]
fn scenes_save_and_load_in_quickjs() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("scene.js", SCENE).unwrap();
    let mut host = ComponentHost::new();
    let mut scene = ScriptScene::new();
    let trip = scene.create_engine_object("SceneTrip");
    host.spawn_on(&mut rt, "SceneTrip", Some(trip), "SceneTrip")
        .unwrap();
    tick(&mut rt, &mut host, &mut scene);
    tick(&mut rt, &mut host, &mut scene);
    assert_eq!(
        rt.eval("__scene_trip()").unwrap(),
        Value::String("Room+Lamp 3 true 7 DontDestroyOnLoad".into())
    );
    // The first copies went; the loaded ones and the kept trip object are left.
    assert_eq!(scene.scene.len(), 4);
    let door = scene.scene.find("Door").unwrap();
    assert_eq!(
        scene.scene.world(door).position,
        glam::Vec3::new(0.0, 0.0, -5.0)
    );
    // Door and Light on the loaded objects, and the trip itself.
    assert_eq!(host.len(), 3);
}
