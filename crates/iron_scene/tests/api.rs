//! The scene's `#[iron_api]` namespace: handles, values and the current scene. IDs are in
//! TESTS.md (M9-API-*).
use glam::Quat;
use iron_api::Handle;
use iron_scene::api::{enter, scene_api as api};
use iron_scene::{ObjectId, Scene};

// M9-API-12
//
// A handle is the index and generation offset so that no live object is 0 (0 is "no object" in a
// script's `GameObject | null`), and it converts back exactly.
#[test]
fn an_object_handle_is_never_zero_and_round_trips() {
    for id in [
        ObjectId {
            index: 0,
            generation: 0,
        },
        ObjectId {
            index: 7,
            generation: 3,
        },
        ObjectId {
            index: u32::MAX,
            generation: u32::MAX - 1,
        },
    ] {
        let raw = id.to_raw();
        assert_ne!(raw, 0, "{id:?}");
        assert_eq!(ObjectId::from_raw(raw), id);
    }
    // And the "no object" handle names no slot.
    let mut scene = Scene::new();
    let first = scene.create("a");
    assert!(!scene.contains(ObjectId::from_raw(0)));
    assert_ne!(first.to_raw(), 0);
}

// M9-API-13
#[test]
fn the_shims_create_move_and_destroy_objects_in_the_current_scene() {
    let mut scene = Scene::new();
    {
        let _scope = enter(&mut scene);
        let a = api::iron_Scene_create();
        let b = api::iron_Scene_create();
        assert_ne!(a, 0);
        assert_ne!(a, b);
        assert_eq!(api::iron_Scene_count(), 2);
        assert!(api::iron_Scene_exists(a));

        // A Vector3 argument is three f32s; the result comes back through a pointer.
        api::iron_Scene_setLocalPosition(a, 1.0, 2.0, 3.0);
        let mut out = [0.0f32; 3];
        unsafe { api::iron_Scene_localPosition(a, out.as_mut_ptr()) };
        assert_eq!(out, [1.0, 2.0, 3.0]);

        // Parenting: the world position includes the parent's.
        api::iron_Scene_setLocalPosition(b, 10.0, 0.0, 0.0);
        api::iron_Scene_setParent(a, b, false);
        assert_eq!(api::iron_Scene_parent(a), b);
        unsafe { api::iron_Scene_position(a, out.as_mut_ptr()) };
        assert_eq!(out, [11.0, 2.0, 3.0]);

        // Setting the world position works back through the parent.
        api::iron_Scene_setPosition(a, 10.0, 5.0, 0.0);
        unsafe { api::iron_Scene_localPosition(a, out.as_mut_ptr()) };
        assert_eq!(out, [0.0, 5.0, 0.0]);

        // Destroying the parent destroys the child; the ids stop working.
        api::iron_Scene_destroy(b);
        assert!(!api::iron_Scene_exists(a));
        assert!(!api::iron_Scene_exists(b));
        assert_eq!(api::iron_Scene_count(), 0);
        unsafe { api::iron_Scene_position(a, out.as_mut_ptr()) };
        assert_eq!(out, [0.0, 0.0, 0.0]);
        // The slot is reused with a new generation: the old handle still misses.
        let c = api::iron_Scene_create();
        assert!(api::iron_Scene_exists(c));
        assert!(!api::iron_Scene_exists(a));
    }
    // Without a current scene every function answers its default.
    assert_eq!(api::iron_Scene_create(), 0);
    assert_eq!(api::iron_Scene_count(), 0);
}

// M9-API-14
#[test]
fn rotation_is_a_quaternion_value_and_flags_are_booleans() {
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    let a = api::iron_Scene_create();
    let q = Quat::from_rotation_y(0.5);
    api::iron_Scene_setLocalRotation(a, q.x, q.y, q.z, q.w);
    let mut out = [0.0f32; 4];
    unsafe { api::iron_Scene_localRotation(a, out.as_mut_ptr()) };
    let back = Quat::from_xyzw(out[0], out[1], out[2], out[3]);
    assert!(back.abs_diff_eq(q, 1e-6), "{back:?} vs {q:?}");
    assert!(api::iron_Scene_enabled(a));
    api::iron_Scene_setEnabled(a, false);
    assert!(!api::iron_Scene_enabled(a));
    assert!(!api::iron_Scene_active(a));
    api::iron_Scene_setLayer(a, 7);
    assert_eq!(api::iron_Scene_layer(a), 7);
    api::iron_Scene_setLayer(a, 999);
    assert_eq!(api::iron_Scene_layer(a), 31);
}

