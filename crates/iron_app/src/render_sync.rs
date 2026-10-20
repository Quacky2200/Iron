//! Feeds the 3D renderer from what a compiled game left in the `iron_render_api` registries.
//!
//! The QuickJS path hands the renderer `render.*` ops; a native game writes typed data (cameras,
//! lights, mesh renderers, materials, textures, meshes). Each frame this turns that data into the
//! same op calls (`WorldViews::apply`, `texture_op`), so there is one renderer path. The
//! components are rebuilt from the registries every frame (cheap: tens of items), which also
//! handles removal. Native objects get script ids through `ScriptScene::adopt`.
//!
//! Wired: cameras, lights, mesh renderers (primitives and script meshes, materials and their
//! maps), textures from files and pixels. Not yet: sprites, line renderers, particles, cloth,
//! models, planes and world panels, mirrors and probes, render textures, `RenderSettings`.

use crate::world_view::WorldViews;
use iron_core::Value;
use iron_scene::ScriptScene;
use std::path::Path;

fn n(x: f64) -> Value {
    Value::Number(x)
}

fn arr(v: &[f64]) -> Value {
    Value::Array(v.iter().copied().map(n).collect())
}

/// A texture handle as an op argument: 0 is none.
fn tex(id: u32) -> Value {
    if id == 0 {
        Value::Null
    } else {
        n(f64::from(id))
    }
}

