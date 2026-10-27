// SPDX-License-Identifier: Apache-2.0

//! The parallel update against the update in turn.

use crate::animator::AnimatorPlayer;
use crate::clip::{Clip, ClipEvent, ClipId, World};
use crate::controller::{
    AnimParameter, BlendChild, BlendTree, Condition, ConditionMode, Controller, Motion, Node,
    ParameterType, Transition,
};
use crate::culling::{CullingMode, Visibility};
use crate::curve::{Curve, WrapMode};
use crate::player::{AnimationPlayer, FiredEvent, update_on};
use crate::pose::NoComponents;
use platypus_scene::{ObjectId, Scene, Transform};

/// A looping clip over a figure's bones: each bone turns and slides at its own pace.
fn clip(w: &mut World, name: &str, length: f64, bones: &[&str], event: bool) -> ClipId {
    let mut c = Clip::new(name);
    c.wrap_mode = WrapMode::Loop;
    let id = w.add_clip(c);
    for (i, bone) in bones.iter().enumerate() {
        let k = (i + 1) as f64;
        let x = w.add_curve(Curve::linear(0.0, 0.0, length, 0.3 * k));
        w.set_curve(id, bone, "Transform", "localPosition.x", Some(x));
        let a = (0.4 * k).sin() * 0.5;
        for (axis, v) in [("x", a), ("y", 0.0), ("z", 0.1 * k), ("w", 1.0)] {
            let mut c = Curve::linear(0.0, v, length * 0.5, -v + 0.2);
            c.add_key(crate::curve::Keyframe::new(length, v));
            let id2 = w.add_curve(c);
            w.set_curve(
                id,
                bone,
                "Transform",
                &format!("localRotation.{axis}"),
                Some(id2),
            );
        }
    }
    if event {
        w.add_event(
            id,
            ClipEvent {
                time: length * 0.5,
                function_name: format!("{name}Step"),
                parameter: None,
            },
        );
    }
    id
}

/// Seen by a rule of figure and frame.
struct Rule(u64);

impl Visibility for Rule {
    fn visible(&mut self, _: &Scene, root: ObjectId) -> bool {
        !(u64::from(root.index) + self.0 / 7).is_multiple_of(3)
    }
}

/// A crowd: `n` figures (a hips, spine and arm chain each) with animators blending a walk and a
/// run by a speed that changes, a trigger into a wave, every culling mode; and a lamp with a
/// legacy `Animation` among them.
fn crowd(n: usize) -> (World, Scene, Vec<ObjectId>) {
    let (mut w, mut scene) = (World::default(), Scene::new());
    let names = ["Hips", "Hips/Spine", "Hips/Spine/Arm"];
    let walk = clip(&mut w, "Walk", 1.1, &names, true);
    let run = clip(&mut w, "Run", 0.7, &names, true);
    let wave = clip(&mut w, "Wave", 1.6, &names[1..], false);
    let blend = w.graphs.add_blend(BlendTree {
        parameter: "speed".into(),
        children: vec![
            BlendChild {
                clip: walk,
                threshold: 0.0,
                time_scale: None,
            },
            BlendChild {
                clip: run,
                threshold: 1.0,
                time_scale: Some(1.2),
            },
        ],
    });
    let move_node = w.graphs.add_node(Node {
        name: "Move".into(),
        motion: Motion::Blend(blend),
        speed: None,
        speed_parameter: None,
    });
    let wave_node = w.graphs.add_node(Node {
        name: "Wave".into(),
        motion: Motion::Clip(wave),
        speed: Some(1.3),
        speed_parameter: None,
    });
    let speed = w.graphs.add_parameter(AnimParameter {
        name: "speed".into(),
        ty: ParameterType::Float,
        default: None,
    });
    let hello = w.graphs.add_parameter(AnimParameter {
        name: "hello".into(),
        ty: ParameterType::Trigger,
        default: None,
    });
    let when = w.graphs.add_condition(Condition {
        parameter: "hello".into(),
        mode: ConditionMode::If,
        threshold: None,
    });
    let to_wave = w.graphs.add_transition(Transition {
        from: "Move".into(),
        to: "Wave".into(),
        conditions: vec![when],
        has_exit_time: false,
        exit_time: None,
        duration: Some(0.3),
        offset: None,
    });
    let back = w.graphs.add_transition(Transition {
        from: "Wave".into(),
        to: "Move".into(),
        conditions: Vec::new(),
        has_exit_time: true,
        exit_time: Some(0.9),
        duration: Some(0.2),
        offset: Some(0.25),
    });
    let controller = w.graphs.add_controller(Controller {
        parameters: vec![speed, hello],
        nodes: vec![move_node, wave_node],
        transitions: vec![to_wave, back],
        default_state: None,
    });
    let street = scene.create("Street");
    let mut roots = Vec::new();
    for i in 0..n {
        let go = scene.create(&format!("Figure{i}"));
        scene.set_parent(go, Some(street), false);
        let hips = scene.create("Hips");
        scene.set_parent(hips, Some(go), false);
        let spine = scene.create("Spine");
        scene.set_parent(spine, Some(hips), false);
        let arm = scene.create("Arm");
        scene.set_parent(arm, Some(spine), false);
        let mut a = AnimatorPlayer::new(go);
        a.controller = Some(controller);
        a.culling_mode = [
            CullingMode::AlwaysAnimate,
            CullingMode::CullUpdateTransforms,
            CullingMode::CullCompletely,
        ][i % 3];
        a.speed = 0.8 + 0.05 * (i % 7) as f64;
        w.animators.push(a);
        roots.push(go);
        if i == n / 2 {
            // A lamp with a legacy Animation in the middle of the street.
            let lamp = scene.create("Lamp");
            scene.set_parent(lamp, Some(street), false);
            let swing = clip(&mut w, "Swing", 0.9, &[""], true);
            let mut p = AnimationPlayer::new(lamp);
            p.clip = Some(swing);
            w.players.push(p);
            roots.push(lamp);
        }
    }
    (w, scene, roots)
}

