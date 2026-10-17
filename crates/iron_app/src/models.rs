//! glTF models (`.glb` / `.gltf`): meshes, materials, textures and the node tree.
//!
//! `render.model id src` loads one: its meshes and textures are uploaded under engine
//! ids (from [`ENGINE_IDS`], so they never meet script ids) and scripts get the node tree
//! as JSON through `__iron_models`, from which `Model.instantiate` builds GameObjects.
//! Files the model refers to (buffers, images) must be inside the game folder too.

use std::path::Path;

use gltf::image::Source as ImageSource;
use iron_render::{MeshData, TextureData, TextureFilter, TextureWrap};
use serde_json::{Value, json};

/// Engine-made meshes and textures start here (scripts count up from 1).
pub const ENGINE_IDS: u32 = 1 << 30;

/// A loaded model: GPU uploads to make and the description scripts get.
#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<(u32, MeshData)>,
    pub textures: Vec<(u32, TextureData)>,
    /// `{ nodes, roots, materials }` for `Model.instantiate`.
    pub description: Value,
}

/// Hands out engine ids for model meshes and textures. A reload hands out a model's
/// old ids first (`replay`), so what scripts hold keeps working.
#[derive(Debug)]
pub struct EngineIds {
    last: u32,
    replay: std::collections::VecDeque<u32>,
    /// The ids handed out since `record` started.
    recorded: Vec<u32>,
}

impl Default for EngineIds {
    fn default() -> Self {
        Self {
            last: ENGINE_IDS,
            replay: Default::default(),
            recorded: Vec::new(),
        }
    }
}

impl EngineIds {
    fn next(&mut self) -> u32 {
        let id = self.replay.pop_front().unwrap_or_else(|| {
            self.last += 1;
            self.last
        });
        self.recorded.push(id);
        id
    }

    /// Starts recording ids, handing out `replay` first.
    pub fn record(&mut self, replay: &[u32]) {
        self.replay = replay.iter().copied().collect();
        self.recorded.clear();
    }

    /// The ids handed out since `record`.
    pub fn take_recorded(&mut self) -> Vec<u32> {
        self.replay.clear();
        std::mem::take(&mut self.recorded)
    }
}

/// Reads a file the model names: a `data:` URL, or a path relative to the model,
/// which must stay inside the game folder.
fn read_uri(root: &Path, model_dir: &str, uri: &str) -> Result<Vec<u8>, String> {
    if uri.starts_with("data:") {
        return iron_core::decode_data_url(uri).ok_or_else(|| "bad data URL".into());
    }
    let uri = percent_decode(uri);
    let relative = if model_dir.is_empty() {
        uri.clone()
    } else {
        format!("{model_dir}/{uri}")
    };
    let path = iron_core::resolve_asset(root, &relative)
        .ok_or_else(|| format!("`{uri}` is outside the game folder"))?;
    std::fs::read(&path).map_err(|e| format!("`{uri}`: {e}"))
}

