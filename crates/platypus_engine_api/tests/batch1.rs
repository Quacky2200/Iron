// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The first batch of modules moved from `js/engine` to Rust (random, layers, log, prefs, assets,
//! app, time classes) through their QuickJS bindings and host functions. IDs are in TESTS.md
//! (M9-API-37..).
use platypus_engine_api::{app, assets, clock, datetime, input, layers, log, prefs, random, time};
use std::cell::RefCell;
use std::rc::Rc;

fn with_js<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        random::random_api::register_quickjs(&ctx).unwrap();
        layers::layer_mask_api::register_quickjs(&ctx).unwrap();
        log::debug_api::register_quickjs(&ctx).unwrap();
        log::log_api::register_quickjs(&ctx).unwrap();
        log::console_api::register_quickjs(&ctx).unwrap();
        prefs::prefs_api::register_quickjs(&ctx).unwrap();
        assets::assets_api::register_quickjs(&ctx).unwrap();
        app::application_api::register_quickjs(&ctx).unwrap();
        app::system_info_api::register_quickjs(&ctx).unwrap();
        clock::time_since_api::register_quickjs(&ctx).unwrap();
        clock::time_until_api::register_quickjs(&ctx).unwrap();
        input::input_api::register_quickjs(&ctx).unwrap();
        datetime::time_span_api::register_quickjs(&ctx).unwrap();
        datetime::date_time_api::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-37
//
// `Random` in QuickJS: the same numbers as `js/engine/src/random.ts` for a seed (the expected
// values come from running that file in bun), with `bool`'s chance left out and given.
#[test]
fn random_gives_the_numbers_of_the_typescript_generator() {
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                Random.initState(2024);
                JSON.stringify([Random.value, Random.range(5, 6), Random.rangeInt(0, 100),
                  Random.bool(), Random.bool(0.1)]);
                "#,
            )
            .unwrap();
        assert_eq!(s, "[0.811762373894453,5.710821494925767,65,false,false]");
    });
}

// M9-API-38
//
// `LayerMask` in QuickJS: the built-in names, the game names, masks as numbers and `contains`.
#[test]
fn layer_masks_use_the_builtin_and_game_names() {
    layers::set_game_layers([(8, "Player".to_string())]);
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                JSON.stringify([LayerMask.getMask("Default", "UI"), LayerMask.getMask("Player"),
                  LayerMask.getMask(), LayerMask.everything, LayerMask.nothing,
                  LayerMask.nameToLayer("UI"), LayerMask.nameToLayer("x"),
                  LayerMask.layerToName(2), LayerMask.layerToName(8), LayerMask.layerToName(3),
                  LayerMask.contains(33, 5), LayerMask.contains(33, 4), LayerMask.contains(4294967295, 31)]);
                "#,
            )
            .unwrap();
        assert_eq!(
            s,
            r#"[33,256,0,4294967295,0,5,-1,"Ignore Raycast","Player","",true,false,true]"#
        );
    });
    layers::set_game_layers([]);
}

// M9-API-39
//
// Logging goes to the sink the host set, with its level; `Debug.assert` logs only when false,
// with the text given or a default.
#[test]
fn log_lines_reach_the_sink_of_the_host() {
    let lines = Rc::new(RefCell::new(Vec::new()));
    let sink = lines.clone();
    log::set_sink(Some(Box::new(move |l, m| {
        sink.borrow_mut().push((l, m.to_string()))
    })));
    with_js(|ctx| {
        ctx.eval::<(), _>(
            r#"
            Debug.log("a"); Debug.logWarning("b"); Debug.logError("c"); Log.info("d");
            Log.trace("e"); Console.writeLine("f"); Console.warn("g");
            Debug.assert(true, "unseen"); Debug.assert(false); Debug.assert(false, "boom");
            "#,
        )
        .unwrap();
    });
    log::set_sink(None);
    use log::Level::*;
    let want = [
        (Log, "a"),
        (Warn, "b"),
        (Error, "c"),
        (Info, "d"),
        (Trace, "e"),
        (Log, "f"),
        (Warn, "g"),
        (Error, "Assertion failed: assertion failed"),
        (Error, "Assertion failed: boom"),
    ];
    let got = lines.borrow();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert_eq!((g.0, g.1.as_str()), w);
    }
}

