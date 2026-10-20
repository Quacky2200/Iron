//! `RenderSettings`: scene-wide lighting (ambient light, the procedural sky, bloom, tonemapping),
//! as `rendering.ts`'s `RenderSettings` object has it. The app reads [`settings`] when
//! [`version`] changed. Scene files save and load it under `renderSettings` (the active scene).

use super::*;

/// A procedural sky: gradients from the zenith to the horizon to the ground, and a sun disc.
#[derive(Clone, Debug, PartialEq)]
pub struct Sky {
    pub zenith: Rgba,
    pub horizon: Rgba,
    pub ground: Rgba,
    /// Towards the sun; none for no sun disc.
    pub sun_direction: Option<V3>,
    pub sun_color: Rgba,
    /// The disc's size in degrees.
    pub sun_size: f64,
}

impl Default for Sky {
    fn default() -> Self {
        let c = |r, g, b| Rgba { r, g, b, a: 1.0 };
        Sky {
            zenith: c(0.16, 0.32, 0.62),
            horizon: c(0.62, 0.72, 0.82),
            ground: c(0.22, 0.2, 0.18),
            sun_direction: Some(V3 {
                x: 0.4,
                y: 0.6,
                z: 0.3,
            }),
            sun_color: c(20.0, 18.0, 15.0),
            sun_size: 1.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bloom {
    /// Linear brightness where glowing starts (1: only what is brighter than white).
    pub threshold: f64,
    pub intensity: f64,
    /// 0..1: how far it spreads.
    pub scatter: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSettingsData {
    /// Light everything gets regardless of lights (linear).
    pub ambient_light: Rgba,
    /// `skybox` (with a sky) or `color` (`ambient_light`).
    pub ambient_mode: String,
    pub reflection_intensity: f64,
    pub skybox: Option<Sky>,
    pub bloom: Option<Bloom>,
    /// `none`, `neutral` or `aces`.
    pub tonemapping: String,
    /// Stops, before tonemapping.
    pub post_exposure: f64,
}

impl Default for RenderSettingsData {
    fn default() -> Self {
        RenderSettingsData {
            ambient_light: Rgba {
                r: 0.2,
                g: 0.2,
                b: 0.22,
                a: 1.0,
            },
            ambient_mode: "skybox".to_string(),
            reflection_intensity: 1.0,
            skybox: None,
            bloom: None,
            tonemapping: "none".to_string(),
            post_exposure: 0.0,
        }
    }
}

thread_local! {
    static SETTINGS: RefCell<(RenderSettingsData, u64)> = RefCell::new((RenderSettingsData::default(), 0));
}

/// The settings now.
pub fn settings() -> RenderSettingsData {
    SETTINGS.with(|s| s.borrow().0.clone())
}

/// A number that grows with every change, to tell the renderer to look again.
pub fn version() -> u64 {
    SETTINGS.with(|s| s.borrow().1)
}

fn change<R>(f: impl FnOnce(&mut RenderSettingsData) -> R) -> R {
    SETTINGS.with(|s| {
        let mut s = s.borrow_mut();
        s.1 += 1;
        f(&mut s.0)
    })
}

fn read<R>(f: impl FnOnce(&RenderSettingsData) -> R) -> R {
    SETTINGS.with(|s| f(&s.borrow().0))
}

pub(crate) fn reset_render_settings() {
    SETTINGS.with(|s| *s.borrow_mut() = (RenderSettingsData::default(), 0));
}

fn color_json(c: Rgba) -> Value {
    json!({ "$type": "Color", "value": [c.r, c.g, c.b, c.a] })
}

/// The settings as a scene file holds them (`renderSettings`).
pub fn save_render_settings() -> Value {
    let s = settings();
    let sky = s.skybox.as_ref().map_or(Value::Null, |k| {
        json!({
            "zenith": color_json(k.zenith),
            "horizon": color_json(k.horizon),
            "ground": color_json(k.ground),
            "sunDirection": k.sun_direction.map_or(Value::Null, |d| json!({ "$type": "Vector3", "value": [d.x, d.y, d.z] })),
            "sunColor": color_json(k.sun_color),
            "sunSize": k.sun_size,
        })
    });
    let bloom = s.bloom.as_ref().map_or(
        Value::Null,
        |b| json!({ "threshold": b.threshold, "intensity": b.intensity, "scatter": b.scatter }),
    );
    json!({
        "ambientLight": color_json(s.ambient_light),
        "ambientMode": s.ambient_mode,
        "reflectionIntensity": s.reflection_intensity,
        "skybox": sky,
        "bloom": bloom,
    })
}

/// Applies the `renderSettings` of a scene file (what a scene loaded with is applied when it
/// becomes the active scene). Missing keys keep the current value.
pub fn apply_render_settings(saved: &Value) {
    change(|s| {
        if let Some(c) = saved.get("ambientLight").and_then(color_of) {
            s.ambient_light = c;
        }
        if let Some(m) = saved.get("ambientMode").and_then(Value::as_str)
            && (m == "skybox" || m == "color")
        {
            s.ambient_mode = m.to_string();
        }
        if let Some(r) = saved.get("reflectionIntensity").and_then(Value::as_f64) {
            s.reflection_intensity = r;
        }
        match saved.get("skybox") {
            Some(Value::Null) => s.skybox = None,
            Some(k) if k.is_object() => {
                let mut sky = Sky::default();
                if let Some(c) = k.get("zenith").and_then(color_of) {
                    sky.zenith = c;
                }
                if let Some(c) = k.get("horizon").and_then(color_of) {
                    sky.horizon = c;
                }
                if let Some(c) = k.get("ground").and_then(color_of) {
                    sky.ground = c;
                }
                match k.get("sunDirection") {
                    Some(Value::Null) => sky.sun_direction = None,
                    Some(d) => {
                        if let Some(v) = v3_of(d) {
                            sky.sun_direction = Some(v);
                        }
                    }
                    None => {}
                }
                if let Some(c) = k.get("sunColor").and_then(color_of) {
                    sky.sun_color = c;
                }
                if let Some(z) = k.get("sunSize").and_then(Value::as_f64) {
                    sky.sun_size = z;
                }
                s.skybox = Some(sky);
            }
            _ => {}
        }
        match saved.get("bloom") {
            Some(Value::Null) => s.bloom = None,
            Some(b) if b.is_object() => {
                let n = |k: &str, d: f64| b.get(k).and_then(Value::as_f64).unwrap_or(d);
                s.bloom = Some(Bloom {
                    threshold: n("threshold", 1.0),
                    intensity: n("intensity", 1.0),
                    scatter: n("scatter", 0.5),
                });
            }
            _ => {}
        }
    });
}

/// Whether the settings differ from a fresh game's (a scene then saves them).
pub fn settings_changed() -> bool {
    read(|s| *s != RenderSettingsData::default())
}

#[iron_api(
    namespace = "RenderSettings",
    values(
        Color = Rgba [r: f64, g: f64, b: f64, a: f64],
        Vector3 = V3 [x: f64, y: f64, z: f64]
    )
)]
pub mod render_settings_api {
    use super::*;

    /// Light everything gets regardless of lights (linear).
    #[property]
    pub fn ambient_light() -> Rgba {
        read(|s| s.ambient_light)
    }

    pub fn set_ambient_light(c: Rgba) {
        change(|s| s.ambient_light = c);
    }

    /// Where ambient light comes from: the sky (`skybox`, with one) or `ambientLight` (`color`).
    #[property]
    pub fn ambient_mode() -> String {
        read(|s| s.ambient_mode.clone())
    }

    pub fn set_ambient_mode(mode: &str) {
        if mode == "skybox" || mode == "color" {
            change(|s| s.ambient_mode = mode.to_string());
        }
    }

    /// How bright the sky's reflections and light are.
    #[property]
    pub fn reflection_intensity() -> f64 {
        read(|s| s.reflection_intensity)
    }

    pub fn set_reflection_intensity(i: f64) {
        change(|s| s.reflection_intensity = i);
    }

    /// Whether there is a procedural sky: what reflections show, where sky light comes from, and
    /// what cameras with `clearFlags: "skybox"` show. Turning it on gives the default sky.
    #[property]
    pub fn skybox_enabled() -> bool {
        read(|s| s.skybox.is_some())
    }

    pub fn set_skybox_enabled(on: bool) {
        change(|s| {
            if on {
                s.skybox.get_or_insert_with(Sky::default);
            } else {
                s.skybox = None;
            }
        });
    }

    #[property]
    pub fn sky_zenith() -> Rgba {
        read(|s| s.skybox.clone().unwrap_or_default().zenith)
    }

    /// Setting a sky property turns the sky on.
    pub fn set_sky_zenith(c: Rgba) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).zenith = c);
    }

    #[property]
    pub fn sky_horizon() -> Rgba {
        read(|s| s.skybox.clone().unwrap_or_default().horizon)
    }

    pub fn set_sky_horizon(c: Rgba) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).horizon = c);
    }

    #[property]
    pub fn sky_ground() -> Rgba {
        read(|s| s.skybox.clone().unwrap_or_default().ground)
    }

    pub fn set_sky_ground(c: Rgba) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).ground = c);
    }

    /// Towards the sun.
    #[property]
    pub fn sky_sun_direction() -> V3 {
        read(|s| {
            s.skybox
                .clone()
                .unwrap_or_default()
                .sun_direction
                .unwrap_or(V3::ZERO)
        })
    }

    pub fn set_sky_sun_direction(d: V3) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).sun_direction = Some(d));
    }

    /// Whether the sky has a sun disc.
    #[property]
    pub fn sky_has_sun() -> bool {
        read(|s| s.skybox.as_ref().is_none_or(|k| k.sun_direction.is_some()))
    }

    pub fn set_sky_has_sun(on: bool) {
        change(|s| {
            let sky = s.skybox.get_or_insert_with(Sky::default);
            sky.sun_direction = on.then(|| {
                sky.sun_direction
                    .unwrap_or(Sky::default().sun_direction.unwrap_or(V3::UP))
            });
        });
    }

    #[property]
    pub fn sky_sun_color() -> Rgba {
        read(|s| s.skybox.clone().unwrap_or_default().sun_color)
    }

    pub fn set_sky_sun_color(c: Rgba) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).sun_color = c);
    }

    /// The disc's size in degrees.
    #[property]
    pub fn sky_sun_size() -> f64 {
        read(|s| s.skybox.clone().unwrap_or_default().sun_size)
    }

    pub fn set_sky_sun_size(z: f64) {
        change(|s| s.skybox.get_or_insert_with(Sky::default).sun_size = z);
    }

    /// Bloom over every camera: bright things glow onto their surroundings. Turning it on gives
    /// threshold 1, intensity 1, scatter 0.5.
    #[property]
    pub fn bloom_enabled() -> bool {
        read(|s| s.bloom.is_some())
    }

    pub fn set_bloom_enabled(on: bool) {
        change(|s| {
            s.bloom = on.then(|| {
                s.bloom.clone().unwrap_or(Bloom {
                    threshold: 1.0,
                    intensity: 1.0,
                    scatter: 0.5,
                })
            })
        });
    }

    #[property]
    pub fn bloom_threshold() -> f64 {
        read(|s| s.bloom.as_ref().map_or(1.0, |b| b.threshold))
    }

    pub fn set_bloom_threshold(v: f64) {
        change(|s| bloom_of(s).threshold = v);
    }

    #[property]
    pub fn bloom_intensity() -> f64 {
        read(|s| s.bloom.as_ref().map_or(1.0, |b| b.intensity))
    }

    pub fn set_bloom_intensity(v: f64) {
        change(|s| bloom_of(s).intensity = v);
    }

    #[property]
    pub fn bloom_scatter() -> f64 {
        read(|s| s.bloom.as_ref().map_or(0.5, |b| b.scatter))
    }

    pub fn set_bloom_scatter(v: f64) {
        change(|s| bloom_of(s).scatter = v);
    }

    /// How HDR light (brighter than white) becomes screen colours: `none` clips it, `neutral`
    /// keeps colours and rolls highlights off softly, `aces` is filmic.
    #[property]
    pub fn tonemapping() -> String {
        read(|s| s.tonemapping.clone())
    }

    pub fn set_tonemapping(mode: &str) {
        if ["none", "neutral", "aces"].contains(&mode) {
            change(|s| s.tonemapping = mode.to_string());
        }
    }

    /// Brightens (positive) or darkens (negative) everything, in stops, before tonemapping.
    #[property]
    pub fn post_exposure() -> f64 {
        read(|s| s.post_exposure)
    }

    pub fn set_post_exposure(ev: f64) {
        change(|s| s.post_exposure = ev);
    }

    fn bloom_of(s: &mut RenderSettingsData) -> &mut Bloom {
        s.bloom.get_or_insert(Bloom {
            threshold: 1.0,
            intensity: 1.0,
            scatter: 0.5,
        })
    }
}
