//! The native rendering data API from compiled programs (`bench/render_native.ts`) and its
//! scene-file kinds, in the normal, stress GC and collection-off modes. IDs are in TESTS.md
//! (M9-TS-1220..).
use iron_render_api as render;
use iron_scene::Scene;
use iron_ts_rt::{Str, link_anchor, ts_rt_gc_enable, ts_rt_gc_stress};
use iron_ts_tests::render_native_t::*;

fn reset() {
    render::reset();
    iron_engine_api::camera::reset();
    iron_engine_api::assets::reset();
    iron_scenes::reset();
    iron_scenes::take_warnings();
}

fn in_every_gc_mode(f: impl Fn()) {
    let _serial = iron_ts_tests::serial();
    assert_ne!(link_anchor(), 0);
    assert_ne!(iron_ts_tests::engine_api_anchor(), 0);
    let run = || {
        reset();
        let mut scene = Scene::new();
        let _scope = iron_scene::api::enter(&mut scene);
        unsafe { ts_rdn_init() };
        f();
    };
    run();
    ts_rt_gc_stress(true);
    run();
    ts_rt_gc_stress(false);
    ts_rt_gc_enable(false);
    run();
    ts_rt_gc_enable(true);
    reset();
}

fn text(p: *mut u8) -> String {
    assert!(!p.is_null());
    String::from_utf16_lossy(&unsafe { Str::from_raw(p as *const Str) }.to_utf16())
}

// M9-TS-1220
#[test]
fn a_light_keeps_its_settings_and_rejects_unknown_names() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_rdn_light()), "spot|2.5|0.5|40|soft|5|20|true");
        let lights = render::lights();
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].1.r#type, "spot");
    });
}

// M9-TS-1221
#[test]
fn mesh_renderers_take_primitives_by_name_script_meshes_and_materials() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_rdn_meshes()), "sphere|3|1|cutout|off|cube|0.5|0.25");
        let renderers = render::mesh_renderers();
        assert_eq!(renderers.len(), 3);
        // The material of the first renderer is the one made by the script.
        let m = render::with_material(render::MaterialRef(renderers[0].1.material), |d| d.color.r);
        assert_eq!(m, Some(1.0));
        let cmds = render::take_commands();
        assert!(
            cmds.iter()
                .any(|c| matches!(c, render::RenderCommand::Mesh(_)))
        );
    });
}

// M9-TS-1222
#[test]
fn textures_come_from_files_pixels_and_render_targets_and_cameras_use_them() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(
            text(ts_rdn_textures()),
            "point|4|false|2x1|true|64|64|skybox|3|0.5|30|img/a.png"
        );
        let a = render::texture_info(render::TextureRef(1)).unwrap();
        assert_eq!(a.src.as_deref(), Some("img/a.png"));
        assert!(!a.loaded);
        render::texture_loaded(render::TextureRef(1), 8, 4);
        let a = render::texture_info(render::TextureRef(1)).unwrap();
        assert!(a.loaded && a.width == 8);
        // Pixels: sRGB bytes, the top row first; 0.5 alpha is 128.
        let b = render::texture_info(render::TextureRef(2)).unwrap();
        assert_eq!(b.pixels.unwrap(), [255, 0, 0, 255, 0, 0, 255, 128]);
        let cams = iron_engine_api::camera::cameras();
        assert_eq!(cams[0].1.target_texture.0, 3);
        assert_eq!(cams[0].1.rect, [0.5, 0.0, 0.5, 1.0]);
        assert_eq!(cams[0].1.layer_cull_distances[4], 30.0);
    });
}

// M9-TS-1223
#[test]
fn sprites_have_a_size_from_their_texture_and_a_renderer() {
    in_every_gc_mode(|| unsafe {
        // A 4x4 texture at 50 pixels per unit is 0.08 units wide; a quarter cell at 100 is 0.02.
        assert_eq!(text(ts_rdn_sprites()), "0.08|0.02|true|3|true");
    });
}

// M9-TS-1224
#[test]
fn lod_levels_must_go_down_and_hold_renderers() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_rdn_lods()), "2|0.2|1|true");
        assert_eq!(render::lod_groups()[0].1.forced, 1);
    });
}

// M9-TS-1225
#[test]
fn mirrors_probes_planes_panels_and_strokes_hold_their_data() {
    in_every_gc_mode(|| unsafe {
        assert_eq!(text(ts_rdn_others()), "6|256|everyFrame|4|0.8|2|1");
        assert_eq!(render::reflection_probes()[0].1.render_requests, 2.0);
    });
}

