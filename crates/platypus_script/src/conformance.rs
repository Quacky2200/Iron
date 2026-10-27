// SPDX-License-Identifier: Apache-2.0

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
            timers_fire_in_due_order,
            zero_delay_timers_fire_in_the_same_tick,
            timer_errors_are_reported_and_others_still_run,
            microtasks_run_before_returning,
            microtasks_run_between_timers,
            commits_arrive_whole_and_in_order,
            reset_drops_pending_timers_and_commits,
            typed_arrays_arrive_as_number_lists,
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
    rt.load_bundle("a.js", "globalThis.leftover = 1; __platypus_op('pending');")
        .unwrap();
    rt.reset();
    assert_eq!(rt.eval("typeof leftover").unwrap(), "undefined".into());
    assert!(
        rt.drain_ops().is_empty(),
        "ops from before reset must be dropped"
    );
    // Natives and the log sink survive the reset.
    rt.load_bundle("b.js", "console.log('after'); __platypus_op('x');")
        .unwrap();
    assert_eq!(logs.borrow().last().unwrap().1, "after");
    assert_eq!(rt.drain_ops().len(), 1);
}

// M1-SCR-06
pub fn ops_arrive_in_order(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle(
        "ops.js",
        "for (let i = 0; i < 3; i++) __platypus_op('create', i, 'box'); __platypus_op('flush');",
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

// M7-FOG-09
pub fn typed_arrays_arrive_as_number_lists(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle(
        "typed.js",
        "__platypus_op('mesh', new Float32Array([0.5, -2, 3]), new Uint32Array([0, 1, 2]),          new Float32Array([1, 2, 3, 4]).subarray(1, 3), new Uint8Array([255]));",
    )
    .unwrap();
    let ops: Vec<_> = rt.drain_ops().into_iter().collect();
    let n = |v: &[f64]| Value::Array(v.iter().map(|&x| Value::Number(x)).collect());
    assert_eq!(
        ops[0].args,
        [
            n(&[0.5, -2.0, 3.0]),
            n(&[0.0, 1.0, 2.0]),
            n(&[2.0, 3.0]),
            n(&[255.0])
        ]
    );
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

fn log_messages(logs: &RefCell<Vec<(LogLevel, String)>>) -> Vec<String> {
    logs.borrow().iter().map(|(_, m)| m.clone()).collect()
}

// M2-SCR-01
pub fn timers_fire_in_due_order(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "timers.js",
        "setTimeout(() => console.log('c'), 30);
         setTimeout(() => console.log('a'), 10);
         setTimeout((x, y) => console.log('b', x, y), 10, 1, 2);
         const id = setTimeout(() => console.log('cancelled'), 5);
         clearTimeout(id);",
    )
    .unwrap();
    assert!(rt.tick(9.0).is_empty());
    assert!(logs.borrow().is_empty(), "nothing is due before 10ms");
    assert!(rt.tick(10.0).is_empty());
    assert_eq!(log_messages(&logs), ["a", "b 1 2"]);
    assert!(rt.tick(100.0).is_empty());
    assert_eq!(log_messages(&logs), ["a", "b 1 2", "c"]);
}

// M2-SCR-01: React's scheduler falls back to setTimeout(0); it must not cost a frame.
pub fn zero_delay_timers_fire_in_the_same_tick(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "chain.js",
        "let n = 0;
         function step() { console.log('step ' + n); if (++n < 3) setTimeout(step, 0); }
         setTimeout(step, 0);
         setTimeout(function forever() { setTimeout(forever, 0); }, 0);",
    )
    .unwrap();
    // The endless chain is cut off by MAX_TIMERS_PER_TICK instead of hanging.
    assert!(rt.tick(16.0).is_empty());
    assert_eq!(log_messages(&logs), ["step 0", "step 1", "step 2"]);
}

// M2-SCR-01
pub fn timer_errors_are_reported_and_others_still_run(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "boom.js",
        "setTimeout(() => { throw new Error('timer failed'); }, 1);
         setTimeout(() => console.log('still running'), 2);",
    )
    .unwrap();
    let errors = rt.tick(5.0);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Error: timer failed");
    assert_eq!(errors[0].file.as_deref(), Some("boom.js"));
    assert_eq!(log_messages(&logs), ["still running"]);
    assert!(rt.tick(10.0).is_empty(), "errors are returned once");
}

// M2-SCR-02
pub fn microtasks_run_before_returning(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "micro.js",
        "Promise.resolve().then(() => console.log('promise'));
         queueMicrotask(() => console.log('microtask'));
         (async () => { await null; console.log('async'); })();
         console.log('sync');",
    )
    .unwrap();
    assert_eq!(
        log_messages(&logs),
        ["sync", "promise", "microtask", "async"]
    );
}

// M2-SCR-02
pub fn microtasks_run_between_timers(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "order.js",
        "setTimeout(() => { console.log('a'); queueMicrotask(() => console.log('a micro')); }, 1);
         setTimeout(() => console.log('b'), 1);",
    )
    .unwrap();
    assert!(rt.tick(1.0).is_empty());
    assert_eq!(log_messages(&logs), ["a", "a micro", "b"]);
}

// M2-SCR-03
pub fn commits_arrive_whole_and_in_order(rt: &mut dyn ScriptRuntime) {
    rt.load_bundle(
        "commit.js",
        "__platypus_commit([[0, 1, 'box'], [2, 0, 1]]);
         __platypus_commit([[8, 1]]);",
    )
    .unwrap();
    let commits = rt.drain_commits();
    assert_eq!(commits.len(), 2);
    assert_eq!(
        commits[0].ops,
        [
            Value::Array(vec![0.0.into(), 1.0.into(), "box".into()]),
            Value::Array(vec![2.0.into(), 0.0.into(), 1.0.into()]),
        ]
    );
    assert_eq!(commits[1].ops, [Value::Array(vec![8.0.into(), 1.0.into()])]);
    assert!(rt.drain_commits().is_empty());
}

// M2-SCR-03
pub fn reset_drops_pending_timers_and_commits(rt: &mut dyn ScriptRuntime) {
    let logs = capture_logs(rt);
    rt.load_bundle(
        "pending.js",
        "setTimeout(() => console.log('old timer'), 1); __platypus_commit([[0, 1, 'box']]);",
    )
    .unwrap();
    rt.reset();
    assert!(rt.drain_commits().is_empty());
    assert!(rt.tick(100.0).is_empty());
    assert!(
        logs.borrow().is_empty(),
        "timers from before reset must not fire"
    );
    // Timers work in the fresh context.
    rt.load_bundle("fresh.js", "setTimeout(() => console.log('new timer'), 1);")
        .unwrap();
    rt.tick(200.0);
    assert_eq!(log_messages(&logs), ["new timer"]);
}
