// SPDX-License-Identifier: Apache-2.0

//! Animators posed on several threads. The frame's walk (in turn) lists what runs, in order; the
//! animators then advance their state machines, blend their clips and work out their bones' new
//! local transforms in parallel, each into its own lists, reading the scene only; last, in the
//! walk's order, the players run and the animators' transforms are written and their events kept.
//! Each animator's work is the same as in turn and is written in the same order, so the result is
//! the same bit for bit (`M5-ANI-17`).
//!
//! Used only while every clip moves transforms only (a clip switching an object off would change
//! the walk, and component properties go through a sink that is not shared) and no animated
//! object is under another (then their bones are apart, so a bone's transform read in parallel is
//! the one it has when written). Otherwise the frame runs in turn.

use crate::animator::AnimatorPlayer;
use crate::clip::World;
use crate::player::{AnimationPlayer, FiredEvent};
use crate::pose::{NoComponents, Pose, PropertySink};
use iron_scene::{ObjectId, Scene, Transform};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

static THREADS: AtomicUsize = AtomicUsize::new(0);

/// How many threads the frame's update poses animators on (0 or 1: in turn, the default).
pub fn set_threads(n: usize) {
    THREADS.store(n, Ordering::Relaxed);
}

pub fn threads() -> usize {
    THREADS.load(Ordering::Relaxed)
}

/// Fewer animators than this per thread are posed in turn (threads cost more than they save).
const MIN_PER_THREAD: usize = 8;

/// What the walk met, in order.
pub(crate) enum Job {
    Player(usize),
    /// An animator by index, and whether it was seen.
    Animator(usize, bool),
}

/// One thread's results.
#[derive(Default)]
struct Out {
    events: Vec<FiredEvent>,
    locals: Vec<(ObjectId, Transform)>,
    /// Per animator: its job, its events and its transforms.
    done: Vec<(usize, Range<usize>, Range<usize>)>,
}

pub(crate) struct Run<'a> {
    pub world: &'a World,
    pub players: &'a mut [AnimationPlayer],
    pub animators: &'a mut [AnimatorPlayer],
    pub dt: f64,
    pub threads: usize,
}

impl Run<'_> {
    /// Runs the jobs: in parallel when they are apart, else in turn.
    pub(crate) fn go(
        &mut self,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        jobs: &[Job],
        events: &mut Vec<FiredEvent>,
    ) {
        let count = jobs
            .iter()
            .filter(|j| matches!(j, Job::Animator(..)))
            .count();
        let n = self.threads.min(count / MIN_PER_THREAD);
        if n < 2 || self.nested(scene, jobs) {
            self.in_turn(scene, sink, jobs, events);
        } else {
            self.in_parallel(scene, sink, jobs, events, n);
        }
    }

    /// The object a job animates.
    fn object(&self, job: &Job) -> ObjectId {
        match *job {
            Job::Player(i) => self.players[i].object,
            Job::Animator(i, _) => self.animators[i].object,
        }
    }

    /// Is an animated object under another one (or animated twice)?
    fn nested(&self, scene: &Scene, jobs: &[Job]) -> bool {
        let mut roots = HashSet::with_capacity(jobs.len());
        if !jobs.iter().all(|j| roots.insert(self.object(j))) {
            return true;
        }
        jobs.iter().any(|j| {
            let mut at = scene.parent(self.object(j));
            while let Some(a) = at {
                if roots.contains(&a) {
                    return true;
                }
                at = scene.parent(a);
            }
            false
        })
    }

    fn in_turn(
        &mut self,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        jobs: &[Job],
        events: &mut Vec<FiredEvent>,
    ) {
        let (world, dt) = (self.world, self.dt);
        for job in jobs {
            match *job {
                Job::Player(i) => self.players[i].animate(world, scene, sink, dt, events),
                Job::Animator(i, seen) => {
                    self.animators[i].animate_seen(world, scene, sink, dt, events, seen);
                }
            }
        }
    }

    fn in_parallel(
        &mut self,
        scene: &mut Scene,
        sink: &mut dyn PropertySink,
        jobs: &[Job],
        events: &mut Vec<FiredEvent>,
        n: usize,
    ) {
        let (world, dt) = (self.world, self.dt);
        // Each animator handed to one thread; threads take every n-th one so the seen and the
        // culled spread evenly.
        let mut slots: Vec<Option<&mut AnimatorPlayer>> =
            self.animators.iter_mut().map(Some).collect();
        let mut parts: Vec<Vec<(usize, &mut AnimatorPlayer, bool)>> =
            (0..n).map(|_| Vec::new()).collect();
        let mut k = 0;
        for (j, job) in jobs.iter().enumerate() {
            if let Job::Animator(i, seen) = *job
                && let Some(a) = slots[i].take()
            {
                parts[k % n].push((j, a, seen));
                k += 1;
            }
        }
        let lap = crate::profile::start();
        let work = |part: Vec<(usize, &mut AnimatorPlayer, bool)>, scene: &Scene| {
            let mut out = Out::default();
            let mut pose = Pose::new();
            for (job, a, seen) in part {
                let (e, l) = (out.events.len(), out.locals.len());
                let mut none = NoComponents;
                if a.step(
                    world,
                    scene,
                    &mut none,
                    dt,
                    &mut out.events,
                    seen,
                    &mut pose,
                ) {
                    pose.take_locals(scene, &mut out.locals);
                }
                out.done
                    .push((job, e..out.events.len(), l..out.locals.len()));
            }
            out
        };
        let shared: &Scene = scene;
        let outs: Vec<Out> = std::thread::scope(|s| {
            let handles: Vec<_> = parts
                .into_iter()
                .map(|p| s.spawn(move || work(p, shared)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("an animation thread panicked"))
                .collect()
        });
        crate::profile::stop("animation: animators in parallel", lap);
        // Where each job's results are, then everything written in the walk's order.
        let mut at: Vec<Option<(usize, usize)>> = vec![None; jobs.len()];
        for (t, out) in outs.iter().enumerate() {
            for (d, &(job, ..)) in out.done.iter().enumerate() {
                at[job] = Some((t, d));
            }
        }
        for (j, job) in jobs.iter().enumerate() {
            if let Job::Player(i) = *job {
                self.players[i].animate(world, scene, sink, dt, events);
                continue;
            }
            let Some((t, d)) = at[j] else {
                continue;
            };
            let (_, e, l) = &outs[t].done[d];
            let lap = crate::profile::start();
            for &(go, local) in &outs[t].locals[l.clone()] {
                scene.set_local(go, local);
            }
            crate::profile::stop("animation: set_local", lap);
            events.extend_from_slice(&outs[t].events[e.clone()]);
        }
    }
}
