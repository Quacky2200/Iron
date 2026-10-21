//! The scene as scripts drive it: `scene.*` ops with script-chosen object ids (like
//! UI nodes and physics bodies), mapped to [`ObjectId`]s.
//!
//! Scripts mirror each object's local transform, so only changes the *engine* makes
//! (physics moving a body, ...) go back to them, as one flat batch per tick from
//! [`ScriptScene::take_batch`]. Objects the engine creates for scripts (the starting
//! components' objects) take ids from 2^30 up, so they never meet script ids.

use std::collections::HashMap;

use glam::{Quat, Vec3};
use iron_core::Value;

use crate::{ObjectId, Scene, Transform};

/// What a script calls a GameObject.
pub type ScriptObject = u32;

/// The first id of objects the engine creates.
pub const ENGINE_IDS: u32 = 1 << 30;

/// Something a `scene.*` op asks of the rest of the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum SceneEvent {
    /// `addComponent`: the script created `component` of `class` on `object`.
    ComponentAdded {
        object: ScriptObject,
        component: u32,
        class: String,
    },
    ComponentRemoved(u32),
    /// These objects were destroyed (the object first, then its descendants); their
    /// components and physics go with them.
    Destroyed(Vec<ScriptObject>),
}

#[derive(Default)]
pub struct ScriptScene {
    pub scene: Scene,
    ids: HashMap<ScriptObject, ObjectId>,
    scripts: HashMap<ObjectId, ScriptObject>,
    /// Objects the engine moved since the last batch.
    engine_moved: Vec<ObjectId>,
    /// Objects scripts moved (or re-parented) since [`ScriptScene::take_script_moved`].
    script_moved: Vec<ObjectId>,
    next_engine_id: u32,
}

impl ScriptScene {
    pub fn new() -> Self {
        Self {
            next_engine_id: ENGINE_IDS,
            ..Default::default()
        }
    }

    pub fn object(&self, id: ScriptObject) -> Option<ObjectId> {
        self.ids.get(&id).copied()
    }

    pub fn script_id(&self, id: ObjectId) -> Option<ScriptObject> {
        self.scripts.get(&id).copied()
    }

    /// Whether a script object is active (enabled with its parents); unknown ids aren't.
    pub fn active(&self, id: ScriptObject) -> bool {
        self.object(id).is_some_and(|o| self.scene.active(o))
    }

    /// A script object's layer (0 for unknown ids).
    pub fn layer(&self, id: ScriptObject) -> u8 {
        self.object(id).map_or(0, |o| self.scene.layer(o))
    }

    fn insert(&mut self, script: ScriptObject, name: &str) -> ObjectId {
        if let Some(old) = self.ids.remove(&script) {
            for gone in self.scene.destroy(old) {
                self.scripts.remove(&gone);
            }
        }
        let id = self.scene.create(name);
        self.ids.insert(script, id);
        self.scripts.insert(id, script);
        id
    }

    /// The script id of an object made outside scripts (a compiled game's), giving it one on the
    /// first call. The renderer looks objects up by script id.
    pub fn adopt(&mut self, id: ObjectId) -> ScriptObject {
        if let Some(s) = self.scripts.get(&id) {
            return *s;
        }
        let script = self.next_engine_id;
        self.next_engine_id += 1;
        self.ids.insert(script, id);
        self.scripts.insert(id, script);
        script
    }

    /// Forgets every id (the scene was replaced: a compiled game reloaded).
    pub fn clear_ids(&mut self) {
        self.ids.clear();
        self.scripts.clear();
    }

    /// Creates a root object for the engine's own use (the starting components'),
    /// returning the id scripts know it by.
    pub fn create_engine_object(&mut self, name: &str) -> ScriptObject {
        let script = self.next_engine_id;
        self.next_engine_id += 1;
        self.insert(script, name);
        script
    }

