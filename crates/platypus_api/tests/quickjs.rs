// SPDX-License-Identifier: Apache-2.0

//! Typed QuickJS bindings for value classes and strings: a JS string is a Rust `&str`/`String`, a
//! value class is an object with the fields' names (a result is a new object). IDs are in
//! TESTS.md (M9-API-*).
use platypus_api::{ValueType, platypus_api};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ValueType<3> for Vec3 {
    fn from_parts(p: [f64; 3]) -> Self {
        Vec3 {
            x: p[0] as f32,
            y: p[1] as f32,
            z: p[2] as f32,
        }
    }
    fn to_parts(self) -> [f64; 3] {
        [self.x as f64, self.y as f64, self.z as f64]
    }
}

#[platypus_api(namespace = "Geo", quickjs, values(Vector3 = Vec3 [x: f32, y: f32, z: f32]))]
pub mod geo {
    use super::Vec3;

    pub fn length(v: Vec3) -> f32 {
        (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
    }

    pub fn scaled(v: Vec3, k: f32) -> Vec3 {
        Vec3 {
            x: v.x * k,
            y: v.y * k,
            z: v.z * k,
        }
    }

    pub fn shout(s: &str, times: f64) -> String {
        format!("{}{}", s.to_uppercase(), "!".repeat(times as usize))
    }

    pub fn chars(s: String) -> u32 {
        s.chars().count() as u32
    }
}

fn with_geo<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        geo::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-23
#[test]
fn value_classes_are_plain_objects_in_quickjs() {
    with_geo(|ctx| {
        let len: f64 = ctx.eval("Geo.length({ x: 3, y: 4, z: 0 })").unwrap();
        assert_eq!(len, 5.0);
        let o: rquickjs::Object = ctx.eval("Geo.scaled({ x: 1, y: 2, z: 3 }, 2)").unwrap();
        let xyz: Vec<f64> = ["x", "y", "z"]
            .iter()
            .map(|k| o.get::<_, f64>(*k).unwrap())
            .collect();
        assert_eq!(xyz, [2.0, 4.0, 6.0]);
        // Results chain into arguments.
        let len: f64 = ctx
            .eval("Geo.length(Geo.scaled({ x: 0, y: 3, z: 4 }, 2))")
            .unwrap();
        assert_eq!(len, 10.0);
    });
}

// M9-API-24
#[test]
fn strings_are_js_strings_in_quickjs() {
    with_geo(|ctx| {
        let s: String = ctx.eval("Geo.shout('héllo', 2)").unwrap();
        assert_eq!(s, "HÉLLO!!");
        let n: u32 = ctx.eval("Geo.chars('a😀b')").unwrap();
        assert_eq!(n, 3);
    });
}

// M9-API-25
#[test]
fn a_value_missing_a_field_is_a_js_exception() {
    with_geo(|ctx| {
        let r: rquickjs::Result<f64> = ctx.eval("Geo.length({ x: 3, y: 4 })");
        assert!(r.is_err());
        // The exception is catchable from script.
        let caught: bool = ctx
            .eval("(() => { try { Geo.length({ x: 1 }); return false; } catch (e) { return true; } })()")
            .unwrap();
        assert!(caught);
    });
}
