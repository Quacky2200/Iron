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
    assert_eq!(m.handles, &["GameObject"]);
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
