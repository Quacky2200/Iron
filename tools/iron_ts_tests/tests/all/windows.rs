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
