// SPDX-License-Identifier: Apache-2.0

//! `#[options]`: QuickJS functions whose optional parameters may come as one object, by camelCase
//! name (`Window.create({ title: "T", minWidth: 200 })`). IDs are in TESTS.md (M9-API-14xx).
use platypus_api::{ValueType, platypus_api};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl ValueType<3> for V3 {
    fn from_parts(p: [f64; 3]) -> Self {
        V3 {
            x: p[0],
            y: p[1],
            z: p[2],
        }
    }
    fn to_parts(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

#[platypus_api(namespace = "Opts", quickjs, values(Vec3 = V3 [x: f64, y: f64, z: f64]))]
pub mod opts {
    use super::V3;

    #[options]
    #[optional(b = 2, min_width, c, s)]
    pub fn make(a: f64, b: f64, min_width: f64, c: bool, s: &str) -> String {
        format!("{a}|{b}|{min_width}|{c}|{s}")
    }

    /// Moves a vector; `by` and `flip` may be given as an object.
    #[options]
    #[optional(by = 1, flip)]
    pub fn shift(v: V3, by: f64, flip: bool) -> f64 {
        (v.x + v.y + v.z) * by * if flip { -1.0 } else { 1.0 }
    }

    /// Positional only.
    #[optional(b = 2)]
    pub fn plain(a: f64, b: f64) -> f64 {
        a * 10.0 + b
    }
}

fn with_js<R>(f: impl for<'js> FnOnce(&rquickjs::Ctx<'js>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        opts::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

fn eval_s(ctx: &rquickjs::Ctx<'_>, src: &str) -> String {
    ctx.eval::<String, _>(src).unwrap()
}

// M9-API-1430
#[test]
fn an_options_object_fills_optional_parameters_by_name() {
    with_js(|ctx| {
        assert_eq!(
            eval_s(
                ctx,
                "Opts.make(1, { b: 5, minWidth: 100, c: true, s: 'x' })"
            ),
            "1|5|100|true|x"
        );
        // Keys the function does not have are ignored; order does not matter.
        assert_eq!(
            eval_s(ctx, "Opts.make(7, { s: 'y', other: 1, c: true })"),
            "7|2|NaN|true|y"
        );
    });
}

// M9-API-1431
#[test]
fn left_out_keys_take_the_declared_defaults() {
    with_js(|ctx| {
        assert_eq!(eval_s(ctx, "Opts.make(1, {})"), "1|2|NaN|false|");
        assert_eq!(
            eval_s(ctx, "Opts.make(1, { b: undefined, s: null })"),
            "1|2|NaN|false|"
        );
        assert_eq!(eval_s(ctx, "Opts.make(1)"), "1|2|NaN|false|");
        // An options object right after required parameters of a function with a value first.
        let n: f64 = ctx
            .eval("Opts.shift(new Vec3(1, 2, 3), { flip: true, by: 2 })")
            .unwrap();
        assert_eq!(n, -12.0);
        let n: f64 = ctx.eval("Opts.shift(new Vec3(1, 2, 3), {})").unwrap();
        assert_eq!(n, 6.0);
    });
}

// M9-API-1432
#[test]
fn positional_calls_still_work_and_a_value_class_argument_is_not_an_options_object() {
    with_js(|ctx| {
        assert_eq!(
            eval_s(ctx, "Opts.make(1, 3, 50, true, 'z')"),
            "1|3|50|true|z"
        );
        assert_eq!(eval_s(ctx, "Opts.make(1, 3)"), "1|3|NaN|false|");
        let n: f64 = ctx.eval("Opts.shift(new Vec3(1, 1, 1), 4, true)").unwrap();
        assert_eq!(n, -12.0);
        let n: f64 = ctx.eval("Opts.plain(3)").unwrap();
        assert_eq!(n, 32.0);
        // What counts as an options object: plain objects only.
        let plain: Vec<bool> = [
            "({ a: 1 })",
            "Object.create(null)",
            "[1]",
            "new Vec3(1, 2, 3)",
            "(() => 1)",
            "new Date()",
            "5",
            "'s'",
            "null",
            "new (class Foo {})()",
        ]
        .iter()
        .map(|src| {
            let v: rquickjs::Value = ctx.eval(*src).unwrap();
            platypus_api::qjs::options_object(ctx, &v).is_some()
        })
        .collect();
        assert_eq!(
            plain,
            [
                true, true, false, false, false, false, false, false, false, false
            ]
        );
    });
}
