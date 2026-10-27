// SPDX-License-Identifier: Apache-2.0

//! The `Window` API in QuickJS: options objects, static properties, listeners. IDs are in TESTS.md
//! (M9-API-14xx, M9-TS-28xx).
mod common;
use common::{js, with_world};
use platypus_engine_api::window::{self, WindowRef, WindowRequest};

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

// M9-API-1455
#[test]
fn quickjs_reads_open_error_is_supported_and_on_open_failed() {
    with_world(|ctx| {
        window::set_supported(false);
        let _: () = js(
            ctx,
            "globalThis.w = Window.create({ title: 'F' });\
             globalThis.log = [];\
             w.onOpenFailed((e) => log.push(e));\
             globalThis.before = w.openError;",
        );
        window::open_failed(
            WindowRef(2),
            "Cannot open Window due to platform restrictions (platform: web)",
        );
        let r: String = js(
            ctx,
            "[before, w.openError, log.length, log[0], Window.isSupported, w.isOpen, Window.main.openError].join('|')",
        );
        assert_eq!(
            r,
            "|Cannot open Window due to platform restrictions (platform: web)|1|Cannot open Window due to platform restrictions (platform: web)|false|false|"
        );
        window::set_supported(true);
    });
}

// ---- Messages (step M3) ----------------------------------------------------------------------

fn run_jobs(ctx: &rquickjs::Ctx<'_>) {
    while ctx.execute_pending_job() {}
}

// M9-TS-2844
#[test]
fn quickjs_post_message_and_on_message_round_trip() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.log = [];\
             globalThis.iso = Window.create({ title: 'Inspector' });\
             iso.onMessage((from, msg) => log.push('win ' + from.handle + ' ' + msg.kind + ' ' + msg.p.join(',') + ' ' + msg.list.length));\
             iso.run(() => {\
               log.push('run in ' + Window.current.domain);\
               Window.onMessage((from, msg) => log.push('dom ' + msg.kind));\
             });",
        );
        let iso = WindowRef(2);
        let d = window::DomainId(2);
        // The posting is queued; the value class travels in its compact form.
        let ok: bool = js(
            ctx,
            "iso.postMessage({ kind: 'select', p: new Vector3(1, 2, 3), list: [1, 2] })",
        );
        assert!(ok);
        assert_eq!(window::pending(d), 1);
        window::deliver(d, 0.0);
        let log: String = js(ctx, "log.join('|')");
        assert_eq!(log, "run in 2|win 1 select 1,2,3 2|dom select");
        // The reverse way: the isolated domain posts to the main window, which a listener of
        // the main domain hears (statics: Window.onMessage and Window.broadcast).
        let _: () = js(
            ctx,
            "globalThis.heard = [];\
             Window.onMessage((from, msg) => heard.push(from.handle + ':' + JSON.stringify(msg)));\
             Window.main.onMessage((from, msg) => heard.push('main ' + msg.n));",
        );
        let _g = window::enter_domain(d);
        let n: u32 = js(ctx, "Window.broadcast({ n: 5 })");
        assert_eq!(n, 1);
        drop(_g);
        window::deliver(window::DomainId::MAIN, 0.0);
        let heard: String = js(ctx, "heard.join('|')");
        assert_eq!(heard, "main 5|2:{\"n\":5}");
        // Bad JSON on the wire throws inside the listener; the other listeners still run.
        assert!(iso.post_message_json("{oops"));
        window::deliver(d, 0.0);
        let n: u32 = js(ctx, "log.length");
        assert_eq!(n, 3, "nothing new was logged");
    });
}

