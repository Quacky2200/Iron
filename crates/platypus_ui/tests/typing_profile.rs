// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Profiles typing into `examples/form` headlessly: the real bundle and stylesheet,
//! keystrokes through `Interaction` and QuickJS like the app, then commits and the
//! draw list. Ignored by default (needs `bun run build` and is slow in debug):
//! `cargo test --release -p platypus_ui --test typing_profile -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use platypus_core::Value;
use platypus_script::{QuickJsRuntime, ScriptRuntime};
use platypus_ui::{Input, Interaction, UiTree};

#[test]
#[ignore]
fn typing_into_the_form() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/form/build");
    let js = std::fs::read_to_string(root.join("game.js")).expect("run `bun run build` first");
    let css = std::fs::read_to_string(root.join("game.css")).unwrap();

    let t = Instant::now();
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("game.js", &js).unwrap();
    let mut tree = UiTree::with_system_fonts(1280.0, 720.0);
    tree.add_stylesheet(&css);
    for c in rt.drain_commits() {
        tree.apply_commit(&c);
    }
    for op in rt.drain_ops() {
        if op.name == "focus" {
            let id = platypus_core::NodeId(op.args[0].as_f64().unwrap() as u32);
            Interaction::new().handle(&mut tree, Input::Focus(id), &mut |_| false);
        }
    }
    tree.draw_list();
    println!("startup: {:?}", t.elapsed());

    let mut ui = Interaction::new();
    // Focus the name field (the first text field).
    let name = (1..2000)
        .map(platypus_core::NodeId)
        .find(|id| tree.is_editor(*id))
        .unwrap();
    ui.handle(&mut tree, Input::Focus(name), &mut |_| false);

    let (mut dispatch_t, mut commit_t, mut draw_t) =
        (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut worst = Duration::ZERO;
    let text = "Ada Lovelace was here";
    for c in text.chars() {
        let start = Instant::now();
        let rt_ref = &mut rt;
        ui.handle(
            &mut tree,
            Input::KeyDown {
                key: c.to_string(),
                code: String::new(),
                repeat: false,
                text: Some(c.to_string()),
            },
            &mut |e| {
                rt_ref
                    .call_global("__platypus_ui_event", &e.script_args())
                    .unwrap()
                    == Value::Bool(true)
            },
        );
        let a = Instant::now();
        for commit in rt.drain_commits() {
            tree.apply_commit(&commit);
        }
        ui.handle(&mut tree, Input::Refresh, &mut |_| false);
        let b = Instant::now();
        let cmds = tree.draw_list();
        let end = Instant::now();
        dispatch_t += a - start;
        commit_t += b - a;
        draw_t += end - b;
        worst = worst.max(end - start);
        assert!(!cmds.is_empty());
    }
    let n = text.chars().count() as u32;
    println!(
        "per key: dispatch+react {:?}, apply+refresh {:?}, layout+draw list {:?}; worst key {:?}",
        dispatch_t / n,
        commit_t / n,
        draw_t / n,
        worst
    );
    println!("text builds: {}", tree.text_build_count());
    assert_eq!(tree.editor_value(name).unwrap(), text);

    // An idle frame (caret blink only).
    let t = Instant::now();
    tree.set_time(10_000.0);
    tree.draw_list();
    println!("idle frame draw list: {:?}", t.elapsed());
}
