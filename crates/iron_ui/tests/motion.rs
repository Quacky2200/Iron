//! Transitions (M3-ANI-02): started by stylesheet and inline changes, timed with
//! delays and easing, reversed mid-way, cancelled, and reported to scripts.

mod common;

use common::{Dom, n, op, px};
use iron_core::{Commit, NodeId, Value};
use iron_ui::opcodes::SET_STYLE;
use iron_ui::{ElementState, Input, Interaction, Rgba, UiEvent, UiTree};

fn set_style(tree: &mut UiTree, id: u32, prop: &str, value: Value) {
    let errors = tree.apply_commit(&Commit {
        ops: vec![op(SET_STYLE, &[n(id), prop.into(), value])],
    });
    assert!(errors.is_empty(), "{errors:?}");
}

fn at(tree: &mut UiTree, ms: f64) {
    tree.set_time(ms);
    tree.layout_if_needed();
}

fn opacity(tree: &UiTree, id: u32) -> f32 {
    tree.visual(NodeId(id)).unwrap().opacity
}

fn hover(tree: &mut UiTree, id: u32, on: bool) {
    tree.set_state(
        NodeId(id),
        ElementState {
            hover: on,
            ..ElementState::default()
        },
    );
}

// M3-ANI-02
#[test]
fn hover_rules_transition_over_time() {
    let mut d = Dom::new();
    let b = d.add(0, "box", &[]);
    d.set_class(b, "b");
    let mut tree =
        d.build(".b { opacity: 1; transition: opacity 1s linear } .b:hover { opacity: 0 }");
    at(&mut tree, 0.0);
    assert_eq!(opacity(&tree, b), 1.0);
    hover(&mut tree, b, true);
    at(&mut tree, 0.0);
    assert_eq!(opacity(&tree, b), 1.0, "starts from the old value");
    at(&mut tree, 250.0);
    assert!(
        (opacity(&tree, b) - 0.75).abs() < 1e-3,
        "{}",
        opacity(&tree, b)
    );
    assert!(tree.is_animating());
    at(&mut tree, 1000.0);
    assert_eq!(opacity(&tree, b), 0.0);
    assert!(!tree.is_animating(), "finished");
    let kinds: Vec<&str> = tree
        .take_motion_events()
        .iter()
        .map(|e| e.kind.name())
        .collect();
    assert_eq!(kinds, ["transitionrun", "transitionstart", "transitionend"]);
}

// M3-ANI-02
#[test]
fn reversing_midway_starts_from_the_current_value() {
    let mut d = Dom::new();
    let b = d.add(0, "box", &[]);
    d.set_class(b, "b");
    let mut tree =
        d.build(".b { opacity: 1; transition: opacity 1s linear } .b:hover { opacity: 0 }");
    at(&mut tree, 0.0);
    hover(&mut tree, b, true);
    at(&mut tree, 0.0);
    at(&mut tree, 400.0);
    assert!((opacity(&tree, b) - 0.6).abs() < 1e-3);
    hover(&mut tree, b, false);
    at(&mut tree, 400.0);
    assert!((opacity(&tree, b) - 0.6).abs() < 1e-3, "no jump");
    at(&mut tree, 900.0);
    assert!(
        (opacity(&tree, b) - 0.8).abs() < 1e-3,
        "{}",
        opacity(&tree, b)
    );
    let kinds: Vec<&str> = tree
        .take_motion_events()
        .iter()
        .map(|e| e.kind.name())
        .collect();
    assert!(kinds.contains(&"transitioncancel"), "{kinds:?}");
}