// M9-TS-2845
#[test]
fn quickjs_post_message_refuses_handles_and_functions() {
    with_world(|ctx| {
        let r: String = js(
            ctx,
            "const w = Window.create({ title: 'T' });\
             const tries = [\
               () => w.postMessage({ go: new GameObject('x'), n: 1 }),\
               () => w.postMessage({ list: [{ who: Window.main }] }),\
               () => w.postMessage({ f: () => 1 }),\
               () => w.postMessage(undefined),\
               () => w.postMessage(() => 1),\
               () => { const a = {}; a.a = a; return w.postMessage(a); },\
             ];\
             tries.map((t) => { try { t(); return 'sent'; } catch (e) { return e.constructor.name + ': ' + e.message; } }).join('\\n')",
        );
        let lines: Vec<&str> = r.lines().collect();
        assert!(
            lines[0].starts_with("TypeError")
                && lines[0].contains("\"go\"")
                && lines[0].contains("GameObject handle"),
            "{r}"
        );
        assert!(
            lines[1].contains("\"who\"") && lines[1].contains("Window handle"),
            "{r}"
        );
        assert!(
            lines[2].contains("\"f\"") && lines[2].contains("function"),
            "{r}"
        );
        assert!(
            lines[3].starts_with("TypeError") && lines[3].contains("undefined"),
            "{r}"
        );
        assert!(lines[4].contains("function"), "{r}");
        assert!(lines[5].starts_with("TypeError"), "{r}");
        assert_eq!(
            window::pending(window::DomainId(2)),
            0,
            "nothing was queued"
        );
        // Structured data goes: nested objects, arrays, null, value classes, a string.
        let ok: String = js(
            ctx,
            "[w.postMessage({ a: [1, { b: null }], v: new Vector3(1, 2, 3), s: 'x' }), w.postMessage('plain'), w.postMessage(null)].join(',')",
        );
        assert_eq!(ok, "true,true,true");
        // Over 1 MiB is refused with false, not a throw.
        let big: bool = js(ctx, "w.postMessage('x'.repeat(1 << 20))");
        assert!(!big);
    });
}

// M9-TS-2846
#[test]
fn quickjs_request_resolves_after_reply() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.out = [];\
             globalThis.iso = Window.create({ title: 'Inspector' });\
             globalThis.quiet = Window.create({ title: 'Quiet' });\
             iso.onRequest((from, req, reply) => reply({ n: req.kind.length, from: from.handle }));\
             quiet.onRequest(() => {});\
             iso.request({ kind: 'stats' }).then((r) => out.push('ok ' + r.n + ' ' + r.from));\
             quiet.request({ kind: 'x' }, 50).catch((e) => out.push(e.constructor.name + ' ' + e.message));\
             Window.create({ title: 'Mute' }).request(1).catch((e) => out.push('mute ' + e.message));",
        );
        run_jobs(ctx);
        let out: String = js(ctx, "out.join('|')");
        assert_eq!(out, "", "nothing settles before the domains run");
        for h in [2, 3, 4] {
            window::deliver(window::DomainId(h), 0.0);
        }
        run_jobs(ctx);
        let out: String = js(ctx, "out.join('|')");
        assert_eq!(out, "", "the replies are on their way to the main domain");
        window::deliver(window::DomainId::MAIN, 10.0);
        run_jobs(ctx);
        let out: String = js(ctx, "out.join('|')");
        assert_eq!(out, "ok 5 1|mute no request listener");
        window::deliver(window::DomainId::MAIN, 51.0);
        run_jobs(ctx);
        let out: String = js(ctx, "out.join('|')");
        assert_eq!(out, "ok 5 1|mute no request listener|Error timed out");
        // A request to a closed window is refused at once.
        window::closed(WindowRef(2));
        let r: String = js(
            ctx,
            "globalThis.r2 = ''; iso.request(1).catch((e) => { globalThis.r2 = e.message; }); 'queued'",
        );
        assert_eq!(r, "queued");
        run_jobs(ctx);
        let r: String = js(ctx, "globalThis.r2");
        assert!(r.starts_with("request refused"), "{r}");
    });
}

