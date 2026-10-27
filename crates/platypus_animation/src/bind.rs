// SPDX-License-Identifier: Apache-2.0

//! A clip's bindings resolved once per animated root (Unity's `Animator.Rebind`): each curve gets
//! the bone slot it moves and a channel number, so a frame does no path walks or string compares.
//! A root's bindings are kept against its [`Scene::tree_version`] (renames, re-parenting, objects
//! destroyed or added under it drop them) and a clip's against [`World`]'s edit count (any change to
//! a clip or curve).

use crate::clip::{Clip, ClipId, CurveId, World};
use crate::pose::resolve;
use iron_scene::{ObjectId, Scene};
use std::collections::HashMap;
use std::sync::Arc;

/// Transform channels per bone: position x y z, rotation x y z w, scale x y z, Euler angles x y z.
pub(crate) const CHANNELS: usize = 13;
pub(crate) const POS: usize = 0;
pub(crate) const ROT: usize = 3;
pub(crate) const SCALE: usize = 7;
pub(crate) const EULER: usize = 10;
/// A `Transform` property that is not a channel (`localPosition.w`): it touches its bone only.
pub(crate) const NO_CHANNEL: u8 = u8::MAX;

const AXES: &str = "xyzw";

/// The channel of a `Transform` property (`localRotation.w`), or [`NO_CHANNEL`].
pub(crate) fn channel(property: &str) -> u8 {
    let Some((field, axis)) = property.split_once('.') else {
        return NO_CHANNEL;
    };
    let Some(i) = AXES.find(axis).filter(|_| axis.len() == 1) else {
        return NO_CHANNEL;
    };
    let base = match (field, i) {
        ("localPosition", 0..=2) => POS,
        ("localRotation", 0..=3) => ROT,
        ("localScale", 0..=2) => SCALE,
        ("localEulerAngles", 0..=2) => EULER,
        _ => return NO_CHANNEL,
    };
    (base + i) as u8
}

/// A `Transform` curve.
pub(crate) struct Track {
    pub slot: u32,
    pub channel: u8,
    /// A `localRotation.*` property: flipped with its bone's quaternion when blends align.
    pub rotation: bool,
    pub curve: CurveId,
    /// The track (this one or an earlier one on the same bone) whose key search this one reuses:
    /// their curves have the same key times.
    pub lead: u32,
}

/// A curve on a `GameObject` (`active`) or a component property.
pub(crate) struct PropTrack {
    pub object: ObjectId,
    /// Into [`RootBinds::prop_keys`].
    pub key: u32,
    pub game_object: bool,
    pub curve: CurveId,
}

/// A bone's quaternion curves (track indexes, x y z w), for aligning blends.
pub(crate) struct RotationSet {
    pub slot: u32,
    pub axes: [Option<u32>; 4],
}

pub(crate) struct BoundClip {
    /// The world's edit count when bound.
    edits: u64,
    /// In the clip's binding order.
    pub tracks: Vec<Track>,
    pub props: Vec<PropTrack>,
    /// One per bone with a rotation curve, in the order first seen.
    pub rotations: Vec<RotationSet>,
}

/// What one animated root's clips move.
#[derive(Default)]
pub(crate) struct RootBinds {
    /// The root's tree version when bound.
    version: u64,
    /// The objects moved, by slot.
    pub bones: Vec<ObjectId>,
    index: HashMap<ObjectId, u32>,
    /// Component properties' class and property names, by key.
    pub prop_keys: Vec<(Arc<str>, Arc<str>)>,
    clips: Vec<(ClipId, BoundClip)>,
}

impl RootBinds {
    fn slot(&mut self, go: ObjectId) -> u32 {
        if let Some(&s) = self.index.get(&go) {
            return s;
        }
        let s = self.bones.len() as u32;
        self.bones.push(go);
        self.index.insert(go, s);
        s
    }

    fn key(&mut self, type_name: &str, property: &str) -> u32 {
        if let Some(k) = self
            .prop_keys
            .iter()
            .position(|(t, p)| &**t == type_name && &**p == property)
        {
            return k as u32;
        }
        self.prop_keys.push((type_name.into(), property.into()));
        (self.prop_keys.len() - 1) as u32
    }

