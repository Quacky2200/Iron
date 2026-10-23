// SPDX-License-Identifier: Apache-2.0

//! The game world's objects (DESIGN.md §7.5): GameObjects in a generational arena,
//! each with a name, an enabled flag, a parent and children, and a local transform.
//! World transforms are worked out lazily from the hierarchy.
//!
//! Components live elsewhere (script components in `iron_script`, physics in
//! `iron_physics`, ...) and refer to objects by [`ObjectId`]. There's no ECS: scenes are
//! small enough that plain parent/child links and a dirty flag per object do.

pub mod api;
mod script;

pub use script::{ENGINE_IDS, SceneEvent, ScriptObject, ScriptScene};

use glam::{Affine3A, Mat4, Quat, Vec3};

/// A GameObject. The generation makes stale ids (of destroyed objects) miss instead
/// of finding whatever reused the slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId {
    pub index: u32,
    pub generation: u32,
}

/// Position, rotation and scale, relative to the parent (or the world, for roots).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Self::IDENTITY
        }
    }

    pub fn to_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// The transform an affine matrix describes (shear, from non-uniform scale under a
    /// rotation, is lost).
    pub fn from_affine(m: Affine3A) -> Self {
        let (scale, rotation, position) = m.to_scale_rotation_translation();
        Self {
            position,
            rotation: rotation.normalize(),
            scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

struct Object {
    name: String,
    enabled: bool,
    /// Render group 0 to 31 (Unity's `layer`).
    layer: u8,
    /// `HideFlags` bits: what the editor shows and saves.
    hide_flags: u32,
    /// The scene (a loader's handle) the object was made in; 0 for none.
    tag: u32,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
    local: Transform,
    /// Cached world matrix, valid unless `dirty`.
    world: Affine3A,
    dirty: bool,
    /// In [`Scene::moved`]. With `dirty` too, so is every descendant (`mark_moved` stops here).
    listed: bool,
    /// Changes when a name or a parent link changes in the subtree (see [`Scene::tree_version`]).
    tree_version: u64,
    /// The [`Scene::epoch`] its world transform last changed in.
    moved_at: u64,
}

struct Slot {
    generation: u32,
    object: Option<Object>,
}

/// All the GameObjects in a world.
#[derive(Default)]
pub struct Scene {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// Root objects, in order.
    roots: Vec<ObjectId>,
    /// Objects whose world transform changed since [`Scene::take_moved`].
    moved: Vec<ObjectId>,
    len: usize,
    /// The tag new objects get.
    new_tag: u32,
    /// The last tree version handed out.
    versions: u64,
    /// `mark_moved`'s stack, kept between calls.
    mark_stack: Vec<ObjectId>,
    /// Counts [`Scene::update_world`] calls: moves are stamped with it.
    epoch: u64,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, id: ObjectId) -> bool {
        self.get(id).is_some()
    }

    fn get(&self, id: ObjectId) -> Option<&Object> {
        let slot = self.slots.get(id.index as usize)?;
        (slot.generation == id.generation).then_some(slot.object.as_ref()?)
    }

    fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.object.as_mut()
    }

    /// Creates an enabled root object at the origin.
    pub fn create(&mut self, name: &str) -> ObjectId {
        let object = Object {
            name: name.to_owned(),
            enabled: true,
            layer: 0,
            hide_flags: 0,
            tag: self.new_tag,
            parent: None,
            children: Vec::new(),
            local: Transform::IDENTITY,
            world: Affine3A::IDENTITY,
            dirty: true,
            listed: true,
            tree_version: {
                self.versions += 1;
                self.versions
            },
            moved_at: self.epoch,
        };
        let id = match self.free.pop() {
            Some(index) => {
                let slot = &mut self.slots[index as usize];
                slot.object = Some(object);
                ObjectId {
                    index,
                    generation: slot.generation,
                }
            }
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    object: Some(object),
                });
                ObjectId {
                    index: self.slots.len() as u32 - 1,
                    generation: 0,
                }
            }
        };
        self.roots.push(id);
        self.moved.push(id);
        self.len += 1;
        id
    }

    /// Destroys an object and everything under it. Returns the destroyed ids (the
    /// object first, then its descendants), so their components can go too.
    pub fn destroy(&mut self, id: ObjectId) -> Vec<ObjectId> {
        if !self.contains(id) {
            return Vec::new();
        }
        if let Some(parent) = self.parent(id) {
            self.touch_tree(parent);
        }
        self.unlink(id);
        let mut gone = Vec::new();
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            let slot = &mut self.slots[id.index as usize];
            if let Some(object) = slot.object.take() {
                slot.generation = slot.generation.wrapping_add(1);
                self.free.push(id.index);
                self.len -= 1;
                // Children in order, so `gone` reads parent-first, depth-first.
                stack.extend(object.children.iter().rev());
                gone.push(id);
            }
        }
        // The destroyed ids no longer match their slots' generations.
        let slots = &self.slots;
        self.moved.retain(|m| {
            slots
                .get(m.index as usize)
                .is_some_and(|s| s.generation == m.generation && s.object.is_some())
        });
        gone
    }

    /// Removes `id` from its parent's children (or the roots).
    fn unlink(&mut self, id: ObjectId) {
        let parent = self.get(id).and_then(|o| o.parent);
        match parent.and_then(|p| self.get_mut(p)) {
            Some(p) => p.children.retain(|c| *c != id),
            None => self.roots.retain(|r| *r != id),
        }
    }

    pub fn name(&self, id: ObjectId) -> Option<&str> {
        self.get(id).map(|o| o.name.as_str())
    }

    pub fn set_name(&mut self, id: ObjectId, name: &str) {
        if let Some(o) = self.get_mut(id) {
            if o.name == name {
                return;
            }
            o.name = name.to_owned();
            self.touch_tree(id);
        }
    }

    /// A number that changes whenever a name or a parent link changes anywhere under `id` (`id`
    /// included): an object renamed, re-parented, destroyed or given a child. What depends on
    /// paths by name below an object (animation bindings) is cached against it. 0 for unknown ids.
    pub fn tree_version(&self, id: ObjectId) -> u64 {
        self.get(id).map_or(0, |o| o.tree_version)
    }

    /// Gives `id` and its ancestors a new tree version.
    fn touch_tree(&mut self, id: ObjectId) {
        self.versions += 1;
        let v = self.versions;
        let mut at = Some(id);
        while let Some(a) = at {
            match self.get_mut(a) {
                Some(o) => {
                    o.tree_version = v;
                    at = o.parent;
                }
                None => break,
            }
        }
    }

    /// The object's layer (0 for unknown objects).
    pub fn layer(&self, id: ObjectId) -> u8 {
        self.get(id).map_or(0, |o| o.layer)
    }

    pub fn set_layer(&mut self, id: ObjectId, layer: u8) {
        if let Some(o) = self.get_mut(id) {
            o.layer = layer.min(31);
        }
    }

    /// The object's `HideFlags` bits (0 for unknown objects).
    /// The scene tag of an object (0 when it has none).
    pub fn tag(&self, id: ObjectId) -> u32 {
        self.get(id).map_or(0, |o| o.tag)
    }

    pub fn set_tag(&mut self, id: ObjectId, tag: u32) {
        if let Some(o) = self.get_mut(id) {
            o.tag = tag;
        }
    }

    /// The tag objects made from now on get.
    pub fn set_new_tag(&mut self, tag: u32) {
        self.new_tag = tag;
    }

    pub fn new_tag(&self) -> u32 {
        self.new_tag
    }

    pub fn hide_flags(&self, id: ObjectId) -> u32 {
        self.get(id).map_or(0, |o| o.hide_flags)
    }

    pub fn set_hide_flags(&mut self, id: ObjectId, flags: u32) {
        if let Some(o) = self.get_mut(id) {
            o.hide_flags = flags;
        }
    }

    /// The first object with this name, depth-first from the roots.
    pub fn find(&self, name: &str) -> Option<ObjectId> {
        self.walk().find(|id| self.name(*id) == Some(name))
    }

    pub fn parent(&self, id: ObjectId) -> Option<ObjectId> {
        self.get(id)?.parent
    }

    pub fn children(&self, id: ObjectId) -> &[ObjectId] {
        self.get(id).map_or(&[], |o| &o.children)
    }

    pub fn roots(&self) -> &[ObjectId] {
        &self.roots
    }

    /// Every object, depth-first in hierarchy order.
    pub fn walk(&self) -> impl Iterator<Item = ObjectId> + '_ {
        let mut stack: Vec<ObjectId> = self.roots.iter().rev().copied().collect();
        std::iter::from_fn(move || {
            let id = stack.pop()?;
            stack.extend(self.children(id).iter().rev());
            Some(id)
        })
    }

    /// Whether `id` is `ancestor` or under it.
    pub fn is_descendant(&self, id: ObjectId, ancestor: ObjectId) -> bool {
        let mut at = Some(id);
        while let Some(a) = at {
            if a == ancestor {
                return true;
            }
            at = self.parent(a);
        }
        false
    }

    /// Moves `id` under `parent` (`None`: to the roots), at the end of its children.
    /// With `keep_world`, the local transform changes so the object stays where it is
    /// in the world; otherwise it keeps its local transform and moves with the new
    /// parent. Refuses (returns false) to parent an object under itself.
    pub fn set_parent(&mut self, id: ObjectId, parent: Option<ObjectId>, keep_world: bool) -> bool {
        if !self.contains(id)
            || parent.is_some_and(|p| !self.contains(p) || self.is_descendant(p, id))
        {
            return false;
        }
        let world = keep_world.then(|| self.world_matrix(id));
        if let Some(old) = self.parent(id) {
            self.touch_tree(old);
        }
        self.unlink(id);
        match parent {
            Some(p) => self.get_mut(p).expect("checked").children.push(id),
            None => self.roots.push(id),
        }
        self.get_mut(id).expect("checked").parent = parent;
        self.touch_tree(id);
        if let Some(world) = world {
            let parent_world = parent.map_or(Affine3A::IDENTITY, |p| self.world_matrix(p));
            let local = Transform::from_affine(parent_world.inverse() * world);
            self.get_mut(id).expect("checked").local = local;
        }
        self.mark_moved(id);
        true
    }

    /// Enabled here. See [`Self::active`] for the hierarchy.
    pub fn enabled(&self, id: ObjectId) -> bool {
        self.get(id).is_some_and(|o| o.enabled)
    }

    pub fn set_enabled(&mut self, id: ObjectId, enabled: bool) {
        if let Some(o) = self.get_mut(id) {
            o.enabled = enabled;
        }
    }

    /// Enabled, along with every ancestor: disabling an object turns off its subtree.
    pub fn active(&self, id: ObjectId) -> bool {
        let mut at = Some(id);
        while let Some(a) = at {
            match self.get(a) {
                Some(o) if o.enabled => at = o.parent,
                _ => return false,
            }
        }
        true
    }

    pub fn local(&self, id: ObjectId) -> Option<Transform> {
        self.get(id).map(|o| o.local)
    }

    pub fn set_local(&mut self, id: ObjectId, local: Transform) {
        if let Some(o) = self.get_mut(id) {
            if o.local == local {
                return;
            }
            o.local = local;
            self.mark_moved(id);
        }
    }

    /// Marks the object's and its descendants' world transforms stale and lists them as
    /// moved. (A stale object always has stale descendants, which `world_matrix` relies on,
    /// and a listed one listed descendants: so an object both stale and listed ends the walk.)
    fn mark_moved(&mut self, id: ObjectId) {
        let mut stack = std::mem::take(&mut self.mark_stack);
        let epoch = self.epoch;
        stack.push(id);
        while let Some(id) = stack.pop() {
            let Some(o) = self.get_mut(id) else { continue };
            if o.dirty && o.listed {
                continue;
            }
            o.dirty = true;
            o.moved_at = epoch;
            stack.extend(o.children.iter().copied());
            let new = !o.listed;
            o.listed = true;
            if new {
                self.moved.push(id);
            }
        }
        self.mark_stack = stack;
    }

    /// The object's world matrix (identity for unknown ids).
    pub fn world_matrix(&self, id: ObjectId) -> Affine3A {
        let Some(o) = self.get(id) else {
            return Affine3A::IDENTITY;
        };
        if !o.dirty {
            return o.world;
        }
        let parent = o
            .parent
            .map_or(Affine3A::IDENTITY, |p| self.world_matrix(p));
        parent * o.local.to_affine()
    }

    pub fn world(&self, id: ObjectId) -> Transform {
        Transform::from_affine(self.world_matrix(id))
    }

    /// Puts the object at a world position (and rotation), keeping its scale.
    pub fn set_world(&mut self, id: ObjectId, position: Vec3, rotation: Quat) {
        let Some(parent) = self.get(id).map(|o| o.parent) else {
            return;
        };
        let parent_world = parent.map_or(Affine3A::IDENTITY, |p| self.world_matrix(p));
        let (_, parent_rotation, _) = parent_world.to_scale_rotation_translation();
        let mut local = self.get(id).expect("checked").local;
        local.position = parent_world.inverse().transform_point3(position);
        local.rotation = (parent_rotation.inverse() * rotation).normalize();
        self.set_local(id, local);
    }

    /// Brings cached world matrices up to date (call once per frame, before reading
    /// many of them; reads work without it, just slower).
    pub fn update_world(&mut self) {
        let roots = self.roots.clone();
        let mut stack: Vec<(ObjectId, Affine3A)> =
            roots.into_iter().map(|r| (r, Affine3A::IDENTITY)).collect();
        while let Some((id, parent)) = stack.pop() {
            let Some(o) = self.get_mut(id) else { continue };
            if o.dirty {
                o.world = parent * o.local.to_affine();
                o.dirty = false;
            }
            let world = o.world;
            stack.extend(o.children.iter().map(|c| (*c, world)));
        }
        self.epoch += 1;
    }

    /// The number [`Scene::update_world`] moves on by one each call. Something worked out from
    /// world matrices right after one can be kept while [`Scene::moved_since`] says no.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Whether the object's world transform changed in or after `epoch` (or is stale, or the
    /// object is gone): a move stamps the object and all below it with the current epoch.
    pub fn moved_since(&self, id: ObjectId, epoch: u64) -> bool {
        self.get(id).is_none_or(|o| o.dirty || o.moved_at >= epoch)
    }

    /// Objects whose world transform changed since the last call (including new ones),
    /// for syncing scripts, physics and the renderer.
    pub fn take_moved(&mut self) -> Vec<ObjectId> {
        let moved = std::mem::take(&mut self.moved);
        for id in &moved {
            if let Some(o) = self.get_mut(*id) {
                o.listed = false;
            }
        }
        moved
    }

    /// The view-projection-ready matrix of an object (for cameras and drawing).
    pub fn world_mat4(&self, id: ObjectId) -> Mat4 {
        Mat4::from(self.world_matrix(id))
    }
}
