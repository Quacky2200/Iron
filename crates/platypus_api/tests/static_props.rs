// SPDX-License-Identifier: Apache-2.0

//! Static properties of handle classes: `Demo.current` is a getter on the class in the manifest and in
//! QuickJS (`Window.main` reads the same in both backends). IDs are in TESTS.md (M9-API-*).
use iron_api::{FnKind, Handle, Ty, iron_api};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemoId(pub u64);
impl Handle for DemoId {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        DemoId(raw)
    }
}

thread_local! {
    static CURRENT: Cell<u64> = const { Cell::new(1) };
    static LEVEL: Cell<u32> = const { Cell::new(3) };
    static READS: Cell<u32> = const { Cell::new(0) };
}

#[iron_api(namespace = "Demo", quickjs, handles(Demo = DemoId))]
pub mod demo {
    use super::{CURRENT, DemoId, LEVEL, READS};

    impl DemoId {
        /// Moves the current one on and returns it.
        pub fn advance() -> DemoId {
            CURRENT.with(|c| c.set(c.get() + 1));
            DemoId(CURRENT.with(|c| c.get()))
        }

        /// The current one, read through the getter each time.
        #[property]
        pub fn current() -> DemoId {
            READS.with(|r| r.set(r.get() + 1));
            DemoId(CURRENT.with(|c| c.get()))
        }

        /// A writable static.
        #[property]
        pub fn level() -> u32 {
            LEVEL.with(|l| l.get())
        }

        pub fn set_level(v: u32) {
            LEVEL.with(|l| l.set(v));
        }

        #[property]
        pub fn id(self) -> f64 {
            self.0 as f64
        }

        #[property]
        #[js_name = "level"]
        pub fn level_of(self) -> f64 {
            f64::from(LEVEL.with(|l| l.get())) * 10.0
        }
    }
}

fn with_js<R>(f: impl for<'js> FnOnce(&rquickjs::Ctx<'js>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        demo::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-1490
#[test]
fn a_static_property_of_a_handle_class_is_an_accessor_in_quickjs() {
    CURRENT.with(|c| c.set(1));
    READS.with(|r| r.set(0));
    // The manifest: a getter of the class without a receiver.
    let class = demo::MANIFEST
        .classes
        .iter()
        .find(|c| c.name == "Demo")
        .unwrap();
    let cur = class
        .functions
        .iter()
        .find(|f| f.name == "current")
        .unwrap();
    assert_eq!(cur.kind, FnKind::Get);
    assert!(cur.params.is_empty(), "no `this`");
    assert_eq!(cur.ret, Ty::Handle("Demo"));
    let d = demo::MANIFEST.to_dts();
    assert!(d.contains("static readonly current: Demo;"), "{d}");
    assert!(d.contains("static get level(): number;"), "{d}");
    assert!(d.contains("static set level(value: number);"), "{d}");
    assert!(d.contains("get id(): number;"), "{d}");
    assert!(d.contains("get level(): number;"), "{d}");
    let j = demo::MANIFEST.to_json();
    assert!(
        j.contains("{\"name\":\"current\",\"symbol\":\"iron_Demo_current\",\"kind\":\"get\",\"params\":[],\"returns\":\"handle:Demo\",\"doc\":\"The current one, read through the getter each time.\"}"),
        "{j}"
    );
    // The shim has no receiver either.
    assert_eq!(demo::iron_Demo_current(), 1);

    with_js(|ctx| {
        let t: String = ctx.eval("typeof Demo.current").unwrap();
        assert_eq!(t, "object");
        let is_fn: bool = ctx
            .eval("typeof Object.getOwnPropertyDescriptor(Demo, 'current').get === 'function'")
            .unwrap();
        assert!(is_fn);
        let enumerable: bool = ctx
            .eval("Object.getOwnPropertyDescriptor(Demo, 'current').enumerable")
            .unwrap();
        assert!(!enumerable);
        // Read through the getter each time: advancing changes what it returns.
        let ids: String = ctx
            .eval(
                "const a = Demo.current.id; Demo.advance(); const b = Demo.current.id; a + ',' + b",
            )
            .unwrap();
        assert_eq!(ids, "1,2");
        // The instance getter of the same script name still lives on the prototype.
        let lv: String = ctx.eval("Demo.current.level + ',' + Demo.level").unwrap();
        assert_eq!(lv, "30,3");
        let on_proto: bool = ctx
            .eval("Object.getOwnPropertyDescriptor(Demo.prototype, 'level') !== undefined")
            .unwrap();
        assert!(on_proto);
    });
    assert!(READS.with(|r| r.get()) >= 3);
}

// M9-API-1491
#[test]
fn a_static_setter_writes_through_and_a_read_only_one_ignores_writes() {
    LEVEL.with(|l| l.set(3));
    with_js(|ctx| {
        let v: u32 = ctx.eval("Demo.level = 9; Demo.level").unwrap();
        assert_eq!(v, 9);
        assert_eq!(LEVEL.with(|l| l.get()), 9);
        // A write to a getter-only property throws in strict mode (rquickjs evaluates strictly).
        assert!(ctx.eval::<(), _>("Demo.current = null;").is_err());
        let still: String = ctx.eval("typeof Demo.current").unwrap();
        assert_eq!(still, "object");
        let err: String = ctx
            .eval("(() => { 'use strict'; try { Demo.current = null; return 'no error'; } catch (e) { return e.constructor.name; } })()")
            .unwrap();
        assert_eq!(err, "TypeError");
    });
}
