// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The rendering data API from QuickJS: components and assets are plain data the renderer reads.
//! IDs are in TESTS.md (M9-API-*).
mod common;
use common::{js, throws, with_world};
use platypus_render_api::{RenderCommand, take_commands};

// M9-API-139
//
// A Light: enum-like strings, colors (value classes), numbers, the layer mask, the host's view of
// the data (`lights()`), and the same component found again.
#[test]
fn light_and_mesh_renderer_components() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const go = GameObject.create();
            const light = Light.attach(go);
            light.type = 'spot';
            light.color = new Color(1, 0.5, 0.25, 1);
            light.intensity = 2.5;
            light.spotAngle = 40;
            light.shadows = 'soft';
            const mr = MeshRenderer.attach(go);
            mr.mesh = Mesh.fromString('sphere');
            const mat = Material.create();
            mat.color = new Color(1, 0, 0, 1);
            mat.metallic = 0.8;
            mat.renderingMode = 'transparent';
            mr.material = mat;
            mr.shadowCastingMode = 'off';
            JSON.stringify([light.type, light.color.g, light.intensity, light.spotAngle, light.shadows,
                Light.findOn(go) !== null, light.gameObject.name === go.name, mr.mesh.primitive,
                mr.material.metallic, mr.shadowCastingMode, mat.renderingMode, mat.color,
                String(light), String(mat), String(mr.mesh)])
            "#,
        );
        assert_eq!(
            out,
            r#"["spot",0.5,2.5,40,"soft",true,true,"sphere",0.8,"off","transparent",[1,0,0,1],"[object Light]","[object Material]","[object Mesh]"]"#
        );
        let lights = platypus_render_api::lights();
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].1.intensity, 2.5);
        assert!(throws(ctx, "light.intensity = 'bright'").starts_with("TypeError"));
        assert!(throws(ctx, "mr.mesh = go").starts_with("TypeError"));
    });
}

// M9-API-140
//
// Textures and meshes are handles to data the host uploads; arrays of vectors and numbers go in.
#[test]
fn textures_meshes_and_sprites() {
    with_world(|ctx| {
        let _ = take_commands();
        let out: String = js(
            ctx,
            r#"
            const tex = Texture.load('textures/a.png');
            const rt = Texture.createRenderTexture(256, 128);
            const mesh = Mesh.create(
                [new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(0, 1, 0)], [0, 1, 2]);
            const sprite = Sprite.create(tex);
            sprite.pixelsPerUnit = 50;
            const cell = Sprite.gridCell(tex, 4, 4, 1, 2);
            const sr = SpriteRenderer.attach(GameObject.create());
            sr.sprite = sprite;
            sr.sortingOrder = 3;
            sr.flipX = true;
            tex.filterMode = 'point';
            JSON.stringify([tex.src, tex.isLoaded, tex.filterMode, rt.width, rt.height, rt.isRenderTexture,
                mesh.vertexCount, mesh.triangleCount, sprite.pixelsPerUnit, sr.sortingOrder, sr.flipX,
                sr.sprite.texture.src, cell.hasRect])
            "#,
        );
        assert_eq!(
            out,
            r#"["textures/a.png",false,"point",256,128,true,3,1,50,3,true,"textures/a.png",true]"#
        );
        // The host reports the load; a script sees it.
        let cmds = take_commands();
        assert!(!cmds.is_empty());
        let _ = cmds.iter().any(|c| matches!(c, RenderCommand::Texture(_)));
        assert!(throws(ctx, "Mesh.create([1, 2], [0])").starts_with("TypeError"));
        assert!(throws(ctx, "Texture.load(7)").starts_with("TypeError"));
    });
}

// M9-API-141
//
// RenderSettings are namespace properties (getter/setter pairs) the renderer reads; Primitives
// make objects with a mesh and collider; LineRenderer and Cloth take arrays.
#[test]
fn render_settings_primitives_lines_and_cloth() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            RenderSettings.bloomEnabled = true;
            RenderSettings.bloomIntensity = 0.7;
            RenderSettings.ambientLight = new Color(0.1, 0.2, 0.3, 1);
            RenderSettings.skySunDirection = new Vector3(0, -1, 0);
            const cube = Primitives.create(PrimitiveType.Cube);
            const line = LineRenderer.attach(GameObject.create());
            line.positions = [new Vector3(0, 0, 0), new Vector3(1, 1, 1), new Vector3(2, 0, 0)];
            line.width = 0.2;
            const cloth = Cloth.attach(GameObject.create());
            cloth.addPin(new Vector3(0, 0, 0));
            cloth.damping = 0.3;
            const ball = GameObject.create();
            SphereCollider.attach(ball);
            cloth.addSphereCollider(ball);
            const ps = ParticleSystem.attach(GameObject.create());
            ps.rateOverTime = 30;
            ps.shapeType = 'sphere';
            ps.setStartLifetimeRange(1, 2);
            JSON.stringify([RenderSettings.bloomEnabled, RenderSettings.bloomIntensity,
                RenderSettings.ambientLight.b, RenderSettings.skySunDirection.y, cube !== null,
                line.positionCount, line.positionAt(1).x, line.width, cloth.damping,
                ps.rateOverTime, ps.shapeType, ps.startLifetime, ps.startLifetimeMax,
                MeshRenderer.findOn(cube) !== null])
            "#,
        );
        assert_eq!(
            out,
            "[true,0.7,0.3,-1,true,3,1,0.2,0.3,30,\"sphere\",1,2,true]"
        );
        assert!(platypus_render_api::settings().bloom.is_some());
        assert!(throws(ctx, "line.positions = 3").starts_with("TypeError"));
        assert!(throws(ctx, "RenderSettings.ambientLight = 5").starts_with("TypeError"));
    });
}

// M9-API-142
//
// `onStopped` is a callback on a particle system; the host reports its state and the JS function
// runs (and `stopAction` is honoured by the host).
#[test]
fn particle_callbacks_run_when_the_host_reports() {
    with_world(|ctx| {
        let _: f64 = js(
            ctx,
            "globalThis.stopped = 0; globalThis.ps2 = ParticleSystem.attach(GameObject.create()); ps2.onStopped(() => { stopped++; }); 0",
        );
        let id = platypus_render_api::particle_systems()[0].0;
        platypus_render_api::report_particles(id, 5, true);
        let none: f64 = js(ctx, "stopped");
        assert_eq!(none, 0.0, "still alive");
        platypus_render_api::report_particles(id, 0, false);
        let once: f64 = js(ctx, "stopped");
        assert_eq!(once, 1.0);
    });
}

// M9-API-143
//
// Models and scene objects: a Model is requested by path, instantiated before it loads, and the
// host answers the load.
#[test]
fn models_are_requested_and_instantiated() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const m = Model.load('models/rock.glb');
            const root = m.instantiate(new Vector3(1, 2, 3), null, null, null, 'none');
            JSON.stringify([m.src, m.isLoaded, m.failed, root !== null, String(m)])
            "#,
        );
        assert!(
            out.starts_with(r#"["models/rock.glb",false,false,true"#),
            "{out}"
        );
        assert!(throws(ctx, "Model.load(1)").starts_with("TypeError"));
    });
}