/// Every object's local transform, bit for bit.
fn locals(scene: &Scene) -> Vec<[u32; 10]> {
    scene
        .walk()
        .map(|o| {
            let t = scene.local(o).unwrap();
            let v = [
                t.position.x,
                t.position.y,
                t.position.z,
                t.rotation.x,
                t.rotation.y,
                t.rotation.z,
                t.rotation.w,
                t.scale.x,
                t.scale.y,
                t.scale.z,
            ];
            v.map(f32::to_bits)
        })
        .collect()
}

/// Runs `frames` frames on `threads` threads, the parameters changing as they go.
/// With `nest`, the second figure rides on the first one's arm (animated objects nested).
fn play(threads: usize, frames: u64, nest: bool) -> (Vec<Vec<[u32; 10]>>, Vec<Vec<FiredEvent>>) {
    let (mut w, mut scene, roots) = crowd(64);
    if nest {
        let arm = crate::pose::resolve(&scene, roots[0], "Hips/Spine/Arm").unwrap();
        scene.set_parent(roots[1], Some(arm), false);
    }
    let (mut poses, mut events) = (Vec::new(), Vec::new());
    for f in 0..frames {
        for (i, a) in w.animators.iter_mut().enumerate() {
            a.set_float(
                "speed",
                ((f as f64 * 0.05 + i as f64 * 0.3).sin() + 1.0) * 0.5,
            );
            if (f + i as u64).is_multiple_of(41) {
                a.set_trigger("hello");
            }
        }
        if f == 30 {
            // The street moves (not animated): poses stay local.
            scene.set_local(scene.roots()[0], Transform::from_position(glam::Vec3::X));
        }
        let dt = 0.011 + 0.004 * (f % 5) as f64;
        events.push(update_on(
            &mut w,
            &mut scene,
            &mut NoComponents,
            dt,
            &mut Rule(f),
            threads,
        ));
        poses.push(locals(&scene));
    }
    (poses, events)
}

// M5-ANI-17
#[test]
fn animators_posed_in_parallel_match_the_update_in_turn_bit_for_bit() {
    // Apart (posed in parallel), and with one figure on another's arm (the frame in turn).
    for nest in [false, true] {
        let (serial, serial_events) = play(1, 90, nest);
        assert!(
            serial_events.iter().map(Vec::len).sum::<usize>() > 50,
            "events fired"
        );
        for threads in [2, 3, 8] {
            let (poses, events) = play(threads, 90, nest);
            for (f, (a, b)) in serial.iter().zip(&poses).enumerate() {
                assert!(a == b, "{threads} threads, frame {f}: the poses differ");
            }
            assert_eq!(
                serial_events, events,
                "{threads} threads: the events differ"
            );
        }
    }
}

// M5-ANI-17
#[test]
fn a_clip_that_moves_more_than_transforms_keeps_the_update_in_turn() {
    let (mut w, mut scene, roots) = crowd(32);
    assert!(w.only_transforms());
    let lamp = roots
        .iter()
        .copied()
        .find(|r| scene.name(*r) == Some("Lamp"))
        .unwrap();
    // The lamp's clip now switches it off half the time: the walk depends on it.
    let swing = w.players[0].clip.unwrap();
    let off = w.add_curve(Curve::constant(0.0, 0.9, 0.0));
    w.set_curve(swing, "", "GameObject", "active", Some(off));
    assert!(!w.only_transforms());
    update_on(&mut w, &mut scene, &mut NoComponents, 0.02, &mut Rule(1), 8);
    assert!(!scene.enabled(lamp), "the clip ran, in turn");
}
