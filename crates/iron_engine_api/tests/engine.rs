// SPDX-License-Identifier: Apache-2.0

//! The engine API crate through its QuickJS bindings (the same functions compiled code calls
//! through the shims) and the host functions. IDs are in TESTS.md (M9-API-*).
use iron_engine_api::{camera, color, gizmos, input, screen, time};
use iron_scene::{ObjectId, Scene};

fn with_js<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        color::color_api::register_quickjs(&ctx).unwrap();
        time::time_api::register_quickjs(&ctx).unwrap();
        input::input_api::register_quickjs(&ctx).unwrap();
        gizmos::gizmos_api::register_quickjs(&ctx).unwrap();
        screen::screen_api::register_quickjs(&ctx).unwrap();
        screen::quality_api::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-30
#[test]
fn color_has_the_engines_constants_hex_and_lerp() {
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r##"
                const w = Color.white, k = Color.black;
                const h = Color.hex("#ff8000");
                const mid = k.lerp(w, 0.25);
                JSON.stringify([w.r, w.a, k.r, h.r, +h.g.toFixed(4), h.b, mid.r, mid.a,
                  Color.hex("#f00").r, Color.hex("#ff000080").a.toFixed(3),
                  Color.hex("#ff8000").toHex(), Color.clear.a, w.withAlpha(0.5).a]);
                "##,
            )
            .unwrap();
        // 0x80 = 128: sRGB 0.50196 is 0.2159 in linear space.
        assert_eq!(
            s,
            "[1,1,0,1,0.2159,0,0.25,1,1,\"0.502\",\"#ff8000ff\",0,0.5]"
        );
    });
}

// M9-API-31
#[test]
fn time_follows_the_hosts_frames_and_the_time_scale() {
    time::reset();
    with_js(|ctx| {
        time::advance(0.5);
        let s: String = ctx
            .eval("JSON.stringify([Time.deltaTime, Time.time, Time.frameCount, Time.timeScale])")
            .unwrap();
        assert_eq!(s, "[0.5,0.5,1,1]");
        ctx.eval::<(), _>("Time.timeScale = 0.5").unwrap();
        time::advance(1.0);
        let s: String = ctx
            .eval("JSON.stringify([Time.deltaTime, Time.unscaledDeltaTime, Time.time, Time.unscaledTime, Time.fixedDeltaTime])")
            .unwrap();
        assert_eq!(s, "[0.5,1,1,1.5,0.016666666666666666]");
        // A bad scale falls back to 1, as the JS engine does.
        ctx.eval::<(), _>("Time.timeScale = -3").unwrap();
        assert_eq!(time::state().time_scale, 1.0);
    });
}

// M9-API-32
#[test]
fn input_reports_this_frames_presses_and_releases() {
    input::reset();
    with_js(|ctx| {
        input::key_down_event("Space");
        input::key_down_event("Space"); // a repeat is not a new press
        input::mouse_button_event(1, true);
        input::mouse_move(10.0, 20.0);
        input::tick();
        let s: String = ctx
            .eval(
                r#"JSON.stringify([input.getKey("Space"), input.getKeyDown("Space"), input.getKeyUp("Space"),
                   input.getMouseButtonDown(1), input.getMouseButton(0), input.mousePosition.x, input.mousePosition.y])"#,
            )
            .unwrap();
        assert_eq!(s, "[true,true,false,true,false,10,20]");
        input::key_up_event("Space");
        input::tick();
        let s: String = ctx
            .eval(r#"JSON.stringify([input.getKey("Space"), input.getKeyDown("Space"), input.getKeyUp("Space"), input.getMouseButtonDown(1), input.getMouseButton(1)])"#)
            .unwrap();
        assert_eq!(s, "[false,false,true,false,true]");
    });
}

