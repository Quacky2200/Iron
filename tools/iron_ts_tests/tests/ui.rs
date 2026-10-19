//! Compiled TSX components rendered by the native reconciler (`bench/ui.ts`): JSX, hooks, keyed
//! lists and events written as ordinary TypeScript, run through `iron_ts_ui` and `iron_ui_native`
//! into a real `UiTree`. Each test runs normally and with a collection at every allocation.
//! IDs are in TESTS.md (M9-TS-575..).
use iron_core::NodeId;
use iron_ts_rt::{link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::uic::*;
use iron_ui::{ROOT, UiTree};
use iron_ui_native::{Event, Runtime};
use std::sync::Mutex;

/// The scripts' globals are shared; one test at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn in_every_gc_mode(f: impl Fn()) {
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_ui::link_anchor(), 0);
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { ts_uic_init() };
    f();
    ts_rt_gc_stress(true);
    unsafe { ts_uic_init() };
    f();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    unsafe { ts_uic_init() };
    f();
    ts_rt_gc_enable(true);
}

fn dump(tree: &UiTree, id: NodeId) -> String {
    let kind = tree.kind(id).unwrap_or("?");
    if kind == "#text" {
        return format!("text:{}", tree.text(id).unwrap_or(""));
    }
    let kids: Vec<String> = tree.children(id).iter().map(|c| dump(tree, *c)).collect();
    format!("{kind}({})", kids.join(","))
}

fn no_errors(errors: Vec<iron_ui::UiError>) {
    assert!(errors.is_empty(), "{errors:?}");
    let script = iron_ts_ui::take_errors();
    assert!(script.is_empty(), "{script:?}");
}

struct Ui {
    tree: UiTree,
    rt: Runtime,
    root: iron_ui_native::RootId,
}

impl Ui {
    fn new() -> Ui {
        let mut rt = Runtime::new(1 << 30);
        let root = rt.create_root(ROOT);
        Ui {
            tree: UiTree::new(300.0, 300.0),
            rt,
            root,
        }
    }

    fn show(&mut self, handle: u64) {
        let el = iron_ts_ui::element_from(handle);
        no_errors(self.rt.render(self.root, el, &mut self.tree));
    }

    fn click(&mut self, node: NodeId) {
        let mut ev = Event::new("click", node);
        self.rt.dispatch(&mut ev, &mut self.tree);
        no_errors(Vec::new());
    }

    fn dump(&self) -> String {
        dump(&self.tree, ROOT)
    }
}

// M9-TS-575
//
// A component written as TSX: `useState`, a host element with a style and a click handler, text
// and a number as children. A click inside the compiled handler changes the state and the text.
#[test]
fn a_counter_component_renders_and_handles_a_click() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_counterApp() });
        assert_eq!(ui.dump(), "root(box(text(text:n=),text:3))");
        let boxn = ui.tree.children(ROOT)[0];
        ui.click(boxn);
        assert_eq!(ui.dump(), "root(box(text(text:n=),text:4))");
        ui.click(boxn);
        ui.click(boxn);
        assert_eq!(ui.dump(), "root(box(text(text:n=),text:6))");
        ui.tree.layout_if_needed();
        assert_eq!(ui.tree.rect(boxn).unwrap().width, 100.0);
    });
}

// M9-TS-576
//
// A keyed list from `items.map(...)`: reversing it moves the nodes, it does not recreate them.
#[test]
fn a_keyed_list_keeps_its_nodes_when_reordered() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_listApp(4.0, false) });
        let list = ui.tree.children(ROOT)[0];
        let before: Vec<NodeId> = ui.tree.children(list).to_vec();
        assert_eq!(
            ui.dump(),
            "root(box(text(text:k0),text(text:k1),text(text:k2),text(text:k3)))"
        );
        ui.show(unsafe { ts_uic_listApp(4.0, true) });
        let after: Vec<NodeId> = ui.tree.children(list).to_vec();
        let want: Vec<NodeId> = before.iter().rev().copied().collect();
        assert_eq!(after, want);
        ui.show(unsafe { ts_uic_listApp(2.0, false) });
        assert_eq!(ui.dump(), "root(box(text(text:k0),text(text:k1)))");
    });
}

// M9-TS-577
//
// `useEffect` with dependencies and a cleanup: the log shows +1, then -1 +2 when the dependency
// changes, nothing when it does not, and the cleanup on unmount.
#[test]
fn effects_run_with_dependencies_and_clean_up() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_fxApp(1.0) });
        assert_eq!(unsafe { ts_uic_getLog() }, 11.0);
        ui.show(unsafe { ts_uic_fxApp(1.0) });
        assert_eq!(
            unsafe { ts_uic_getLog() },
            11.0,
            "same dependency: no rerun"
        );
        ui.show(unsafe { ts_uic_fxApp(2.0) });
        assert_eq!(unsafe { ts_uic_getLog() }, 112_112.0);
        no_errors(ui.rt.unmount_root(ui.root, &mut ui.tree));
        assert_eq!(unsafe { ts_uic_getLog() }, 11_211_222.0);
    });
}

