//! Contact hooks on compiled components (`bench/contacts.ts`): the physics step queues contacts
//! and `NativeHost::dispatch_physics` calls `onCollisionEnter` and friends by name. IDs are in
//! TESTS.md (M9-TS-1000..).
use iron_api::Handle;
use iron_scene::Scene;
use iron_scene::api::scene_api as api;
use iron_ts_rt::{ContactEvent, NativeHost, Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::contacts_t::*;

fn all(_: u64) -> bool {
    true
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    let run = || {
        iron_physics_api::reset();
        unsafe { ts_cnt_init() };
        f();
    };
    run();
    ts_rt_gc_stress(true);
    run();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    run();
    ts_rt_gc_enable(true);
}

fn log() -> String {
    let p = unsafe { ts_cnt_logText() };
    String::from_utf16_lossy(&unsafe { Str::from_raw(p as *const Str) }.to_utf16())
}

fn named(name: &str) -> iron_scene::ObjectId {
    let go = api::create();
    go.set_name(name);
    go
}

// M9-TS-1000
//
// A ball dropped on a floor tells the components of both objects `onCollisionEnter` with the other
// object and the impact speed; both sides get it, and a component without the hook is skipped.
#[test]
fn a_collision_tells_both_objects() {
    in_every_gc_mode(|| unsafe {
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        let mut host = NativeHost::new(classes());
        ts_cnt_clearLog();
        let floor = named("floor");
        let ball = named("ball");
        host.spawn("Probe", Some(floor.to_raw())).unwrap();
        host.spawn("Probe", Some(ball.to_raw())).unwrap();
        ts_cnt_build(floor.to_raw(), ball.to_raw(), 1.5, false);
        let mut enters = String::new();
        for _ in 0..60 {
            ts_cnt_steps(1.0);
            assert!(host.dispatch_physics(&all).is_empty());
            let l = log();
            if l.contains("E(") {
                enters = l;
                break;
            }
        }
        assert!(enters.contains("floor:E(ball,fast)"), "{enters}");
        assert!(enters.contains("ball:E(floor,fast)"), "{enters}");
    });
}

// M9-TS-1001
//
// `onCollisionStay` runs on each dispatch while the contact lasts, and `onCollisionExit` when it
// ends (here: the ball is lifted away); an inherited hook (a subclass) is called too.
#[test]
fn stay_runs_while_touching_and_exit_when_it_ends() {
    in_every_gc_mode(|| unsafe {
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        let mut host = NativeHost::new(classes());
        ts_cnt_clearLog();
        let floor = named("floor");
        let ball = named("ball");
        host.spawn("Child", Some(ball.to_raw())).unwrap();
        ts_cnt_build(floor.to_raw(), ball.to_raw(), 0.9, false);
        for _ in 0..30 {
            ts_cnt_steps(1.0);
            host.dispatch_physics(&all);
        }
        let l = log();
        assert!(l.starts_with("ball:E(floor,"), "{l}");
        assert!(l.matches("ball:S").count() >= 20, "{l}");
        assert!(!l.contains("ball:X"), "{l}");
        ts_cnt_clearLog();
        ts_cnt_lift();
        for _ in 0..3 {
            ts_cnt_steps(1.0);
            host.dispatch_physics(&all);
        }
        let l = log();
        assert!(l.starts_with("ball:X(floor)"), "{l}");
        assert_eq!(l.matches("ball:S").count(), 0, "no Stay after Exit: {l}");
    });
}

// M9-TS-1002
//
// A sensor collider reports `onTriggerEnter` (with `sensor` true) and `onTriggerStay`, not the
// collision hooks.
#[test]
fn a_sensor_calls_the_trigger_hooks() {
    in_every_gc_mode(|| unsafe {
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        let mut host = NativeHost::new(classes());
        ts_cnt_clearLog();
        let floor = named("floor");
        let ball = named("ball");
        host.spawn("Probe", Some(floor.to_raw())).unwrap();
        host.spawn("Probe", Some(ball.to_raw())).unwrap();
        ts_cnt_build(floor.to_raw(), ball.to_raw(), 0.5, true);
        for _ in 0..5 {
            ts_cnt_steps(1.0);
            host.dispatch_physics(&all);
        }
        let l = log();
        assert!(l.contains("ball:TE(floor,true)"), "{l}");
        assert!(l.contains("floor:TE(ball,true)"), "{l}");
        assert!(l.contains("ball:TS"), "{l}");
        assert!(!l.contains(":E(") && !l.contains(":S"), "{l}");
        ts_cnt_clearLog();
        ts_cnt_lift();
        ts_cnt_steps(2.0);
        host.dispatch_physics(&all);
        assert!(log().contains("ball:TX"), "{}", log());
    });
}

// M9-TS-1003
//
// Events can be fed by hand: inactive objects miss them, a hook that destroys its own component
// ends the walk safely, and a hook that throws is reported and disabled.
#[test]
fn events_skip_inactive_objects_and_faults_are_reported() {
    in_every_gc_mode(|| unsafe {
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        let mut host = NativeHost::new(classes());
        ts_cnt_clearLog();
        let a = named("a");
        let b = named("b");
        host.spawn("Quitter", Some(a.to_raw())).unwrap();
        let t = host.spawn("Thrower", Some(a.to_raw())).unwrap();
        let ev = |object: u64| ContactEvent {
            started: true,
            object,
            other: b.to_raw(),
            collider: 7,
            speed: 3.0,
            sensor: false,
        };
        let inactive = |_: u64| false;
        assert!(
            host.dispatch_contacts(&[ev(a.to_raw())], &inactive)
                .is_empty()
        );
        assert_eq!(log(), "");
        let faults = host.dispatch_contacts(&[ev(a.to_raw())], &all);
        assert_eq!(log(), "QT");
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].hook, "onCollisionEnter");
        assert!(!host.is_enabled(t));
        assert_eq!(host.len(), 1, "the quitter is gone");
    });
}
