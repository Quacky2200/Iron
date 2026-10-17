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

/// Hands out engine ids for model meshes and textures.
#[derive(Debug)]
pub struct EngineIds(u32);

impl Default for EngineIds {
    fn default() -> Self {
        Self(ENGINE_IDS)
    }
}

impl EngineIds {
    fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
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

    // Images become textures when a material uses them (with that texture's sampler).
    let mut textures = Vec::new();
    let mut texture_ids: std::collections::HashMap<usize, (u32, u32, u32)> =
        std::collections::HashMap::new();
    let mut texture_for = |texture: gltf::Texture<'_>| -> Option<(u32, u32, u32)> {
        let key = texture.index();
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
            let texture = pbr
                .base_color_texture()
                // A texture whose image can't be read draws the missing checker.
                .map(|t| texture_for(t.texture()).unwrap_or((iron_render::MISSING_TEXTURE, 8, 8)));
            let alpha = match m.alpha_mode() {
                gltf::material::AlphaMode::Opaque => "opaque",
                gltf::material::AlphaMode::Mask => "mask",
                gltf::material::AlphaMode::Blend => "blend",
            };
            json!({
                "name": m.name().unwrap_or(""),
                "color": [r, g, b, if alpha == "blend" { a.min(0.999) } else { 1.0 }],
                "emission": e,
                "smoothness": 1.0 - pbr.roughness_factor(),
                "metallic": pbr.metallic_factor(),
                "unlit": m.unlit(),
                "texture": texture.map(|(id, w, h)| json!([id, w, h])),
                "alpha": alpha,
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
            let data = MeshData {
                positions,
                normals,
                uvs,
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
    Ok(Model {
        meshes,
        textures,
        description: json!({ "nodes": nodes, "roots": roots, "materials": materials }),
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
