//! The Window API from compiled programs (`bench/windows.ts`, prefix `win_`), in the normal, stress GC
//! and collection-off modes. The test plays the host: it reads the requests and reports `opened`,
//! `resized`, `close_requested` and `closed`. IDs are in TESTS.md (M9-TS-2800..).
use iron_engine_api::window::{self, MAIN, Opened, WindowRef, WindowRequest};
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress, ts_rt_str_new};
use iron_ts_tests::windows_t::*;

fn reset() {
    window::reset();
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_ui::link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        reset();
        let _ = iron_ts_ui::take_mounts();
        let _ = iron_ts_ui::take_window_mounts();
        unsafe { ts_win_init() };
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

fn script_str(s: &str) -> *mut u8 {
    unsafe { ts_rt_str_new(s.as_ptr(), s.len() as i64) as *mut u8 }
}

/// What the platform tells the app once it made the window.
fn opened(w: WindowRef) {
    window::opened(
        w,
        Opened {
            position: (10.0, 20.0),
            size: (220.0, 320.0),
            scale: 1.0,
            native_handle: 0,
            platform: "virtual".into(),
        },
    );
}

// M9-TS-2800
#[test]
fn create_with_an_options_object_maps_keys_by_name() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createTool();
        assert_eq!(h, 2.0);
        assert_eq!(text(ts_win_describe(h)), "Palette|tool|true|Iron|220x320");
        let st = window::state(WindowRef(2)).unwrap();
        assert_eq!(st.key, "palette");
        assert_eq!(st.parent, Some(MAIN));
        assert!(st.flags.resizable && st.flags.closable && !st.flags.always_on_top);
        assert_eq!(
            window::take_requests(),
            vec![WindowRequest::Create(WindowRef(2))]
        );
        // A dialog: the parent and a flag given by name; the rest default.
        let d = ts_win_createDialog(h);
        assert_eq!(
            text(ts_win_describe(d)),
            "Sure?|dialog|true|Palette|400x300"
        );
        let ds = window::state(WindowRef(d as u32)).unwrap();
        assert_eq!(ds.parent, Some(WindowRef(2)));
        assert!(!ds.flags.resizable && ds.flags.minimizable);
    });
}

// M9-TS-2801
#[test]
fn window_main_reads_as_a_property_and_its_title_goes_to_the_host() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_win_mainTitle()), "Iron");
        ts_win_setMainTitle(script_str("Level 2"));
        assert_eq!(text(ts_win_mainTitle()), "Level 2");
        assert_eq!(
            window::take_requests(),
            vec![WindowRequest::Title(MAIN, "Level 2".into())]
        );
        assert_eq!(text(ts_win_mainInfo()), "1|true|false|1");
    });
}

// M9-TS-2802
#[test]
fn static_and_instance_focused_both_resolve() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createTool();
        assert_eq!(text(ts_win_focusState(h)), "true|false|0");
        window::focus_changed(WindowRef(2), true);
        assert_eq!(text(ts_win_focusState(h)), "false|true|2");
        window::focus_changed(WindowRef(2), false);
        assert_eq!(text(ts_win_focusState(h)), "true|false|0");
    });
}

// M9-TS-2803
#[test]
fn window_listeners_from_compiled_closures_run_and_cancel() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createTool();
        let w = WindowRef(2);
        opened(w);
        ts_win_listen(h);
        assert_eq!(text(ts_win_counts()), "0|0|0|0x0");
        window::resized(w, 300.0, 350.0);
        window::resized(w, 300.0, 350.0);
        assert_eq!(text(ts_win_counts()), "1|0|0|300x350");
        ts_win_setCancel(true);
        assert!(!window::close_requested(w), "the listener cancelled");
        ts_win_setCancel(false);
        assert!(window::close_requested(w));
        assert_eq!(text(ts_win_counts()), "1|0|2|300x350");
        window::closed(w);
        window::closed(w);
        assert_eq!(text(ts_win_counts()), "1|1|2|300x350");
        assert_eq!(iron_api::callback::take_errors().len(), 0);
    });
}

// M9-TS-2804
#[test]
fn count_and_at_list_the_windows_in_order() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_win_list()), "Iron");
        let h = ts_win_createTool();
        let d = ts_win_createDialog(h);
        assert_eq!(text(ts_win_list()), "Iron,Palette,Sure?");
        window::closed(WindowRef(h as u32));
        assert_eq!(text(ts_win_list()), "Iron,Sure?");
        assert_eq!(
            text(ts_win_describe(d)),
            "Sure?|dialog|true|Palette|400x300"
        );
    });
}

