//! `bench/app.ts` compiled against the native engine API and compared with the original
//! TypeScript run in bun (`bench/gen_app_expected.ts`), in the normal, stress GC and
//! collection-off modes. IDs are in TESTS.md (M9-TS-621..).
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::app_t::*;
use serde_json::Value;

/// The manifest and the machine as the engine installs them.
fn reset() {
    use iron_engine_api::app::{self, AppInfo, SystemInfo};
    app::reset();
    app::take_requests();
    app::set_app(AppInfo {
        name: "demo".into(),
        product_name: "Demo Game".into(),
        version: "1.2.3".into(),
        company_name: "Acme".into(),
        identifier: "com.acme.demo".into(),
        engine_version: "0.9.1".into(),
        is_editor: false,
        run_in_background: false,
        target_frame_rate: 144,
    });
    app::set_system(SystemInfo {
        operating_system: "Windows 11 Pro".into(),
        operating_system_family: "Windows".into(),
        operating_system_version: "10.0.22631".into(),
        device_type: "Desktop".into(),
        device_name: "PC-1".into(),
        processor_type: "Ryzen".into(),
        processor_count: 16,
        system_memory_size: 32768,
        architecture: "x86_64".into(),
        language: "en-GB".into(),
        touch_supported: false,
        graphics_device_name: "RTX".into(),
        graphics_device_vendor_id: 4318,
        graphics_device_type: "Vulkan".into(),
        graphics_device_kind: "DiscreteGpu".into(),
        graphics_driver: "555.1".into(),
        screen_width: 2560,
        screen_height: 1440,
        screen_refresh_rate: 165,
        dpi: 109,
    });
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    unsafe { ts_apc_init() };
    reset();
    f();
    ts_rt_gc_stress(true);
    reset();
    f();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    reset();
    f();
    ts_rt_gc_enable(true);
}

fn expected() -> Value {
    serde_json::from_str(include_str!("../../bench/app_expected.json")).unwrap()
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

// M9-TS-621
#[test]
fn manifest_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("manifest", ts_apc_manifest());
    });
}

// M9-TS-622
#[test]
fn machine_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("machine", ts_apc_machine());
    });
}

// M9-TS-623
#[test]
fn settable_matches_the_original() {
    in_every_gc_mode(|| unsafe {
        check_str("settable", ts_apc_settable());
    });
}
