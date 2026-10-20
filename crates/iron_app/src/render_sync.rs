//! Feeds the 3D renderer from what a compiled game left in the `iron_render_api` registries.
//!
//! The QuickJS path hands the renderer `render.*` ops; a native game writes typed data (cameras,
//! lights, mesh renderers, materials, textures, meshes, sprites, ...). Each frame this turns that
//! data into the same op calls (`WorldViews::apply`, `texture_op`), so there is one renderer path.
//! The components are rebuilt from the registries every frame (cheap: tens of items), which also
//! handles removal. Native objects get script ids through `ScriptScene::adopt`.
//!
//! Wired here: cameras (and render textures, gizmos), lights, mesh and skinned mesh renderers
//! (primitives, script meshes, model meshes, materials and their maps), sprites, 2D lines, planes,
//! mirrors, reflection probes, LOD groups, line renderers (their tube meshes), particle systems,
//! models (glTF files), textures from files and pixels, `RenderSettings` and `QualitySettings`.
//! What needs the app (world panels, cloth, fonts and stylesheets) comes back in [`Extras`].

use crate::world_view::WorldViews;
use iron_core::Value;
use iron_render_api::MeshRef;
use iron_scene::ScriptScene;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Mesh and texture ids from this number up were made by the host for a model (`models.rs`):
/// they are not in the registries.
const ENGINE_IDS: u32 = crate::models::ENGINE_IDS;

/// Skinned renderers share the renderer table with mesh renderers: their ids start here.
const SKIN_BASE: u32 = 1 << 24;

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

fn rgba(c: iron_engine_api::color::Rgba) -> Value {
    arr(&[c.r, c.g, c.b, c.a])
}

fn rgb(c: iron_engine_api::color::Rgba) -> Value {
    arr(&[c.r, c.g, c.b])
}

