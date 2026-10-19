//! Properties on a namespace: `Time.deltaTime`, `Gizmos.color = c`. A `#[property]` function without
//! a receiver is the getter, `set_x(v)` next to it the setter. IDs are in TESTS.md (M9-API-*).
use iron_api::{FnKind, Ty, iron_api};
use std::cell::Cell;

thread_local! {
    static LEVEL: Cell<u32> = const { Cell::new(1) };
}

#[iron_api(namespace = "Cfg", quickjs)]
pub mod cfg {
    use super::LEVEL;

    /// The current level.
    #[property]
    pub fn level() -> u32 {
        LEVEL.with(|l| l.get())
    }

    pub fn set_level(value: u32) {
        LEVEL.with(|l| l.set(value));
    }

    /// Read only.
    #[property]
    pub fn title() -> String {
        "Iron".to_string()
    }

    pub fn reset() {
        LEVEL.with(|l| l.set(1));
    }
}

// M9-API-27
#[test]
fn namespace_properties_are_getters_and_setters_without_a_receiver() {
    let m = cfg::MANIFEST;
    let kinds: Vec<(&str, FnKind)> = m.functions.iter().map(|f| (f.name, f.kind)).collect();
    assert_eq!(
        kinds,
        [
            ("level", FnKind::Get),
            ("level", FnKind::Set),
            ("title", FnKind::Get),
            ("reset", FnKind::Static),
        ]
    );
    let setter = &m.functions[1];
    assert_eq!(setter.params.len(), 1, "no receiver");
    assert_eq!(setter.params[0].ty, Ty::U32);
    let d = m.to_dts();
    assert!(d.contains("let level: number;"), "{d}");
    assert!(d.contains("const title: string;"), "{d}");
    assert!(d.contains("function reset(): void;"), "{d}");
    assert!(!d.contains("function level"), "{d}");
    let j = m.to_json();
    assert!(j.contains("\"kind\":\"get\""), "{j}");
    assert!(j.contains("\"kind\":\"set\""), "{j}");
}

// M9-API-28
#[test]
fn the_shims_of_namespace_properties_take_no_receiver() {
    cfg::iron_Cfg_reset();
    assert_eq!(cfg::iron_Cfg_level(), 1);
    cfg::iron_Cfg_setLevel(7);
    assert_eq!(cfg::iron_Cfg_level(), 7);
}

// M9-API-29
#[test]
fn quickjs_gets_accessors_for_namespace_properties() {
    cfg::iron_Cfg_reset();
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        cfg::register_quickjs(&ctx).unwrap();
        let v: u32 = ctx.eval("Cfg.level = 9; Cfg.level + 1").unwrap();
        assert_eq!(v, 10);
        let t: String = ctx.eval("Cfg.title").unwrap();
        assert_eq!(t, "Iron");
        ctx.eval::<(), _>("Cfg.reset()").unwrap();
        let v: u32 = ctx.eval("Cfg.level").unwrap();
        assert_eq!(v, 1);
    });
}
