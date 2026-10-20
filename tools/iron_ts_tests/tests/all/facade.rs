//! The TypeScript `@iron/engine` shapes over the native API (`bench/facade.ts`, prefix `fac_`),
//! in the normal, stress GC and collection-off modes. IDs are in TESTS.md (M9-TS-1700..).
use iron_scene::Scene;
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::facade_t::*;

fn reset() {
    iron_render_api::reset();
    iron_engine_api::camera::reset();
    iron_engine_api::assets::reset();
    iron_scenes::reset();
    iron_scenes::take_warnings();
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        reset();
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        unsafe { ts_fac_init() };
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

// M9-TS-1700
#[test]
fn object_valued_render_settings_split_into_the_field_setters() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(
            text(ts_fac_skyAndBloom()),
            "true|1|2|true|false|false|true|1.2|0.55|false|false"
        );
    });
}

// M9-TS-1701
#[test]
fn game_object_options_are_local_to_the_parent_and_null_is_no_parent() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(
            text(ts_fac_gameObjectOptions()),
            "true|true|3|false|1|1|4|3"
        );
    });
}

// M9-TS-1702
#[test]
fn camera_rect_cull_distances_main_camera_and_screen_getters() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(
            text(ts_fac_cameraAndScreen()),
            "0.25|0.25|14|0|true|60|true|true"
        );
    });
}

// M9-TS-1703
#[test]
fn positional_constructors_with_texture_and_create_primitive_options() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_fac_constructors()), "1|0.25|true|4|1|Ball|2|2");
    });
}

// M9-TS-1704
#[test]
fn lod_levels_cloth_options_and_root_objects_go_through_the_element_wise_api() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_fac_structured()), "2|1|0|true|true");
    });
}

// M9-TS-1705
#[test]
fn the_animator_controller_builder_takes_object_options() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_fac_animatorController()), "2|2|2|Locomotion");
    });
}

// M9-TS-1706
#[test]
fn sprite_create_with_a_rect_and_sprite_grid() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_fac_sprites()), "40|true|2|10");
    });
}
