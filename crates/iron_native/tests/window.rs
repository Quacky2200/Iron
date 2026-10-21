//! The `Window` API in QuickJS: options objects, static properties, listeners. IDs are in TESTS.md
//! (M9-API-14xx, M9-TS-28xx).
mod common;
use common::{js, with_world};
use iron_engine_api::window::{self, WindowRef, WindowRequest};

// M9-API-1433
#[test]
fn window_create_takes_an_options_object_in_quickjs() {
    with_world(|ctx| {
        let desc: String = js(
            ctx,
            "const w = Window.create({ title: 'T', style: 'tool', width: 200, key: 'k' });\
             [w.title, w.style, w.isChild, w.parent.handle, w.width, w.height, w.key, w.handle].join('|')",
        );
        assert_eq!(desc, "T|tool|true|1|200|300|k|2");
        // `Window.main` is a property, not a call, and `Window.count` too.
        let m: String = js(
            ctx,
            "Window.main.title = 'Game'; [Window.main.title, Window.main.handle, Window.count, \
             typeof Window.main, Window.main.isMain, Window.at(1).handle].join('|')",
        );
        assert_eq!(m, "Game|1|2|object|true|2");
        assert_eq!(
            window::take_requests(),
            vec![
                WindowRequest::Create(WindowRef(2)),
                WindowRequest::Title(WindowRef(1), "Game".into()),
            ]
        );
        // Positional arguments still work; the style falls back to normal when unknown.
        let p: String = js(
            ctx,
            "const p = Window.create('Pos', 120, 90); p.title + p.width + p.style",
        );
        assert_eq!(p, "Pos120normal");
        // Flags and booleans through the object; left-out ones keep their defaults.
        let f: String = js(
            ctx,
            "const q = Window.create({ resizable: false, alwaysOnTop: true });\
             [q.resizable, q.closable, q.alwaysOnTop, q.visible].join('|')",
        );
        assert_eq!(f, "false|true|true|true");
        // `focused` is a static property and an instance property.
        let fo: String = js(
            ctx,
            "[Window.focused === null, Window.main.focused].join('|')",
        );
        assert_eq!(fo, "true|false");
        window::focus_changed(WindowRef(2), true);
        let fo: String = js(
            ctx,
            "[Window.focused.handle, Window.at(1).focused].join('|')",
        );
        assert_eq!(fo, "2|true");
    });
}

// M9-API-1434
#[test]
fn window_listeners_run_from_quickjs_and_can_cancel_a_close() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.w = Window.create({ title: 'L', style: 'dialog' });\
             globalThis.log = [];\
             w.onCloseRequested(() => w.cancelClose());\
             w.onResized((a, b) => log.push('r' + a + 'x' + b));\
             w.onFocusChanged((f) => log.push('f' + f));\
             w.onClosed(() => log.push('closed'));",
        );
        let w = WindowRef(2);
        assert!(!window::close_requested(w));
        window::resized(w, 301.0, 202.0);
        window::focus_changed(w, true);
        let _: () = js(ctx, "w.onCloseRequested(() => {}); null");
        window::closed(w);
        let log: String = js(ctx, "log.join(',')");
        assert_eq!(log, "r301x202,ftrue,closed");
        // A handle read after the close still has its last values.
        let last: String = js(ctx, "[w.isOpen, w.title, w.width].join('|')");
        assert_eq!(last, "false|L|301");
    });
}