/// What the sync remembers between frames.
#[derive(Default)]
struct Cache {
    /// The last presented gizmo frame: camera id and its lines, kept until the next one.
    gizmos: Option<(u32, Vec<f64>)>,
    /// What each probe was last sent, and its render requests (they render again on a change).
    probes: HashMap<u32, (Vec<Value>, f64)>,
    /// What each particle system was last sent.
    particles: HashMap<u32, Vec<Value>>,
    /// Cloth rebuild counters already handed to the app.
    cloths: HashMap<u32, f64>,
    /// Inverse bind poses of model meshes (flat, 16 per bone), by mesh id.
    bindposes: HashMap<u32, Vec<f64>>,
    settings_version: Option<u64>,
    quality_version: Option<u64>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

fn cache<R>(f: impl FnOnce(&mut Cache) -> R) -> R {
    CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// Forgets what was sent (a compiled game reloaded: its registries start over).
pub fn reset() {
    cache(|c| *c = Cache::default());
}

/// What the sync leaves to the app: things that live outside the 3D world views.
#[derive(Default)]
pub struct Extras {
    /// `cloth.*` ops (`cloth` to set one up, `remove`), for `ClothSystem::apply`.
    pub cloths: Vec<(String, Vec<Value>)>,
    /// `render.panel` args for every live world panel (the panel's own id first).
    pub panels: Vec<Vec<Value>>,
}

/// The mesh argument of a renderer op: a primitive's name, or the id of an uploaded mesh.
fn mesh_arg(mesh: u32) -> Option<Value> {
    if mesh >= ENGINE_IDS {
        return Some(n(f64::from(mesh)));
    }
    let info = iron_render_api::mesh_info(MeshRef(mesh))?;
    Some(match info.primitive {
        Some(name) => Value::String(name),
        None => n(f64::from(mesh)),
    })
}

/// The `meshRenderer` op's arguments for a renderer id, its object and mesh, and a material.
fn renderer_args(
    id: u32,
    object: u32,
    mesh: Value,
    material: u32,
    shadow_casting_mode: &str,
    skin: Option<(Vec<u32>, Vec<f64>)>,
) -> Option<Vec<Value>> {
    let m = iron_render_api::with_material(iron_render_api::MaterialRef(material), |m| m.clone())?;
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
    let mut args = vec![
        n(f64::from(id)),
        n(f64::from(object)),
        mesh,
        rgba(m.color),
        rgb(m.emission),
        n(m.smoothness),
        Value::Bool(m.unlit),
        tex(m.main_texture),
        arr(&[
            m.main_texture_scale.x,
            m.main_texture_scale.y,
            m.main_texture_offset.x,
            m.main_texture_offset.y,
        ]),
        Value::Bool(shadow_casting_mode != "off"),
        n(m.metallic),
        extras,
    ];
    if let Some((bones, bindposes)) = skin {
        args.push(Value::Array(
            bones.into_iter().map(|b| n(f64::from(b))).collect(),
        ));
        args.push(arr(&bindposes));
    }
    Some(args)
}

/// Uploads what the game queued (textures, meshes, models) and rebuilds the renderer's
/// components. Returns what the app has to apply itself.
pub fn sync(views: &mut WorldViews, scene: &mut ScriptScene, root: &Path) -> Extras {
    let mut extras = Extras::default();
    resources(views, scene, root);
    // Line renderers redraw their tubes (they are mesh renderers on the same objects).
    iron_render_api::pre_render_lines();
    resources(views, scene, root);
    views.clear_components();
    settings(views);
    for (r, d) in iron_engine_api::camera::cameras() {
        let object = scene.adopt(d.object);
        let flags = if d.clear_flags == "skybox" {
            "skybox"
        } else {
            "color"
        };
        let args = vec![
            n(f64::from(r.0)),
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
    // Gizmo lines belong to the camera they were presented for, until the next frame of them.
    if let Some(frame) = iron_engine_api::gizmos::take_frame() {
        let mut flat = Vec::with_capacity(frame.lines.len() * 10);
        for l in &frame.lines {
            flat.extend([
                l.from.x, l.from.y, l.from.z, l.to.x, l.to.y, l.to.z, l.color.r, l.color.g,
                l.color.b, l.color.a,
            ]);
        }
        cache(|c| c.gizmos = Some((frame.camera.0, flat)));
    }
    if let Some((camera, flat)) = cache(|c| c.gizmos.clone()) {
        // A camera that is gone has nothing to show them on.
        let _ = views.apply("gizmos", &[n(f64::from(camera)), arr(&flat)]);
    }
    for (r, d) in iron_render_api::lights() {
        let object = scene.adopt(d.object);
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            Value::String(d.r#type.clone()),
            rgb(d.color),
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
    for (r, d) in iron_render_api::mesh_renderers() {
        let object = scene.adopt(d.object);
        let Some(mesh) = mesh_arg(d.mesh) else {
            continue;
        };
        if let Some(args) =
            renderer_args(r.0, object, mesh, d.material, &d.shadow_casting_mode, None)
        {
            warn(views.apply("meshRenderer", &args), "meshRenderer");
        }
    }
    for (r, d) in iron_render_api::skinned_mesh_renderers() {
        let object = scene.adopt(d.object);
        let Some(mesh) = mesh_arg(d.mesh) else {
            continue;
        };
        let bindposes = if d.mesh >= ENGINE_IDS {
            cache(|c| c.bindposes.get(&d.mesh).cloned()).unwrap_or_default()
        } else {
            iron_render_api::mesh_info(MeshRef(d.mesh))
                .map(|i| i.bindposes)
                .unwrap_or_default()
        };
        let bones: Vec<u32> = d.bones.iter().map(|b| scene.adopt(*b)).collect();
        if bindposes.len() != bones.len() * 16 {
            continue;
        }
        if let Some(args) = renderer_args(
            SKIN_BASE + r.0,
            object,
            mesh,
            d.material,
            &d.shadow_casting_mode,
            Some((bones, bindposes)),
        ) {
            warn(views.apply("meshRenderer", &args), "skinnedMeshRenderer");
        }
    }
    let mut kept = Kept::default();
    for (r, d) in iron_render_api::sprite_renderers() {
        let object = scene.adopt(d.object);
        let Some(s) =
            iron_render_api::with_sprite(iron_render_api::SpriteRef(d.sprite), |s| s.clone())
        else {
            continue;
        };
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            n(f64::from(s.texture)),
            if s.has_rect {
                arr(&s.rect)
            } else {
                Value::Null
            },
            arr(&[s.pivot.x, s.pivot.y]),
            n(s.pixels_per_unit),
            rgba(d.color),
            Value::Bool(d.flip_x),
            Value::Bool(d.flip_y),
            n(f64::from(d.sorting_order)),
        ];
        kept.sprites.insert(r.0);
        warn(views.apply("sprite", &args), "sprite");
    }
    for (r, d) in iron_render_api::line_renderer2ds() {
        let object = scene.adopt(d.object);
        let flat: Vec<f64> = d.points.iter().flat_map(|p| [p.x, p.y]).collect();
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            n(d.width),
            rgba(d.color),
            Value::Bool(d.r#loop),
            arr(&flat),
        ];
        kept.strokes.insert(r.0);
        warn(views.apply("stroke", &args), "stroke");
    }
    for (r, d) in iron_render_api::plane_renderers() {
        let object = scene.adopt(d.object);
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            arr(&[d.size.x, d.size.y]),
            rgba(d.color),
        ];
        kept.planes.insert(r.0);
        warn(views.apply("plane", &args), "plane");
    }
    for (r, d) in iron_render_api::mirrors() {
        let object = scene.adopt(d.object);
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            if d.has_normal {
                arr(&[d.normal.x, d.normal.y, d.normal.z])
            } else {
                Value::Null
            },
            n(f64::from(d.culling_mask)),
        ];
        kept.mirrors.insert(r.0);
        warn(views.apply("mirror", &args), "mirror");
    }
    for (r, d) in iron_render_api::reflection_probes() {
        let object = scene.adopt(d.object);
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            arr(&[d.size.x, d.size.y, d.size.z]),
            n(d.resolution),
            Value::Bool(d.box_projection),
            Value::Bool(d.refresh_mode == "everyFrame"),
            n(f64::from(d.culling_mask)),
        ];
        kept.probes.insert(r.0);
        // A probe renders when it is new, changed, or asked to render again.
        let (changed, again) = cache(|c| {
            let old = c.probes.get(&r.0);
            let changed = old.is_none_or(|(a, _)| *a != args);
            let again = old.is_some_and(|(_, k)| *k != d.render_requests);
            c.probes.insert(r.0, (args.clone(), d.render_requests));
            (changed, again)
        });
        if changed {
            warn(views.apply("probe", &args), "probe");
        } else if again {
            warn(views.apply("probeRender", &[n(f64::from(r.0))]), "probe");
        }
    }
    for (r, d) in iron_render_api::lod_groups() {
        let object = scene.adopt(d.object);
        let heights: Vec<f64> = d.lods.iter().map(|l| l.0).collect();
        let levels = Value::Array(
            d.lods
                .iter()
                .map(|l| Value::Array(l.1.iter().map(|id| n(f64::from(*id))).collect()))
                .collect(),
        );
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            n(d.size),
            arr(&heights),
            levels,
            n(f64::from(d.forced)),
        ];
        kept.lods.insert(r.0);
        warn(views.apply("lodGroup", &args), "lodGroup");
    }
    particles(views, scene, &mut kept);
    cloths(scene, &mut extras);
    for (r, d) in iron_render_api::world_panels() {
        let object = scene.adopt(d.object);
        extras.panels.push(vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            arr(&[d.width, d.height]),
            n(d.pixels_per_meter),
            n(d.resolution),
            rgba(d.tint),
            Value::Bool(d.interactive),
        ]);
    }
    views.keep_components(&kept.planes, &kept.sprites, &kept.strokes, &kept.mirrors);
    views.keep_more(&kept.probes, &kept.lods, &kept.particles);
    cache(|c| {
        c.probes.retain(|id, _| kept.probes.contains(id));
        c.particles.retain(|id, _| kept.particles.contains(id));
    });
    extras
}

