//! `bench/comp_query.ts` compiled against the native engine API and compared with the original
//! TypeScript run in bun (`bench/gen_comp_query_expected.ts`), in the normal, stress GC and
//! collection-off modes. IDs are in TESTS.md (M9-TS-703..).
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::comp_query_t::*;
use serde_json::Value;

/// A new scene and a host for the program classes.
fn reset() {
    iron_scene::api::with_scene(|s| *s = iron_scene::Scene::new());
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    let mut scene = iron_scene::Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    unsafe { ts_cqy_init() };
    // Every mode starts with a new host (the registry of live components) and scene.
    let mut host = iron_ts_rt::NativeHost::new(classes());
    reset();
    f();
    ts_rt_gc_stress(true);
    drop(host);
    host = iron_ts_rt::NativeHost::new(classes());
    reset();
    f();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    drop(host);
    host = iron_ts_rt::NativeHost::new(classes());
    reset();
    f();
    ts_rt_gc_enable(true);
    drop(host);
}

fn expected() -> Value {
    serde_json::from_str(include_str!("../../bench/comp_query_expected.json")).unwrap()
}

fn check_str(name: &str, got: *mut u8) {
    let want: Vec<u16> = expected()[name]["s"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect();
    assert!(!got.is_null(), "{name} returned null");
    let got = unsafe { Str::from_raw(got as *const Str) }.to_utf16();
    assert_eq!(
        String::from_utf16_lossy(&got),
        String::from_utf16_lossy(&want),
        "{name}"
    );
}

// M9-TS-703
#[test]
fn lookups_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("lookups", ts_cqy_lookups());
    });
}

// M9-TS-704
#[test]
fn children_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("children", ts_cqy_children());
    });
}

// M9-TS-705
#[test]
fn finding_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("finding", ts_cqy_finding());
    });
}

// M9-TS-706
#[test]
fn removing_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("removing", ts_cqy_removing());
    });
}