// M9-API-15
#[test]
fn the_manifest_declares_the_handle_and_both_value_classes() {
    let m = api::MANIFEST;
    assert_eq!(m.name, "Scene");
    assert_eq!(m.handles, &["GameObject", "Transform"]);
    let names: Vec<&str> = m.values.iter().map(|v| v.name).collect();
    assert_eq!(names, ["Vector3", "Quaternion"]);
    let d = m.to_dts();
    assert!(d.contains("function create(): GameObject;"), "{d}");
    assert!(
        d.contains("function setPosition(object: GameObject, position: Vector3): void;"),
        "{d}"
    );
    assert!(
        d.contains("function rotation(object: GameObject): Quaternion;"),
        "{d}"
    );
    assert!(d.contains("declare class Quaternion {"), "{d}");
}

// M9-API-19
//
// `GameObject` and `Transform` as classes: the shims take the receiver first, a transform shares
// its object's handle, and the declaration file has accessors and methods.
#[test]
fn game_object_and_transform_classes_work_through_their_shims() {
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    let go = api::iron_GameObject_create();
    assert!(api::iron_GameObject_exists(go));
    let t = api::iron_GameObject_transform(go);
    assert_eq!(t, go, "a transform is the object's own handle");
    assert_eq!(api::iron_Transform_gameObject(t), go);
    api::iron_Transform_setLocalScale(t, 2.0, 2.0, 2.0);
    api::iron_Transform_setPosition(t, 1.0, 2.0, 3.0);
    let mut out = [0.0f32; 3];
    unsafe { api::iron_Transform_position(t, out.as_mut_ptr()) };
    assert_eq!(out, [1.0, 2.0, 3.0]);
    api::iron_GameObject_setEnabled(go, false);
    assert!(!api::iron_GameObject_activeInHierarchy(go));
    assert_eq!(api::iron_Transform_parent(t), 0, "a root has no parent");
    api::iron_GameObject_destroy(go);
    assert!(!api::iron_GameObject_exists(go));

    let d = api::MANIFEST.to_dts();
    assert!(d.contains("declare class GameObject {"), "{d}");
    assert!(d.contains("static create(): GameObject;"), "{d}");
    assert!(d.contains("get transform(): Transform;"), "{d}");
    assert!(d.contains("get name(): string;"), "{d}");
    assert!(d.contains("set name(value: string);"), "{d}");
    assert!(d.contains("static find(name: string): GameObject;"), "{d}");
    assert!(d.contains("get position(): Vector3;"), "{d}");
    assert!(d.contains("set position(value: Vector3);"), "{d}");
    assert!(
        d.contains("setParent(parent: Transform, keepWorld: boolean): void;"),
        "{d}"
    );
}

// M9-API-26
//
// The same Scene API from QuickJS: typed functions on the current scene, values as plain objects.
// This is what the compiled path replaces, so both must agree.
#[test]
fn the_scene_api_works_from_quickjs() {
    let mut scene = Scene::new();
    let _scope = enter(&mut scene);
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        api::register_quickjs(&ctx).unwrap();
        let out: String = ctx
            .eval(
                r#"
                const parent = Scene.create();
                const child = Scene.create();
                Scene.setLocalPosition(parent, { x: 10, y: 0, z: 0 });
                Scene.setParent(child, parent, false);
                Scene.setLocalPosition(child, { x: 1, y: 2, z: 3 });
                const p = Scene.position(child);
                const r = Scene.rotation(child);
                const before = Scene.count();
                Scene.destroy(parent);
                JSON.stringify([p.x, p.y, p.z, r.w, before, Scene.count(), Scene.exists(child)]);
                "#,
            )
            .unwrap();
        assert_eq!(out, "[11,2,3,1,2,0,false]");
    });
}

// M9-API-36
//
// Value classes are real JS classes in QuickJS: `new Vector3(..)`, methods and getters from the
// Rust functions, results that are instances.
#[test]
fn value_classes_have_constructors_methods_and_getters_in_quickjs() {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        api::register_quickjs(&ctx).unwrap();
        let out: String = ctx
            .eval(
                r#"
                const a = new Vector3(1, 2, 2);
                const b = a.add(new Vector3(1, 1, 1)).scale(2);
                const q = new Quaternion(0, 0, 0, 1);
                const r = q.rotate(new Vector3(0, 1, 0));
                JSON.stringify([a.magnitude, b.x, b.y, b.z, b instanceof Vector3, r.y, a.dot(a)]);
                "#,
            )
            .unwrap();
        assert_eq!(out, "[3,4,6,6,true,1,9]");
    });
}