/// The ids of the components that exist this frame, per kind (what is not here is dropped).
#[derive(Default)]
struct Kept {
    planes: HashSet<u32>,
    sprites: HashSet<u32>,
    strokes: HashSet<u32>,
    mirrors: HashSet<u32>,
    probes: HashSet<u32>,
    lods: HashSet<u32>,
    particles: HashSet<u32>,
}

/// `RenderSettings` and `QualitySettings`, when they changed.
fn settings(views: &mut WorldViews) {
    let v = iron_render_api::version();
    if cache(|c| c.settings_version.replace(v)) != Some(v) {
        let s = iron_render_api::settings();
        warn(
            views.apply("ambient", &[n(0.0), rgb(s.ambient_light)]),
            "ambient",
        );
        let sky = match &s.skybox {
            None => vec![n(0.0), Value::Null],
            Some(k) => vec![
                n(0.0),
                rgb(k.zenith),
                rgb(k.horizon),
                rgb(k.ground),
                k.sun_direction
                    .map_or(Value::Null, |d| arr(&[d.x, d.y, d.z])),
                rgb(k.sun_color),
                n(k.sun_size),
                n(s.reflection_intensity),
                Value::Bool(s.ambient_mode == "skybox"),
            ],
        };
        warn(views.apply("sky", &sky), "sky");
        let bloom = match &s.bloom {
            None => vec![n(0.0), Value::Null],
            Some(b) => vec![n(0.0), n(b.threshold), n(b.intensity), n(b.scatter)],
        };
        warn(views.apply("bloom", &bloom), "bloom");
        warn(
            views.apply(
                "post",
                &[n(0.0), n(s.post_exposure), Value::String(s.tonemapping)],
            ),
            "post",
        );
    }
    let v = iron_engine_api::screen::quality_version();
    if cache(|c| c.quality_version.replace(v)) != Some(v) {
        let q = iron_engine_api::screen::quality();
        warn(
            views.apply(
                "quality",
                &[
                    n(0.0),
                    n(f64::from(q.anti_aliasing)),
                    n(q.render_scale),
                    Value::String(q.shadows.clone()),
                    n(if q.soft_shadow_quality == "high" {
                        2.0
                    } else {
                        1.0
                    }),
                    n(q.lod_bias),
                    n(f64::from(q.v_sync_count)),
                    Value::String(q.anisotropic_filtering.clone()),
                    n(f64::from(q.global_texture_mipmap_limit)),
                ],
            ),
            "quality",
        );
        warn(
            views.apply(
                "shadowSettings",
                &[
                    n(0.0),
                    n(q.shadow_distance),
                    n(f64::from(q.shadow_resolution)),
                    n(f64::from(q.shadow_cascades)),
                ],
            ),
            "shadowSettings",
        );
    }
}