// M9-API-40
//
// `PlayerPrefs`: typed reads with fallbacks, writes truncating ints, and the changes the host
// writes to the file, in order.
#[test]
fn player_prefs_read_write_and_report_changes() {
    prefs::reset();
    prefs::load([("saved".to_string(), prefs::Pref::Num(7.9))]);
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                PlayerPrefs.setInt("a", 3.9); PlayerPrefs.setString("s", "x");
                PlayerPrefs.deleteKey("saved"); PlayerPrefs.save();
                JSON.stringify([PlayerPrefs.getInt("a"), PlayerPrefs.getInt("zz", 4),
                  PlayerPrefs.getFloat("zz"), PlayerPrefs.getString("s"),
                  PlayerPrefs.getString("zz", "d"), PlayerPrefs.hasKey("saved"), PlayerPrefs.hasKey("a")]);
                "#,
            )
            .unwrap();
        assert_eq!(s, r#"[3,4,0,"x","d",false,true]"#);
    });
    use prefs::{Change::*, Pref::*};
    assert_eq!(
        prefs::take_changes(),
        vec![
            Set("a".into(), Num(3.0)),
            Set("s".into(), Str("x".into())),
            Delete("saved".into()),
            Save
        ]
    );
    assert!(prefs::take_changes().is_empty());
    prefs::set_json("platypus.quality", r#"{"level":2}"#);
    assert_eq!(
        prefs::json("platypus.quality").as_deref(),
        Some(r#"{"level":2}"#)
    );
    prefs::reset();
}

// M9-API-41
//
// `Assets`: lookups by GUID and path, `guid:` sources, importer settings as JSON, and the changed
// paths a script polls.
#[test]
fn assets_look_up_guids_paths_and_changes() {
    assets::reset();
    assets::set_assets(vec![assets::AssetEntry {
        path: "a/b.png".into(),
        guid: "g1".into(),
        importer_json: r#"{"type":"texture"}"#.into(),
    }]);
    assets::changed(["x.png".to_string(), "y.ogg".to_string()]);
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                JSON.stringify([Assets.count(), Assets.pathAt(0), Assets.guidAt(5), Assets.guidToPath("g1"),
                  Assets.pathToGuid("./a/b.png"), Assets.resolve("guid:g1"), Assets.resolve("guid:no"),
                  Assets.importer("guid:g1"), Assets.importer("zzz"), Assets.takeChanges(), Assets.takeChanges()]);
                "#,
            )
            .unwrap();
        assert_eq!(
            s,
            r#"[1,"a/b.png","","a/b.png","g1","a/b.png","guid:no","{\"type\":\"texture\"}","{}","x.png\ny.ogg",""]"#
        );
    });
    assets::reset();
}

