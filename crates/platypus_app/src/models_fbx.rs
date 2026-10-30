// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! FBX models (`.fbx`, binary or ASCII) through the ufbx loader, made into the same [`Model`] as a
//! glTF file (`models.rs`): meshes split by material, materials, textures (embedded or beside the
//! file, inside the game folder), the node tree, skins and the takes as animations. ufbx converts
//! the file's units and axes to metres and Y-up (right-handed) as it loads.

use std::collections::HashMap;
use std::path::Path;

use platypus_render::{MeshData, TextureData, TextureFilter, TextureWrap};
use serde_json::{Value, json};

use crate::models::{EngineIds, Model, bounds};

fn v3(v: ufbx::Vec3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

fn quat(q: ufbx::Quat) -> [f32; 4] {
    [q.x as f32, q.y as f32, q.z as f32, q.w as f32]
}

/// A 3x4 ufbx matrix as a 4x4 column-major one (glTF's order).
fn mat4(m: &ufbx::Matrix) -> [f32; 16] {
    [
        m.m00, m.m10, m.m20, 0.0, m.m01, m.m11, m.m21, 0.0, m.m02, m.m12, m.m22, 0.0, m.m03, m.m13,
        m.m23, 1.0,
    ]
    .map(|x| x as f32)
}

/// The `scaleFactor` of a model's `.meta` importer (`{"importer": {"scaleFactor": 0.5}}`), 1 when
/// there is none or it isn't a positive number.
fn scale_factor(root: &Path, src: &str) -> f64 {
    platypus_core::read_asset(root, &format!("{src}.meta"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|m| m["importer"]["scaleFactor"].as_f64())
        .filter(|s| s.is_finite() && *s > 0.0)
        .unwrap_or(1.0)
}

/// Loads `src` (relative to the game folder `root`).
pub fn load(root: &Path, src: &str, ids: &mut EngineIds) -> Result<Model, String> {
    let bytes = platypus_core::read_asset(root, src).map_err(|e| e.to_string())?;
    let model_dir = src.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    let scale = scale_factor(root, src);
    let opts = ufbx::LoadOpts {
        target_axes: ufbx::CoordinateAxes::right_handed_y_up(),
        // Metres, times the `.meta` importer's `scaleFactor` (Unity's model import scale).
        target_unit_meters: 1.0 / scale,
        // Units and axes go into the geometry and the transforms, not a root node of their own.
        space_conversion: ufbx::SpaceConversion::ModifyGeometry,
        // FBX's geometric transforms (pivots) go into the geometry, or a helper node if shared.
        geometry_transform_handling: ufbx::GeometryTransformHandling::ModifyGeometry,
        generate_missing_normals: true,
        clean_skin_weights: true,
        ignore_missing_external_files: true,
        ..Default::default()
    };
    let scene =
        ufbx::load_memory(&bytes, opts).map_err(|e| format!("`{src}`: {}", &*e.description))?;

    // Textures: embedded content, else the file named, relative to the model.
    let mut textures = Vec::new();
    let mut texture_ids: HashMap<(u32, bool), (u32, u32, u32)> = HashMap::new();
    let mut texture_for = |t: &ufbx::Texture, srgb: bool| -> Option<(u32, u32, u32)> {
        let key = (t.element.typed_id, srgb);
        if let Some(found) = texture_ids.get(&key) {
            return Some(*found);
        }
        let data: Vec<u8> = if !t.content.is_empty() {
            t.content.to_vec()
        } else {
            let names = [&*t.relative_filename, &*t.filename];
            let read = names.iter().filter(|n| !n.is_empty()).find_map(|n| {
                let file = n.replace('\\', "/");
                // An absolute path from the artist's machine: the file's name beside the model.
                let name = if file.contains(':') || file.starts_with('/') {
                    file.rsplit('/').next().unwrap_or(&file).to_owned()
                } else {
                    file
                };
                let path = if model_dir.is_empty() {
                    name
                } else {
                    format!("{model_dir}/{name}")
                };
                platypus_core::read_asset(root, &path).ok()
            });
            match read {
                Some(d) => d,
                None => {
                    tracing::warn!("model `{src}`: texture {:?} not found", &*t.filename);
                    return None;
                }
            }
        };
        let image = match image::load_from_memory(&data) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                tracing::warn!("model `{src}`: texture {:?}: {e}", &*t.filename);
                return None;
            }
        };
        let (w, h) = image.dimensions();
        let wrap = match t.wrap_u {
            ufbx::WrapMode::Clamp => TextureWrap::Clamp,
            _ => TextureWrap::Repeat,
        };
        let id = ids.next_id();
        textures.push((
            id,
            TextureData {
                width: w,
                height: h,
                rgba: image.into_raw(),
                filter: TextureFilter::Trilinear,
                wrap,
                mipmaps: true,
                anisotropy: 1,
                srgb,
            },
        ));
        texture_ids.insert(key, (id, w, h));
        Some((id, w, h))
    };

    // Materials in the scene's order (`typed_id`).
    let materials: Vec<Value> = scene
        .materials
        .iter()
        .map(|m| {
            let pbr = &m.pbr;
            let factor = if pbr.base_factor.has_value { pbr.base_factor.value_vec4.x } else { 1.0 };
            let c = pbr.base_color.value_vec4;
            let color = [c.x * factor, c.y * factor, c.z * factor, 1.0].map(|x| x as f32);
            let e = pbr.emission_color.value_vec4;
            let ef = if pbr.emission_factor.has_value { pbr.emission_factor.value_vec4.x } else { 1.0 };
            let emission = [e.x * ef, e.y * ef, e.z * ef].map(|x| x as f32);
            let roughness = if pbr.roughness.has_value { pbr.roughness.value_vec4.x } else { 0.5 };
            let metallic = if pbr.metalness.has_value { pbr.metalness.value_vec4.x } else { 0.0 };
            let mut map = |t: &ufbx::Texture, srgb: bool| {
                texture_for(t, srgb).unwrap_or((platypus_render::MISSING_TEXTURE, 8, 8))
            };
            let texture = pbr.base_color.texture.as_ref().map(|t| map(t, true));
            let normal = pbr.normal_map.texture.as_ref().map(|t| map(t, false).0);
            let emissive = pbr.emission_color.texture.as_ref().map(|t| map(t, true).0);
            json!({
                "name": &*m.element.name,
                "color": color,
                "emission": if emission.iter().any(|x| *x > 0.0) { json!(emission) } else { Value::Null },
                "smoothness": (1.0 - roughness).clamp(0.0, 1.0),
                "metallic": metallic.clamp(0.0, 1.0),
                "unlit": false,
                "texture": texture.map(|(id, w, h)| json!([id, w, h])),
                "alpha": "opaque",
                "alphaCutoff": 0.5,
                "doubleSided": false,
                "normal": normal.map(|id| json!([id, 1.0])),
                "metallicRoughness": Value::Null,
                "occlusion": Value::Null,
                "emissive": emissive,
            })
        })
        .collect();

    // Nodes: every node but ufbx's root, by `typed_id`.
    let node_list: Vec<&ufbx::Node> = scene
        .nodes
        .iter()
        .map(|n| &**n)
        .filter(|n| !n.is_root)
        .collect();
    let index_of: HashMap<u32, usize> = node_list
        .iter()
        .enumerate()
        .map(|(i, n)| (n.element.typed_id, i))
        .collect();

    // Skins: one per skin deformer, its clusters' bones and geometry-to-bone matrices.
    let skin_index: HashMap<u32, usize> = scene
        .skin_deformers
        .iter()
        .enumerate()
        .map(|(i, s)| (s.element.typed_id, i))
        .collect();
    let skins: Vec<Value> = scene
        .skin_deformers
        .iter()
        .map(|s| {
            let joints: Vec<usize> = s
                .clusters
                .iter()
                .map(|c| {
                    c.bone_node
                        .as_ref()
                        .and_then(|b| index_of.get(&b.element.typed_id))
                        .copied()
                        .unwrap_or(0)
                })
                .collect();
            // Mesh space at bind time to the bone's (glTF's inverse bind matrices).
            let inverse: Vec<f32> = s
                .clusters
                .iter()
                .flat_map(|c| mat4(&c.geometry_to_bone))
                .collect();
            json!({ "joints": joints, "inverseBind": inverse })
        })
        .collect();

    // Meshes: one engine mesh per material part of each mesh, made once however many nodes use it.
    let mut meshes = Vec::new();
    let mut mesh_parts: HashMap<u32, Vec<Value>> = HashMap::new();
    for mesh in scene.meshes.iter() {
        let skin = mesh.skin_deformers.first();
        let mut parts = Vec::new();
        // A mesh without material parts is one part of all its faces.
        let all: Vec<u32> = (0..mesh.faces.len() as u32).collect();
        let part_faces: Vec<(Option<u32>, Vec<u32>)> = if mesh.material_parts.is_empty() {
            vec![(None, all)]
        } else {
            mesh.material_parts
                .iter()
                .filter(|p| p.num_triangles > 0)
                .map(|p| {
                    let material = mesh
                        .materials
                        .get(p.index as usize)
                        .map(|m| m.element.typed_id);
                    (material, p.face_indices.to_vec())
                })
                .collect()
        };
        let mut tri = Vec::new();
        for (material, faces) in part_faces {
            let mut data = MeshData::default();
            let mut seen: HashMap<(u32, u32, u32), u32> = HashMap::new();
            for f in faces {
                let face = mesh.faces[f as usize];
                if face.num_indices < 3 {
                    continue;
                }
                tri.resize((face.num_indices as usize - 2) * 3, 0);
                let n = mesh.triangulate_face(&mut tri, face) as usize;
                for &corner in &tri[..n * 3] {
                    let ci = corner as usize;
                    let key = (
                        mesh.vertex_position.indices[ci],
                        if mesh.vertex_normal.exists {
                            mesh.vertex_normal.indices[ci]
                        } else {
                            0
                        },
                        if mesh.vertex_uv.exists {
                            mesh.vertex_uv.indices[ci]
                        } else {
                            0
                        },
                    );
                    let index = *seen.entry(key).or_insert_with(|| {
                        data.positions.push(v3(mesh.vertex_position[ci]));
                        if mesh.vertex_normal.exists {
                            data.normals.push(v3(mesh.vertex_normal[ci]));
                        }
                        if mesh.vertex_uv.exists {
                            let uv = mesh.vertex_uv[ci];
                            // FBX's V goes up the image; ours (glTF's) down.
                            data.uvs.push([uv.x as f32, 1.0 - uv.y as f32]);
                        }
                        if let Some(skin) = skin {
                            let vertex = mesh.vertex_indices[ci] as usize;
                            let (joints, weights) = skin_weights(skin, vertex);
                            data.joints.push(joints);
                            data.weights.push(weights);
                        }
                        data.positions.len() as u32 - 1
                    });
                    data.indices.push(index);
                }
            }
            if data.indices.is_empty() {
                continue;
            }
            let data = data.complete();
            if !data.is_valid() {
                tracing::warn!(
                    "model `{src}`: mesh {:?} is malformed; skipped",
                    &*mesh.element.name
                );
                continue;
            }
            let [min, max] = bounds(&data.positions);
            let id = ids.next_id();
            meshes.push((id, data));
            parts.push(json!({ "mesh": id, "material": material, "bounds": [min, max] }));
        }
        mesh_parts.insert(mesh.element.typed_id, parts);
    }

    let nodes: Vec<Value> = node_list
        .iter()
        .map(|n| {
            let t = &n.local_transform;
            let mesh = n.mesh.as_ref();
            json!({
                "name": &*n.element.name,
                "position": v3(t.translation),
                "rotation": quat(t.rotation),
                "scale": v3(t.scale),
                "children": n.children.iter().filter_map(|c| index_of.get(&c.element.typed_id)).collect::<Vec<_>>(),
                "primitives": mesh.and_then(|m| mesh_parts.get(&m.element.typed_id)).cloned().unwrap_or_default(),
                "skin": mesh.and_then(|m| m.skin_deformers.first()).and_then(|s| skin_index.get(&s.element.typed_id)),
            })
        })
        .collect();
    let roots: Vec<usize> = scene
        .root_node
        .children
        .iter()
        .filter_map(|c| index_of.get(&c.element.typed_id).copied())
        .collect();

    let animations = takes(&scene, &index_of, src);
    Ok(Model {
        meshes,
        textures,
        cooked_textures: Vec::new(),
        description: json!({
            "nodes": nodes,
            "roots": roots,
            "materials": materials,
            "skins": skins,
            "animations": animations,
        }),
    })
}

