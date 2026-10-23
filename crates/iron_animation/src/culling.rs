//! Animation culling (Unity's `Animator.cullingMode` and `Animation.cullingType`): what an animated
//! object does while none of its renderers was drawn. The host says what was drawn ([`Visibility`]);
//! the default animates always.

use crate::clip::World;
use iron_scene::{ObjectId, Scene};

/// What an `Animator` does while its object is not visible (Unity's `AnimatorCullingMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CullingMode {
    /// Animates whether seen or not.
    #[default]
    AlwaysAnimate,
    /// The state machine, clock, parameters and events go on, but no transform (or other
    /// property) is written: once seen again the pose is where it would have been.
    CullUpdateTransforms,
    /// Nothing runs and the clock stops: it holds its last pose and resumes from there.
    CullCompletely,
}

/// What an `Animation` does while its object is not visible (Unity's `AnimationCullingType`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CullingType {
    #[default]
    AlwaysAnimate,
    /// Paused while not visible: nothing is evaluated and its states' times stand still.
    BasedOnRenderers,
}

/// Whether an animated object was seen: any renderer in its hierarchy drawn by a camera or into
/// the shadow map of a light (one frame late). An object with no renderers counts as seen.
pub trait Visibility {
    fn visible(&mut self, scene: &Scene, root: ObjectId) -> bool;
}

/// Everything is seen (no culling).
pub struct AllVisible;

impl Visibility for AllVisible {
    fn visible(&mut self, _: &Scene, _: ObjectId) -> bool {
        true
    }
}