// M9-TS-2861
#[test]
fn quickjs_emit_reaches_on_by_name_and_off_removes_it() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.log = [];\
             globalThis.iso = Window.create({ title: 'Inspector' });\
             globalThis.tok = iso.on('stats', (s, from) => log.push('stats ' + s.n + ' ' + s.tags.join('+') + ' from ' + from.handle));\
             iso.on('ping', (p) => log.push('ping ' + p));\
             iso.onMessage((from, msg) => log.push('raw ' + JSON.stringify(msg)));",
        );
        let d = window::DomainId(2);
        let ok: String = js(
            ctx,
            "[iso.emit('stats', { n: 3, tags: ['a', 'b'] }), iso.emit('ping'), iso.emit('other', 1), iso.postMessage({ kind: 5 })].join(',')",
        );
        assert_eq!(ok, "true,true,true,true");
        window::deliver(d, 0.0);
        let log: String = js(ctx, "log.join('|')");
        // The raw listener sees the raw message only; events, even of unknown names, are not its
        // business (M9-TS-2865).
        assert_eq!(log, "stats 3 a+b from 1|ping undefined|raw {\"kind\":5}");
        let _: () = js(
            ctx,
            "log.length = 0; iso.off(tok); iso.emit('stats', { n: 1, tags: [] }); iso.emit('ping', null);",
        );
        window::deliver(d, 1.0);
        let log: String = js(ctx, "log.join('|')");
        assert_eq!(log, "ping null");
        // Across the domain boundary: the isolated window emits to the main window's listener.
        let _: () = js(
            ctx,
            "globalThis.heard = [];\
             Window.main.on('hello', (v, from) => heard.push(v.who + ' from ' + from.handle));\
             iso.run(() => { Window.main.emit('hello', { who: 'iso' }); });",
        );
        window::deliver(d, 2.0);
        window::deliver(window::DomainId::MAIN, 2.0);
        let heard: String = js(ctx, "heard.join('|')");
        assert_eq!(heard, "iso from 2");
        // emitAll: the root windows of the other families get it.
        let n: u32 = js(ctx, "Window.emitAll('all', { k: 1 })");
        assert_eq!(n, 1);
        let n: u32 = js(ctx, "Window.emitAll('all2')");
        assert_eq!(n, 1);
    });
}

// M9-TS-2862
#[test]
fn quickjs_ask_resolves_with_the_named_answer() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.out = [];\
             globalThis.iso = Window.create({ title: 'Inspector' });\
             iso.answer('stats', (req, from) => ({ n: req.kind.length, from: from.handle }));\
             iso.answer('count', (xs) => xs.length);\
             iso.answer('void', () => {});\
             iso.ask('stats', { kind: 'abc' }).then((r) => out.push('stats ' + r.n + ' ' + r.from));\
             iso.ask('count', [1, 2, 3]).then((r) => out.push('count ' + r));\
             iso.ask('void').then((r) => out.push('void ' + r));\
             iso.ask('nobody', undefined, 50).catch((e) => out.push('nobody ' + e.message));\
             Window.create({ title: 'Mute' }).ask('stats').catch((e) => out.push('mute ' + e.message));",
        );
        for h in [2, 3] {
            window::deliver(window::DomainId(h), 0.0);
        }
        window::deliver(window::DomainId::MAIN, 10.0);
        run_jobs(ctx);
        let out: String = js(ctx, "out.join('|')");
        // Names nobody answers fail at delivery, not after the timeout (M9-TS-2866).
        assert_eq!(
            out,
            "stats 3 1|count 3|void null|nobody no answer for \"nobody\"|mute no answer for \"stats\""
        );
    });
}

