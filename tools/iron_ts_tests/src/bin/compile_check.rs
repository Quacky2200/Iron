//! `compile_check file.ts`: compiles a program with every engine API manifest (scene, colour, time,
//! input, gizmos, camera, screen, quality settings) and prints the first error, or the number of
//! functions. A development aid for the swap-out work (`docs/research/iron-ts-sine-demo.md`).
fn main() {
    // Link everything the test crate's objects call (they are linked into every binary).
    assert_ne!(iron_ts_rt::link_anchor(), 0);
    assert_ne!(iron_ts_ui::link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    assert_ne!(iron_ts_tests::demo_api_anchor(), 0);
    let _ = iron_ts_tests::scene_anchor();
    let path = std::env::args().nth(1).expect("compile_check file.ts");
    let src = std::fs::read_to_string(&path).expect("read the file");
    let manifests = vec![
        iron_scene::api::scene_api::MANIFEST.to_json(),
        iron_engine_api::color::color_api::MANIFEST.to_json(),
        iron_engine_api::time::time_api::MANIFEST.to_json(),
        iron_engine_api::input::input_api::MANIFEST.to_json(),
        iron_engine_api::gizmos::gizmos_api::MANIFEST.to_json(),
        iron_engine_api::camera::camera_api::MANIFEST.to_json(),
        iron_engine_api::screen::screen_api::MANIFEST.to_json(),
        iron_engine_api::screen::quality_api::MANIFEST.to_json(),
        iron_engine_api::audio::audio_api::MANIFEST.to_json(),
    ];
    // Lowering and code generation both (set IRON_TS_TRACE=1 to see each function as it is built).
    match iron_ts::compile_with_manifests(&src, "speed", "chk_", 40000, &manifests) {
        Ok(out) => println!("ok: {} bytes of object code", out.object.len()),
        Err(e) => {
            println!("error: {e}");
            std::process::exit(1);
        }
    }
}