// M9-TS-2805
#[test]
fn create_root_of_a_window_queues_a_window_mount() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createTool();
        ts_win_mount(h);
        let mounts = iron_ts_ui::take_window_mounts();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].0, 2);
        assert!(mounts[0].1 != 0);
        assert!(
            iron_ts_ui::take_mounts().is_empty(),
            "not a screen or panel mount"
        );
        // The main window is the screen: target 0.
        ts_win_mountMain();
        let screen = iron_ts_ui::take_mounts();
        assert_eq!(screen.len(), 1);
        assert_eq!(screen[0].0, 0);
        assert!(iron_ts_ui::take_window_mounts().is_empty());
    });
}

// ---- Messages (step M2) ----------------------------------------------------------------------

/// Captures what reaches a window's `onMessageJson` listener, as `from:text`.
fn capture(w: WindowRef) -> std::rc::Rc<std::cell::RefCell<Vec<String>>> {
    let log: std::rc::Rc<std::cell::RefCell<Vec<String>>> = Default::default();
    let l = log.clone();
    w.on_message_json(iron_api::Callback::from_fn(
        move |(from, text): (iron_api::callback::HandleArg<WindowRef>, String)| {
            l.borrow_mut().push(format!("{}:{text}", from.0.0))
        },
    ));
    log
}

fn deliver(w: WindowRef, now: f64) {
    window::deliver(window::DomainId(w.0), now);
}

// M9-TS-2838
#[test]
fn post_message_sends_the_static_type_as_json() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createInspector();
        let w = WindowRef(h as u32);
        let log = capture(w);
        assert_eq!(h, 2.0);
        assert_eq!(text(ts_win_familyInfo(h)), "2|true|2|false");
        assert!(ts_win_postSelect(h));
        assert!(log.borrow().is_empty(), "queued, not delivered");
        assert_eq!(window::pending(window::DomainId(2)), 1);
        deliver(w, 0.0);
        assert_eq!(
            *log.borrow(),
            ["1:{\"kind\":\"select\",\"p\":[1,2,3],\"ids\":[1,2],\"name\":\"Player\"}"]
        );
    });
}

// M9-TS-2839
#[test]
fn on_message_parses_into_the_declared_type() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createInspector();
        let w = WindowRef(h as u32);
        ts_win_listenSelect(h);
        ts_win_listenInferred(h);
        let _ = iron_api::callback::take_errors();
        assert!(w.post_message_json(
            "{\"kind\":\"select\",\"p\":[1,2.5,3],\"ids\":[4,5,6],\"name\":\"N\"}"
        ));
        deliver(w, 0.0);
        assert_eq!(
            text(ts_win_receivedMessages()),
            "1:select|2.5|3|N;inferred 3;"
        );
        // A message that does not have the shape throws inside the listener, which the engine
        // logs; the listener's body does not run.
        assert!(w.post_message_json("{\"kind\":5}"));
        deliver(w, 0.0);
        assert_eq!(
            text(ts_win_receivedMessages()),
            "1:select|2.5|3|N;inferred 3;"
        );
        assert!(!iron_api::callback::take_errors().is_empty());
    });
}

// M9-TS-2840
#[test]
fn posting_a_handle_is_a_compile_error() {
    let src = "export function bad(): boolean {\n  const go = new GameObject(\"x\");\n  return Window.main.postMessage({ go: go, n: 1 });\n}\n";
    let manifests: Vec<String> = {
        let mut all = vec![iron_scene::api::scene_api::MANIFEST.to_json()];
        all.extend(iron_engine_api::manifests());
        all
    };
    let Err(e) = iron_ts::compile_with_manifests(src, "speed", "bad_", 7500, &manifests) else {
        panic!("a handle in a message compiled")
    };
    assert!(
        e.contains("`message.go`") && e.contains("GameObject handle"),
        "{e}"
    );
    // The same through a nested array and a request's data.
    let src = "export function bad2(): number {\n  const go = new GameObject(\"x\");\n  Window.main.request<number>({ list: [{ target: go }] });\n  return 0;\n}\n";
    let Err(e) = iron_ts::compile_with_manifests(src, "speed", "bad_", 7500, &manifests) else {
        panic!("a handle in a request compiled")
    };
    assert!(e.contains("`message.list[].target`"), "{e}");
    // A payload without handles compiles.
    let ok = "export function good(): boolean {\n  return Window.main.postMessage({ n: 1, v: new Vector3(1, 2, 3), tags: [\"a\"] });\n}\n";
    iron_ts::compile_with_manifests(ok, "speed", "bad_", 7500, &manifests).unwrap();
    // `request` needs its reply type, and the reply may not hold handles either.
    let src = "export function bad4(): number {
  Window.main.request({ n: 1 });
  return 0;
}
";
    let Err(e) = iron_ts::compile_with_manifests(src, "speed", "bad_", 7500, &manifests) else {
        panic!("a request without a reply type compiled")
    };
    assert!(e.contains("needs the reply type"), "{e}");
    let src = "export function bad5(): number {
  Window.main.request<{ who: GameObject }>(1);
  return 0;
}
";
    let Err(e) = iron_ts::compile_with_manifests(src, "speed", "bad_", 7500, &manifests) else {
        panic!("a handle in a reply compiled")
    };
    assert!(e.contains("`reply.who`"), "{e}");
    // The type must be given or inferable.
    let src = "export function bad3(): number {\n  return Window.main.onMessage((from, msg) => { });\n}\n";
    let Err(e) = iron_ts::compile_with_manifests(src, "speed", "bad_", 7500, &manifests) else {
        panic!("an untyped listener compiled")
    };
    assert!(e.contains("cannot infer the type argument `T`"), "{e}");
}

