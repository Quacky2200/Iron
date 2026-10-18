//! Compiled scripts driving the real engine scene (`bench/scene.ts`): `iron_scene`'s `Scene`
//! API, written with `#[iron_api]`, is called directly by native code the compiler made from
//! TypeScript. The scene is made current with `iron_scene::api::enter`, as the host will around
//! running scripts. Every test runs normally, in stress GC mode and with collection off.
//! IDs are in TESTS.md (M9-TS-151..).
use iron_scene::Scene;
use iron_ts_rt::{link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::scn::*;

fn in_every_gc_mode(f: impl Fn()) {
    assert_ne!(link_anchor(), 0);
    unsafe { ts_scn_init() };
    f();
    ts_rt_gc_stress(true);
    f();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    f();
    ts_rt_gc_enable(true);
}

// M9-TS-151
//
// A parent and child made, moved and destroyed from compiled code; the child's world position
// includes the parent's, `Scene.parent(x)` is truthy only when there is one, and destroying the
// parent takes the child with it.
#[test]
fn compiled_code_builds_and_moves_a_hierarchy_in_the_real_scene() {
    let mut scene = Scene::new();
    let scope = iron_scene::api::enter(&mut scene);
    in_every_gc_mode(|| unsafe {
        assert_eq!(ts_scn_moveAround(), 110_203_100.0);
    });
    drop(scope);
    assert!(scene.is_empty());
}

// M9-TS-152
#[test]
fn a_per_frame_update_loop_moves_an_object_round_a_circle() {
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    in_every_gc_mode(|| unsafe {
        let want: f64 = (0..50)
            .map(|i| {
                let a = i as f64 * 0.1;
                a.cos() * 5.0 + a.sin() * 5.0
            })
            .sum();
        let got = ts_scn_orbit(50.0);
        assert!((got - want).abs() < 1e-3, "{got} vs {want}");
    });
}

// M9-TS-153
#[test]
fn stale_object_handles_miss_even_when_the_slot_is_reused() {
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    in_every_gc_mode(|| unsafe {
        assert_eq!(ts_scn_staleHandles(), 10.0);
    });
}

// M9-TS-154
#[test]
fn a_quaternion_value_goes_in_and_comes_back_normalised() {
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    in_every_gc_mode(|| unsafe {
        let q = glam::Quat::from_xyzw(0.0, 0.5, 0.0, 0.5).normalize();
        assert_eq!(ts_scn_rotation(), q.y as f64 * 1000.0 + q.w as f64);
    });
}

// M9-TS-155
//
// Thirty objects, their handles in an array, between collections at every allocation.
#[test]
fn many_objects_and_their_handles_survive_collections() {
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    in_every_gc_mode(|| unsafe {
        assert_eq!(ts_scn_crowd(30.0), 870_030.0);
        assert_eq!(iron_ts_tests::scene_anchor(), 0);
    });
}
