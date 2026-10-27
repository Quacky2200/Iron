// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! M4-HOT-07: a hot reload that keeps state, with 1,000 script components (each with a
//! child object) and a ~1 MB bundle. Measures saving in the old runtime, loading the
//! bundle into a fresh one, restoring and applying the ops it queues. Target: under
//! 100 ms. Run with `cargo bench -p platypus_scene --bench reload`.
//!
//! 2026-10-17: about 146 ms, of which ~54 ms is QuickJS parsing the 1 MB bundle,
//! ~27 ms saving, ~58 ms restoring (mostly building 2,000 GameObjects, ~10 µs each
//! in QuickJS) and ~2 ms applying ops. See TODO.md for the follow-ups.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use platypus_core::{ComponentId, Value};
use platypus_scene::{SceneEvent, ScriptScene};
use platypus_script::{ComponentHost, QuickJsRuntime, ScriptRuntime};

const SCENE: &str = include_str!("../../../tests/fixtures/scene.js");

/// The fixture padded to about 1 MB with code that only has to be parsed.
fn big_bundle() -> String {
    let mut code = String::from(SCENE);
    let mut i = 0;
    while code.len() < 1_000_000 {
        code.push_str(&format!(
            "\nfunction unused{i}(a, b) {{ const list = [a, b, {i}]; return list.map((x) => x * 2).filter((x) => x > {i}).join(','); }}"
        ));
        i += 1;
    }
    code
}

fn apply(rt: &mut QuickJsRuntime, host: &mut ComponentHost, scene: &mut ScriptScene) {
    for op in rt.drain_ops() {
        let Some(name) = op.name.strip_prefix("scene.") else {
            continue;
        };
        if let Some(SceneEvent::ComponentAdded {
            object,
            component,
            class,
        }) = scene.apply(name, &op.args).unwrap()
        {
            host.adopt(ComponentId(component), &class, Some(object));
        }
    }
}

fn reload(c: &mut Criterion) {
    let code = big_bundle();
    c.bench_function("reload keeping 1000 components", |b| {
        b.iter_batched(
            || {
                let mut rt = QuickJsRuntime::new();
                rt.load_bundle("scene.js", &code).unwrap();
                rt.eval("__hot_spawn(1000)").unwrap();
                rt.drain_ops();
                rt
            },
            |mut old| {
                let saved = old.call_global("__platypus_save_state", &[]).unwrap();
                let mut rt = QuickJsRuntime::new();
                rt.eval("globalThis.__platypus_reloading = true").unwrap();
                rt.load_bundle("scene.js", &code).unwrap();
                rt.eval("globalThis.__platypus_reloading = false").unwrap();
                let ok = rt
                    .call_global("__platypus_restore_state", &[saved])
                    .unwrap();
                assert_eq!(ok, Value::Bool(true));
                let (mut host, mut scene) = (ComponentHost::new(), ScriptScene::new());
                apply(&mut rt, &mut host, &mut scene);
                assert_eq!(host.len(), 1000);
                black_box((rt, host, scene))
            },
            BatchSize::PerIteration,
        )
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = reload
}
criterion_main!(benches);
