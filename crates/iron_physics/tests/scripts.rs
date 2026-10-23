// SPDX-License-Identifier: Apache-2.0

//! Physics driven by a real script bundle in QuickJS, the way the app does it.

use iron_physics::Physics;
use iron_script::{ComponentHost, QuickJsRuntime, ScriptRuntime, Value};

const PHYSICS: &str = include_str!("../../../tests/fixtures/physics.js");

// M4-PHY-02
#[test]
fn collision_events_and_raycasts_reach_ts_components() {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("physics.js", PHYSICS).unwrap();
    let mut host = ComponentHost::new();
    host.spawn(&mut rt, "Dropper").unwrap();
    let mut physics = Physics::new();

    for _ in 0..90 {
        assert!(host.update(&mut rt, 1.0 / 60.0).is_empty());
        for op in rt.drain_ops() {
            // The engine's own ops (`render.quality` on load) aren't physics' business.
            let Some(name) = op.name.strip_prefix("physics.") else {
                continue;
            };
            physics.apply(name, &op.args).unwrap();
        }
        physics.step();
        let batch = physics.take_batch();
        if !batch.is_empty() {
            rt.call_global("__iron_physics", &batch.to_args()).unwrap();
        }
    }

    // The ball's centre is 0.5 m above the floor when it first touches, and the ray from
    // y = 5 hits its top (collider 2, at y = 1) 4 m down.
    let log = rt.eval("__physics_log()").unwrap();
    assert_eq!(log, Value::String("hit world at y=0.5, ray 2 4.00".into()));
}