/// Uploads what the game queued (textures, meshes) and rebuilds the renderer's components.
pub fn sync(views: &mut WorldViews, scene: &mut ScriptScene, root: &Path) {
    resources(views, root);
    views.clear_components();
    for (i, (_, d)) in iron_engine_api::camera::cameras().into_iter().enumerate() {
        let object = scene.adopt(d.object);
        let flags = if d.clear_flags == "skybox" {
            "skybox"
        } else {
            "color"
        };
        let args = vec![
            n(i as f64 + 1.0),
            n(f64::from(object)),
            n(f64::from(d.fov)),
            n(f64::from(d.near)),
            n(f64::from(d.far)),
            arr(&[
                d.background.r,
                d.background.g,
                d.background.b,
                d.background.a,
            ]),
            n(f64::from(d.depth)),
            Value::Bool(d.orthographic),
            n(f64::from(d.orthographic_size)),
            arr(&d.rect),
            Value::String(flags.into()),
            n(f64::from(d.culling_mask)),
            if d.layer_cull_distances.iter().any(|v| *v > 0.0) {
                arr(&d.layer_cull_distances)
            } else {
                Value::Null
            },
            Value::Bool(d.layer_cull_spherical),
            Value::String(d.antialiasing.clone()),
            if d.target_texture.0 == 0 {
                Value::Null
            } else {
                n(f64::from(d.target_texture.0))
            },
            Value::Bool(d.scene_grid),
        ];
        warn(views.apply("camera", &args), "camera");
    }
    for (i, (_, d)) in iron_render_api::lights().into_iter().enumerate() {
        let object = scene.adopt(d.object);
        let args = vec![
            n(i as f64 + 1.0),
            n(f64::from(object)),
            Value::String(d.r#type.clone()),
            arr(&[d.color.r, d.color.g, d.color.b]),
            n(d.intensity),
            n(d.range),
            n(d.spot_angle),
            n(d.inner_spot_angle),
            n(d.halo_size),
            n(d.beam_intensity),
            Value::String(d.shadows.clone()),
            n(d.shadow_strength),
            n(f64::from(d.culling_mask)),
        ];
        warn(views.apply("light", &args), "light");
    }
    for (i, (_, d)) in iron_render_api::mesh_renderers().into_iter().enumerate() {
        let object = scene.adopt(d.object);
        let mesh = match iron_render_api::mesh_info(iron_render_api::MeshRef(d.mesh)) {
            Some(info) => match info.primitive {
                Some(name) => Value::String(name),
                None => n(f64::from(d.mesh)),
            },
            None => continue,
        };
        let Some(m) =
            iron_render_api::with_material(iron_render_api::MaterialRef(d.material), |m| m.clone())
        else {
            continue;
        };
        let extras = Value::Array(vec![
            tex(m.normal_map),
            n(m.normal_scale),
            tex(m.metallic_gloss_map),
            Value::Bool(m.gltf_packing),
            tex(m.occlusion_map),
            n(m.occlusion_strength),
            tex(m.emission_map),
            tex(m.detail_albedo_map),
            arr(&[
                m.detail_albedo_map_scale.x,
                m.detail_albedo_map_scale.y,
                m.detail_albedo_map_offset.x,
                m.detail_albedo_map_offset.y,
            ]),
            if m.has_specular_color {
                arr(&[m.specular_color.r, m.specular_color.g, m.specular_color.b])
            } else {
                Value::Null
            },
            Value::String(m.rendering_mode.clone()),
            n(m.alpha_cutoff),
            Value::Bool(m.double_sided),
        ]);
        let args = vec![
            n(i as f64 + 1.0),
            n(f64::from(object)),
            mesh,
            arr(&[m.color.r, m.color.g, m.color.b, m.color.a]),
            arr(&[m.emission.r, m.emission.g, m.emission.b]),
            n(m.smoothness),
            Value::Bool(m.unlit),
            tex(m.main_texture),
            arr(&[
                m.main_texture_scale.x,
                m.main_texture_scale.y,
                m.main_texture_offset.x,
                m.main_texture_offset.y,
            ]),
            Value::Bool(d.shadow_casting_mode != "off"),
            n(m.metallic),
            extras,
        ];
        warn(views.apply("meshRenderer", &args), "meshRenderer");
    }
}

fn warn(r: Result<(), String>, what: &str) {
    if let Err(e) = r {
        tracing::warn!(target: "render", "{what}: {e}");
    }
}

/// Textures and meshes the game made since the last frame.
fn resources(views: &mut WorldViews, root: &Path) {
    use iron_render_api::{MeshRef, RenderCommand, TextureRef};
    for cmd in iron_render_api::take_commands() {
        match cmd {
            RenderCommand::Texture(t) => {
                let Some(info) = iron_render_api::texture_info(t) else {
                    continue;
                };
                if info.render_texture {
                    tracing::warn!(target: "render", "render textures are not wired for compiled games yet");
                    continue;
                }
                let pixels = info.pixels.as_ref().map_or(Value::Null, |p| {
                    Value::Array(p.iter().map(|b| n(f64::from(*b))).collect())
                });
                let args = vec![
                    n(f64::from(t.0)),
                    info.src.clone().map_or(Value::Null, Value::String),
                    Value::String(info.filter_mode.clone()),
                    Value::String(info.wrap_mode.clone()),
                    Value::Bool(info.mipmaps),
                    n(f64::from(info.width)),
                    n(f64::from(info.height)),
                    pixels,
                    n(f64::from(info.aniso_level)),
                    Value::Bool(info.linear),
                ];
                match views.texture_op("texture", root, &args) {
                    Ok(Some((_, w, h))) => iron_render_api::texture_loaded(t, w, h),
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(target: "render", "texture {}: {e}", t.0);
                        iron_render_api::texture_loaded(t, 0, 0);
                    }
                }
            }
            RenderCommand::TextureRemove(TextureRef(id)) => {
                let _ = views.texture_op("textureRemove", root, &[n(f64::from(id))]);
            }
            RenderCommand::Mesh(m) => {
                let Some(info) = iron_render_api::mesh_info(m) else {
                    continue;
                };
                if info.primitive.is_some() {
                    continue;
                }
                let f = |v: &[f64]| arr(v);
                let tri: Vec<f64> = info.triangles.iter().map(|i| f64::from(*i)).collect();
                let args = vec![
                    n(f64::from(m.0)),
                    f(&info.vertices),
                    f(&info.normals),
                    f(&info.uv),
                    f(&tri),
                    f(&info.joints),
                    f(&info.weights),
                ];
                warn(views.apply("mesh", &args), "mesh");
            }
            RenderCommand::MeshRemove(MeshRef(id)) => {
                warn(
                    views.apply("mesh", &[n(f64::from(id)), Value::Null]),
                    "mesh",
                );
            }
            RenderCommand::Model(_) => {
                tracing::warn!(target: "render", "models are not wired for compiled games yet");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::native::{Bundle, Native};
    use super::*;
    use iron_ui::UiTree;

    // M9-API-620
    #[test]
    fn a_compiled_games_camera_light_and_cube_reach_the_renderers_views() {
        let mut scene = ScriptScene::new();
        let mut ui = UiTree::new(100.0, 100.0);
        let bundle = Bundle {
            source: "class Stage extends Component {\n\
                override onStart(): void {\n\
                  const cam = new GameObject(\"Cam\");\n\
                  cam.addComponent(Camera, { fieldOfView: 50 });\n\
                  cam.transform.position = new Vector3(0, 0, 5);\n\
                  const sun = new GameObject(\"Sun\");\n\
                  sun.addComponent(Light, { type: \"point\", intensity: 2 });\n\
                  const box = new GameObject(\"Box\");\n\
                  box.addComponent(MeshRenderer, { mesh: \"cube\", material: new Material({ color: new Color(1, 0, 0, 1) }) });\n\
                }\n\
              }\n\
              register(Stage);"
                .into(),
            ..Default::default()
        };
        let mut native = Native::load(bundle, &mut scene.scene, &mut ui).unwrap();
        native.frame(&mut scene.scene, &mut ui, 0.016, 16.0);
        let mut views = WorldViews::default();
        sync(&mut views, &mut scene, Path::new("."));
        scene.scene.update_world();
        let v = views.views(&scene, (200, 100), &[]);
        assert_eq!(v.len(), 1, "one camera");
        assert_eq!(v[0].meshes.len(), 1);
        assert_eq!(v[0].meshes[0].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(v[0].lights.len(), 1);
    }
}