// M9-API-33
#[test]
fn screen_and_quality_settings_are_read_and_set() {
    with_js(|ctx| {
        screen::set_size(800, 600);
        let s: String = ctx
            .eval("JSON.stringify([Screen.width, Screen.height, Screen.fullScreen, QualitySettings.vSyncCount])")
            .unwrap();
        assert_eq!(s, "[800,600,false,0]", "vsync is off by default");
        ctx.eval::<(), _>("Screen.fullScreen = !Screen.fullScreen; QualitySettings.vSyncCount = 1")
            .unwrap();
        assert!(screen::state().full_screen);
        assert_eq!(screen::quality().v_sync_count, 1);
    });
}

// M9-API-914
//
// The levels start with vsync off, and a saved choice with vsync on (older saves) doesn't turn it on.
#[test]
fn vsync_is_off_by_default_and_a_saved_choice_cannot_turn_it_on() {
    use iron_engine_api::prefs;
    screen::reset();
    assert_eq!(screen::quality().v_sync_count, 0);
    prefs::set_json(
        "iron.quality",
        r#"{"level":"High","current":{"renderScale":0.5,"vSyncCount":1}}"#,
    );
    screen::apply_default_quality();
    let q = screen::quality();
    assert_eq!(
        q.render_scale, 0.5,
        "the rest of the saved choice still applies"
    );
    assert_eq!(q.v_sync_count, 0);
    prefs::reset();
}

// M9-API-34
#[test]
fn gizmos_collect_lines_and_present_them_to_a_camera() {
    gizmos::reset();
    camera::reset();
    let mut scene = Scene::new();
    let go = scene.create("cam");
    let cam = camera::camera_api::iron_Camera_attach(iron_api::Handle::to_raw(go));
    assert_ne!(cam, 0);
    with_js(|ctx| {
        // The camera is an opaque handle object in QuickJS; the host makes one from the id.
        let handle = iron_api::qjs::handle_new(ctx, "Camera", cam).unwrap();
        ctx.globals().set("cam", handle).unwrap();
        ctx.eval::<(), _>(
            r#"Gizmos.clear(); Gizmos.color = Color.red;
               Gizmos.drawLine({x:0,y:0,z:0}, {x:1,y:2,z:3});
               Gizmos.color = Color.blue;
               Gizmos.drawLine({x:1,y:2,z:3}, {x:4,y:5,z:6});
               Gizmos.present(cam);"#,
        )
        .unwrap();
    });
    let frame = gizmos::take_frame().expect("presented");
    assert_eq!(frame.camera.0 as u64, cam);
    assert_eq!(frame.lines.len(), 2);
    assert_eq!(frame.lines[0].color.r, 1.0);
    assert_eq!(frame.lines[1].color.b, 1.0);
    assert_eq!(frame.lines[1].to.z, 6.0);
    assert!(gizmos::take_frame().is_none(), "taken once");
}

// M9-API-35
#[test]
fn a_camera_has_the_engines_defaults_and_clamps_the_field_of_view() {
    camera::reset();
    let go = ObjectId {
        index: 3,
        generation: 0,
    };
    let c = camera::camera_api::iron_Camera_attach(iron_api::Handle::to_raw(go));
    assert_eq!(camera::camera_api::iron_Camera_fieldOfView(c), 60.0);
    assert!(!camera::camera_api::iron_Camera_orthographic(c));
    camera::camera_api::iron_Camera_setFieldOfView(c, 500.0);
    assert_eq!(camera::camera_api::iron_Camera_fieldOfView(c), 179.0);
    camera::camera_api::iron_Camera_setOrthographicSize(c, 7.5);
    camera::camera_api::iron_Camera_setOrthographic(c, true);
    let all = camera::cameras();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].1.orthographic_size, 7.5);
    assert_eq!(all[0].1.object, go);
    camera::remove_on(go);
    assert!(camera::cameras().is_empty());
    // A gone camera answers with defaults.
    assert_eq!(camera::camera_api::iron_Camera_fieldOfView(c), 60.0);
}
