use glam::{Quat, Vec3};
use iron_scene::{Scene, Transform};

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-4
}

// M5-OBJ-01
#[test]
fn objects_nest_and_destroying_takes_the_subtree() {
    let mut s = Scene::new();
    let root = s.create("root");
    let arm = s.create("arm");
    let hand = s.create("hand");
    let other = s.create("other");
    assert!(s.set_parent(arm, Some(root), false));
    assert!(s.set_parent(hand, Some(arm), false));
    assert_eq!(s.roots(), [root, other]);
    assert_eq!(s.walk().collect::<Vec<_>>(), [root, arm, hand, other]);
    assert_eq!(s.find("hand"), Some(hand));

    // No cycles.
    assert!(!s.set_parent(root, Some(hand), false));
    assert!(!s.set_parent(root, Some(root), false));

    assert_eq!(s.destroy(root), [root, arm, hand]);
    assert_eq!(s.len(), 1);
    assert!(!s.contains(hand));
    // A reused slot gets a new generation, so the stale id misses.
    let again = s.create("again");
    assert_eq!(again.index, hand.index);
    assert_eq!(again.generation, hand.generation + 1);
    assert!(!s.contains(hand) && s.contains(again));
    assert_eq!(s.name(root), None);
    assert_eq!(s.destroy(root), []);
}

// M5-OBJ-02
#[test]
fn world_transforms_follow_the_hierarchy() {
    let mut s = Scene::new();
    let parent = s.create("parent");
    let child = s.create("child");
    s.set_parent(child, Some(parent), false);
    s.set_local(
        parent,
        Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::splat(2.0),
        },
    );
    s.set_local(child, Transform::from_position(Vec3::new(1.0, 0.0, 0.0)));
    // +x rotated 90° about y is -z, scaled by 2.
    assert!(close(s.world(child).position, Vec3::new(10.0, 0.0, -2.0)));
    s.update_world();
    assert!(close(s.world(child).position, Vec3::new(10.0, 0.0, -2.0)));

    // Moving the parent moves the child, cached or not.
    s.set_local(parent, Transform::from_position(Vec3::new(0.0, 5.0, 0.0)));
    assert!(close(s.world(child).position, Vec3::new(1.0, 5.0, 0.0)));

    // Re-parenting keeping the world position.
    s.set_parent(child, None, true);
    assert!(close(
        s.local(child).unwrap().position,
        Vec3::new(1.0, 5.0, 0.0)
    ));
    s.set_parent(child, Some(parent), true);
    assert!(close(
        s.local(child).unwrap().position,
        Vec3::new(1.0, 0.0, 0.0)
    ));
    assert!(close(s.world(child).position, Vec3::new(1.0, 5.0, 0.0)));

    // Setting a world position under a rotated parent.
    s.set_local(
        parent,
        Transform {
            rotation: Quat::from_rotation_z(1.0),
            ..Transform::from_position(Vec3::new(3.0, 0.0, 0.0))
        },
    );
    s.set_world(child, Vec3::new(0.0, 0.0, 7.0), Quat::IDENTITY);
    let w = s.world(child);
    assert!(close(w.position, Vec3::new(0.0, 0.0, 7.0)), "{w:?}");
    assert!(w.rotation.angle_between(Quat::IDENTITY) < 1e-4);
}

// M5-OBJ-03
#[test]
fn moved_objects_and_active_state() {
    let mut s = Scene::new();
    let a = s.create("a");
    let b = s.create("b");
    s.set_parent(b, Some(a), false);
    let c = s.create("c");
    s.take_moved();

    s.set_local(a, Transform::from_position(Vec3::X));
    let mut moved = s.take_moved();
    moved.sort();
    assert_eq!(moved, [a, b], "children move with their parent");
    // Setting the same transform again isn't a move.
    s.set_local(a, Transform::from_position(Vec3::X));
    assert!(s.take_moved().is_empty());
    s.set_local(c, Transform::from_position(Vec3::Y));
    assert_eq!(s.take_moved(), [c]);

    s.set_enabled(a, false);
    assert!(
        s.enabled(b) && !s.active(b),
        "a disabled parent turns off its subtree"
    );
    s.set_enabled(a, true);
    assert!(s.active(b));
}

// M5-OBJ-09
#[test]
fn tree_versions_change_with_names_and_links_below_but_not_with_moves() {
    let mut s = Scene::new();
    let root = s.create("root");
    let arm = s.create("arm");
    s.set_parent(arm, Some(root), false);
    let hand = s.create("hand");
    s.set_parent(hand, Some(arm), false);
    let other = s.create("other");
    let mut last = s.tree_version(root);
    let mut changed = |s: &Scene| {
        let v = s.tree_version(root);
        let c = v != last;
        last = v;
        c
    };
    s.set_local(hand, Transform::from_position(Vec3::X));
    s.set_enabled(hand, false);
    assert!(!changed(&s), "moves and flags keep the version");
    s.set_name(hand, "hand");
    assert!(!changed(&s), "the same name is no change");
    s.set_name(hand, "fist");
    assert!(changed(&s), "a rename below");
    let child = s.create("child");
    assert!(!changed(&s), "a new root elsewhere");
    s.set_parent(child, Some(hand), false);
    assert!(changed(&s), "a new child below");
    s.set_parent(child, Some(other), false);
    assert!(changed(&s), "a child moved away");
    s.set_parent(other, None, false);
    s.set_name(other, "elsewhere");
    assert!(!changed(&s), "changes in another tree");
    s.destroy(hand);
    assert!(changed(&s), "an object destroyed below");
    assert_eq!(s.tree_version(hand), 0);
}