    /// Moves an object from the engine side (physics): scripts hear about it in the
    /// next batch.
    pub fn set_world_from_engine(&mut self, id: ScriptObject, position: Vec3, rotation: Quat) {
        let Some(o) = self.object(id) else { return };
        self.scene.set_world(o, position, rotation);
        if !self.engine_moved.contains(&o) {
            self.engine_moved.push(o);
        }
    }

    /// Runs one `scene.*` op (the name without the prefix).
    pub fn apply(&mut self, name: &str, args: &[Value]) -> Result<Option<SceneEvent>, String> {
        let a = Args(args);
        match name {
            // `create id name parent|null [p] [q] [s]`
            "create" => {
                let script = a.id(0)?;
                let parent = match a.opt_id(2) {
                    Some(p) => Some(self.object(p).ok_or_else(|| format!("no object {p}"))?),
                    None => None,
                };
                let id = self.insert(script, a.str(1).unwrap_or("GameObject"));
                if let Some(p) = parent {
                    self.scene.set_parent(id, Some(p), false);
                }
                self.scene.set_local(id, a.transform(3)?);
                self.script_moved.push(id);
            }
            "destroy" => {
                let Some(id) = self.object(a.id(0)?) else {
                    return Ok(None);
                };
                let gone: Vec<ScriptObject> = self
                    .scene
                    .destroy(id)
                    .into_iter()
                    .filter_map(|o| {
                        let s = self.scripts.remove(&o)?;
                        self.ids.remove(&s);
                        Some(s)
                    })
                    .collect();
                self.engine_moved.retain(|o| self.scene.contains(*o));
                return Ok(Some(SceneEvent::Destroyed(gone)));
            }
            // `parent id parent|null`: keeps the local transform (scripts send the new
            // local transform after, when they keep the world one).
            "parent" => {
                let id = self.require(a.id(0)?)?;
                let parent = match a.opt_id(1) {
                    Some(p) => Some(self.require(p)?),
                    None => None,
                };
                if !self.scene.set_parent(id, parent, false) {
                    return Err("can't parent an object under itself".into());
                }
                self.script_moved.push(id);
            }
            // `local id [p] [q] [s]`
            "local" => {
                let id = self.require(a.id(0)?)?;
                let t = a.transform(1)?;
                self.scene.set_local(id, t);
                self.script_moved.push(id);
            }
            // `locals [id px py pz qx qy qz qw sx sy sz ...]`: many `local`s in one op (what
            // scripts send each turn); objects already gone are skipped.
            "locals" => {
                let Some(Value::Array(flat)) = args.first() else {
                    return Err("locals needs [id, position, rotation, scale, ...]".into());
                };
                for chunk in flat.as_chunks::<11>().0 {
                    let n = chunk.each_ref().map(|v| v.as_f64().unwrap_or(0.0));
                    if !(n[0].is_finite() && n[0] >= 0.0) {
                        continue;
                    }
                    let Some(id) = self.object(n[0] as u32) else {
                        continue;
                    };
                    let f = |i: usize| n[i] as f32;
                    let q = Quat::from_xyzw(f(4), f(5), f(6), f(7));
                    let t = Transform {
                        position: Vec3::new(f(1), f(2), f(3)),
                        rotation: if q.length_squared() < 1e-12 {
                            Quat::IDENTITY
                        } else {
                            q.normalize()
                        },
                        scale: Vec3::new(f(8), f(9), f(10)),
                    };
                    self.scene.set_local(id, t);
                    self.script_moved.push(id);
                }
            }
            "enabled" => {
                let id = self.require(a.id(0)?)?;
                self.scene
                    .set_enabled(id, matches!(args.get(1), Some(Value::Bool(true))));
            }
            "name" => {
                let id = self.require(a.id(0)?)?;
                self.scene.set_name(id, a.str(1)?);
            }
            // `layer id 0..31`
            "layer" => {
                let id = self.require(a.id(0)?)?;
                let layer = args
                    .get(1)
                    .and_then(Value::as_f64)
                    .filter(|l| (0.0..32.0).contains(l))
                    .ok_or("layer must be 0 to 31")?;
                self.scene.set_layer(id, layer as u8);
            }
            // `addComponent object component class`
            "addComponent" => {
                let object = a.id(0)?;
                self.require(object)?;
                return Ok(Some(SceneEvent::ComponentAdded {
                    object,
                    component: a.id(1)?,
                    class: a.str(2)?.to_owned(),
                }));
            }
            "removeComponent" => return Ok(Some(SceneEvent::ComponentRemoved(a.id(0)?))),
            other => return Err(format!("unknown scene op `{other}`")),
        }
        Ok(None)
    }