/// glTF URIs are percent-encoded (`my%20texture.png`).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Loads `src` (relative to the game folder `root`).
pub fn load(root: &Path, src: &str, ids: &mut EngineIds) -> Result<Model, String> {
    let path = iron_core::resolve_asset(root, src)
        .ok_or_else(|| format!("`{src}` must be a relative path inside the game folder"))?;
    let bytes = std::fs::read(&path).map_err(|e| format!("`{src}`: {e}"))?;
    let model_dir = src.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|e| format!("`{src}`: {e}"))?;
    let doc = &gltf.document;

    let buffers: Vec<Vec<u8>> = doc
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => gltf
                .blob
                .clone()
                .ok_or_else(|| "the binary chunk is missing".to_owned()),
            gltf::buffer::Source::Uri(uri) => read_uri(root, &model_dir, uri),
        })
        .collect::<Result<_, _>>()?;

    // Images become textures when a material uses them (with that texture's sampler):
    // colours (base, emissive) as sRGB, data (normal, metallic-roughness, occlusion) as
    // linear, each once.
    let mut textures = Vec::new();
    let mut texture_ids: std::collections::HashMap<(usize, bool), (u32, u32, u32)> =
        std::collections::HashMap::new();
    let mut texture_for = |texture: gltf::Texture<'_>, srgb: bool| -> Option<(u32, u32, u32)> {
        let key = (texture.index(), srgb);
        if let Some(t) = texture_ids.get(&key) {
            return Some(*t);
        }
        let data = match texture.source().source() {
            ImageSource::View { view, .. } => {
                let buffer = buffers.get(view.buffer().index())?;
                buffer
                    .get(view.offset()..view.offset() + view.length())?
                    .to_vec()
            }
            ImageSource::Uri { uri, .. } => match read_uri(root, &model_dir, uri) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("model `{src}`: image {e}");
                    return None;
                }
            },
        };
        let image = match image::load_from_memory(&data) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                tracing::warn!("model `{src}`: image {}: {e}", texture.source().index());
                return None;
            }
        };
        let (w, h) = image.dimensions();
        let sampler = texture.sampler();
        use gltf::texture::{MagFilter, MinFilter, WrappingMode};
        let filter = match (sampler.mag_filter(), sampler.min_filter()) {
            (Some(MagFilter::Nearest), _) => TextureFilter::Point,
            (_, Some(MinFilter::LinearMipmapLinear)) | (_, None) => TextureFilter::Trilinear,
            _ => TextureFilter::Bilinear,
        };
        let wrap = match sampler.wrap_s() {
            WrappingMode::ClampToEdge => TextureWrap::Clamp,
            WrappingMode::MirroredRepeat => TextureWrap::Mirror,
            WrappingMode::Repeat => TextureWrap::Repeat,
        };
        let id = ids.next();
        textures.push((
            id,
            TextureData {
                width: w,
                height: h,
                rgba: image.into_raw(),
                filter,
                wrap,
                mipmaps: !matches!(
                    sampler.min_filter(),
                    Some(MinFilter::Nearest | MinFilter::Linear)
                ),
                anisotropy: 1,
                srgb,
            },
        ));
        texture_ids.insert(key, (id, w, h));
        Some((id, w, h))
    };

    let materials: Vec<Value> = doc
        .materials()
        .map(|m| {
            let pbr = m.pbr_metallic_roughness();
            let [r, g, b, a] = pbr.base_color_factor();
            let strength = m.emissive_strength().unwrap_or(1.0);
            let e = m.emissive_factor().map(|c| c * strength);
            // A texture whose image can't be read draws the missing checker.
            let mut map = |t: gltf::Texture<'_>, srgb: bool| {
                texture_for(t, srgb).unwrap_or((iron_render::MISSING_TEXTURE, 8, 8))
            };
            let texture = pbr.base_color_texture().map(|t| map(t.texture(), true));
            let normal = m
                .normal_texture()
                .map(|t| (map(t.texture(), false).0, t.scale()));
            let metallic = pbr
                .metallic_roughness_texture()
                .map(|t| map(t.texture(), false).0);
            let occlusion = m
                .occlusion_texture()
                .map(|t| (map(t.texture(), false).0, t.strength()));
            let emissive = m.emissive_texture().map(|t| map(t.texture(), true).0);
            let alpha = match m.alpha_mode() {
                gltf::material::AlphaMode::Opaque => "opaque",
                gltf::material::AlphaMode::Mask => "mask",
                gltf::material::AlphaMode::Blend => "blend",
            };
            json!({
                "name": m.name().unwrap_or(""),
                // Blending comes from the rendering mode, so the alpha can stay as given.
                "color": [r, g, b, if alpha == "blend" { a } else { 1.0 }],
                "emission": e,
                "smoothness": 1.0 - pbr.roughness_factor(),
                "metallic": pbr.metallic_factor(),
                "unlit": m.unlit(),
                "texture": texture.map(|(id, w, h)| json!([id, w, h])),
                "alpha": alpha,
                "alphaCutoff": m.alpha_cutoff().unwrap_or(0.5),
                "doubleSided": m.double_sided(),
                "normal": normal.map(|(id, scale)| json!([id, scale])),
                "metallicRoughness": metallic,
                "occlusion": occlusion.map(|(id, strength)| json!([id, strength])),
                "emissive": emissive,
            })
        })
        .collect();

    // Each primitive is its own engine mesh.
    let mut meshes = Vec::new();
    let mut mesh_primitives: Vec<Vec<Value>> = Vec::new();
    for mesh in doc.meshes() {
        let mut prims = Vec::new();
        for (i, p) in mesh.primitives().enumerate() {
            if p.mode() != gltf::mesh::Mode::Triangles {
                tracing::warn!(
                    "model `{src}`: mesh {} part {i} isn't triangles; skipped",
                    mesh.index()
                );
                continue;
            }
            let reader = p.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = positions.collect();
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(Iterator::collect)
                .unwrap_or_default();
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|t| t.into_f32().collect())
                .unwrap_or_default();
            let indices: Vec<u32> = match reader.read_indices() {
                Some(i) => i.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };
            // glTF's tangents follow the same convention as ours (UVs top-down, the
            // bitangent cross(n, t) * w pointing up the image); made from the UVs if missing.
            let tangents: Vec<[f32; 4]> = reader
                .read_tangents()
                .map(Iterator::collect)
                .unwrap_or_default();
            // Skinned: bone indices into the node's skin's joints, and weights (made to
            // sum to 1).
            let joints: Vec<[u16; 4]> = reader
                .read_joints(0)
                .map(|j| j.into_u16().collect())
                .unwrap_or_default();
            let weights: Vec<[f32; 4]> = reader
                .read_weights(0)
                .map(|w| {
                    w.into_f32()
                        .map(|w| {
                            let sum: f32 = w.iter().sum();
                            if sum > 0.0 {
                                w.map(|x| x / sum)
                            } else {
                                [1.0, 0.0, 0.0, 0.0]
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            let skinned = !joints.is_empty() && joints.len() == weights.len();
            let data = MeshData {
                positions,
                normals,
                uvs,
                tangents,
                joints: if skinned { joints } else { Vec::new() },
                weights: if skinned { weights } else { Vec::new() },
                indices,
            }
            .complete();
            if !data.is_valid() {
                tracing::warn!(
                    "model `{src}`: mesh {} part {i} is malformed; skipped",
                    mesh.index()
                );
                continue;
            }
            let [min, max] = bounds(&data.positions);
            let id = ids.next();
            meshes.push((id, data));
            prims.push(json!({
                "mesh": id,
                "material": p.material().index(),
                "bounds": [min, max],
            }));
        }
        mesh_primitives.push(prims);
    }

    let nodes: Vec<Value> = doc
        .nodes()
        .map(|n| {
            let (t, r, s) = n.transform().decomposed();
            json!({
                "name": n.name().unwrap_or(""),
                "position": t,
                "rotation": r,
                "scale": s,
                "children": n.children().map(|c| c.index()).collect::<Vec<_>>(),
                "primitives": n.mesh().map_or(Vec::new(), |m| mesh_primitives[m.index()].clone()),
                "skin": n.skin().map(|s| s.index()),
            })
        })
        .collect();
    let scene = doc.default_scene().or_else(|| doc.scenes().next());
    let roots: Vec<usize> = match scene {
        Some(s) => s.nodes().map(|n| n.index()).collect(),
        // No scenes: every node nothing else parents.
        None => {
            let children: std::collections::HashSet<usize> = doc
                .nodes()
                .flat_map(|n| n.children().map(|c| c.index()))
                .collect();
            (0..doc.nodes().len())
                .filter(|i| !children.contains(i))
                .collect()
        }
    };
    // Skins: their joint nodes and inverse bind matrices (column-major), in joint order.
    let skins: Vec<Value> = doc
        .skins()
        .map(|s| {
            let reader = s.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
            let joints: Vec<usize> = s.joints().map(|j| j.index()).collect();
            let inverse: Vec<[[f32; 4]; 4]> = reader
                .read_inverse_bind_matrices()
                .map(Iterator::collect)
                .unwrap_or_else(|| vec![iron_core::glam::Mat4::IDENTITY.to_cols_array_2d(); joints.len()]);
            json!({
                "joints": joints,
                "inverseBind": inverse.iter().flat_map(|m| m.iter().flatten().copied()).collect::<Vec<f32>>(),
            })
        })
        .collect();
    // Animations: per channel, the node, what it moves, how it interpolates, and the
    // keys (cubic splines bring in-tangent, value, out-tangent per key).
    let animations: Vec<Value> = doc
        .animations()
        .map(|a| {
            let channels: Vec<Value> = a
                .channels()
                .filter_map(|c| {
                    use gltf::animation::util::ReadOutputs;
                    let reader = c.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
                    let times: Vec<f32> = reader.read_inputs()?.collect();
                    let (path, values): (&str, Vec<f32>) = match reader.read_outputs()? {
                        ReadOutputs::Translations(t) => ("translation", t.flatten().collect()),
                        ReadOutputs::Rotations(r) => ("rotation", r.into_f32().flatten().collect()),
                        ReadOutputs::Scales(s) => ("scale", s.flatten().collect()),
                        // Morph target weights aren't supported yet.
                        ReadOutputs::MorphTargetWeights(_) => return None,
                    };
                    let interpolation = match c.sampler().interpolation() {
                        gltf::animation::Interpolation::Linear => "linear",
                        gltf::animation::Interpolation::Step => "step",
                        gltf::animation::Interpolation::CubicSpline => "cubic",
                    };
                    Some(json!({
                        "node": c.target().node().index(),
                        "path": path,
                        "interpolation": interpolation,
                        "times": times,
                        "values": values,
                    }))
                })
                .collect();
            json!({ "name": a.name().unwrap_or(""), "channels": channels })
        })
        .collect();
    Ok(Model {
        meshes,
        textures,
        description: json!({
            "nodes": nodes,
            "roots": roots,
            "materials": materials,
            "skins": skins,
            "animations": animations,
        }),
    })
}

fn bounds(points: &[[f32; 3]]) -> [[f32; 3]; 2] {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in points {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    [min, max]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
    }

    // M5-GLB-01
    #[test]
    fn a_glb_gives_meshes_textures_materials_and_nodes() {
        let mut ids = EngineIds::default();
        let m = load(&fixtures(), "assets/house.glb", &mut ids).unwrap();
        // The box and the roof's two parts, each its own engine mesh.
        assert_eq!(m.meshes.len(), 3);
        assert!(
            m.meshes
                .iter()
                .all(|(id, d)| *id > ENGINE_IDS && d.is_valid())
        );
        // The roof came without normals: they were made up.
        let roof = &m.meshes[1].1;
        assert_eq!(roof.normals.len(), roof.positions.len());
        // The checker image, nearest-filtered as its sampler says.
        assert_eq!(m.textures.len(), 1);
        let (tex_id, tex) = &m.textures[0];
        assert_eq!(
            (tex.width, tex.height, tex.filter),
            (4, 4, TextureFilter::Point)
        );

        let d = &m.description;
        assert_eq!(d["roots"], json!([0]));
        assert_eq!(d["nodes"][0]["name"], "House");
        assert_eq!(d["nodes"][0]["children"], json!([1, 2]));
        assert_eq!(d["nodes"][1]["primitives"].as_array().unwrap().len(), 2);
        assert_eq!(d["nodes"][1]["position"], json!([0.0, 0.5, 0.0]));
        let lamp = d["nodes"][2]["rotation"][1].as_f64().unwrap();
        assert!((lamp - 0.3826834).abs() < 1e-5);
        assert_eq!(d["materials"][1]["texture"], json!([tex_id, 4, 4]));
        assert_eq!(d["materials"][2]["unlit"], true);
        let red = d["materials"][0]["smoothness"].as_f64().unwrap();
        assert!((red - 0.6).abs() < 1e-5);
    }

    // M5-SKN-01
    #[test]
    fn a_skinned_glb_gives_joints_weights_skins_and_animations() {
        let mut ids = EngineIds::default();
        let m = load(&fixtures(), "assets/worm.glb", &mut ids).unwrap();
        let body = &m.meshes[0].1;
        assert_eq!(body.joints.len(), body.positions.len());
        // Weights sum to 1; the middle ring is half Bone0, half Bone1.
        assert!(
            body.weights
                .iter()
                .all(|w| (w.iter().sum::<f32>() - 1.0).abs() < 1e-5)
        );
        assert_eq!(body.joints[8], [0, 1, 0, 0]);
        assert!((body.weights[8][0] - 0.0).abs() < 1e-5);
        let d = &m.description;
        assert_eq!(d["nodes"][4]["skin"], json!(0));
        assert_eq!(d["skins"][0]["joints"], json!([1, 2, 3]));
        let ibm = d["skins"][0]["inverseBind"].as_array().unwrap();
        assert_eq!(ibm.len(), 48);
        // The second joint's bind pose is 1 m up: its inverse moves 1 m down.
        assert_eq!(ibm[16 + 13], json!(-1.0));
        let wave = &d["animations"][0];
        assert_eq!(wave["name"], "wave");
        assert_eq!(wave["channels"].as_array().unwrap().len(), 3);
        assert_eq!(wave["channels"][0]["path"], "rotation");
        assert_eq!(wave["channels"][0]["values"].as_array().unwrap().len(), 12);
        assert_eq!(wave["channels"][2]["interpolation"], "step");
    }

    #[test]
    fn files_outside_the_game_folder_are_refused() {
        let dir = std::env::temp_dir().join(format!("iron-gltf-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("game")).unwrap();
        std::fs::write(dir.join("secret.bin"), [0u8; 12]).unwrap();
        let gltf =
            r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"../secret.bin","byteLength":12}]}"#;
        std::fs::write(dir.join("game/m.gltf"), gltf).unwrap();
        let err = load(&dir.join("game"), "m.gltf", &mut EngineIds::default()).unwrap_err();
        assert!(err.contains("outside the game folder"), "{err}");
        assert!(
            load(
                &dir.join("game"),
                "../secret.bin",
                &mut EngineIds::default()
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("my%20tex.png"), "my tex.png");
        assert_eq!(percent_decode("plain.png"), "plain.png");
    }
}
