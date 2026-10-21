//! Timers per event domain in the QuickJS runtime. IDs are in TESTS.md (M9-TS-28xx).
use iron_script::{QuickJsRuntime, ScriptRuntime, Value};
use std::cell::RefCell;

thread_local! {
    /// The host's domain stack: the last entry is current.
    static DOMAINS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn install(ctx: &rquickjs::Ctx<'_>) -> rquickjs::Result<()> {
    use rquickjs::Function;
    let g = ctx.globals();
    g.set(
        "__iron_domain",
        Function::new(ctx.clone(), || {
            DOMAINS.with(|d| d.borrow().last().copied().unwrap_or(1))
        })?,
    )?;
    g.set(
        "__iron_enter_domain",
        Function::new(ctx.clone(), |d: u32| {
            DOMAINS.with(|s| s.borrow_mut().push(d))
        })?,
    )?;
    g.set(
        "__iron_leave_domain",
        Function::new(ctx.clone(), || {
            DOMAINS
                .with(|s| s.borrow_mut().pop())
                .map(|_| ())
                .unwrap_or(())
        })?,
    )?;
    Ok(())
}

// M9-TS-2833
#[test]
fn quickjs_timers_keep_their_domain() {
    let mut rt = QuickJsRuntime::new();
    rt.add_native(install).unwrap();
    rt.eval("globalThis.log = []; globalThis.here = () => __iron_domain();")
        .unwrap();
    DOMAINS.with(|d| d.borrow_mut().push(5));
    rt.eval("setTimeout(() => log.push('a' + here()), 10)")
        .unwrap();
    DOMAINS.with(|d| d.borrow_mut().pop());
    rt.eval("setTimeout(() => log.push('b' + here()), 10)")
        .unwrap();
    // The main domain's tick leaves the other domain's timer alone.
    assert!(rt.tick(100.0).is_empty());
    assert_eq!(
        rt.eval("log.join(',')").unwrap(),
        Value::String("b1".into())
    );
    // Its own tick fires it with the domain entered, and leaves the stack as it was.
    assert!(rt.tick_domain(100.0, 5).is_empty());
    assert_eq!(
        rt.eval("log.join(',')").unwrap(),
        Value::String("b1,a5".into())
    );
    assert!(DOMAINS.with(|d| d.borrow().is_empty()));
}