    /// Objects whose world transform scripts changed since the last call (the ones they
    /// moved and everything under them), for physics to follow.
    pub fn take_script_moved(&mut self) -> Vec<ScriptObject> {
        let mut out = Vec::new();
        let mut stack = std::mem::take(&mut self.script_moved);
        while let Some(o) = stack.pop() {
            if let Some(script) = self.script_id(o)
                && !out.contains(&script)
            {
                out.push(script);
                stack.extend(self.scene.children(o));
            }
        }
        out
    }

    fn require(&self, id: ScriptObject) -> Result<ObjectId, String> {
        self.object(id).ok_or_else(|| format!("no object {id}"))
    }

    /// Objects the engine moved since the last call, as `__iron_scene(moved)`'s flat
    /// array: `id, px, py, pz, qx, qy, qz, qw, sx, sy, sz` (local) each.
    pub fn take_batch(&mut self) -> Vec<Value> {
        let mut out = Vec::with_capacity(self.engine_moved.len() * 11);
        for o in std::mem::take(&mut self.engine_moved) {
            let (Some(script), Some(t)) = (self.script_id(o), self.scene.local(o)) else {
                continue;
            };
            out.push(script.into());
            let nums = t
                .position
                .to_array()
                .into_iter()
                .chain(t.rotation.to_array())
                .chain(t.scale.to_array());
            out.extend(nums.map(|v| Value::Number(v as f64)));
        }
        out
    }
}

struct Args<'a>(&'a [Value]);

impl Args<'_> {
    /// Ids go through `f64`: `f32` can't hold integers past 2^24 (script component
    /// and engine object ids are bigger).
    fn id(&self, i: usize) -> Result<u32, String> {
        self.0
            .get(i)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0 && *v <= u32::MAX as f64)
            .map(|v| v as u32)
            .ok_or_else(|| format!("argument {i} must be an id"))
    }

    fn opt_id(&self, i: usize) -> Option<u32> {
        self.id(i).ok()
    }

    fn str(&self, i: usize) -> Result<&str, String> {
        self.0
            .get(i)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("argument {i} must be a string"))
    }

    fn floats<const N: usize>(&self, i: usize) -> Option<[f32; N]> {
        let Some(Value::Array(items)) = self.0.get(i) else {
            return None;
        };
        let mut out = [0.0; N];
        for (k, slot) in out.iter_mut().enumerate() {
            *slot = items
                .get(k)
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .filter(|v| v.is_finite())?;
        }
        Some(out)
    }

    /// `[p] [q] [s]` from argument `i`, each optional (identity parts).
    fn transform(&self, i: usize) -> Result<Transform, String> {
        let mut t = Transform::IDENTITY;
        if let Some(p) = self.floats::<3>(i) {
            t.position = Vec3::from_array(p);
        }
        if let Some(q) = self.floats::<4>(i + 1) {
            let q = Quat::from_array(q);
            if q.length_squared() < 1e-12 {
                return Err("rotation is not a quaternion".into());
            }
            t.rotation = q.normalize();
        }
        if let Some(s) = self.floats::<3>(i + 2) {
            t.scale = Vec3::from_array(s);
        }
        Ok(t)
    }
}
