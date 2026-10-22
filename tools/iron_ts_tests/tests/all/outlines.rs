//! The native `Shapes` from compiled code, against the TypeScript the facade used to
//! compile (`bench/outlines.ts`, prefix `otl_`), in the normal, stress GC and collection-off modes.
use iron_scene::Scene;
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::outlines_t::*;

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        unsafe { ts_otl_init() };
        f();
    };
    run();
    ts_rt_gc_stress(true);
    run();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    run();
    ts_rt_gc_enable(true);
}

fn text(p: *mut u8) -> String {
    assert!(!p.is_null());
    String::from_utf16_lossy(&unsafe { Str::from_raw(p as *const Str) }.to_utf16())
}

// M9-TS-3110
#[test]
fn native_shapes_equal_the_facade_typescript_bit_for_bit() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_otl_shapesMatch()), "");
    });
}

// M9-TS-3111
#[test]
fn shape_points_are_vector2_objects_and_object_literals_pass_where_points_go() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_otl_shapePointsAreVectors()), "4|3|200|2|2");
    });
}
