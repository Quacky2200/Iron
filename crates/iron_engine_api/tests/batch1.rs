//! The first batch of modules moved from `js/engine` to Rust (random, layers, log, prefs, assets,
//! app, time classes) through their QuickJS bindings and host functions. IDs are in TESTS.md
//! (M9-API-37..).
use iron_engine_api::{app, assets, clock, layers, log, prefs, random, time};
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
    prefs::set_json("iron.quality", r#"{"level":2}"#);
    assert_eq!(
        prefs::json("iron.quality").as_deref(),
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
        ..app::AppInfo::default()
    });
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
            app::Request::Quit
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