// M9-TS-578
//
// `useMemo` with dependencies: recomputed only when they change.
#[test]
fn use_memo_recomputes_only_when_its_dependencies_change() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_memoApp(3.0, 0.0) });
        assert_eq!(ui.dump(), "root(box(text:9,text:0))");
        ui.show(unsafe { ts_uic_memoApp(3.0, 5.0) });
        assert_eq!(ui.dump(), "root(box(text:9,text:5))");
        assert_eq!(unsafe { ts_uic_memoCount() }, 1.0);
        ui.show(unsafe { ts_uic_memoApp(4.0, 5.0) });
        assert_eq!(ui.dump(), "root(box(text:16,text:5))");
        assert_eq!(unsafe { ts_uic_memoCount() }, 2.0);
    });
}

// M9-TS-579
//
// Boolean and string state and `useRef`: a click flips both, the ref counts renders and survives.
#[test]
fn bool_and_string_state_and_use_ref_persist_across_renders() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_toggleApp() });
        assert_eq!(ui.dump(), "root(box(text:off,text:1))");
        let b = ui.tree.children(ROOT)[0];
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text:on,text:2))");
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text:off,text:3))");
    });
}

// M9-TS-580
//
// Context: a consumer reads the nearest provider's value, the default without one, and follows
// the provider when it changes.
#[test]
fn context_reaches_consumers_and_follows_the_provider() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_ctxNone() });
        assert_eq!(ui.dump(), "root(text(text:light))");
        ui.show(unsafe { ts_uic_ctxApp(true) });
        assert_eq!(ui.dump(), "root(text(text:dark))");
        ui.show(unsafe { ts_uic_ctxApp(false) });
        assert_eq!(ui.dump(), "root(text(text:light))");
    });
}

// M9-TS-581
//
// `ref={r}` writes the node into the ref when the element attaches (the effect sees it) and
// clears it on unmount.
#[test]
fn a_node_ref_is_set_when_the_element_attaches() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_refApp() });
        let node = ui.tree.children(ROOT)[0];
        assert_eq!(unsafe { ts_uic_getSeenNode() }, f64::from(node.raw()));
    });
}

// M9-TS-582
//
// `useImperativeHandle`: a child fills the ref its parent passed; the parent effect (which runs
// after the child's) reads it.
#[test]
fn use_imperative_handle_fills_the_ref_before_the_parents_effect() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_handleApp() });
        assert_eq!(unsafe { ts_uic_getHandleSeen() }, 42.0);
    });
}

// M9-TS-583
//
// `useSyncExternalStore`: the component shows the store, subscribes once, re-renders when the
// store notifies, and unsubscribes on unmount.
#[test]
fn use_sync_external_store_follows_a_store() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_storeApp() });
        assert_eq!(ui.dump(), "root(box(text:5))");
        assert_eq!(unsafe { ts_uic_listenerCount() }, 1.0);
        unsafe { ts_uic_bumpStore() };
        no_errors(ui.rt.flush(&mut ui.tree));
        assert_eq!(ui.dump(), "root(box(text:6))");
        assert_eq!(
            unsafe { ts_uic_listenerCount() },
            1.0,
            "not subscribed again"
        );
        no_errors(ui.rt.unmount_root(ui.root, &mut ui.tree));
        assert_eq!(unsafe { ts_uic_listenerCount() }, 0.0);
    });
}

// M9-TS-584
//
// `memo`: a child whose props are equal is not rendered when its parent renders again; a changed
// prop renders it.
#[test]
fn memo_skips_a_child_with_equal_props() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_memoParentApp(1.0, 0.0) });
        assert_eq!(ui.dump(), "root(box(text:0,box(text:1)))");
        assert_eq!(unsafe { ts_uic_leafRenders() }, 1.0);
        ui.show(unsafe { ts_uic_memoParentApp(1.0, 1.0) });
        assert_eq!(ui.dump(), "root(box(text:1,box(text:1)))");
        assert_eq!(unsafe { ts_uic_leafRenders() }, 1.0, "skipped");
        ui.show(unsafe { ts_uic_memoParentApp(2.0, 1.0) });
        assert_eq!(ui.dump(), "root(box(text:1,box(text:2)))");
        assert_eq!(unsafe { ts_uic_leafRenders() }, 2.0);
    });
}

