// SPDX-License-Identifier: Apache-2.0

//! Lists of handles, numbers and values as parameters, results and properties of the real engine
//! namespaces from QuickJS: the same native code compiled games call
//! (`tools/platypus_ts_tests/tests/engine_api/facade.rs`, M9-TS-3350). IDs are in TESTS.md (M9-API-163x).
mod common;
use common::{js, throws, with_world};

// M9-API-1634
#[test]
fn handle_and_number_lists_are_properties_that_replace_and_copy() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const named = (n) => { const o = GameObject.create(); o.name = n; return o; };
            const root = named('root');
            const b1 = named('b1');
            const b2 = named('b2');
            const skin = SkinnedMeshRenderer.attach(root);
            skin.bones = [b1, b2];
            const two = skin.bones;
            skin.bones = [b2];
            const cloth = Cloth.attach(root);
            cloth.sphereColliders = [b1];
            cloth.sphereColliders = [b2, b1];
            cloth.pins = [new Vector3(1, 2, 3)];
            cloth.pinBox = { min: { x: 0, y: 0, z: 0 }, max: new Vector3(1, 2, 3) };
            const box = cloth.pinBox;
            cloth.pinBox = null;
            const cam = Camera.attach(root);
            cam.layerCullDistances = [1, 2, -3];
            const d = cam.layerCullDistances;
            d[0] = 99;
            const names = QualitySettings.names;
            JSON.stringify([two.length, two[1] instanceof GameObject, two[1].name, skin.bones.length,
              cloth.sphereColliders.map(o => o.name), cloth.pins[0].z, box.max.z, box.min.x, cloth.pinBox,
              d.length, cam.layerCullDistances[0], cam.layerCullDistances[1], cam.layerCullDistances[2],
              names.length > 0, typeof names[0], root.constructor.name]);
            "#,
        );
        assert_eq!(
            out,
            r#"[2,true,"b2",1,["b2","b1"],3,3,0,null,32,1,2,0,true,"string","GameObject"]"#
        );
        // A list takes handles of the right class only.
        assert!(throws(ctx, "skin.bones = [1]").starts_with("TypeError"));
        assert!(throws(ctx, "skin.bones = [cam]").starts_with("TypeError"));
        assert!(throws(ctx, "cloth.pins = 3").starts_with("TypeError"));
    });
}

// M9-API-1635
#[test]
fn handle_list_results_and_the_scene_roots() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const named = (n) => { const o = GameObject.create(); o.name = n; return o; };
            const a = named('a');
            const b = named('b');
            const c = named('c');
            c.setParent(a, false);
            const roots = SceneManager.getActiveScene().getRootGameObjects();
            const sheet = Texture.create(4, 2, [Color.white, Color.white, Color.white, Color.white,
              Color.white, Color.white, Color.white, Color.white]);
            const grid = Sprite.grid(sheet, 2, 2, { x: 0, y: 0 }, 10);
            const plain = Sprite.grid(sheet, 2, 1);
            JSON.stringify([roots.map(o => o.name), grid.length, grid[3].pixelsPerUnit, grid[0].pivot.x,
              plain.length, plain[0].pixelsPerUnit, plain[0].pivot.x]);
            "#,
        );
        assert_eq!(out, r#"[["a","b"],4,10,0,2,100,0.5]"#);
    });
}

// M9-API-1639
#[test]
fn record_lists_with_handles_and_optional_keys_cross_as_json_from_quickjs() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const go = GameObject.create();
            const r1 = MeshRenderer.attach(go), r2 = MeshRenderer.attach(go);
            const lod = LODGroup.attach(go);
            const same = lod.setLODs([
              { screenRelativeTransitionHeight: 0.6, renderers: [r1, r2] },
              { screenRelativeTransitionHeight: 0.2, renderers: [r2] },
              { screenRelativeTransitionHeight: 0.9, renderers: [r1] },
            ]);
            const ps = ParticleSystem.attach(go);
            ps.bursts = [{ time: 0, count: 5 }, { time: 1, count: 2, cycles: 3, interval: 0.5 }];
            ps.colorOverLifetime = [{ time: 0, color: new Color(1, 0, 0, 1) }, { time: 1, color: Color.white }];
            ps.sizeOverLifetime = [{ time: 0, value: 1 }, { time: 1, value: 0.5 }];
            ps.shape = { type: 'box', size: new Vector3(1, 2, 3) };
            const b = ps.bursts, c = ps.colorOverLifetime, s = ps.shape;
            JSON.stringify([lod.lodCount, lod.lodRendererCount(0), lod.lodRendererCount(1), same.lodCount,
              b.length, b[0].cycles, b[1].interval, c[0].color instanceof Color, c[0].color.r, ps.sizeOverLifetime[1].value,
              s.type, s.size.y, s.angle]);
            "#,
        );
        assert_eq!(out, r#"[2,2,1,2,2,1,0.5,true,1,0.5,"box",2,25]"#);
        assert!(throws(ctx, "lod.setLODs(3)").starts_with("TypeError"));
        assert!(throws(ctx, "lod.setLODs([{ renderers: [1] }])").starts_with("TypeError"));
    });
}
