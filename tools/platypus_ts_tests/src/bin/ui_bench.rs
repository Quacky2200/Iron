// SPDX-License-Identifier: Apache-2.0

//! `ui_bench [rows]`: the same UI app (a list of keyed rows: mount, change one row, change every
//! row, reverse) rendered two ways into a real `UiTree`:
//! - React 19 + `@platypus/react` running in QuickJS (the engine today): the script builds the
//!   commit, Rust decodes and applies it;
//! - the compiled TSX (`bench/ui_perf.ts`) rendered by the native reconciler (`platypus_ui_native`).
//!
//! Both include applying the ops to the tree; the native time includes building the descriptions
//! in compiled code and the diff.
use platypus_core::Value;
use platypus_script::{QuickJsRuntime, ScriptRuntime};
use platypus_ts_rt::link_anchor;
use platypus_ts_tests::uip::*;
use platypus_ui::{ROOT, UiTree};
use platypus_ui_native::Runtime;
use std::process::Command;
use std::time::Instant;

const PHASES: [&str; 4] = ["mount", "change one row", "change every row", "reverse"];

/// (rev, tick, all) of each phase.
const STEPS: [(bool, f64, bool); 4] = [
    (false, 0.0, false),
    (false, 1.0, false),
    (false, 2.0, true),
    (true, 2.0, true),
];

fn react_run(bundle: &str, n: usize) -> ([f64; 4], usize) {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle("ui_perf", bundle).expect("bundle");
    let mut tree = UiTree::new(1000.0, 1000.0);
    let mut times = [0.0; 4];
    let mut ops = 0;
    for (i, (rev, tick, all)) in STEPS.iter().enumerate() {
        let t = Instant::now();
        rt.call_global(
            "__uiStep",
            &[
                Value::Number(n as f64),
                Value::Bool(*rev),
                Value::Number(*tick),
                Value::Bool(*all),
            ],
        )
        .expect("step");
        for c in rt.drain_commits() {
            ops += c.ops.len();
            let errors = tree.apply_commit(&c);
            assert!(errors.is_empty(), "{errors:?}");
        }
        times[i] = t.elapsed().as_secs_f64() * 1000.0;
    }
    (times, ops)
}

fn native_run(n: usize) -> ([f64; 4], usize) {
    let mut tree = UiTree::new(1000.0, 1000.0);
    let mut rt = Runtime::new(1 << 30);
    let root = rt.create_root(ROOT);
    let mut times = [0.0; 4];
    for (i, (rev, tick, all)) in STEPS.iter().enumerate() {
        let t = Instant::now();
        let h = unsafe { ts_uip_perfApp(n as f64, *rev, *tick, *all) };
        let el = platypus_ts_ui::element_from(h);
        let errors = rt.render(root, el, &mut tree);
        assert!(errors.is_empty(), "{errors:?}");
        times[i] = t.elapsed().as_secs_f64() * 1000.0;
    }
    (times, rt.ops_applied())
}

fn main() {
    assert_ne!(link_anchor(), 0);
    assert_ne!(platypus_ts_ui::link_anchor(), 0);
    platypus_ts_tests::gc_mode_from_env();
    unsafe { ts_uip_init() };
    let rows: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // React twice: a production bundle (what ships) and a development one (what `bun run dev` runs).
    let bundle_for = |mode: &str| {
        let path = std::env::temp_dir().join(format!("platypus_ts_ui_perf_{mode}.js"));
        let st = Command::new("bun")
            .arg("build")
            .arg(dir.join("../../js/react/test/perf_entry.tsx"))
            .args(["--format=iife", "--target=browser"])
            .arg(format!("--define=process.env.NODE_ENV=\"{mode}\""))
            .arg("--outfile")
            .arg(&path)
            .status()
            .expect("bun build");
        assert!(st.success());
        std::fs::read_to_string(&path).unwrap()
    };
    let prod = bundle_for("production");
    let dev = bundle_for("development");
    println!("a list of {rows} keyed rows rendered into a real UiTree (best of 7, ms):");
    let best = |f: &mut dyn FnMut() -> ([f64; 4], usize)| {
        let mut best = [f64::MAX; 4];
        let mut ops = 0;
        for _ in 0..7 {
            let (t, o) = f();
            ops = o;
            for i in 0..4 {
                best[i] = best[i].min(t[i]);
            }
        }
        (best, ops)
    };
    let (r, rops) = best(&mut || react_run(&prod, rows));
    let (d, _) = best(&mut || react_run(&dev, rows));
    let (n, nops) = best(&mut || native_run(rows));
    println!(
        "{:>18} {:>14} {:>14} {:>11} {:>8}",
        "", "React prod/QJS", "React dev/QJS", "native TSX", "prod/nat"
    );
    for i in 0..4 {
        println!(
            "{:>18} {:>14.3} {:>14.3} {:>11.3} {:>7.1}x",
            PHASES[i],
            r[i],
            d[i],
            n[i],
            r[i] / n[i]
        );
    }
    println!(
        "{:>18} {:>14} {:>14} {:>11}",
        "ops applied", rops, rops, nops
    );
}