// M9-TS-585
//
// `forwardRef`: the component gets the parent's ref as its second parameter and fills it with
// `useImperativeHandle`.
#[test]
fn forward_ref_passes_the_ref_to_the_component() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_fwdApp() });
        assert_eq!(ui.dump(), "root(text(text:hello))");
        assert_eq!(unsafe { ts_uic_getFwdSeen() }, 5.0);
    });
}

// M9-TS-586
//
// `useReducer`: dispatch applies the reducer to the latest state, so two dispatches in one
// handler both count.
#[test]
fn use_reducer_applies_actions_to_the_latest_state() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_reducerApp() });
        assert_eq!(ui.dump(), "root(box(text:10))");
        let b = ui.tree.children(ROOT)[0];
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text:11))");
        let mut ev = Event::new("pointerdown", b);
        ui.rt.dispatch(&mut ev, &mut ui.tree);
        assert_eq!(ui.dump(), "root(box(text:9))");
    });
}

/// The component table the compiler exported: name -> (id, hooks).
fn signatures() -> std::collections::HashMap<String, (u32, String)> {
    let text = unsafe { iron_api::script_str(ts_uic___uiSignatures() as *const u8) }.into_owned();
    text.split(';')
        .map(|e| {
            let mut p = e.splitn(3, ':');
            let name = p.next().unwrap().to_string();
            let id: u32 = p.next().unwrap().parse().unwrap();
            (name, (id, p.next().unwrap().to_string()))
        })
        .collect()
}

// M9-TS-587
//
// The compiler exports each component's stable id and its ordered hook list.
#[test]
fn the_compiler_exports_component_ids_and_hook_lists() {
    in_every_gc_mode(|| {
        let sig = signatures();
        assert_eq!(sig["Counter"].1, "useState");
        assert_eq!(sig["Fx"].1, "useEffect");
        assert_eq!(sig["Toggle"].1, "useState,useState,useRef");
        assert_eq!(sig["Memo"].1, "useMemo");
        assert_eq!(sig["Reader"].1, "useContext");
        assert_ne!(sig["Counter"].0, sig["Fx"].0);
    });
}

// M9-TS-588
//
// Hot reload: rendering the new code's element over the old tree keeps the state of components
// whose hook list is unchanged and starts over those in the remount list.
#[test]
fn refresh_keeps_state_unless_the_component_is_remounted() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_counterApp() });
        let b = ui.tree.children(ROOT)[0];
        ui.click(b);
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text(text:n=),text:5))");
        let id = signatures()["Counter"].0;
        let el = iron_ts_ui::element_from(unsafe { ts_uic_counterApp() });
        no_errors(ui.rt.refresh(ui.root, el, &[], &mut ui.tree));
        assert_eq!(ui.dump(), "root(box(text(text:n=),text:5))", "state kept");
        let el = iron_ts_ui::element_from(unsafe { ts_uic_counterApp() });
        no_errors(ui.rt.refresh(ui.root, el, &[id], &mut ui.tree));
        assert_eq!(
            ui.dump(),
            "root(box(text(text:n=),text:3))",
            "remounted: initial state"
        );
    });
}

// M9-TS-589
//
// The updater form of a setter: two updaters in one handler both apply to the latest state.
#[test]
fn updater_setters_apply_to_the_latest_state() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_plusApp() });
        assert_eq!(ui.dump(), "root(box(text:0))");
        let b = ui.tree.children(ROOT)[0];
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text:11))");
        ui.click(b);
        assert_eq!(ui.dump(), "root(box(text:22))");
    });
}

// M9-TS-590
//
// `style={obj}` with a typed object: its fields are the style, an optional field left out is not
// set, and a field that goes away is reset.
#[test]
fn a_dynamic_style_object_sets_its_fields() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_styledApp(40.0, true) });
        let b = ui.tree.children(ROOT)[0];
        ui.tree.layout_if_needed();
        assert_eq!(ui.tree.rect(b).unwrap().width, 40.0);
        ui.show(unsafe { ts_uic_styledApp(70.0, false) });
        ui.tree.layout_if_needed();
        assert_eq!(ui.tree.rect(b).unwrap().width, 70.0);
    });
}

// M9-TS-591
//
// `{...obj}` on a host element sets the object's fields as attributes; on a component it spreads
// into the props (a later attribute wins).
#[test]
fn spread_attributes_and_props() {
    in_every_gc_mode(|| {
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_spreadApp() });
        let b = ui.tree.children(ROOT)[0];
        let a = ui.tree.attrs(b).unwrap();
        assert_eq!(a.id.as_deref(), Some("main"));
        assert_eq!(a.classes, ["a", "b"]);
        assert_eq!(a.tab_index, Some(3));
        let mut ui = Ui::new();
        ui.show(unsafe { ts_uic_outerApp() });
        assert_eq!(ui.dump(), "root(box(text:1,text:9))");
    });
}
