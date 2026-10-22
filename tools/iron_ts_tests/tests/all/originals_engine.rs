//! Engine API shapes the original (QuickJS-era) example sources use, from compiled code
//! (`bench/originals_engine.ts`, prefix `ogn_`), in the normal, stress GC and collection-off
//! modes. IDs are in TESTS.md (M9-TS-3184..).
use iron_scene::Scene;
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::originals_engine_t::*;

fn reset() {
    iron_render_api::reset();
    iron_engine_api::camera::reset();
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        reset();
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        unsafe { ts_ogn_init() };
        f();
    };
    run();
    ts_rt_gc_stress(true);
    run();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    run();
    ts_rt_gc_enable(true);
    reset();
}

fn text(p: *mut u8) -> String {
    assert!(!p.is_null());
    String::from_utf16_lossy(&unsafe { Str::from_raw(p as *const Str) }.to_utf16())
}

// M9-TS-3184
#[test]
fn pop_and_shift_over_handle_arrays_give_a_handle_or_nothing() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_ogn_handleArrays()), "true|true|true|true|0");
    });
}

// M9-TS-3185
#[test]
fn line_renderer_points_read_and_write_as_a_property() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_ogn_lineRendererPoints()), "3|3|4|true|0");
    });
}

// M9-TS-3186
#[test]
fn colors_come_from_spread_arrays_with_alpha_defaulting_to_one() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_ogn_colorFromArrays()), "0.5|0.75|0.5|1");
    });
}

// M9-TS-3187
#[test]
fn flat_meshes_take_only_some_of_their_lists() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_ogn_partialFlatMesh()), "3|1|3");
    });
}

// M9-TS-3188
#[test]
fn engine_enum_strings_and_the_programs_literal_unions_meet() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(
            text(ts_ogn_enumsMeetUnions()),
            "exponential,linear,off,exponentialSquared,exponential|exponential|false|E"
        );
    });
}

// M9-TS-3189
#[test]
fn handles_are_map_values_map_keys_and_set_members() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_ogn_handlesInMaps()), "beta|true|3|2|alphabeta");
    });
}