// M9-API-1151
//
// `Assets.exists` / `list` / `packOf` answer from the index the host fills: sorted keys, the
// glob rules of `project.json`, `guid:` and `data:` sources, and the pack of each path.
#[test]
fn assets_exists_list_and_pack_of_answer_from_the_index() {
    assets::reset();
    assets::set_assets(vec![assets::AssetEntry {
        path: "assets/sfx/b.wav".into(),
        guid: "g1".into(),
        importer_json: "{}".into(),
    }]);
    // Given out of order: the index sorts it.
    assets::set_asset_index(vec![
        ("assets/sfx/b.wav".into(), "assets".into()),
        ("assets/emoji/1f600.svg".into(), "assets".into()),
        ("assets/sfx/a.wav".into(), "scene-level1".into()),
        ("assets/sfx/loud/c.wav".into(), "assets".into()),
        ("assets/sfx/a.mp3".into(), "".into()),
        ("scenes/level1.scene.json".into(), "scene-level1".into()),
    ]);
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                JSON.stringify([
                  Assets.exists("assets/sfx/a.wav"), Assets.exists("./assets\\sfx\\a.wav"), Assets.exists("assets/sfx/none.wav"),
                  Assets.exists("data:,hi"), Assets.exists("guid:g1"), Assets.exists("guid:nope"),
                  Assets.list("assets/sfx/*.wav"), Assets.list("assets/sfx/**/*.wav"), Assets.list("assets/sfx"),
                  Assets.list("assets/SFX/*.wav"), Assets.list("*.wav"),
                  Assets.packOf("assets/sfx/a.wav"), Assets.packOf("assets/sfx/b.wav"), Assets.packOf("assets/sfx/a.mp3"),
                  Assets.packOf("assets/none.wav"), Assets.packOf("guid:g1")]);
                "#,
            )
            .unwrap();
        assert_eq!(
            s,
            r#"[true,true,false,true,true,false,["assets/sfx/a.wav","assets/sfx/b.wav"],["assets/sfx/a.wav","assets/sfx/b.wav","assets/sfx/loud/c.wav"],["assets/sfx/a.mp3","assets/sfx/a.wav","assets/sfx/b.wav","assets/sfx/loud/c.wav"],[],[],"scene-level1","assets","","","assets"]"#
        );
    });
    // Without an index nothing exists.
    assets::reset();
    with_js(|ctx| {
        let s: String = ctx
            .eval(r#"JSON.stringify([Assets.exists("a.png"), Assets.list("**"), Assets.packOf("a.png")])"#)
            .unwrap();
        assert_eq!(s, r#"[false,[],""]"#);
    });
}

// M9-API-42
//
// `Application` and `SystemInfo`: the manifest the host installs, writes by scripts returned to
// the host as requests (frame rate rounded, 0 or less meaning -1), and focus changes the host can
// react to.
#[test]
fn application_reads_the_manifest_and_returns_script_writes() {
    app::reset();
    app::set_app(app::AppInfo {
        name: "demo".into(),
        target_frame_rate: 144,
        subtitle: "Series 2".into(),
        ..app::AppInfo::default()
    });
    // M9-VSP-13: `Application.subtitle` reads the manifest's.
    assert_eq!(app::application_api::subtitle(), "Series 2");
    app::set_system(app::SystemInfo {
        operating_system_family: "Linux".into(),
        device_type: "Handheld".into(),
        screen_width: 1920,
        ..app::SystemInfo::default()
    });
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                const before = [Application.name, Application.version, Application.platform,
                  Application.isMobilePlatform, Application.targetFrameRate, Application.runInBackground,
                  SystemInfo.screenWidth, SystemInfo.screenDpi, Application.isFocused].join();
                Application.targetFrameRate = 59.6; Application.runInBackground = true;
                Application.targetFrameRate = -3; Application.quit();
                before + "|" + Application.targetFrameRate + "," + Application.runInBackground;
                "#,
            )
            .unwrap();
        assert_eq!(s, "demo,0.0.0,Linux,true,144,false,1920,96,true|-1,true");
    });
    assert_eq!(
        app::take_requests(),
        vec![
            app::Request::TargetFrameRate(60),
            app::Request::RunInBackground(true),
            app::Request::TargetFrameRate(-1),
            app::Request::Quit(0)
        ]
    );
    let c = app::set_focus(false, true);
    assert!(c.focus_changed && c.pause_changed);
    let c = app::set_focus(false, false);
    assert!(!c.focus_changed && c.pause_changed);
    app::reset();
}

// M9-API-43
//
// `TimeSince` and `TimeUntil` are value classes on the game clock: they follow `Time.time`.
#[test]
fn time_since_and_until_follow_the_game_clock() {
    time::reset();
    time::advance(1.0);
    with_js(|ctx| {
        ctx.eval::<(), _>("globalThis.s = TimeSince.now(); globalThis.u = TimeUntil.in(2);")
            .unwrap();
        time::advance(0.5);
        let r: String = ctx
            .eval(
                "JSON.stringify([s.seconds, s.over(0.4), s.over(0.5), u.seconds, u.done, u.fraction, TimeSince.ago(3).seconds])",
            )
            .unwrap();
        assert_eq!(r, "[0.5,true,false,1.5,false,0.25,3]");
        time::advance(2.0);
        let r: String = ctx
            .eval("JSON.stringify([u.seconds, u.done, u.fraction])")
            .unwrap();
        assert_eq!(r, "[0,true,1]");
    });
    time::reset();
}