// M3-ANI-02
#[test]
fn inline_styles_transition_without_stylesheets() {
    let mut d = Dom::new();
    let b = d.add(
        0,
        "box",
        &[
            ("width", px(100.0)),
            ("height", px(10.0)),
            ("transition", "width 1s linear 0.5s".into()),
        ],
    );
    let mut tree = d.build("");
    at(&mut tree, 0.0);
    set_style(&mut tree, b, "width", px(200.0));
    at(&mut tree, 0.0);
    assert_eq!(tree.rect(NodeId(b)).unwrap().width, 100.0);
    at(&mut tree, 400.0);
    assert_eq!(
        tree.rect(NodeId(b)).unwrap().width,
        100.0,
        "still in its delay"
    );
    at(&mut tree, 1000.0);
    assert_eq!(tree.rect(NodeId(b)).unwrap().width, 150.0);
    at(&mut tree, 1500.0);
    assert_eq!(tree.rect(NodeId(b)).unwrap().width, 200.0);
}

// M3-ANI-02
#[test]
fn colours_transforms_and_easing() {
    let mut d = Dom::new();
    let b = d.add(
        0,
        "box",
        &[
            ("backgroundColor", "red".into()),
            ("transform", "none".into()),
            (
                "transition",
                "background-color 1s linear, transform 1s ease-in".into(),
            ),
        ],
    );
    let mut tree = d.build("");
    at(&mut tree, 0.0);
    set_style(&mut tree, b, "backgroundColor", "blue".into());
    set_style(&mut tree, b, "transform", "translateX(100px)".into());
    at(&mut tree, 0.0);
    at(&mut tree, 500.0);
    let v = tree.visual(NodeId(b)).unwrap();
    assert_eq!(v.background, Rgba::new(0.5, 0.0, 0.5, 1.0));
    assert!(v.transform.is_some(), "a transform mid-way");
    // ease-in at half time is well short of half way: probe through hit testing.
    let r = tree.rect(NodeId(b)).unwrap();
    let local = tree.to_local(NodeId(b), r.x + 40.0, r.y + 1.0).unwrap();
    assert!(local.0 > r.x + 1.0, "moved right a little: {local:?}");
    assert!(local.0 < r.x + 35.0, "less than halfway: {local:?}");
}

// M3-ANI-02
#[test]
fn only_listed_properties_transition() {
    let mut d = Dom::new();
    let b = d.add(
        0,
        "box",
        &[
            ("opacity", px(1.0)),
            ("width", px(10.0)),
            ("transition", "opacity 1s".into()),
        ],
    );
    let mut tree = d.build("");
    at(&mut tree, 0.0);
    set_style(&mut tree, b, "width", px(50.0));
    at(&mut tree, 0.0);
    assert_eq!(tree.rect(NodeId(b)).unwrap().width, 50.0, "width snaps");
    set_style(&mut tree, b, "opacity", px(0.0));
    at(&mut tree, 0.0);
    at(&mut tree, 100.0);
    assert!(opacity(&tree, b) > 0.5);
    // Dropping the transition finishes it at once.
    set_style(&mut tree, b, "transition", "none".into());
    at(&mut tree, 150.0);
    assert_eq!(opacity(&tree, b), 0.0);
}

// M3-ANI-02
#[test]
fn transition_events_reach_scripts_with_the_property() {
    let mut d = Dom::new();
    let b = d.add(
        0,
        "box",
        &[("opacity", px(1.0)), ("transition", "opacity 100ms".into())],
    );
    let mut tree = d.build("");
    let mut ui = Interaction::new();
    at(&mut tree, 0.0);
    set_style(&mut tree, b, "opacity", px(0.5));
    let mut seen = Vec::new();
    for ms in [0.0, 50.0, 100.0] {
        at(&mut tree, ms);
        ui.handle(&mut tree, Input::Refresh, &mut |e: &UiEvent| {
            seen.push((
                e.kind.name(),
                e.path[0],
                e.value.clone().unwrap_or_default(),
                e.delta_x,
            ));
            false
        });
    }
    let ends: Vec<_> = seen.iter().filter(|e| e.0 == "transitionend").collect();
    assert_eq!(ends.len(), 1, "{seen:?}");
    assert_eq!((ends[0].1, ends[0].2.as_str()), (NodeId(b), "opacity"));
    assert!((ends[0].3 - 0.1).abs() < 1e-6, "elapsed seconds");
}
