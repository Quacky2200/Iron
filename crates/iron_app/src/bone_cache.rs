//! Skinned renderers' bone matrices (each bone's world matrix times its inverse bind pose), kept
//! between frames: a skeleton's run is worked out again only when one of its bones moved
//! (`Scene::moved_since`), so a culled crowd, whose animators write nothing, costs a check per
//! bone. The parts of one figure share a skeleton and so one run. The list is shared by every
//! view (`WorldView::bones`); the renderer sends only the runs it draws.

use crate::world_view::Skin;
use iron_core::glam::Mat4;
use iron_scene::{ObjectId, ScriptObject, ScriptScene};
use std::collections::HashMap;
use std::sync::Arc;

/// The skinned renderers' skins, counting changes (the cache lays its runs out again).
#[derive(Default)]
pub(crate) struct Skins {
    map: HashMap<u32, Skin>,
    version: u64,
}

impl Skins {
    pub(crate) fn insert(&mut self, id: u32, skin: Skin) {
        if self.map.get(&id) == Some(&skin) {
            return;
        }
        self.map.insert(id, skin);
        self.version += 1;
    }

    pub(crate) fn remove(&mut self, id: &u32) {
        if self.map.remove(id).is_some() {
            self.version += 1;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.version += 1;
    }

    pub(crate) fn retain(&mut self, keep: impl FnMut(&u32, &mut Skin) -> bool) {
        let before = self.map.len();
        self.map.retain(keep);
        if self.map.len() != before {
            self.version += 1;
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&u32, &Skin)> {
        self.map.iter()
    }
}

/// Each skinned renderer's first matrix and count in the list.
pub(crate) type Ranges = HashMap<u32, (u32, u32)>;

/// One skeleton's matrices in the list.
struct Run {
    /// A skin with these bones and bind poses.
    skin: u32,
    start: u32,
    /// Its bones' objects (`None`: no object for the script id yet).
    objects: Vec<Option<ObjectId>>,
    /// The scene epoch the matrices were worked out in.
    built: Option<u64>,
}

#[derive(Default)]
pub(crate) struct BoneCache {
    /// The `Skins::version` the runs were laid out for.
    version: Option<u64>,
    ranges: Ranges,
    runs: Vec<Run>,
    bones: Arc<Vec<Mat4>>,
}

impl BoneCache {
    /// The frame's matrices and each renderer's run in them, after the scene's
    /// `update_world` (stale bones are worked out on the fly).
    pub(crate) fn update(
        &mut self,
        skins: &Skins,
        scene: &ScriptScene,
    ) -> (Arc<Vec<Mat4>>, &Ranges) {
        if self.version != Some(skins.version) {
            self.lay_out(skins, scene);
        }
        let epoch = scene.scene.epoch();
        for run in &mut self.runs {
            let Some(skin) = skins.map.get(&run.skin) else {
                continue;
            };
            for (o, b) in run.objects.iter_mut().zip(&skin.bones) {
                if o.is_none() {
                    *o = scene.object(*b);
                }
            }
            let fresh = run.built.is_some_and(|e| {
                run.objects
                    .iter()
                    .all(|o| o.is_some_and(|o| !scene.scene.moved_since(o, e)))
            });
            if fresh {
                continue;
            }
            let bones = Arc::make_mut(&mut self.bones);
            let out = &mut bones[run.start as usize..run.start as usize + skin.bones.len()];
            for ((m, o), inverse) in out.iter_mut().zip(&run.objects).zip(&skin.bindposes) {
                // A missing bone holds its bind pose.
                let world = o.map_or(inverse.inverse(), |o| {
                    Mat4::from(scene.scene.world_matrix(o))
                });
                *m = world * *inverse;
            }
            run.built = Some(epoch);
        }
        (self.bones.clone(), &self.ranges)
    }

    /// One run per skeleton (skins with the same bones and bind poses share one).
    fn lay_out(&mut self, skins: &Skins, scene: &ScriptScene) {
        self.version = Some(skins.version);
        self.ranges.clear();
        self.runs.clear();
        let mut bones = Vec::new();
        let mut by_ends: HashMap<(ScriptObject, ScriptObject, usize), Vec<usize>> = HashMap::new();
        for (&id, skin) in skins.iter() {
            let n = skin.bones.len() as u32;
            if let (Some(&first), Some(&last)) = (skin.bones.first(), skin.bones.last()) {
                let same = by_ends.entry((first, last, skin.bones.len())).or_default();
                if let Some(&r) = same.iter().find(|&&r| {
                    let other = &skins.map[&self.runs[r].skin];
                    other.bones == skin.bones && other.bindposes == skin.bindposes
                }) {
                    self.ranges.insert(id, (self.runs[r].start, n));
                    continue;
                }
                same.push(self.runs.len());
            }
            let start = bones.len() as u32;
            bones.resize(bones.len() + skin.bones.len(), Mat4::IDENTITY);
            self.ranges.insert(id, (start, n));
            self.runs.push(Run {
                skin: id,
                start,
                objects: skin.bones.iter().map(|b| scene.object(*b)).collect(),
                built: None,
            });
        }
        self.bones = Arc::new(bones);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_core::glam::Vec3;
    use iron_scene::Transform;

    // M5-CUL-07
    #[test]
    fn bone_runs_are_worked_out_again_only_when_a_bone_moved() {
        let mut scene = ScriptScene::new();
        let mut make = |name: &str| {
            let s = scene.create_engine_object(name);
            (s, scene.object(s).unwrap())
        };
        let (a_root, ar) = make("a");
        let (a_arm, aa) = make("a arm");
        let (b_root, br) = make("b");
        scene.scene.set_parent(aa, Some(ar), false);
        let bind = vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::Y)];
        let skin = |bones: Vec<ScriptObject>| Skin {
            bones,
            bindposes: bind.clone(),
        };
        let mut skins = Skins::default();
        skins.insert(1, skin(vec![a_root, a_arm]));
        skins.insert(2, skin(vec![a_root, a_arm]));
        skins.insert(3, skin(vec![b_root, b_root]));
        let mut cache = BoneCache::default();
        let expect = |skins: &Skins,
                      scene: &ScriptScene,
                      bones: &[Mat4],
                      ranges: &HashMap<u32, (u32, u32)>| {
            for (id, s) in skins.iter() {
                let (start, n) = ranges[id];
                for i in 0..n as usize {
                    let o = scene.object(s.bones[i]).unwrap();
                    let want = Mat4::from(scene.scene.world_matrix(o)) * s.bindposes[i];
                    assert_eq!(bones[start as usize + i], want, "skin {id} bone {i}");
                }
            }
        };
        scene.scene.update_world();
        let (bones, ranges) = cache.update(&skins, &scene);
        assert_eq!(bones.len(), 4, "the parts of one figure share a run");
        assert_eq!(ranges[&1], ranges[&2]);
        expect(&skins, &scene, &bones, ranges);
        // Nothing moved: the same matrices, not worked out (a stale value would show).
        let stale = Arc::as_ptr(&bones);
        drop(bones);
        scene.scene.update_world();
        let (bones, _) = cache.update(&skins, &scene);
        assert_eq!(Arc::as_ptr(&bones), stale, "kept in place");
        drop(bones);
        // A's arm moves: right again (and B's run untouched).
        scene
            .scene
            .set_local(aa, Transform::from_position(Vec3::new(2.0, 0.0, 0.0)));
        scene.scene.update_world();
        let (bones, ranges) = cache.update(&skins, &scene);
        expect(&skins, &scene, &bones, ranges);
        drop(bones);
        // B's root moves before the update (stale when asked): still right.
        scene
            .scene
            .set_local(br, Transform::from_position(Vec3::new(0.0, 0.0, 5.0)));
        let (bones, ranges) = cache.update(&skins, &scene);
        expect(&skins, &scene, &bones, ranges);
        drop(bones);
        // A skin removed: laid out again.
        skins.remove(&3);
        scene.scene.update_world();
        let (bones, ranges) = cache.update(&skins, &scene);
        assert_eq!(bones.len(), 2);
        expect(&skins, &scene, &bones, ranges);
    }
}
