//! Animation culling's eyes (`Animator.cullingMode`, `Animation.cullingType`): which renderers
//! under culled animated objects the renderer saw. Each frame the animation update asks about the
//! last frame's drawing (one frame late), then the renderers under culled objects are watched while
//! this frame draws (`MeshInstance::visibility_key`, `WorldRenderer::seen`).

use super::render_sync::SKIN_BASE;
use iron_scene::{ObjectId, ScriptObject, ScriptScene};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct Watch {
    /// Renderers (component ids) whose sight the renderer reports this frame.
    watched: HashSet<u32>,
    /// Those of the frame last drawn, and which of them it saw.
    drawn: HashSet<u32>,
    seen: HashSet<u32>,
}

/// Whether each culled object had a renderer seen; objects with no renderers are seen.
pub(crate) struct Sight(HashMap<ObjectId, bool>);

impl iron_animation::Visibility for Sight {
    fn visible(&mut self, _: &iron_scene::Scene, root: ObjectId) -> bool {
        self.0.get(&root).copied().unwrap_or(true)
    }
}

impl Watch {
    /// The key a renderer's mesh is drawn with (0: not watched).
    pub(crate) fn key(&self, renderer: u32) -> u32 {
        if self.watched.contains(&renderer) {
            renderer + 1
        } else {
            0
        }
    }

    /// The renderer's report on the frame just drawn (keys from [`Self::key`]), passed on to
    /// `Renderer.isVisible`.
    pub(crate) fn note_seen(&mut self, keys: &[u32]) {
        self.drawn.clone_from(&self.watched);
        self.seen.clear();
        self.seen.extend(keys.iter().map(|k| k - 1));
        iron_render_api::set_seen_renderers(self.seen.iter().map(|&id| renderer_id(id)));
    }

    /// What the last frame saw of each of `roots` (through any renderer on it or below it), and
    /// watches those renderers this frame, and those scripts asked `isVisible` of. A renderer
    /// not watched last frame counts as seen.
    pub(crate) fn sight(
        &mut self,
        scene: &ScriptScene,
        renderers: impl Iterator<Item = (u32, ScriptObject)>,
        roots: &[ObjectId],
    ) -> Sight {
        self.watched.clear();
        self.watched.extend(
            iron_render_api::watched_renderers()
                .into_iter()
                .map(|r| match r {
                    iron_render_api::RendererId::Mesh(id) => id,
                    iron_render_api::RendererId::Skinned(id) => SKIN_BASE + id,
                }),
        );
        let mut out = HashMap::new();
        if roots.is_empty() {
            return Sight(out);
        }
        let roots: HashSet<ObjectId> = roots.iter().copied().collect();
        for (id, object) in renderers {
            let Some(go) = scene.object(object) else {
                continue;
            };
            let seen = !self.drawn.contains(&id) || self.seen.contains(&id);
            let mut at = Some(go);
            while let Some(a) = at {
                if roots.contains(&a) {
                    self.watched.insert(id);
                    *out.entry(a).or_insert(false) |= seen;
                }
                at = scene.scene.parent(a);
            }
        }
        Sight(out)
    }
}

/// A renderer table id as its component (skinned renderers from `SKIN_BASE`).
fn renderer_id(id: u32) -> iron_render_api::RendererId {
    match id.checked_sub(SKIN_BASE) {
        Some(skinned) => iron_render_api::RendererId::Skinned(skinned),
        None => iron_render_api::RendererId::Mesh(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_animation::Visibility;

    // M5-ANI-13
    #[test]
    fn a_culled_object_is_seen_through_any_renderer_below_it_one_frame_late() {
        let mut scene = ScriptScene::new();
        let figure = scene.create_engine_object("Figure");
        let body = scene.create_engine_object("Body");
        let empty = scene.create_engine_object("Empty");
        let (f, b, e) = (
            scene.object(figure).unwrap(),
            scene.object(body).unwrap(),
            scene.object(empty).unwrap(),
        );
        scene.scene.set_parent(b, Some(f), false);
        let mut w = Watch::default();
        let renderers = || [(7u32, body)].into_iter();
        // Not watched before: seen (no report yet); and now watched.
        let mut s = w.sight(&scene, renderers(), &[f, e]);
        assert!(s.visible(&scene.scene, f));
        assert!(s.visible(&scene.scene, e), "no renderers: always seen");
        assert_eq!(w.key(7), 8);
        assert_eq!(w.key(9), 0);
        // The frame drew without seeing it.
        w.note_seen(&[]);
        let mut s = w.sight(&scene, renderers(), &[f]);
        assert!(!s.visible(&scene.scene, f));
        w.note_seen(&[8]);
        assert!(w.sight(&scene, renderers(), &[f]).visible(&scene.scene, f));
        // No culled objects: nothing watched.
        w.sight(&scene, renderers(), &[]);
        assert_eq!(w.key(7), 0);
    }

    // M5-CUL-04
    #[test]
    fn renderers_scripts_ask_is_visible_of_are_watched_and_told_what_was_seen() {
        iron_render_api::reset();
        let scene = ScriptScene::new();
        let mut w = Watch::default();
        let (mesh, skinned) = (
            iron_render_api::MeshRendererRef(4),
            iron_render_api::SkinnedMeshRendererRef(2),
        );
        let skin_id = SKIN_BASE + 2;
        assert!(!mesh.is_visible() && !skinned.is_visible());
        // Asked: watched with no animator culling at all.
        w.sight(&scene, std::iter::empty(), &[]);
        assert_eq!(w.key(4), 5);
        assert_eq!(w.key(skin_id), skin_id + 1);
        w.note_seen(&[skin_id + 1]);
        assert!(!mesh.is_visible());
        assert!(skinned.is_visible());
        w.note_seen(&[5]);
        assert!(mesh.is_visible() && !skinned.is_visible());
        iron_render_api::reset();
    }
}
