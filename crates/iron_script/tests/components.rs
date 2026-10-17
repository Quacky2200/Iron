use iron_script::conformance::capture_logs;
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};

const HELLO: &str = include_str!("../../../tests/fixtures/hello.js");
const THROWS: &str = include_str!("../../../tests/fixtures/throws.js");

fn messages(logs: &std::cell::RefCell<Vec<(iron_script::LogLevel, String)>>) -> Vec<String> {
    logs.borrow().iter().map(|(_, m)| m.clone()).collect()
}

// M1-CMP-01
#[test]
fn bundle_registers_component_classes() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("hello.js", HELLO).unwrap();
    assert_eq!(
        ComponentHost::registered_classes(&mut rt).unwrap(),
        ["Hello"]
    );
}

// M1-CMP-02, M1-CMP-03
#[test]
fn on_start_runs_once_and_on_update_every_tick() {
    let mut rt = QuickJsRuntime::new();
    let logs = capture_logs(&mut rt);
    rt.load_bundle("hello.js", HELLO).unwrap();

    let mut host = ComponentHost::new();
    host.spawn(&mut rt, "Hello").unwrap();
    for _ in 0..3 {
        assert!(host.update(&mut rt, 1.0 / 60.0).is_empty());
    }
    assert_eq!(messages(&logs), ["hello", "tick", "tick", "tick"]);
}

#[test]
fn spawning_an_unknown_class_fails() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("hello.js", HELLO).unwrap();
    let mut host = ComponentHost::new();
    let err = host.spawn(&mut rt, "Nope").unwrap_err();
    assert!(
        err.message.contains("unknown component class 'Nope'"),
        "{err}"
    );
    assert!(host.is_empty());
}

// M1-CMP-04
#[test]
fn a_throwing_component_is_disabled_and_others_keep_running() {
    let mut rt = QuickJsRuntime::new();
    let logs = capture_logs(&mut rt);
    rt.load_bundle("throws.js", THROWS).unwrap();

    let mut host = ComponentHost::new();
    let a = host.spawn(&mut rt, "Hello").unwrap();
    let broken = host.spawn(&mut rt, "Broken").unwrap();
    let b = host.spawn(&mut rt, "Hello").unwrap();

    let errors = host.update(&mut rt, 0.016);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, broken);
    assert_eq!(errors[0].1.message, "Error: broken on purpose");
    assert_eq!(errors[0].1.file.as_deref(), Some("throws.js"));

    assert!(
        host.update(&mut rt, 0.016).is_empty(),
        "broken component must not run again"
    );
    assert!(host.is_enabled(a) && host.is_enabled(b) && !host.is_enabled(broken));
    assert_eq!(
        messages(&logs),
        [
            format!("tick {a}"),
            format!("tick {b}"),
            format!("tick {a}"),
            format!("tick {b}")
        ]
    );
}

// M4-HOT-01
#[test]
fn a_reload_runs_the_new_code_on_the_next_tick() {
    let mut rt = QuickJsRuntime::new();
    let logs = capture_logs(&mut rt);
    rt.load_bundle("hello.js", HELLO).unwrap();
    let mut host = ComponentHost::new();
    host.spawn(&mut rt, "Hello").unwrap();
    host.update(&mut rt, 1.0 / 60.0);

    let edited = HELLO.replace("\"tick\"", "\"tock\"");
    assert_ne!(edited, HELLO);
    let mut fresh = QuickJsRuntime::new();
    let new_logs = capture_logs(&mut fresh);
    let classes = iron_script::reload(&mut rt, fresh, &mut host, "hello.js", &edited).unwrap();
    assert_eq!(classes, ["Hello"]);
    host.update(&mut rt, 1.0 / 60.0);
    assert_eq!(messages(&logs), ["hello", "tick"]);
    // Fresh components start over: onStart runs again (no state carried yet).
    assert_eq!(messages(&new_logs), ["hello", "tock"]);
}

// M4-HOT-04
#[test]
fn a_broken_reload_keeps_the_old_code_running() {
    let mut rt = QuickJsRuntime::new();
    let logs = capture_logs(&mut rt);
    rt.load_bundle("hello.js", HELLO).unwrap();
    let mut host = ComponentHost::new();
    host.spawn(&mut rt, "Hello").unwrap();

    for (broken, expected) in [
        ("this is not javascript (", "SyntaxError"),
        ("throw new Error('top level')", "top level"),
    ] {
        let err = iron_script::reload(&mut rt, QuickJsRuntime::new(), &mut host, "game.js", broken)
            .unwrap_err();
        assert!(err.message.contains(expected), "{err}");
    }
    host.update(&mut rt, 1.0 / 60.0);
    assert_eq!(messages(&logs), ["hello", "tick"]);
}
