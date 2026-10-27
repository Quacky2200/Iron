// SPDX-License-Identifier: Apache-2.0

use glam::{Quat, Vec3};
use platypus_scene::{Scene, Transform};

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

/// The moved list as `mark_moved` kept it before it stopped at listed, stale objects: the whole
/// subtree walked every time, each object added once.
fn mark_every_time(
    s: &Scene,
    id: platypus_scene::ObjectId,
    moved: &mut Vec<platypus_scene::ObjectId>,
) {
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        if !s.contains(id) {
            continue;
        }
        stack.extend(s.children(id).iter().copied());
        if !moved.contains(&id) {
            moved.push(id);
        }
    }
}

/// The world matrix from the locals up the chain, with no cache.
fn uncached(s: &Scene, id: platypus_scene::ObjectId) -> glam::Affine3A {
    let mut m = glam::Affine3A::IDENTITY;
    let mut at = Some(id);
    while let Some(a) = at {
        m = s.local(a).unwrap().to_affine() * m;
        at = s.parent(a);
    }
    m
}

// M5-OBJ-10
#[test]
fn moves_in_deep_hierarchies_list_the_same_objects_and_keep_world_matrices_right() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = |n: usize| {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        (seed.wrapping_mul(0x2545_f491_4f6c_dd1d) % n as u64) as usize
    };
    let mut s = Scene::new();
    let mut expected = Vec::new();
    // Chains 40 deep with branches.
    let mut all = Vec::new();
    for _ in 0..3 {
        let mut parent = s.create("root");
        expected.push(parent);
        all.push(parent);
        for d in 0..40 {
            let go = s.create(&format!("bone{d}"));
            expected.push(go);
            s.set_parent(go, Some(parent), false);
            mark_every_time(&s, go, &mut expected);
            all.push(go);
            if d % 3 == 0 {
                let leaf = s.create("leaf");
                expected.push(leaf);
                s.set_parent(leaf, Some(go), false);
                mark_every_time(&s, leaf, &mut expected);
                all.push(leaf);
            }
            parent = go;
        }
    }
    for step in 0..3000 {
        all.retain(|o| s.contains(*o));
        let go = all[next(all.len())];
        match next(20) {
            0 => {
                assert_eq!(s.take_moved(), std::mem::take(&mut expected), "step {step}");
            }
            1 => s.update_world(),
            2 => {
                let to = all[next(all.len())];
                if s.set_parent(go, Some(to), next(2) == 0) {
                    mark_every_time(&s, go, &mut expected);
                }
            }
            3 if all.len() > 20 => {
                let gone = s.destroy(go);
                expected.retain(|m| !gone.contains(m));
            }
            4 => {
                let c = s.create("new");
                expected.push(c);
                all.push(c);
            }
            _ => {
                let t = Transform::from_position(Vec3::new(step as f32, 1.0, -0.5));
                s.set_local(go, t);
                mark_every_time(&s, go, &mut expected);
            }
        }
        if step % 97 == 0 {
            for o in &all {
                if s.contains(*o) {
                    let (a, b) = (s.world_matrix(*o), uncached(&s, *o));
                    assert!(a.abs_diff_eq(b, 1e-3), "step {step}");
                }
            }
        }
    }
    assert_eq!(s.take_moved(), expected);
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

// M5-OBJ-11
#[test]
fn moved_since_says_which_world_matrices_changed_after_an_update() {
    let mut s = Scene::new();
    let root = s.create("root");
    let arm = s.create("arm");
    s.set_parent(arm, Some(root), false);
    let hand = s.create("hand");
    s.set_parent(hand, Some(arm), false);
    let other = s.create("other");
    assert!(
        s.moved_since(hand, s.epoch()),
        "stale before the first update"
    );
    s.update_world();
    let built = s.epoch();
    for o in [root, arm, hand, other] {
        assert!(!s.moved_since(o, built));
    }
    // A move stamps the object and everything below it, not its parents or others.
    s.set_local(arm, Transform::from_position(Vec3::X));
    s.set_local(hand, Transform::from_position(Vec3::Y));
    s.update_world();
    assert!(s.moved_since(arm, built) && s.moved_since(hand, built));
    assert!(!s.moved_since(root, built) && !s.moved_since(other, built));
    let built = s.epoch();
    assert!(!s.moved_since(hand, built), "settled after the next update");
    // A parent's move reaches the children, even after a move of theirs this epoch.
    s.set_local(hand, Transform::from_position(Vec3::Z));
    s.set_local(root, Transform::from_position(Vec3::X));
    assert!(s.moved_since(hand, built), "stale: not updated yet");
    s.update_world();
    assert!(s.moved_since(arm, built) && s.moved_since(hand, built));
    let built = s.epoch();
    // Re-parenting moves; a destroyed object always counts as moved.
    s.set_parent(other, Some(hand), false);
    s.update_world();
    assert!(s.moved_since(other, built) && !s.moved_since(hand, built));
    s.destroy(other);
    assert!(s.moved_since(other, s.epoch()));
}
