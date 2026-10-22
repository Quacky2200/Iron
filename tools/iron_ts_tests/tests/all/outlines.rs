//! The native `Shapes` and `Spline` from compiled code, against the TypeScript the facade used to
//! compile (`bench/outlines.ts`, prefix `otl_`), in the normal, stress GC and collection-off modes.
use iron_scene::Scene;
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::outlines_t::*;

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        iron_engine_api::spline::reset();
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
    iron_engine_api::spline::reset();
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

// M9-TS-3112
#[test]
fn native_splines_equal_the_facade_typescript_bit_for_bit() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_otl_splineMatch()), "");
    });
}

// M9-TS-3114
#[test]
fn spline_knots_hits_and_enum_names_are_plain_objects_and_strings() {
    in_every_gc_mode(|| unsafe {
        // The hit is in the first segment half way; the point (1, 3, 0) is 3 from the line;
        // `parse` gives the index of a name (-1 for none) and `name` the name of an index.
        assert_eq!(
            text(ts_otl_splineSnapshotsAreObjects()),
            "2|linear|2|50|0|300|1|linear|bezier|-1|0.25"
        );
    });
}