    fn bind(&mut self, scene: &Scene, world: &World, root: ObjectId, clip: &Clip) -> BoundClip {
        let mut tracks: Vec<Track> = Vec::new();
        let mut props = Vec::new();
        let mut track_of = vec![None; clip.bindings.len()];
        for (i, b) in clip.bindings.iter().enumerate() {
            let Some(go) = resolve(scene, root, &b.path) else {
                continue;
            };
            if b.type_name == "Transform" {
                let at = tracks.len() as u32;
                track_of[i] = Some(at);
                let slot = self.slot(go);
                // The first earlier curve on this bone with the same key times leads.
                let lead = world.curve(b.curve).and_then(|c| {
                    tracks.iter().enumerate().find_map(|(j, t)| {
                        (t.slot == slot
                            && t.lead == j as u32
                            && world.curve(t.curve).is_some_and(|o| o.same_times(c)))
                        .then_some(j as u32)
                    })
                });
                tracks.push(Track {
                    slot,
                    channel: channel(&b.property),
                    rotation: b.property.starts_with("localRotation."),
                    curve: b.curve,
                    lead: lead.unwrap_or(at),
                });
            } else {
                props.push(PropTrack {
                    object: go,
                    key: self.key(&b.type_name, &b.property),
                    game_object: b.type_name == "GameObject",
                    curve: b.curve,
                });
            }
        }
        // Each bone's quaternion, from the first curve of each axis on the path that first
        // rotates it (as the string search did).
        let mut rotations: Vec<RotationSet> = Vec::new();
        for (i, b) in clip.bindings.iter().enumerate() {
            let Some(t) = track_of[i].map(|t| &tracks[t as usize]) else {
                continue;
            };
            if !t.rotation || rotations.iter().any(|r| r.slot == t.slot) {
                continue;
            }
            let mut axes = [None; 4];
            for (a, axis) in AXES.chars().enumerate() {
                let prop = format!("localRotation.{axis}");
                axes[a] = clip
                    .bindings
                    .iter()
                    .position(|o| {
                        o.path == b.path && o.type_name == "Transform" && o.property == prop
                    })
                    .and_then(|j| track_of[j]);
            }
            rotations.push(RotationSet { slot: t.slot, axes });
        }
        BoundClip {
            edits: world.edits,
            tracks,
            props,
            rotations,
        }
    }
}

/// Every root's bindings.
#[derive(Default)]
pub(crate) struct Binds {
    roots: HashMap<ObjectId, RootBinds>,
    /// How many roots there were at the last sweep of destroyed ones.
    swept_at: usize,
}

impl Binds {
    /// Forgets the roots that are gone, once their number has doubled since the last sweep.
    pub(crate) fn sweep(&mut self, scene: &Scene) {
        if self.roots.len() <= 2 * self.swept_at + 8 {
            return;
        }
        self.roots.retain(|root, _| scene.contains(*root));
        self.swept_at = self.roots.len();
    }
}

impl World {
    /// Runs `f` on `clip`'s bindings under `root` (binding them first if needed), with the root's
    /// bones and property names. None for an unknown clip.
    pub(crate) fn with_bound<R>(
        &self,
        scene: &Scene,
        root: ObjectId,
        clip: ClipId,
        f: impl FnOnce(&RootBinds, &BoundClip) -> R,
    ) -> Option<R> {
        let c = self.clip(clip)?;
        let version = scene.tree_version(root);
        // Bound already (every frame but the first): shared, so animators posing in parallel
        // don't wait on each other.
        {
            let binds = self.binds.read().expect("not poisoned");
            if let Some(rb) = binds.roots.get(&root).filter(|rb| rb.version == version)
                && let Some((_, bound)) = rb
                    .clips
                    .iter()
                    .find(|(id, b)| *id == clip && b.edits == self.edits)
            {
                return Some(f(rb, bound));
            }
        }
        let mut binds = self.binds.write().expect("not poisoned");
        let rb = binds.roots.entry(root).or_default();
        if rb.version != version {
            *rb = RootBinds {
                version,
                ..RootBinds::default()
            };
        }
        let at = match rb.clips.iter().position(|(id, _)| *id == clip) {
            Some(i) if rb.clips[i].1.edits == self.edits => i,
            found => {
                let bound = rb.bind(scene, self, root, c);
                match found {
                    Some(i) => {
                        rb.clips[i].1 = bound;
                        i
                    }
                    None => {
                        rb.clips.push((clip, bound));
                        rb.clips.len() - 1
                    }
                }
            }
        };
        let (rb, bound) = {
            let rb: &RootBinds = rb;
            (rb, &rb.clips[at].1)
        };
        Some(f(rb, bound))
    }
}