// M9-TS-1228
#[test]
fn render_settings_and_the_cursor_hold_their_state_and_ignore_unknown_names() {
    in_every_gc_mode(|| unsafe {
        iron_engine_api::cursor::reset();
        assert_eq!(
            text(ts_rdn_settings()),
            "0.5|true|3|2|true|aces|true|skybox"
        );
        let s = render::settings();
        assert_eq!(s.skybox.unwrap().sun_size, 3.0);
        assert_eq!(s.bloom.unwrap().intensity, 2.0);
        assert!(render::version() > 0);
        assert_eq!(text(ts_rdn_cursor()), "false|locked");
        let c = iron_engine_api::cursor::state();
        assert_eq!(c.image, Some(("ui/hand.png".to_string(), Some([4.0, 5.0]))));
    });
}

// M9-TS-1229
#[test]
fn camera_follow_and_orbit_move_their_objects_when_the_host_runs_them() {
    in_every_gc_mode(|| unsafe {
        ts_rdn_behaviours();
        render::late_update_camera_follows(0.016);
        render::late_update_camera_orbits(1.0);
        let pos = |name: &str| {
            iron_scene::api::with_scene(|sc| {
                let o = sc.find(name).unwrap();
                sc.world(o).position
            })
            .unwrap()
        };
        // Smooth time 0: it sits at the target plus the offset.
        let f = pos("Follower");
        assert_eq!((f.x, f.y, f.z), (10.0, 2.0, 5.0));
        // yaw 90 + 90 degrees a second for one second = 180: behind the centre at +z.
        let o = pos("Orbiter");
        assert!(
            (o.x).abs() < 1e-4 && (o.z - 4.0).abs() < 1e-4 && o.y.abs() < 1e-4,
            "{o:?}"
        );
    });
}

// M9-TS-1230
//
// `renderSettings` of a scene file applies when the scene becomes active, and the active scene
// saves the current settings.
#[test]
fn scene_files_apply_and_save_render_settings() {
    let _serial = iron_ts_tests::serial();
    reset();
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    let text = r#"{"format":"iron.scene","version":1,"name":"Sky","objects":[],
      "renderSettings":{"ambientLight":{"$type":"Color","value":[0.1,0.2,0.3,1]},"ambientMode":"color",
        "reflectionIntensity":0.5,
        "skybox":{"zenith":{"$type":"Color","value":[0,0,1,1]},"sunDirection":null},
        "bloom":{"threshold":0.8,"intensity":1.5,"scatter":0.3}}}"#;
    let handle = iron_scenes::load_scene(text, false).expect("loads");
    let s = render::settings();
    assert_eq!(s.ambient_mode, "color");
    assert_eq!(s.ambient_light.b, 0.3);
    assert_eq!(s.reflection_intensity, 0.5);
    let sky = s.skybox.unwrap();
    assert_eq!((sky.zenith.b, sky.sun_direction), (1.0, None));
    assert_eq!(s.bloom.unwrap().threshold, 0.8);
    let saved: serde_json::Value =
        serde_json::from_str(&iron_scenes::serialize(handle).unwrap()).unwrap();
    assert_eq!(saved["renderSettings"]["ambientMode"], "color");
    assert_eq!(saved["renderSettings"]["bloom"]["intensity"], 1.5);
    assert!(saved["renderSettings"]["skybox"]["sunDirection"].is_null());
    reset();
}

// M9-TS-1226
//
// The example game's scene file loads its Light, MeshRenderer (primitive mesh, inline Material)
// and Camera extras, and saves them back.
#[test]
fn a_real_scene_file_loads_lights_renderers_materials_and_camera_settings() {
    let _serial = iron_ts_tests::serial();
    reset();
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    let text = include_str!("../../../../examples/scenes/scenes/level1.scene.json");
    let handle = iron_scenes::load_scene(text, false).expect("loads");
    let warnings = iron_scenes::take_warnings();
    assert!(
        warnings.iter().all(|w| w.contains("Spinner")),
        "only the game's own component is unknown: {warnings:?}"
    );
    let cams = iron_engine_api::camera::cameras();
    assert_eq!(cams[0].1.clear_flags, "skybox");
    let lights = render::lights();
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].1.r#type, "directional");
    assert_eq!(lights[0].1.intensity, 1.3);
    assert_eq!(lights[0].1.shadows, "soft");
    let renderers = render::mesh_renderers();
    assert!(renderers.len() > 3);
    let ground = &renderers[0].1;
    assert_eq!(
        render::primitive_name(render::MeshRef(ground.mesh)).unwrap(),
        "plane"
    );
    let (smooth, g) = render::with_material(render::MaterialRef(ground.material), |m| {
        (m.smoothness, m.color.g)
    })
    .unwrap();
    assert_eq!((smooth, g), (0.2, 0.36));
    // Saving writes the primitive as its name and the material inline, without default extras.
    let saved: serde_json::Value =
        serde_json::from_str(&iron_scenes::serialize(handle).unwrap()).unwrap();
    let ground_json = &saved["objects"][2]["components"][0]["properties"];
    assert_eq!(ground_json["mesh"], "plane");
    assert_eq!(ground_json["material"]["$type"], "Material");
    assert_eq!(ground_json["material"]["smoothness"], 0.2);
    assert!(ground_json["material"].get("normalMap").is_none());
    assert_eq!(
        saved["objects"][1]["components"][0]["properties"]["type"],
        "directional"
    );
    reset();
}

