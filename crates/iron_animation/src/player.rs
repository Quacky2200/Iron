//! The `Animation` component (`js/engine/src/animation.ts`): named states that play clips, with
//! `play`, `crossFade` and `stop`, advanced every frame after the components' updates. Clip events
//! are collected as [`FiredEvent`]s for the host to deliver to the object's components.

use crate::clip::{ClipId, Parameter, World};
use crate::culling::{AllVisible, CullingMode, CullingType, Visibility};
use crate::curve::{WrapMode, wrap};
use crate::pose::PropertySink;
use iron_scene::{ObjectId, Scene};

/// A clip event that came due: call `function_name` on the object's components.
#[derive(Clone, Debug, PartialEq)]
pub struct FiredEvent {
    pub object: ObjectId,
    pub function_name: String,
    pub parameter: Option<Parameter>,
}

/// A clip playing on an `Animation`, with its time, speed and weight (Unity's `AnimationState`).
#[derive(Clone, Debug, PartialEq)]
pub struct AnimState {
    pub name: String,
    pub clip: ClipId,
    pub time: f64,
    pub speed: f64,
    pub weight: f64,
    pub enabled: bool,
    pub wrap_mode: WrapMode,
    /// Weight change per second while fading.
    pub fade: f64,
    /// The weight it fades to.
    pub target: f64,
}

impl AnimState {
    fn new(world: &World, name: &str, clip: ClipId) -> AnimState {
        AnimState {
            name: name.to_string(),
            clip,
            time: 0.0,
            speed: 1.0,
            weight: 0.0,
            enabled: false,
            wrap_mode: world.clip(clip).map_or(WrapMode::Once, |c| c.wrap_mode),
            fade: 0.0,
            target: 0.0,
        }
    }

    pub fn length(&self, world: &World) -> f64 {
        world.clip_length(self.clip)
    }
}

/// Plays clips on its object (Unity's legacy `Animation`).
#[derive(Clone, Debug)]
pub struct AnimationPlayer {
    pub object: ObjectId,
    /// The default clip.
    pub clip: Option<ClipId>,
    pub clips: Vec<ClipId>,
    pub play_automatically: bool,
    /// By name, in the order first made.
    pub states: Vec<AnimState>,
    /// Paused while not visible, or not.
    pub culling_type: CullingType,
    started: bool,
}

impl AnimationPlayer {
    pub fn new(object: ObjectId) -> AnimationPlayer {
        AnimationPlayer {
            object,
            clip: None,
            clips: Vec::new(),
            play_automatically: true,
            states: Vec::new(),
            culling_type: CullingType::AlwaysAnimate,
            started: false,
        }
    }

    /// Adds a clip under a name (its own by default).
    pub fn add_clip(&mut self, world: &World, clip: ClipId, name: Option<&str>) {
        if !self.clips.contains(&clip) {
            self.clips.push(clip);
        }
        let name = name.map_or_else(
            || world.clip(clip).map(|c| c.name.clone()).unwrap_or_default(),
            str::to_string,
        );
        let state = AnimState::new(world, &name, clip);
        match self.states.iter_mut().find(|s| s.name == name) {
            Some(s) => *s = state,
            None => self.states.push(state),
        }
    }

    /// The index of the state of a clip by name, making it if the clip is known.
    pub fn state_index(&mut self, world: &World, name: &str) -> Option<usize> {
        if let Some(i) = self.states.iter().position(|s| s.name == name) {
            return Some(i);
        }
        let found = self
            .clips
            .iter()
            .copied()
            .chain(self.clip)
            .find(|c| world.clip(*c).is_some_and(|c| c.name == name))?;
        self.states.push(AnimState::new(world, name, found));
        Some(self.states.len() - 1)
    }

    /// Plays a clip from the start at full weight, stopping the others. False if unknown.
    pub fn play(&mut self, world: &World, name: Option<&str>) -> bool {
        let default = self
            .clip
            .and_then(|c| world.clip(c))
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let name = name.unwrap_or(&default).to_string();
        let Some(i) = self.state_index(world, &name) else {
            return false;
        };
        for (j, other) in self.states.iter_mut().enumerate() {
            if j != i {
                other.enabled = false;
                other.weight = 0.0;
            }
        }
        let s = &mut self.states[i];
        s.enabled = true;
        s.time = 0.0;
        s.weight = 1.0;
        s.fade = 0.0;
        s.target = 1.0;
        true
    }

    /// Fades a clip in over `fade_length` seconds while the others fade out.
    pub fn cross_fade(&mut self, world: &World, name: &str, fade_length: f64) {
        let Some(i) = self.state_index(world, name) else {
            return;
        };
        if !self.states[i].enabled {
            self.states[i].enabled = true;
            self.states[i].time = 0.0;
        }
        let rate = 1.0 / iron_math::jsnum::max(1e-4, fade_length);
        self.states[i].target = 1.0;
        self.states[i].fade = rate;
        for (j, other) in self.states.iter_mut().enumerate() {
            if j != i && other.enabled {
                other.target = 0.0;
                other.fade = rate;
            }
        }
    }

    pub fn stop(&mut self, name: Option<&str>) {
        for s in &mut self.states {
            if name.is_none_or(|n| s.name == n) {
                s.enabled = false;
                s.weight = 0.0;
                s.time = 0.0;
            }
        }
    }

    pub fn is_playing(&self, name: Option<&str>) -> bool {
        self.states
            .iter()
            .any(|s| s.enabled && name.is_none_or(|n| s.name == n))
    }