// M9-API-44
//
// Input in QuickJS follows the tick: events become "this frame" at `time::advance`, the stick wins
// the axis when pushed further than the keys, pads come and go, and a blur clears held keys.
#[test]
fn input_events_become_a_frame_at_the_tick_and_pads_are_read_by_id() {
    input::reset();
    time::reset();
    with_js(|ctx| {
        input::key_down("KeyD", false);
        input::mouse_delta(2.0, 3.0);
        input::gamepad_connected(4, "Pad");
        input::gamepad_axis(4, "LeftStickX", -0.5);
        input::gamepad_button(4, "South", 1.0);
        let before: String = ctx
            .eval(r#"JSON.stringify([input.getKey("KeyD"), input.getKeyDown("KeyD"), input.getAxis("Horizontal"), input.mouseDelta.y])"#)
            .unwrap();
        assert_eq!(
            before, "[true,false,1,0]",
            "nothing is this frame before the tick"
        );
        time::advance(0.016);
        let after: String = ctx
            .eval(
                r#"JSON.stringify([input.getKeyDown("KeyD"), input.getAxis("Horizontal"), input.getAxis("Mouse Y"),
                  input.mouseDelta.y, input.getButton("Jump"), input.gamepadCount(), input.gamepadId(0),
                  input.gamepadName(4), input.gamepadButton(4, "South"), input.isButtonDown("South")])"#,
            )
            .unwrap();
        assert_eq!(
            after,
            r#"[true,1,-0.30000000000000004,-3,true,1,4,"Pad",1,true]"#
        );
        input::blur();
        input::gamepad_disconnected(4);
        let end: String = ctx
            .eval(r#"JSON.stringify([input.getKey("KeyD"), input.gamepadCount(), input.gamepadButton(4, "South"), input.gamepadId(0)])"#)
            .unwrap();
        assert_eq!(end, "[false,0,0,-1]");
    });
    input::reset();
    time::reset();
}

// M9-API-45
//
// `TimeSpan` and `DateTime` in QuickJS: `new X(..)` goes through `create` (rounding, carrying,
// the kind), members and statics work on the value classes, in either registration order.
#[test]
fn time_span_and_date_time_work_as_javascript_classes() {
    datetime::set_zone(Some("America/New_York")).unwrap();
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                const t = new TimeSpan(1500.4);
                const d = new DateTime(2024, 3, 5, 9, 7, 3, 45);
                const u = new DateTime(2024, 13, 1, 0, 0, 0, 0, "utc");
                JSON.stringify([t.totalMilliseconds, TimeSpan.of(1, 2).add(TimeSpan.fromMinutes(1)).toString(),
                  TimeSpan.parse("1.02:03:04.5").totalSeconds, TimeSpan.zero.toString("hh"),
                  d.toString("o"), u.toString("o"), u.kind, d.addMonths(11).toString("s"),
                  d.subtract(u).days, d.date.hour, d.dayOfWeek, DateTime.daysInMonth(2024, 2),
                  DateTime.fromEpoch(0, "utc").toISOString(), DateTime.parse("2024-05-03T10:20:30+02:00").epochMilliseconds]);
                "#,
            )
            .unwrap();
        assert_eq!(
            s,
            r#"[1500,"1.02:01:00",93784.5,"00","2024-03-05T09:07:03.045-05:00","2025-01-01T00:00:00.000Z","utc","2025-02-05T09:07:03",-301,0,2,29,"1970-01-01T00:00:00.000Z",1714724430000]"#
        );
    });
    datetime::set_zone(None).unwrap();
}
