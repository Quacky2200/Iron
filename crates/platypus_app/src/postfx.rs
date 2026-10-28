// SPDX-License-Identifier: Apache-2.0

//! Post-process passes from scripts (`PostProcess`, in `iron_render_api`) to the renderer's
//! [`PostEffect`]s: the passes are sent when the registry changes and each camera's view gets
//! the ones aimed at it (or at every camera), in order.

use std::collections::HashMap;
use std::sync::Arc;

use iron_render::{POST_PARAMS, PostEffect, PostParamType, PostShader};
use iron_render_api::{MAX_PARAMS, ParamType, PostPass, ShaderDef};

const _: () = assert!(MAX_PARAMS == POST_PARAMS);

#[derive(Default)]
pub struct PostSet {
    /// Each pass with the camera it applies to (0: all), in drawing order.
    passes: Vec<(u32, PostEffect)>,
    /// The renderer's shader for each of the API's, by its address (the registry shares one
    /// `Arc` per shader, so a pass keeps the same pipeline from frame to frame). The API's
    /// `Arc` is held too, so an address is never reused by another shader.
    shaders: HashMap<usize, (Arc<ShaderDef>, Arc<PostShader>)>,
}

fn param_type(ty: ParamType) -> PostParamType {
    match ty {
        ParamType::Float => PostParamType::Float,
        ParamType::Vec2 => PostParamType::Vec2,
        ParamType::Vec3 => PostParamType::Vec3,
        ParamType::Vec4 => PostParamType::Vec4,
    }
}

impl PostSet {
    /// Replaces the passes (the registry's enabled ones, in order).
    pub fn set(&mut self, passes: Vec<PostPass>) {
        let mut shaders = HashMap::new();
        self.passes = passes
            .into_iter()
            .map(|p| {
                let key = Arc::as_ptr(&p.shader) as usize;
                let (_, shader) = shaders.entry(key).or_insert_with(|| {
                    self.shaders.remove(&key).unwrap_or_else(|| {
                        let d = &p.shader;
                        let params = d
                            .params
                            .iter()
                            .map(|(n, t)| (n.clone(), param_type(*t)))
                            .collect();
                        let shader = PostShader::new(&d.name, &d.source, params, d.passes, d.depth);
                        (p.shader.clone(), Arc::new(shader))
                    })
                });
                // `set_passes` and `set_uses_depth` replace the API's shader with a new one, so
                // the renderer's is made again for it.
                let mut effect = PostEffect::new(shader.clone());
                effect.params = p.values;
                (p.camera, effect)
            })
            .collect();
        self.shaders = shaders;
    }

    /// The passes for a camera's view, in order.
    pub fn for_camera(&self, camera: u32) -> Vec<PostEffect> {
        self.passes
            .iter()
            .filter(|(c, _)| *c == 0 || *c == camera)
            .map(|(_, e)| e.clone())
            .collect()
    }
}