// M9-TS-2841
#[test]
fn request_resolves_with_the_typed_reply() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createInspector();
        let w = WindowRef(h as u32);
        ts_win_serve(h);
        ts_win_resetAsk();
        let p = ts_win_ask(h, f64::NAN);
        assert!(!p.is_null());
        // The request waits in the inspector's queue; its listener runs in its own domain.
        assert_eq!(text(ts_win_askLog()), "");
        deliver(w, 0.0);
        assert_eq!(text(ts_win_requestLog()), "1:stats;");
        assert_eq!(
            text(ts_win_askLog()),
            "",
            "the reply travels back to the main domain"
        );
        window::deliver(window::DomainId::MAIN, 1.0);
        iron_ts_rt::ts_rt_microtasks_run();
        assert_eq!(text(ts_win_askLog()), "ok 5");
    });
}

// M9-TS-2842
#[test]
fn request_rejects_on_timeout_and_on_close() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createInspector();
        let w = WindowRef(h as u32);
        ts_win_serveQuiet(h);
        ts_win_resetAsk();
        ts_win_ask(h, 50.0);
        deliver(w, 0.0);
        window::deliver(window::DomainId::MAIN, 49.0);
        iron_ts_rt::ts_rt_microtasks_run();
        assert_eq!(text(ts_win_askLog()), "", "not yet");
        window::deliver(window::DomainId::MAIN, 51.0);
        iron_ts_rt::ts_rt_microtasks_run();
        assert_eq!(text(ts_win_askLog()), "error timed out");

        // The target closes while the request is outstanding.
        let h2 = ts_win_createInspector();
        let w2 = WindowRef(h2 as u32);
        ts_win_serveQuiet(h2);
        ts_win_resetAsk();
        ts_win_ask(h2, f64::NAN);
        deliver(w2, 60.0);
        window::closed(w2);
        window::deliver(window::DomainId::MAIN, 61.0);
        iron_ts_rt::ts_rt_microtasks_run();
        assert_eq!(text(ts_win_askLog()), "error window closed");

        // No request listener in the domain.
        let h3 = ts_win_createInspector();
        ts_win_resetAsk();
        ts_win_ask(h3, f64::NAN);
        deliver(WindowRef(h3 as u32), 70.0);
        window::deliver(window::DomainId::MAIN, 71.0);
        iron_ts_rt::ts_rt_microtasks_run();
        assert_eq!(text(ts_win_askLog()), "error no request listener");
    });
}

// M9-TS-2843
#[test]
fn broadcast_and_the_domain_listener_receive_messages() {
    in_every_gc_mode(|| unsafe {
        let h = ts_win_createInspector();
        ts_win_listenDomain();
        // Posted from the inspector's domain: reaches the main domain's `Window.onMessage`.
        {
            let _d = window::enter_domain(window::DomainId(h as u32));
            assert_eq!(ts_win_broadcastHi(), 1.0);
        }
        window::deliver(window::DomainId::MAIN, 0.0);
        assert_eq!(text(ts_win_domainMessages()), "2:7;");
        // From the main family: the inspector gets it, the sender's own domain does not.
        assert_eq!(ts_win_broadcastHi(), 1.0);
        assert_eq!(window::pending(window::DomainId::MAIN), 0);
        assert_eq!(window::pending(window::DomainId(h as u32)), 1);
    });
}