/// The objects whose animation may be culled: the host watches their renderers.
pub fn culled_roots(world: &World) -> Vec<ObjectId> {
    let animators = world
        .animators
        .iter()
        .filter(|a| a.enabled && a.culling_mode != CullingMode::AlwaysAnimate)
        .map(|a| a.object);
    let players = world
        .players
        .iter()
        .filter(|p| p.culling_type != CullingType::AlwaysAnimate)
        .map(|p| p.object);
    animators
        .chain(players)
        .filter(|o| o.index != u32::MAX)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animator::AnimatorPlayer;
    use crate::clip::{Clip, ClipEvent, ClipId};
    use crate::controller::{Controller, Motion, Node};
    use crate::curve::{Curve, WrapMode};
    use crate::player::{AnimationPlayer, update_culled};
    use crate::pose::NoComponents;

    /// Seen or not, as set; counts the questions.
    struct Seen(bool, usize);

    impl Visibility for Seen {
        fn visible(&mut self, _: &Scene, _: ObjectId) -> bool {
            self.1 += 1;
            self.0
        }
    }

    /// A looping 2 s clip: x from 0 to 2 on the arm, an event at 1 s.
    fn swing(w: &mut World) -> ClipId {
        let mut c = Clip::new("swing");
        c.wrap_mode = WrapMode::Loop;
        let id = w.add_clip(c);
        let curve = w.add_curve(Curve::linear(0.0, 0.0, 2.0, 2.0));
        w.set_curve(id, "Arm", "Transform", "localPosition.x", Some(curve));
        w.add_event(
            id,
            ClipEvent {
                time: 1.0,
                function_name: "Step".into(),
                parameter: None,
            },
        );
        id
    }

    /// A figure with an arm, its Animator in `mode` playing `swing`.
    fn figure(w: &mut World, scene: &mut Scene, mode: CullingMode) -> ObjectId {
        let clip = swing(w);
        let node = w.graphs.add_node(Node {
            name: "Swing".into(),
            motion: Motion::Clip(clip),
            speed: None,
            speed_parameter: None,
        });
        let controller = w.graphs.add_controller(Controller {
            nodes: vec![node],
            ..Default::default()
        });
        let go = scene.create("Figure");
        let arm = scene.create("Arm");
        scene.set_parent(arm, Some(go), false);
        let mut a = AnimatorPlayer::new(go);
        a.controller = Some(controller);
        a.culling_mode = mode;
        w.animators.push(a);
        arm
    }

    fn arm_x(scene: &Scene, arm: ObjectId) -> f32 {
        scene.local(arm).unwrap().position.x
    }

    fn norm(w: &mut World) -> f64 {
        let mut a = w.animators[0].clone();
        let n = a.current_info(w).normalized_time;
        w.animators[0] = a;
        n
    }

    // M5-ANI-09
    #[test]
    fn an_unseen_animator_that_culls_transforms_keeps_its_clock_and_events_but_writes_nothing() {
        let (mut w, mut scene) = (World::default(), Scene::new());
        let arm = figure(&mut w, &mut scene, CullingMode::CullUpdateTransforms);
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.5,
            &mut Seen(true, 0),
        );
        assert_eq!(arm_x(&scene, arm), 0.5);
        let mut unseen = Seen(false, 0);
        let events = update_culled(&mut w, &mut scene, &mut NoComponents, 0.75, &mut unseen);
        assert_eq!(arm_x(&scene, arm), 0.5, "the transform is not written");
        assert_eq!(norm(&mut w), 0.625, "the clock went on");
        assert_eq!(events.len(), 1, "the event at 1 s still fired");
        assert_eq!(unseen.1, 1);
        // Seen again: where it would have been.
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.25,
            &mut Seen(true, 0),
        );
        assert_eq!(arm_x(&scene, arm), 1.5);
    }

    // M5-ANI-10
    #[test]
    fn an_unseen_animator_that_culls_completely_freezes_and_resumes_where_it_stopped() {
        let (mut w, mut scene) = (World::default(), Scene::new());
        let arm = figure(&mut w, &mut scene, CullingMode::CullCompletely);
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.5,
            &mut Seen(true, 0),
        );
        for _ in 0..10 {
            let events = update_culled(
                &mut w,
                &mut scene,
                &mut NoComponents,
                0.4,
                &mut Seen(false, 0),
            );
            assert!(events.is_empty());
        }
        assert_eq!(arm_x(&scene, arm), 0.5, "it holds its last pose");
        assert_eq!(norm(&mut w), 0.25, "the clock stopped");
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.25,
            &mut Seen(true, 0),
        );
        assert_eq!(arm_x(&scene, arm), 0.75, "and resumes from there");
    }

    // M5-ANI-11
    #[test]
    fn seen_again_a_transform_culled_animator_matches_an_always_animated_twin() {
        let (mut wa, mut sa) = (World::default(), Scene::new());
        let (mut wb, mut sb) = (World::default(), Scene::new());
        let arm_a = figure(&mut wa, &mut sa, CullingMode::AlwaysAnimate);
        let arm_b = figure(&mut wb, &mut sb, CullingMode::CullUpdateTransforms);
        for frame in 0..40 {
            let dt = 0.013 + 0.007 * f64::from(frame % 5);
            let seen = !(5..30).contains(&frame);
            update_culled(&mut wa, &mut sa, &mut NoComponents, dt, &mut Seen(false, 0));
            update_culled(&mut wb, &mut sb, &mut NoComponents, dt, &mut Seen(seen, 0));
            if seen {
                assert_eq!(sa.local(arm_a), sb.local(arm_b), "frame {frame}");
            }
        }
    }

    // M5-ANI-12
    #[test]
    fn by_default_animation_is_never_culled_nor_asks() {
        let (mut w, mut scene) = (World::default(), Scene::new());
        let arm = figure(&mut w, &mut scene, CullingMode::default());
        let clip = swing(&mut w);
        let mut p = AnimationPlayer::new(arm);
        p.clip = Some(clip);
        w.players.push(p);
        assert!(culled_roots(&w).is_empty());
        let mut unseen = Seen(false, 0);
        update_culled(&mut w, &mut scene, &mut NoComponents, 0.5, &mut unseen);
        assert_eq!(unseen.1, 0, "visibility is not asked");
        assert_eq!(norm(&mut w), 0.25);
        assert_eq!(w.players[0].states[0].time, 0.5);
    }

    // M5-ANI-12
    #[test]
    fn an_animation_based_on_renderers_pauses_while_unseen() {
        let (mut w, mut scene) = (World::default(), Scene::new());
        let go = scene.create("Lamp");
        let arm = scene.create("Arm");
        scene.set_parent(arm, Some(go), false);
        let clip = swing(&mut w);
        let mut p = AnimationPlayer::new(go);
        p.clip = Some(clip);
        p.culling_type = CullingType::BasedOnRenderers;
        w.players.push(p);
        assert_eq!(culled_roots(&w), [go]);
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.5,
            &mut Seen(true, 0),
        );
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.5,
            &mut Seen(false, 0),
        );
        assert_eq!(w.players[0].states[0].time, 0.5);
        assert_eq!(arm_x(&scene, arm), 0.5);
        update_culled(
            &mut w,
            &mut scene,
            &mut NoComponents,
            0.25,
            &mut Seen(true, 0),
        );
        assert_eq!(arm_x(&scene, arm), 0.75);
    }
}
