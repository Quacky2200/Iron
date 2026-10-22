//! Enums, nullables and aliases of the real engine namespaces from QuickJS: the same native code
//! compiled games call (`tools/iron_ts_tests/tests/engine_api/facade.rs`, M9-TS-3340). IDs are in
//! TESTS.md (M9-API-162x).
mod common;
use common::{js, throws, with_world};

// M9-API-1627
#[test]
fn engine_enums_take_and_give_names_and_reject_unknown_ones() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            RenderSettings.fogMode = 'linear';
            const a = RenderSettings.fogMode;
            RenderSettings.volumetricFogResolution = 'ultra';
            const tex = Texture.create(1, 1, [new Color(1, 1, 1, 1)], 'point', 'mirror');
            const first = [tex.filterMode, tex.wrapMode];
            tex.filterMode = 'trilinear';
            const cube = Primitives.create(PrimitiveType.Cube, 'Box');
            const capsule = Primitives.create('capsule');
            const kinds = Screen.supportedFullScreenModes;
            Screen.fullScreenMode = 'Windowed';
            QualitySettings.shadows = 'hardOnly';
            JSON.stringify([
              a, RenderSettings.volumetricFogResolution, first, tex.filterMode,
              PrimitiveType.Cube, PrimitiveMesh === PrimitiveType, cube.name, capsule.name,
              kinds, Screen.fullScreenMode, QualitySettings.shadows,
              FogMode.name(2), FogMode.parse('exponential'), FullScreenMode.parse('Windowed'),
              WindowStyle.name(3), LoadSceneMode.parse('additive'),
              AnimatorController === AnimationController,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["linear","ultra",["point","mirror"],"trilinear","cube",true,"Box","Capsule",["ExclusiveFullScreen","FullScreenWindow","MaximizedWindow","Windowed"],"Windowed","hardOnly","exponentialSquared",1,3,"popup",1,true]"#
        );
        for bad in [
            "RenderSettings.fogMode = 'fuzzy'",
            "Texture.create(1, 1, [new Color(1, 1, 1, 1)], 'fuzzy')",
            "Primitives.create('torus')",
            "Window.create({ style: 'floating' })",
            "Screen.fullScreenMode = 'huge'",
        ] {
            assert!(throws(ctx, bad).starts_with("TypeError"), "{bad}");
        }
    });
}

// M9-API-1628
#[test]
fn engine_nullables_are_null_when_none_and_take_null() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const none = [RenderSettings.fixedExposure, Camera.main, Window.focused, Window.find('nope'), Window.at(99),
              Window.byHandle(99), Window.main.parent, Window.main.openError];
            RenderSettings.fixedExposure = 4.5;
            const set = RenderSettings.fixedExposure;
            RenderSettings.fixedExposure = null;
            const line = LineRenderer2D.attach(new GameObject('l'));
            const unfilled = line.fillColor;
            line.fillColor = new Color(0, 1, 0, 1);
            const filled = line.fillColor;
            line.fillColor = null;
            const w = Window.create({ title: 'A', style: 'tool' });
            JSON.stringify([none, set, RenderSettings.fixedExposure, unfilled, filled.g, line.fillColor,
              w.parent !== null, w.style, Window.at(0).handle === Window.main.handle]);
            "#,
        );
        assert_eq!(
            out,
            r#"[[null,null,null,null,null,null,null,null],4.5,null,null,1,null,true,"tool",true]"#
        );
    });
}

// M9-API-1629
#[test]
fn result_records_have_strings_and_lists_of_records_are_arrays() {
    with_world(|ctx| {
        iron_engine_api::screen::set_display(
            "Windowed",
            (1024, 768),
            vec![
                iron_engine_api::screen::Resolution {
                    width: 1920,
                    height: 1080,
                    refresh_rate: 144.0,
                },
                iron_engine_api::screen::Resolution {
                    width: 1280,
                    height: 720,
                    refresh_rate: 60.0,
                },
            ],
            iron_engine_api::screen::Resolution {
                width: 1920,
                height: 1080,
                refresh_rate: 144.0,
            },
            vec![],
        );
        let out: String = js(
            ctx,
            r#"
            const info = Animator.attach(new GameObject('a')).getCurrentAnimatorStateInfo();
            JSON.stringify([Screen.windowSize, Screen.currentResolution, Screen.resolutions, info]);
            "#,
        );
        assert_eq!(
            out,
            r#"[{"width":1024,"height":768},{"width":1920,"height":1080,"refreshRate":144},[{"width":1920,"height":1080,"refreshRate":144},{"width":1280,"height":720,"refreshRate":60}],{"name":"","normalizedTime":0,"length":0}]"#
        );
    });
}

// M9-API-1633
#[test]
fn render_settings_objects_are_properties_that_take_objects_and_null_and_read_back() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            RenderSettings.heightFog = { density: 0.2, baseHeight: 3, falloff: 0.5 };
            const h = RenderSettings.heightFog;
            RenderSettings.heightFog = null;
            RenderSettings.skybox = { zenith: new Color(1, 0, 0, 1), sunDirection: null };
            const sky = RenderSettings.skybox;
            RenderSettings.bloom = { threshold: 1.2, intensity: 0.5, scatter: 0.4 };
            RenderSettings.autoExposure = { minEV: -4, metering: 'spot', curve: [[-2, -1], [6, 0]] };
            RenderSettings.volumetricFog = { intensity: 2, resolution: 'ultra', albedo: new Color(0, 1, 0, 1) };
            const ae = RenderSettings.autoExposure;
            const vf = RenderSettings.volumetricFog;
            JSON.stringify([
              h, RenderSettings.heightFog, sky.sunDirection, sky.zenith instanceof Color, sky.zenith.r,
              RenderSettings.bloom.intensity, ae.minEV, ae.maxEV, ae.curve, ae.metering,
              vf.intensity, vf.resolution, vf.denoising, vf.albedo.g, RenderSettings.skyHasSun,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"[{"density":0.2,"baseHeight":3,"falloff":0.5},null,null,true,1,0.5,-4,20,[[-2,-1],[6,0]],"spot",2,"ultra","both",1,false]"#
        );
        for bad in [
            "RenderSettings.heightFog = { density: 1 }",
            "RenderSettings.volumetricFog = { resolution: 'fuzzy' }",
        ] {
            // A bad object is reported and ignored (it sets nothing), not an exception.
            js::<String>(ctx, &format!("{bad}; 'ok'"));
        }
        let kept: bool = js(ctx, "RenderSettings.heightFog === null");
        assert!(kept);
    });
}