// M9-TS-2863
#[test]
fn quickjs_events_reject_empty_names_handles_and_oversize_payloads() {
    with_world(|ctx| {
        let r: String = js(
            ctx,
            "const w = Window.create({ title: 'T' });\
             const tries = [\
               () => w.emit('', 1),\
               () => w.on('', () => {}),\
               () => w.answer('', () => 1),\
               () => w.ask('', 1),\
               () => Window.emitAll(''),\
               () => w.emit('x', { go: new GameObject('x') }),\
             ];\
             tries.map((t) => { try { t(); return 'sent'; } catch (e) { return e.constructor.name + ': ' + e.message; } }).join('\\n')",
        );
        let lines: Vec<&str> = r.lines().collect();
        for l in &lines[..5] {
            assert_eq!(*l, "TypeError: Window event names must not be empty", "{r}");
        }
        assert!(
            lines[5].contains("\"go\"") && lines[5].contains("GameObject handle"),
            "{r}"
        );
        assert_eq!(
            window::pending(window::DomainId(2)),
            0,
            "nothing was queued"
        );
        let big: bool = js(ctx, "w.emit('big', 'x'.repeat(1 << 20))");
        assert!(!big);
        let r: String = js(
            ctx,
            "globalThis.r = ''; w.ask('big', 'x'.repeat(1 << 20)).catch((e) => { globalThis.r = e.message; }); 'queued'",
        );
        assert_eq!(r, "queued");
        run_jobs(ctx);
        let r: String = js(ctx, "globalThis.r");
        assert!(r.starts_with("request refused"), "{r}");
    });
}

// M9-TS-2865
#[test]
fn quickjs_events_and_raw_messages_are_separate_layers() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.log = [];             globalThis.iso = Window.create({ title: 'Inspector' });             iso.onMessage((from, m) => log.push('raw ' + JSON.stringify(m)));             iso.on('hi', (p) => log.push('hi ' + JSON.stringify(p)));             iso.onRequest((from, r, reply) => { log.push('rawreq ' + JSON.stringify(r)); reply('raw'); });             iso.answer('q', () => { log.push('answer q'); return 'named'; });             globalThis.out = [];             iso.emit('hi', 1); iso.postMessage({ '$e': 'hi' }); iso.postMessage('hi');             iso.ask('q').then((r) => out.push('ask ' + r));             iso.request('q').then((r) => out.push('request ' + r));             iso.ask('nope').catch((e) => out.push(e.message));",
        );
        window::deliver(window::DomainId(2), 0.0);
        window::deliver(window::DomainId::MAIN, 1.0);
        run_jobs(ctx);
        let log: String = js(ctx, "log.join('|')");
        // A raw post that merely looks like an envelope is one (documented); `'hi'` is not.
        assert_eq!(log, "hi 1|hi undefined|raw \"hi\"|answer q|rawreq \"q\"");
        let out: String = js(ctx, "out.join('|')");
        assert_eq!(out, "ask named|request raw|no answer for \"nope\"");
    });
}

// M9-TS-2896
#[test]
fn quickjs_async_answer_and_request_handlers_reply_when_resolved_and_reject_to_the_asker() {
    with_world(|ctx| {
        let _: () = js(
            ctx,
            "globalThis.out = [];             globalThis.iso = Window.create({ title: 'Inspector' });             iso.answer('double', async (n) => { await null; return n * 2; });             iso.answer('boom', async (n) => { await null; throw new Error('boom ' + n); });             iso.onRequest(async (from, r, reply) => { await null; if (r.k < 0) throw new Error('negative'); reply(r.k + 1); });             iso.ask('double', 21).then((r) => out.push('double ' + r));             iso.ask('boom', 5).catch((e) => out.push('boom ' + e.message));             iso.request({ k: 1 }).then((r) => out.push('request ' + r));             iso.request({ k: -1 }).catch((e) => out.push('request ' + e.message));",
        );
        // The handlers run on delivery and finish in later jobs; the replies travel back after.
        for t in 0..4 {
            window::deliver(window::DomainId(2), f64::from(t));
            run_jobs(ctx);
            window::deliver(window::DomainId::MAIN, f64::from(t) + 0.5);
            run_jobs(ctx);
        }
        let out: String = js(ctx, "out.sort().join('|')");
        assert_eq!(out, "boom boom 5|double 42|request 2|request negative");
    });
}
