//! A React render error must not take the engine down (M2-LAY-06): React unmounts the
//! root after an uncaught error, and the UI tree applies that commit and lays out.

use iron_script::conformance::capture_logs;
use iron_script::{QuickJsRuntime, ScriptRuntime};
use iron_ui::UiTree;

const RENDER_ERROR: &str = include_str!("../../../tests/fixtures/render_error.js");

// M2-LAY-06
#[test]
fn a_render_error_unmounts_the_ui_without_breaking_the_tree() {
    let mut rt = QuickJsRuntime::new();
    let _logs = capture_logs(&mut rt);
    rt.load_bundle("render_error.js", RENDER_ERROR).unwrap();
    let mut tree = UiTree::new(400.0, 300.0);
    let mut apply = |rt: &mut QuickJsRuntime, tree: &mut UiTree| {
        for commit in rt.drain_commits() {
            for e in tree.apply_commit(&commit) {
                eprintln!("commit error: {e}");
            }
        }
        tree.layout_if_needed();
        let _ = tree.draw_list();
    };
    apply(&mut rt, &mut tree);
    rt.eval("__bump()").unwrap();
    for t in 1..10 {
        rt.tick(f64::from(t) * 100.0);
        apply(&mut rt, &mut tree);
    }
}
