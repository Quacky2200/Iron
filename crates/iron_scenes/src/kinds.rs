//! The component kinds of the engine components that are native: `Camera`, `Animation` and
//! `Animator`. Other engine components register their own as they are ported.

use iron_api::Handle;
use iron_engine_api::animation::AnimationRef;
use iron_engine_api::animator::AnimatorRef;
use iron_engine_api::camera::{self, CameraRef};
use iron_engine_api::color::Rgba;
use iron_kinds::Kind;
use iron_scene::ObjectId;
use serde_json::{Map, Value, json};

fn number(v: &Value) -> Option<f32> {
    v.as_f64().map(|x| x as f32)
}

fn color(v: &Value) -> Option<Rgba> {
    let a = v.get("value")?.as_array()?;
    let n = |i: usize| a.get(i).and_then(Value::as_f64);
    Some(Rgba {
        r: n(0)?,
        g: n(1)?,
        b: n(2)?,
        a: n(3)?,
    })
}

fn camera_kind() -> Kind {
    Kind {
        name: "Camera".into(),
        create: Box::new(|object| Ok(u64::from(CameraRef::attach(ObjectId::from_raw(object)).0))),
        on: Box::new(|object| {
            let o = ObjectId::from_raw(object);
            camera::cameras()
                .into_iter()
                .filter(|(_, d)| d.object == o)
                .map(|(c, _)| u64::from(c.0))
                .collect()
        }),
        save: Box::new(|h| {
            let mut m = Map::new();
            camera::with_camera(CameraRef(h as u32), |d| {
                m.insert("fieldOfView".into(), json!(f64::from(d.fov)));
                m.insert("nearClipPlane".into(), json!(f64::from(d.near)));
                m.insert("farClipPlane".into(), json!(f64::from(d.far)));
                let b = d.background;
                m.insert(
                    "backgroundColor".into(),
                    json!({ "$type": "Color", "value": [b.r, b.g, b.b, b.a] }),
                );
                m.insert("depth".into(), json!(f64::from(d.depth)));
                m.insert("orthographic".into(), json!(d.orthographic));
                m.insert(
                    "orthographicSize".into(),
                    json!(f64::from(d.orthographic_size)),
                );
            });
            m
        }),
        load: Box::new(|h, props| {
            let mut warnings = Vec::new();
            camera::with_camera(CameraRef(h as u32), |d| {
                for (k, v) in props {
                    let ok = match k.as_str() {
                        "fieldOfView" => number(v).map(|x| d.fov = x.clamp(1.0, 179.0)),
                        "nearClipPlane" => number(v).map(|x| d.near = x),
                        "farClipPlane" => number(v).map(|x| d.far = x),
                        "backgroundColor" => color(v).map(|c| d.background = c),
                        "depth" => number(v).map(|x| d.depth = x),
                        "orthographic" => v.as_bool().map(|x| d.orthographic = x),
                        "orthographicSize" => number(v).map(|x| d.orthographic_size = x),
                        other => {
                            warnings.push(format!("no property `{other}` yet; dropped"));
                            continue;
                        }
                    };
                    if ok.is_none() {
                        warnings.push(format!("property `{k}` has a value of the wrong shape"));
                    }
                }
            });
            warnings
        }),
        awake: Box::new(|_| {}),
        object: Box::new(|h| camera::with_camera(CameraRef(h as u32), |d| d.object.to_raw())),
        destroy: Box::new(|h| camera::remove(CameraRef(h as u32))),
    }
}

/// A kind with no saved properties yet: it is made and found, and its data comes later.
fn bare(
    name: &str,
    create: fn(ObjectId) -> u64,
    find: fn(ObjectId) -> u64,
    object: fn(u64) -> Option<ObjectId>,
) -> Kind {
    let n = name.to_owned();
    Kind {
        name: name.into(),
        create: Box::new(move |o| Ok(create(ObjectId::from_raw(o)))),
        on: Box::new(move |o| match find(ObjectId::from_raw(o)) {
            0 => Vec::new(),
            h => vec![h],
        }),
        save: Box::new(|_| Map::new()),
        load: Box::new(move |_, props| {
            props
                .keys()
                .map(|k| format!("{n}.{k} is not loaded from scene files yet; dropped"))
                .collect()
        }),
        awake: Box::new(|_| {}),
        object: Box::new(move |h| object(h).map(Handle::to_raw)),
        destroy: Box::new(|_| {}),
    }
}

/// Registers the engine's kinds on this thread (a later registration replaces an earlier one).
pub fn register_engine_kinds() {
    iron_kinds::register(camera_kind());
    iron_kinds::register(bare(
        "Animation",
        |o| AnimationRef::attach(o).0,
        |o| AnimationRef::find_on(o).0,
        |h| {
            let o = AnimationRef(h).game_object();
            (o.index != u32::MAX).then_some(o)
        },
    ));
    iron_kinds::register(bare(
        "Animator",
        |o| AnimatorRef::attach(o).0,
        |o| AnimatorRef::find_on(o).0,
        |h| {
            let o = AnimatorRef(h).game_object();
            (o.index != u32::MAX).then_some(o)
        },
    ));
}