/// A vertex's four strongest bones (cluster indices) and their weights, summing to 1.
fn skin_weights(skin: &ufbx::SkinDeformer, vertex: usize) -> ([u16; 4], [f32; 4]) {
    let mut joints = [0u16; 4];
    let mut weights = [0f32; 4];
    let Some(v) = skin.vertices.get(vertex) else {
        return (joints, [1.0, 0.0, 0.0, 0.0]);
    };
    let weights_all: &[ufbx::SkinWeight] = &skin.weights;
    let all = &weights_all[v.weight_begin as usize..(v.weight_begin + v.num_weights) as usize];
    // `clean_skin_weights` sorts them, strongest first.
    for (i, w) in all.iter().take(4).enumerate() {
        joints[i] = w.cluster_index as u16;
        weights[i] = w.weight as f32;
    }
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        weights = weights.map(|w| w / sum);
    } else {
        weights = [1.0, 0.0, 0.0, 0.0];
    }
    (joints, weights)
}

/// Each take (animation stack) baked to keys per node: translation, rotation and scale, linear.
fn takes(scene: &ufbx::Scene, index_of: &HashMap<u32, usize>, src: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for stack in scene.anim_stacks.iter() {
        let baked = match ufbx::bake_anim(scene, &stack.anim, ufbx::BakeOpts::default()) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    "model `{src}`: take {:?}: {}",
                    &*stack.element.name,
                    &*e.description
                );
                continue;
            }
        };
        let start = baked.playback_time_begin;
        let mut channels = Vec::new();
        for bn in baked.nodes.iter() {
            let Some(&node) = index_of.get(&bn.typed_id) else {
                continue;
            };
            let vec_channel = |path: &str, keys: &[ufbx::BakedVec3]| {
                json!({
                    "node": node,
                    "path": path,
                    "interpolation": "linear",
                    "times": keys.iter().map(|k| (k.time - start) as f32).collect::<Vec<_>>(),
                    "values": keys.iter().flat_map(|k| v3(k.value)).collect::<Vec<_>>(),
                })
            };
            if !bn.translation_keys.is_empty() && !bn.constant_translation {
                channels.push(vec_channel("translation", &bn.translation_keys));
            }
            if !bn.rotation_keys.is_empty() && !bn.constant_rotation {
                channels.push(json!({
                    "node": node,
                    "path": "rotation",
                    "interpolation": "linear",
                    "times": bn.rotation_keys.iter().map(|k| (k.time - start) as f32).collect::<Vec<_>>(),
                    "values": bn.rotation_keys.iter().flat_map(|k| quat(k.value)).collect::<Vec<_>>(),
                }));
            }
            if !bn.scale_keys.is_empty() && !bn.constant_scale {
                channels.push(vec_channel("scale", &bn.scale_keys));
            }
        }
        out.push(json!({ "name": &*stack.element.name, "channels": channels }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
    }

    fn node<'a>(d: &'a Value, name: &str) -> &'a Value {
        d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["name"] == name)
            .unwrap()
    }

    // M5-FBX-01
    #[test]
    fn a_z_up_centimetre_file_comes_in_as_metres_y_up() {
        let m = load(&fixtures(), "assets/cube.fbx", &mut EngineIds::default()).unwrap();
        let d = &m.description;
        let cube = node(d, "Cube");
        let p: Vec<f64> = cube["position"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert!(
            (p[0]).abs() < 1e-4 && (p[1] - 1.0).abs() < 1e-4 && p[2].abs() < 1e-4,
            "{p:?}"
        );
        // The cube is a metre across.
        let (_, mesh) = &m.meshes[0];
        let b = bounds(&mesh.positions);
        for k in 0..3 {
            assert!((b[1][k] - b[0][k] - 1.0).abs() < 1e-3, "{b:?}");
        }
        // The empty child is there, under the cube, half a metre above it in the world (its
        // local transform may stay in the cube's own axes).
        let pivot = node(d, "Pivot");
        let f = |v: &Value, i: usize| v[i].as_f64().unwrap() as f32;
        use platypus_core::glam::{Quat, Vec3};
        let rot = Quat::from_xyzw(
            f(&cube["rotation"], 0),
            f(&cube["rotation"], 1),
            f(&cube["rotation"], 2),
            f(&cube["rotation"], 3),
        );
        let scale = Vec3::new(
            f(&cube["scale"], 0),
            f(&cube["scale"], 1),
            f(&cube["scale"], 2),
        );
        let local = Vec3::new(
            f(&pivot["position"], 0),
            f(&pivot["position"], 1),
            f(&pivot["position"], 2),
        );
        let world = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) + rot * (local * scale);
        assert!(
            (world - Vec3::new(0.0, 1.5, 0.0)).length() < 1e-4,
            "{world} {cube} {pivot}"
        );
        let pivot_index = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["name"] == "Pivot")
            .unwrap();
        assert!(
            cube["children"]
                .as_array()
                .unwrap()
                .contains(&json!(pivot_index))
        );
        assert_eq!(d["roots"].as_array().unwrap().len(), 1);
    }

    // M5-FBX-06
    #[test]
    fn the_meta_scale_factor_scales_the_model() {
        let dir = std::env::temp_dir().join(format!("platypus-fbx-scale-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(fixtures().join("assets/cube.fbx"), dir.join("cube.fbx")).unwrap();
        std::fs::write(
            dir.join("cube.fbx.meta"),
            r#"{ "guid": "x", "importer": { "type": "model", "scaleFactor": 2 } }"#,
        )
        .unwrap();
        let m = load(&dir, "cube.fbx", &mut EngineIds::default()).unwrap();
        let b = bounds(&m.meshes[0].1.positions);
        assert!((b[1][0] - b[0][0] - 2.0).abs() < 1e-3, "{b:?}");
        assert_eq!(
            node(&m.description, "Cube")["position"][1]
                .as_f64()
                .map(|y| (y - 2.0).abs() < 1e-4),
            Some(true)
        );
        // A bad factor is ignored.
        std::fs::write(
            dir.join("cube.fbx.meta"),
            r#"{ "importer": { "scaleFactor": -1 } }"#,
        )
        .unwrap();
        let m = load(&dir, "cube.fbx", &mut EngineIds::default()).unwrap();
        let b = bounds(&m.meshes[0].1.positions);
        assert!((b[1][0] - b[0][0] - 1.0).abs() < 1e-3, "{b:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // M5-FBX-02
    #[test]
    fn faces_split_by_material_with_colours_and_a_texture_beside_the_file() {
        let m = load(&fixtures(), "assets/cube.fbx", &mut EngineIds::default()).unwrap();
        let d = &m.description;
        let mats = d["materials"].as_array().unwrap();
        let red = mats.iter().find(|m| m["name"] == "Red").unwrap();
        let blue = mats.iter().find(|m| m["name"] == "Blue").unwrap();
        assert_eq!(red["color"][0], json!(1.0));
        assert!(red["texture"].is_null());
        assert!(blue["texture"].is_array(), "{blue}");
        assert_eq!(m.textures.len(), 1, "the checker, once");
        let prims = node(d, "Cube")["primitives"].as_array().unwrap().clone();
        assert_eq!(prims.len(), 2, "one part per material");
        // Three quads each: 6 triangles, 12 corners of their own (normals differ per face).
        for (id, mesh) in &m.meshes {
            assert!(prims.iter().any(|p| p["mesh"] == json!(id)));
            assert_eq!(mesh.indices.len(), 18);
            assert_eq!(mesh.positions.len(), 12);
            assert_eq!(mesh.normals.len(), 12);
            assert_eq!(mesh.uvs.len(), 12);
        }
    }

    // M5-FBX-04
    #[test]
    fn a_skinned_mesh_gets_its_bones_weights_and_bind_matrices() {
        let m = load(&fixtures(), "assets/rig.fbx", &mut EngineIds::default()).unwrap();
        let d = &m.description;
        let skins = d["skins"].as_array().unwrap();
        assert_eq!(skins.len(), 1);
        let joints = skins[0]["joints"].as_array().unwrap();
        let names: Vec<&Value> = joints
            .iter()
            .map(|j| &d["nodes"][j.as_u64().unwrap() as usize]["name"])
            .collect();
        assert_eq!(names, [&json!("Root"), &json!("Tip")]);
        assert_eq!(skins[0]["inverseBind"].as_array().unwrap().len(), 32);
        // Tip's inverse bind moves a point a metre up back to the bone's origin (in the mesh's
        // own axes, the file's Z up: the mesh node turns them).
        let tip_ty = skins[0]["inverseBind"][16 + 14].as_f64().unwrap();
        assert!((tip_ty + 1.0).abs() < 1e-3, "{}", skins[0]["inverseBind"]);
        assert_eq!(node(d, "Column")["skin"], json!(0));
        let (_, mesh) = &m.meshes[0];
        assert_eq!(mesh.joints.len(), mesh.positions.len());
        for (p, (j, w)) in mesh
            .positions
            .iter()
            .zip(mesh.joints.iter().zip(&mesh.weights))
        {
            assert!((w.iter().sum::<f32>() - 1.0).abs() < 1e-5);
            // The upper half follows Tip, the lower Root (height is the mesh's own Z).
            if (p[2] - 1.0).abs() > 1e-3 {
                assert_eq!(j[0], u16::from(p[2] > 1.0), "{p:?}");
            }
        }
    }

    // M5-FBX-05
    #[test]
    fn a_take_is_baked_into_keys() {
        let m = load(&fixtures(), "assets/rig.fbx", &mut EngineIds::default()).unwrap();
        let d = &m.description;
        let anims = d["animations"].as_array().unwrap();
        assert_eq!(anims.len(), 1);
        assert_eq!(anims[0]["name"], "Bend");
        let tip = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["name"] == "Tip")
            .unwrap();
        let rot = anims[0]["channels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["node"] == json!(tip) && c["path"] == "rotation")
            .expect("Tip turns");
        let times = rot["times"].as_array().unwrap();
        assert!(
            (times.last().unwrap().as_f64().unwrap() - 1.0).abs() < 1e-3,
            "{times:?}"
        );
        let v = rot["values"].as_array().unwrap();
        let last: Vec<f64> = v[v.len() - 4..]
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        // 90 degrees about X: (sin 45, 0, 0, cos 45), either sign.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        assert!(
            (last[0].abs() - s).abs() < 1e-3
                && last[1].abs() < 1e-3
                && (last[3].abs() - s).abs() < 1e-3,
            "{last:?}"
        );
    }

    // M5-FBX-03
    #[test]
    fn a_bad_fbx_is_an_error_and_paths_stay_inside() {
        assert!(load(&fixtures(), "assets/make_fbx.py", &mut EngineIds::default()).is_err());
        assert!(load(&fixtures(), "../outside.fbx", &mut EngineIds::default()).is_err());
    }
}
