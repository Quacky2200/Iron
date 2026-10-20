//! Closure parameters from QuickJS: a JS function is kept (reference counted, so the collector
//! sees it as a root) and called with the same arguments compiled code gives. IDs are in TESTS.md
//! (M9-API-*).
use iron_api::callback::{HandleArg, Listeners, emit, take_errors};
use iron_api::{Callback, Handle, iron_api};
use std::cell::RefCell;

#[derive(Clone, Copy)]
pub struct Thing(pub u64);
impl Handle for Thing {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        Thing(raw)
    }
}

thread_local! {
    static EVENTS: RefCell<Listeners<(f64, String)>> = RefCell::new(Listeners::default());
    static THINGS: RefCell<Listeners<(HandleArg<Thing>, bool)>> = RefCell::new(Listeners::default());
    static PLAIN: RefCell<Listeners<()>> = RefCell::new(Listeners::default());
}

#[iron_api(namespace = "Events", quickjs, handles(Thing = Thing))]
pub mod events_api {
    use super::{EVENTS, PLAIN, THINGS, Thing};
    use iron_api::Callback;
    use iron_api::callback::HandleArg;

    pub fn on_event(f: Callback<(f64, String)>) -> u32 {
        EVENTS.with(|l| l.borrow_mut().add(f))
    }

    pub fn on_thing(f: Callback<(HandleArg<Thing>, bool)>) -> u32 {
        THINGS.with(|l| l.borrow_mut().add(f))
    }

    pub fn on_plain(f: Callback<()>) -> u32 {
        PLAIN.with(|l| l.borrow_mut().add(f))
    }

    /// Removes any listener by token.
    pub fn off(token: f64) -> bool {
        let t = token as u32;
        EVENTS.with(|l| l.borrow_mut().remove(t))
            | THINGS.with(|l| l.borrow_mut().remove(t))
            | PLAIN.with(|l| l.borrow_mut().remove(t))
    }
}

fn clear() {
    EVENTS.with(|l| l.borrow_mut().clear());
    THINGS.with(|l| l.borrow_mut().clear());
    PLAIN.with(|l| l.borrow_mut().clear());
}

fn fire(n: f64, text: &str) {
    let cbs = EVENTS.with(|l| l.borrow().snapshot());
    emit(&cbs, (n, text.to_string()));
}

fn run(f: impl FnOnce(&rquickjs::Ctx<'_>)) {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        events_api::register_quickjs(&ctx).unwrap();
        f(&ctx);
    });
    // Callbacks hold script functions: release them before the runtime goes.
    clear();
}

// M9-API-122
#[test]
fn a_js_function_is_kept_and_called_with_the_events_arguments() {
    run(|ctx| {
        let token: f64 = ctx
            .eval("globalThis.log = []; Events.onEvent((n, text) => log.push(n + text))")
            .unwrap();
        assert!(token > 0.0);
        fire(1.0, "a");
        fire(2.0, "b");
        // The closure outlives the script that made it, and the collector keeps it.
        let _: () = ctx
            .eval("globalThis.gc_filler = Array.from({length: 1000}, () => ({}))")
            .unwrap();
        ctx.run_gc();
        fire(3.0, "c");
        let out: String = ctx.eval("JSON.stringify(log)").unwrap();
        assert_eq!(out, r#"["1a","2b","3c"]"#);
        // `off(token)` removes it.
        let removed: bool = ctx.eval(format!("Events.off({token})")).unwrap();
        assert!(removed);
        fire(4.0, "d");
        let n: f64 = ctx.eval("log.length").unwrap();
        assert_eq!(n, 3.0);
        let again: bool = ctx.eval(format!("Events.off({token})")).unwrap();
        assert!(!again);
    });
}

// M9-API-123
#[test]
fn handles_and_flags_reach_a_js_callback_as_objects_and_booleans() {
    run(|ctx| {
        let _: f64 = ctx
            .eval(
                "globalThis.seen = []; Events.onThing((t, flag) => seen.push([t instanceof Thing, String(t), flag])); \
                 Events.onPlain(() => seen.push('plain'))",
            )
            .unwrap();
        let cbs = THINGS.with(|l| l.borrow().snapshot());
        emit(&cbs, (HandleArg(Thing(7)), true));
        emit(&cbs, (HandleArg(Thing(0)), false));
        let cbs = PLAIN.with(|l| l.borrow().snapshot());
        emit(&cbs, ());
        let out: String = ctx.eval("JSON.stringify(seen)").unwrap();
        // A zero handle is null, which is no Thing.
        assert_eq!(
            out,
            r#"[[true,"[object Thing]",true],[false,"null",false],"plain"]"#
        );
    });
}

// M9-API-124
#[test]
fn an_exception_in_a_js_callback_is_recorded_and_the_next_still_run() {
    run(|ctx| {
        let _ = take_errors();
        let _: f64 = ctx
            .eval("globalThis.ok = 0; Events.onEvent(() => { throw new Error('boom'); })")
            .unwrap();
        let _: f64 = ctx.eval("Events.onEvent(() => { ok++; })").unwrap();
        fire(1.0, "x");
        fire(2.0, "y");
        let ok: f64 = ctx.eval("ok").unwrap();
        assert_eq!(ok, 2.0);
        let errors = take_errors();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].contains("boom"), "{errors:?}");
        // A callback may subscribe from inside a call (a snapshot is being emitted).
        let _: f64 = ctx
            .eval("Events.onEvent(() => { Events.onEvent(() => {}); })")
            .unwrap();
        fire(3.0, "z");
        let _ = take_errors();
    });
}

// M9-API-125
#[test]
fn a_callback_survives_a_collection_while_only_the_engine_holds_it() {
    run(|ctx| {
        let _: f64 = ctx
            .eval(
                "(() => { const big = { v: 41 }; return Events.onEvent((n) => { big.v += n; globalThis.last = big.v; }); })()",
            )
            .unwrap();
        for _ in 0..3 {
            let _: () = ctx
                .eval("for (let i = 0; i < 5000; i++) { globalThis.junk = { i, a: [i] }; }")
                .unwrap();
            ctx.run_gc();
        }
        fire(1.0, "");
        let last: f64 = ctx.eval("last").unwrap();
        assert_eq!(last, 42.0);
        let wrong: rquickjs::Result<f64> = ctx.eval("Events.onEvent(5)");
        assert!(wrong.is_err(), "a non-function is a TypeError");
    });
}

#[allow(dead_code)]
fn unused(_: Callback<()>) {}