// M9-TS-1227
//
// Shared assets in a scene file: a texture and a script mesh in the `assets` table load once
// however many components use them, and saving writes the table again; loading what was saved
// and saving again gives the same data.
#[test]
fn the_assets_table_loads_shared_textures_and_meshes_and_saves_them_again() {
    let _serial = iron_ts_tests::serial();
    reset();
    let mut scene = Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    let text = r#"{"format":"iron.scene","version":1,"name":"Assets",
     "assets":{
       "texture1":{"type":"Texture","src":"img/a.png","filterMode":"point","wrapMode":"clamp","mipmaps":false},
       "mesh1":{"type":"Mesh","vertices":[0,0,0,1,0,0,0,1,0],"triangles":[0,1,2],"normals":[],"uv":[]},
       "texture2":{"type":"RenderTexture","width":32,"height":16}},
     "objects":[
      {"id":1,"name":"A","components":[
        {"id":2,"type":"MeshRenderer","properties":{"mesh":{"$asset":"mesh1"},
          "material":{"$type":"Material","mainTexture":{"$asset":"texture1"},"normalMap":{"$asset":"texture1"}}}}]},
      {"id":3,"name":"B","components":[
        {"id":4,"type":"MeshRenderer","properties":{"mesh":{"$asset":"mesh1"}}},
        {"id":5,"type":"SpriteRenderer","properties":{"sprite":{"$type":"Sprite","texture":{"$asset":"texture1"},"rect":{"$type":"Rect","value":[0,0,2,2]}}}},
        {"id":6,"type":"Camera","properties":{"targetTexture":{"$asset":"texture2"}}}]}]}"#;
    let handle = iron_scenes::load_scene(text, false).expect("loads");
    assert!(iron_scenes::take_warnings().is_empty());
    let renderers = render::mesh_renderers();
    assert_eq!(renderers[0].1.mesh, renderers[1].1.mesh, "one mesh, shared");
    let tex = render::with_material(render::MaterialRef(renderers[0].1.material), |m| {
        (m.main_texture, m.normal_map)
    })
    .unwrap();
    assert_eq!(tex.0, tex.1, "one texture, shared");
    let info = render::texture_info(render::TextureRef(tex.0)).unwrap();
    assert_eq!(info.src.as_deref(), Some("img/a.png"));
    assert_eq!(
        (
            info.filter_mode.as_str(),
            info.wrap_mode.as_str(),
            info.mipmaps
        ),
        ("point", "clamp", false)
    );
    let cams = iron_engine_api::camera::cameras();
    let rt = render::texture_info(render::TextureRef(cams[0].1.target_texture.0)).unwrap();
    assert!(rt.render_texture && rt.width == 32);
    let sprite = render::sprite_renderers()[0].1.sprite;
    let s = render::with_sprite(render::SpriteRef(sprite), |d| {
        (d.texture, d.has_rect, d.rect)
    })
    .unwrap();
    assert_eq!(s.0, tex.0);
    assert_eq!((s.1, s.2), (true, [0.0, 0.0, 2.0, 2.0]));

    let first = iron_scenes::serialize(handle).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&first).unwrap();
    let assets = saved["assets"].as_object().unwrap();
    assert_eq!(assets.len(), 3, "{assets:?}");
    let props = |object: usize, ty: &str| {
        saved["objects"][object]["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["type"] == ty)
            .map(|c| c["properties"].clone())
            .unwrap()
    };
    let a = &props(0, "MeshRenderer");
    let b = &props(1, "MeshRenderer");
    assert_eq!(a["mesh"], b["mesh"], "both point at the same table entry");
    let mesh_id = a["mesh"]["$asset"].as_str().unwrap();
    assert_eq!(assets[mesh_id]["type"], "Mesh");
    assert_eq!(assets[mesh_id]["triangles"], serde_json::json!([0, 1, 2]));
    let tex_id = a["material"]["mainTexture"]["$asset"].as_str().unwrap();
    assert_eq!(assets[tex_id]["src"], "img/a.png");
    assert_eq!(assets[tex_id]["filterMode"], "point");
    assert_eq!(a["material"]["normalMap"]["$asset"], tex_id);
    // Loading what was saved and saving again changes nothing.
    let again = iron_scenes::load_scene(&first, false).expect("loads again");
    assert!(iron_scenes::take_warnings().is_empty());
    let second = iron_scenes::serialize(again).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&second).unwrap(),
        saved
    );
    reset();
}