    /// The first update: the default clip plays by itself when `play_automatically`.
    fn start(&mut self, world: &World) {
        if self.started {
            return;
        }
        self.started = true;
        if self.play_automatically
            && let Some(c) = self.clip
        {
            let name = world.clip(c).map(|c| c.name.clone()).unwrap_or_default();
            self.play(world, Some(&name));
        }
    }

    /// Advances by `dt` and poses the object.
    pub fn animate(
        &mut self,
        world: &World,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        dt: f64,
        events: &mut Vec<FiredEvent>,
    ) {
        self.start(world);
        let mut pose = world.scratch.take();
        for s in &mut self.states {
            if !s.enabled {
                continue;
            }
            if s.fade != 0.0 {
                let step = s.fade * dt;
                s.weight = if s.weight < s.target {
                    iron_math::jsnum::min(s.target, s.weight + step)
                } else {
                    iron_math::jsnum::max(s.target, s.weight - step)
                };
                if s.weight == s.target {
                    s.fade = 0.0;
                }
                if s.weight <= 0.0 && s.target == 0.0 {
                    s.enabled = false;
                    continue;
                }
            }
            let before = s.time;
            s.time += dt * s.speed;
            let length = s.length(world);
            fire_events(
                world,
                self.object,
                s.clip,
                before,
                s.time,
                length,
                s.wrap_mode,
                events,
            );
            let lap = crate::profile::start();
            if s.wrap_mode == WrapMode::Once && s.time >= length {
                // Holds the last frame this time, then stops.
                pose.add_aligned(scene, world, sink, self.object, s.clip, length, s.weight);
                crate::profile::stop("animation: pose add", lap);
                s.enabled = false;
                continue;
            }
            let t = wrap(s.time, length, s.wrap_mode);
            pose.add_aligned(scene, world, sink, self.object, s.clip, t, s.weight);
            crate::profile::stop("animation: pose add", lap);
        }
        let lap = crate::profile::start();
        pose.apply(scene, sink);
        crate::profile::stop("animation: pose apply", lap);
        world.scratch.replace(pose);
    }
}

/// Collects the clip events between two times.
#[allow(clippy::too_many_arguments)]
pub fn fire_events(
    world: &World,
    object: ObjectId,
    clip: ClipId,
    from: f64,
    to: f64,
    length: f64,
    wrap_mode: WrapMode,
    out: &mut Vec<FiredEvent>,
) {
    let Some(c) = world.clip(clip) else {
        return;
    };
    if c.events.is_empty() || to <= from {
        return;
    }
    // Unwrapped times: events fire once per pass through a loop.
    let looping = wrap_mode == WrapMode::Loop && length > 0.0;
    let passes = if looping {
        (to / length).floor() - (from / length).floor()
    } else {
        0.0
    };
    let local = |t: f64| {
        if looping {
            ((t % length) + length) % length
        } else {
            t
        }
    };
    let (a, b) = (local(from), local(to));
    for e in &c.events {
        let hit = if passes == 0.0 {
            e.time > a && e.time <= b
        } else {
            e.time > a || e.time <= b || passes > 1.0
        };
        if hit {
            out.push(FiredEvent {
                object,
                function_name: e.function_name.clone(),
                parameter: e.parameter.clone(),
            });
        }
    }
}

/// Every frame, after the components' updates: each player on an active object advances and poses
/// its object. Players go depth first through the scene, an object's before its children's. Returns
/// the clip events that came due, in order.
pub fn update(
    world: &mut World,
    scene: &mut Scene,
    sink: &mut dyn PropertySink,
    dt: f64,
) -> Vec<FiredEvent> {
    update_culled(world, scene, sink, dt, &mut AllVisible)
}

/// [`update`], with culling: players and animators that cull ask `visible` about their object.
pub fn update_culled(
    world: &mut World,
    scene: &mut Scene,
    sink: &mut dyn PropertySink,
    dt: f64,
    visible: &mut dyn Visibility,
) -> Vec<FiredEvent> {
    let lap = crate::profile::start();
    world.binds.get_mut().sweep(scene);
    let mut events = Vec::new();
    // Players of destroyed objects are dropped lazily: they are skipped here.
    let mut players = std::mem::take(&mut world.players);
    let mut animators = std::mem::take(&mut world.animators);
    // Each object's players and animators, in order (a crowd has thousands: no scan per object).
    let mut owners: std::collections::HashMap<ObjectId, (Vec<usize>, Vec<usize>)> =
        std::collections::HashMap::new();
    for (i, p) in players.iter().enumerate() {
        owners.entry(p.object).or_default().0.push(i);
    }
    for (i, a) in animators.iter().enumerate() {
        owners.entry(a.object).or_default().1.push(i);
    }
    let mut stack: Vec<ObjectId> = scene.roots().iter().rev().copied().collect();
    while let Some(go) = stack.pop() {
        // Only children of active objects are walked, so enabled here is active.
        if !scene.enabled(go) {
            continue;
        }
        if let Some((ps, anims)) = owners.get(&go) {
            for &i in ps {
                let p = &mut players[i];
                if p.culling_type == CullingType::BasedOnRenderers && !visible.visible(scene, go) {
                    continue;
                }
                p.animate(world, scene, sink, dt, &mut events);
            }
            for &i in anims {
                let a = &mut animators[i];
                let seen =
                    a.culling_mode == CullingMode::AlwaysAnimate || visible.visible(scene, go);
                a.animate_seen(world, scene, sink, dt, &mut events, seen);
            }
            // Its own clips may have switched it off (`active` on path "").
            if !scene.enabled(go) {
                continue;
            }
        }
        stack.extend(scene.children(go).iter().rev().copied());
    }
    world.players = players;
    world.animators = animators;
    crate::profile::stop("animation: update (all)", lap);
    events
}