/// `[key, value, ...]` of a particle system, as the TypeScript's `pairs()` makes them.
fn particle_pairs(d: &iron_render_api::ParticleData) -> Vec<Value> {
    let range = |m: [f64; 2]| {
        if m[0] == m[1] { n(m[0]) } else { arr(&m) }
    };
    let mut colors = vec![
        d.start_color.r,
        d.start_color.g,
        d.start_color.b,
        d.start_color.a,
    ];
    if let Some(c) = d.start_color2 {
        colors.extend([c.r, c.g, c.b, c.a]);
    }
    let shape = match d.shape_type.as_str() {
        "point" => vec![Value::String("point".into())],
        "cone" => vec![
            Value::String("cone".into()),
            n(d.shape_angle),
            n(d.shape_radius),
        ],
        "box" => vec![
            Value::String("box".into()),
            n(d.shape_size.x),
            n(d.shape_size.y),
            n(d.shape_size.z),
        ],
        other => vec![Value::String(other.into()), n(d.shape_radius)],
    };
    let kv = |k: &str, v: Value| [Value::String(k.into()), v];
    let mut out: Vec<Value> = Vec::new();
    for pair in [
        kv("duration", n(d.duration)),
        kv("looping", Value::Bool(d.r#loop)),
        kv("startDelay", n(d.start_delay)),
        kv("lifetime", range(d.start_lifetime)),
        kv("speed", range(d.start_speed)),
        kv("size", range(d.start_size)),
        kv("rotation", range(d.start_rotation)),
        kv("angularVelocity", range(d.angular_velocity)),
        kv("color", arr(&colors)),
        kv("gravity", n(d.gravity_modifier)),
        kv("drag", n(d.drag)),
        kv("worldSpace", Value::Bool(d.simulation_space == "world")),
        kv("maxParticles", n(f64::from(d.max_particles))),
        kv("rate", n(d.rate_over_time)),
        kv(
            "bursts",
            arr(&d.bursts.iter().flatten().copied().collect::<Vec<_>>()),
        ),
        kv("shape", Value::Array(shape)),
        kv("texture", tex(d.texture)),
        kv("additive", Value::Bool(d.blend_mode == "additive")),
        kv("seed", n(f64::from(d.random_seed))),
    ] {
        out.extend(pair);
    }
    if let Some(k) = &d.size_over_lifetime {
        out.extend(kv(
            "sizeOverLifetime",
            arr(&k.iter().flatten().copied().collect::<Vec<_>>()),
        ));
    }
    if let Some(k) = &d.color_over_lifetime {
        let flat: Vec<f64> = k
            .iter()
            .flat_map(|(t, c)| [*t, c.r, c.g, c.b, c.a])
            .collect();
        out.extend(kv("colorOverLifetime", arr(&flat)));
    }
    if let Some(s) = d.texture_sheet {
        out.extend(kv("sheet", arr(&s)));
    }
    out
}

/// Particle systems: settings when they changed, then what scripts asked of them (`play`, ...).
fn particles(views: &mut WorldViews, scene: &mut ScriptScene, kept: &mut Kept) {
    for (r, d) in iron_render_api::particle_systems() {
        let object = scene.adopt(d.object);
        let pairs = particle_pairs(&d);
        kept.particles.insert(r.0);
        if cache(|c| c.particles.get(&r.0) == Some(&pairs)) {
            continue;
        }
        let first = cache(|c| c.particles.insert(r.0, pairs.clone())).is_none();
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            Value::Array(pairs),
            Value::Bool(first && d.play_on_awake),
        ];
        warn(views.apply("particles", &args), "particles");
    }
    for (r, action, count) in iron_render_api::take_particle_controls() {
        let mut args = vec![n(f64::from(r.0)), Value::String(action)];
        if count > 0.0 {
            args.push(n(count));
        }
        warn(views.apply("particleControl", &args), "particleControl");
    }
}

/// Cloth: a `cloth` op for every new or rebuilt one, `remove` for the ones that are gone.
fn cloths(scene: &mut ScriptScene, extras: &mut Extras) {
    let live = iron_render_api::cloths();
    let ids: HashSet<u32> = live.iter().map(|(r, _)| r.0).collect();
    let gone: Vec<u32> = cache(|c| {
        let gone = c
            .cloths
            .keys()
            .filter(|k| !ids.contains(k))
            .copied()
            .collect();
        c.cloths.retain(|k, _| ids.contains(k));
        gone
    });
    for id in gone {
        extras
            .cloths
            .push(("remove".into(), vec![n(f64::from(id))]));
    }
    for (r, d) in live {
        if d.simulated_mesh == 0
            || cache(|c| c.cloths.insert(r.0, d.rebuild_requests)) == Some(d.rebuild_requests)
        {
            continue;
        }
        let object = scene.adopt(d.object);
        let source = match mesh_arg(d.source_mesh) {
            Some(m) => m,
            None => continue,
        };

        let mut colliders = Vec::new();
        for o in &d.sphere_colliders {
            let c = iron_physics_api::SphereColliderRef::find_on(*o);
            if c.0 != 0 {
                colliders.push(Value::Array(vec![
                    Value::String("sphere".into()),
                    n(f64::from(scene.adopt(*o))),
                    n(c.radius()),
                    arr(&[c.center().x, c.center().y, c.center().z]),
                ]));
            }
        }
        for o in &d.capsule_colliders {
            let c = iron_physics_api::CapsuleColliderRef::find_on(*o);
            if c.0 != 0 {
                colliders.push(Value::Array(vec![
                    Value::String("capsule".into()),
                    n(f64::from(scene.adopt(*o))),
                    n(c.radius()),
                    n(c.height()),
                    arr(&[c.center().x, c.center().y, c.center().z]),
                ]));
            }
        }
        let pins: Vec<f64> = d.pins.iter().flat_map(|p| [p.x, p.y, p.z]).collect();
        let args = vec![
            n(f64::from(r.0)),
            n(f64::from(object)),
            source,
            n(f64::from(d.simulated_mesh)),
            arr(&[
                d.stretching_stiffness,
                d.bending_stiffness,
                d.damping,
                d.friction,
                d.solver_iterations,
            ]),
            arr(&[
                d.external_acceleration.x,
                d.external_acceleration.y,
                d.external_acceleration.z,
            ]),
            arr(&[
                d.random_acceleration.x,
                d.random_acceleration.y,
                d.random_acceleration.z,
            ]),
            Value::Bool(d.use_gravity),
            arr(&pins),
            n(d.pin_radius),
            match d.pin_box {
                Some((a, b)) => arr(&[a.x, a.y, a.z, b.x, b.y, b.z]),
                None => Value::Null,
            },
            Value::Array(colliders),
        ];
        extras.cloths.push(("cloth".into(), args));
    }
}

fn warn(r: Result<(), String>, what: &str) {
    if let Err(e) = r {
        tracing::warn!(target: "render", "{what}: {e}");
    }
}

/// Inverse bind poses of a model's skinned meshes, for [`resources`]: by mesh id.
fn model_bindposes(description: &serde_json::Value) {
    let skins = description["skins"].as_array().cloned().unwrap_or_default();
    let nodes = description["nodes"].as_array().cloned().unwrap_or_default();
    for node in nodes {
        let Some(skin) = node["skin"].as_u64().and_then(|s| skins.get(s as usize)) else {
            continue;
        };
        let inverse: Vec<f64> = skin["inverseBind"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_f64)
            .collect();
        for p in node["primitives"].as_array().into_iter().flatten() {
            if let Some(mesh) = p["mesh"].as_u64() {
                cache(|c| c.bindposes.insert(mesh as u32, inverse.clone()));
            }
        }
    }
}

/// Textures, meshes and models the game made since the last frame.
fn resources(views: &mut WorldViews, scene: &mut ScriptScene, root: &Path) {
    use iron_render_api::{RenderCommand, TextureRef};
    for cmd in iron_render_api::take_commands() {
        match cmd {
            RenderCommand::Texture(t) => {
                let Some(info) = iron_render_api::texture_info(t) else {
                    continue;
                };
                if info.render_texture {
                    let args = vec![
                        n(f64::from(t.0)),
                        n(f64::from(info.width)),
                        n(f64::from(info.height)),
                    ];
                    warn(views.apply("renderTexture", &args), "renderTexture");
                    iron_render_api::texture_loaded(t, info.width, info.height);
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
                        views.mark_missing(t.0);
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
                // Primitives are built in; an engine mesh (a cloth's copy) is filled in later.
                if info.primitive.is_some() || info.engine {
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
            RenderCommand::Model(m) => {
                let Some(src) = iron_render_api::model_src(m) else {
                    continue;
                };
                let args = [n(f64::from(m.0)), Value::String(src.clone())];
                match views.model_op(root, &args) {
                    Ok(description) => {
                        model_bindposes(&description);
                        // Building the nodes makes GameObjects: the scene must be current.
                        let _scope = iron_scene::api::enter(&mut scene.scene);
                        iron_render_api::model_loaded(m, Some(&description.to_string()));
                    }
                    Err(e) => {
                        tracing::warn!(target: "render", "model `{src}`: {e}");
                        let _scope = iron_scene::api::enter(&mut scene.scene);
                        iron_render_api::model_loaded(m, None);
                    }
                }
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

    // M9-API-700
    #[test]
    fn settings_planes_and_quality_reach_the_renderers_views() {
        iron_render_api::reset();
        reset();
        let mut scene = ScriptScene::new();
        let mut ui = UiTree::new(100.0, 100.0);
        let bundle = Bundle {
            source: "class Stage extends Component {
                override onStart(): void {
                  const cam = new GameObject(\"Cam\");
                  cam.addComponent(Camera, { fieldOfView: 50 });
                  cam.transform.position = new Vector3(0, 0, 5);
                  const p = new GameObject(\"Plane\");
                  p.addComponent(PlaneRenderer, { size: new Vector2(2, 3) });
                  RenderSettings.skybox = { sunDirection: null };
                  RenderSettings.bloom = { threshold: 1, intensity: 0.5, scatter: 0.4 };
                  RenderSettings.tonemapping = \"aces\";
                }
              }
              register(Stage);"
                .into(),
            ..Default::default()
        };
        let mut native = Native::load(bundle, &mut scene.scene, &mut ui).unwrap();
        native.frame(&mut scene.scene, &mut ui, 0.016, 16.0);
        let mut views = WorldViews::default();
        let extras = sync(&mut views, &mut scene, Path::new("."));
        assert!(extras.panels.is_empty());
        scene.scene.update_world();
        let v = views.views(&scene, (200, 100), &[]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].quads.len(), 1, "the plane");
        assert!(views.sky.is_some(), "the skybox");
        assert!(views.bloom.is_some());
        assert_eq!(views.grading.tonemapping, iron_render::Tonemapping::Aces);
        // Nothing changed: the second sync leaves the settings alone and keeps the plane.
        sync(&mut views, &mut scene, Path::new("."));
        scene.scene.update_world();
        assert_eq!(views.views(&scene, (200, 100), &[])[0].quads.len(), 1);
    }
}
