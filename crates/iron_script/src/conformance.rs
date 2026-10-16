//! Backend-independent tests every [`ScriptRuntime`] must pass (M1-SCR-08).
//!
//! A backend's test file runs them all with [`runtime_conformance_tests!`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::{LogLevel, ScriptRuntime, Value};

/// Generates one `#[test]` per conformance check for a backend constructor.
#[macro_export]
macro_rules! runtime_conformance_tests {
    ($make:expr) => {
        $crate::runtime_conformance_tests!(@tests $make;
            evaluates_expressions,
            syntax_errors_report_location,
            exceptions_report_message_and_stack,
            console_reaches_log_sink,
            reset_wipes_globals,
            ops_arrive_in_order,
            unicode_round_trips,
            calls_globals_with_arguments,
        );
    };
    (@tests $make:expr; $($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let mut rt = $make;
                $crate::conformance::$name(&mut rt);
            }
        )*
    };
}

/// Collects log lines for assertions.
pub fn capture_logs(rt: &mut dyn ScriptRuntime) -> Rc<RefCell<Vec<(LogLevel, String)>>> {
    let lines = Rc::new(RefCell::new(Vec::new()));
    let sink = lines.clone();
    rt.set_log_sink(Rc::new(move |level, msg| {
        sink.borrow_mut().push((level, msg.to_owned()))
    }));
    lines
}

// M1-SCR-01
pub fn evaluates_expressions(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle("init.js", "var answer = 40;").unwrap();
    assert_eq!(rt.eval("1 + 1").unwrap(), Value::Number(2.0));
    assert_eq!(rt.eval("answer + 2").unwrap(), Value::Number(42.0));
}

// M1-SCR-02
pub fn syntax_errors_report_location(rt: &mut dyn ScriptRuntime) {
    let err = rt
        .load_bundle("broken.js", "let ok = 1;\nlet = = ;\n")
        .unwrap_err();
    assert!(
        err.message.contains("SyntaxError"),
        "message: {}",
        err.message
    );
    assert_eq!(err.file.as_deref(), Some("broken.js"), "{err:?}");
    assert_eq!(err.line, Some(2), "{err:?}");
}

// M1-SCR-03
pub fn exceptions_report_message_and_stack(rt: &mut dyn ScriptRuntime) {
    let err = rt
        .load_bundle(
            "throws.js",
            "function explode() {\n  throw new Error('kaboom');\n}\nexplode();\n",
        )
        .unwrap_err();
    assert_eq!(err.message, "Error: kaboom");
    let stack = err.stack.as_deref().unwrap_or_default();
    assert!(stack.contains("explode"), "stack: {stack}");
    assert_eq!(err.file.as_deref(), Some("throws.js"));
    assert_eq!(err.line, Some(2));
}

// M1-SCR-04
pub fn console_reaches_log_sink(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "log.js",
        "console.log('hello', 42, true); console.warn('careful'); console.error(new Error('bad'));",
    )
    .unwrap();
    assert_eq!(
        *logs.borrow(),
        [
            (LogLevel::Info, "hello 42 true".to_owned()),
            (LogLevel::Warn, "careful".to_owned()),
            (LogLevel::Error, "bad".to_owned()),
        ]
    );
}

// M1-SCR-05
pub fn reset_wipes_globals(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle("a.js", "globalThis.leftover = 1; __iron_op('pending');")
        .unwrap();
    rt.reset();
    assert_eq!(rt.eval("typeof leftover").unwrap(), "undefined".into());
    assert!(
        rt.drain_ops().is_empty(),
        "ops from before reset must be dropped"
    );
    // Natives and the log sink survive the reset.
    rt.load_bundle("b.js", "console.log('after'); __iron_op('x');")
        .unwrap();
    assert_eq!(logs.borrow().last().unwrap().1, "after");
    assert_eq!(rt.drain_ops().len(), 1);
}

// M1-SCR-06
pub fn ops_arrive_in_order(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle(
        "ops.js",
        "for (let i = 0; i < 3; i++) __iron_op('create', i, 'box'); __iron_op('flush');",
    )
    .unwrap();
    let ops: Vec<_> = rt.drain_ops().into_iter().collect();
    assert_eq!(ops.len(), 4);
    for (i, op) in ops[..3].iter().enumerate() {
        assert_eq!(op.name, "create");
        assert_eq!(op.args, [Value::Number(i as f64), "box".into()]);
    }
    assert_eq!(ops[3].name, "flush");
    assert!(rt.drain_ops().is_empty());
}

// M1-SCR-07
pub fn unicode_round_trips(rt: &mut dyn ScriptRuntime) {
    let text = "héllo 世界 🎮";
    rt.load_bundle("echo.js", "function echo(s) { return s; }")
        .unwrap();
    assert_eq!(rt.call_global("echo", &[text.into()]).unwrap(), text.into());
    // Length in UTF-16 code units proves JS saw the real characters, not mangled bytes.
    assert_eq!(
        rt.call_global("eval", &["'héllo 世界 🎮'.length".into()])
            .unwrap(),
        Value::Number(11.0)
    );
    let logs = capture_logs(rt);
    rt.load_bundle("u.js", "console.log('héllo 世界 🎮');")
        .unwrap();
    assert_eq!(logs.borrow()[0].1, text);
}

pub fn calls_globals_with_arguments(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle(
        "f.js",
        "function sum(a, b) { return a + b; } function pair(a) { return [a, String(a)]; }",
    )
    .unwrap();
    assert_eq!(
        rt.call_global("sum", &[2.0.into(), 3.5.into()]).unwrap(),
        Value::Number(5.5)
    );
    assert_eq!(
        rt.call_global("pair", &[7.0.into()]).unwrap(),
        Value::Array(vec![Value::Number(7.0), "7".into()])
    );
    let err = rt.call_global("missing", &[]).unwrap_err();
    assert!(err.message.contains("missing"), "{err}");
}
