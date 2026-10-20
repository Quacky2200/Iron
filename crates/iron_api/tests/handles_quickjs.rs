//! Handle classes in QuickJS: opaque objects with the numeric id in an internal slot, members on the
//! prototype, `null` for the id 0, arrays in, value classes as JSON arrays. IDs are in TESTS.md
//! (M9-API-100..).
use iron_api::{Handle, ValueType, iron_api};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThingId(pub u64);
impl Handle for ThingId {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        ThingId(raw)
    }
}

/// A second handle class over the same ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartId(pub u64);
impl Handle for PartId {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        PartId(raw)
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}
impl ValueType<4> for Rgba {
    fn from_parts(p: [f64; 4]) -> Self {
        Rgba {
            r: p[0],
            g: p[1],
            b: p[2],
            a: p[3],
        }
    }
    fn to_parts(self) -> [f64; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

thread_local! {
    static HP: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[iron_api(
    namespace = "Things",
    quickjs,
    handles(Thing = ThingId, Part = PartId),
    values(Vector3 = V3 [x: f64, y: f64, z: f64], Color = Rgba [r: f64, g: f64, b: f64, a: f64])
)]
pub mod things {
    use super::{HP, PartId, Rgba, ThingId, V3};

    impl ThingId {
        /// Makes a thing; `new Thing(5)` calls it too.
        pub fn create(health: f64) -> ThingId {
            HP.with(|h| {
                let mut h = h.borrow_mut();
                h.push(health);
                ThingId(h.len() as u64)
            })
        }

        /// The thing with a number, or none.
        pub fn find(index: f64) -> ThingId {
            let n = HP.with(|h| h.borrow().len());
            if index >= 1.0 && (index as usize) <= n {
                ThingId(index as u64)
            } else {
                ThingId(0)
            }
        }

        #[property]
        pub fn health(self) -> f64 {
            HP.with(|h| h.borrow()[self.0 as usize - 1])
        }

        pub fn set_health(self, value: f64) {
            HP.with(|h| h.borrow_mut()[self.0 as usize - 1] = value);
        }

        pub fn damage(self, amount: f64) -> bool {
            let left = self.health() - amount;
            self.set_health(left);
            left > 0.0
        }

        #[property]
        pub fn part(self) -> PartId {
            PartId(self.0)
        }

        /// The friend, or none when `friend` is left out.
        #[optional(friend)]
        pub fn with_friend(self, friend: ThingId) -> f64 {
            self.health() + if friend.0 == 0 { 0.0 } else { friend.health() }
        }

        /// A string result and a value result.
        #[property]
        pub fn label(self) -> String {
            format!("thing#{}", self.0)
        }

        pub fn offset(self, by: V3) -> V3 {
            V3 {
                x: by.x + self.0 as f64,
                y: by.y,
                z: by.z,
            }
        }

        /// Two Rust methods with one script name: chosen by argument count.
        #[js_name = "plus"]
        pub fn plus_one(self, a: f64) -> f64 {
            self.0 as f64 + a
        }

        #[js_name = "plus"]
        pub fn plus_two(self, a: f64, b: f64) -> f64 {
            self.0 as f64 + a + b * 10.0
        }

        /// Six parameters: read from the rest list.
        pub fn mix(self, a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
            self.0 as f64 + a + b * 2.0 + c * 3.0 + d * 4.0 + e * 5.0
        }
    }

    impl PartId {
        #[property]
        pub fn weight(self) -> f64 {
            ThingId(self.0).health() * 2.0
        }

        pub fn owner(self) -> ThingId {
            ThingId(self.0)
        }
    }

    /// A namespace function with handles in and out.
    pub fn strongest(a: ThingId, b: ThingId) -> ThingId {
        if a.health() >= b.health() { a } else { b }
    }

    pub fn none() -> ThingId {
        ThingId(0)
    }

    /// `&[f64]` from a JS array.
    pub fn sum(xs: &[f64]) -> f64 {
        xs.iter().sum()
    }

    /// `&[V]` from a JS array of vectors (objects or arrays).
    pub fn centroid(vs: &[V3]) -> V3 {
        let n = vs.len().max(1) as f64;
        V3 {
            x: vs.iter().map(|v| v.x).sum::<f64>() / n,
            y: vs.iter().map(|v| v.y).sum::<f64>() / n,
            z: vs.iter().map(|v| v.z).sum::<f64>() / n,
        }
    }

    pub fn tint(c: Rgba, k: f64) -> Rgba {
        Rgba {
            r: c.r * k,
            g: c.g * k,
            b: c.b * k,
            a: c.a,
        }
    }
}

/// Another namespace that shares the `Thing` class.
#[iron_api(namespace = "Other", quickjs, handles(Thing = ThingId))]
pub mod other {
    use super::ThingId;

    pub fn doubled(t: ThingId) -> f64 {
        t.health() * 2.0
    }
}

/// A class named like its namespace: the namespace object is the class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterId(pub u64);
impl Handle for CounterId {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        CounterId(raw)
    }
}

#[iron_api(namespace = "Counter", quickjs, handles(Counter = CounterId))]
pub mod counter {
    use super::CounterId;

    impl CounterId {
        pub fn create(start: f64) -> CounterId {
            CounterId(start as u64 + 1)
        }

        #[property]
        pub fn value(self) -> f64 {
            (self.0 - 1) as f64
        }
    }

    /// A namespace function next to the class statics.
    pub fn total(a: CounterId, b: CounterId) -> f64 {
        a.value() + b.value()
    }
}

/// A namespace with the same name as the `Part` class.
#[iron_api(namespace = "Part", quickjs)]
pub mod part_ns {
    pub fn kind() -> f64 {
        7.0
    }
}

fn run<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    HP.with(|h| h.borrow_mut().clear());
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        things::register_quickjs(&ctx).unwrap();
        other::register_quickjs(&ctx).unwrap();
        counter::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

fn js<T: for<'a> rquickjs::FromJs<'a>>(ctx: &rquickjs::Ctx<'_>, code: &str) -> T {
    match ctx.eval::<T, _>(code) {
        Ok(v) => v,
        Err(rquickjs::Error::Exception) => panic!("{:?}", ctx.catch()),
        Err(e) => panic!("{e}"),
    }
}

// M9-API-100
#[test]
fn a_handle_is_an_opaque_object_with_members_on_the_prototype() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const t = Thing.create(10);
            JSON.stringify([
                JSON.stringify(t),
                Object.keys(t).length,
                Object.getOwnPropertyNames(t).length,
                Object.prototype.toString.call(t),
                String(t),
                t instanceof Thing,
                typeof t,
                'health' in t,
                t.hasOwnProperty('health'),
                Object.keys(Thing.prototype).length,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["{}",0,0,"[object Thing]","[object Thing]",true,"object",true,false,0]"#
        );
        // Properties the script adds are its own.
        let own: String = js(
            ctx,
            "const u = Thing.create(1); u.tag = 'x'; JSON.stringify([Object.keys(u), JSON.stringify(u)])",
        );
        assert_eq!(own, r#"[["tag"],"{\"tag\":\"x\"}"]"#);
    });
}

// M9-API-101
#[test]
fn statics_methods_properties_and_setters() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const a = Thing.create(10);
            const alive = a.damage(4);
            a.health = a.health + 1;
            const dead = a.damage(100);
            JSON.stringify([alive, dead, a.health, a.part.weight, a.part.owner().health, new Thing(7).health,
                a.label, Things.Thing === Thing])
            "#,
        );
        assert_eq!(out, r#"[true,false,-93,-186,-93,7,"thing#1",true]"#);
        // A getter alone is read only (a TypeError in strict mode, which scripts and modules are).
        let ro: String = js(
            ctx,
            "const p = Thing.create(3).part; try { p.weight = 0; 'no error' } catch (e) { e.name + ' ' + p.weight }",
        );
        assert_eq!(ro, "TypeError 6");
    });
}

// M9-API-102
#[test]
fn id_zero_is_null_in_both_directions() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const a = Thing.create(2), b = Thing.create(5);
            JSON.stringify([
                Things.none(), Thing.find(99), Thing.find(1) !== null,
                a.withFriend(), a.withFriend(null), a.withFriend(b), Things.strongest(a, b) !== null,
            ])
            "#,
        );
        assert_eq!(out, "[null,null,true,2,2,7,true]");
    });
}

// M9-API-103
#[test]
fn a_wrong_class_is_a_type_error() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const t = Thing.create(1);
            const bad = (f) => { try { f(); return 'no error'; } catch (e) { return e.name + ': ' + e.message; } };
            JSON.stringify([
                bad(() => Things.strongest(t.part, t)),
                bad(() => Things.strongest(5, t)),
                bad(() => Things.strongest({}, t)),
                bad(() => t.damage.call({}, 1)),
                bad(() => t.damage.call(undefined, 1)),
                bad(() => new Part()),
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["TypeError: expected a Thing, got a Part","TypeError: expected a Thing or null","TypeError: expected a Thing or null","TypeError: expected a Thing or null","TypeError: this is not a Thing (was the method detached?)","TypeError: Part cannot be constructed"]"#
        );
    });
}

// M9-API-104
#[test]
fn modules_share_a_class_and_a_namespace_can_be_its_class() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const t = Thing.create(21);
            const c = new Counter(4), d = Counter.create(1);
            JSON.stringify([Other.doubled(t), c.value, Counter.total(c, d), c instanceof Counter,
                Object.prototype.toString.call(c), Counter === globalThis.Counter])
            "#,
        );
        assert_eq!(out, r#"[42,4,5,true,"[object Counter]",true]"#);
    });
}

// M9-API-105
#[test]
fn many_parameters_value_results_and_arrays() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const t = Thing.create(1);
            const o = t.offset({ x: 1, y: 2, z: 3 });
            const c = Things.centroid([{ x: 0, y: 0, z: 0 }, [2, 4, 6], new Vector3(4, 8, 12)]);
            JSON.stringify([t.mix(1, 1, 1, 1, 1), o instanceof Vector3, o.x, o.y, o.z,
                Things.sum([1, 2, 3.5]), Things.sum([]), c.x, c.y, c.z])
            "#,
        );
        assert_eq!(out, "[16,true,2,2,3,6.5,0,2,4,6]");
        let bad: String = js(ctx, "try { Things.sum('no'); 'ok' } catch (e) { e.name }");
        assert_eq!(bad, "TypeError");
    });
}

// M9-API-106
#[test]
fn the_same_calls_from_compiled_code_agree() {
    run(|ctx| {
        // Compiled code calls the shims with the raw id; QuickJS goes through the objects.
        let t = things::iron_Thing_create(10.0);
        assert!(things::iron_Thing_damage(t, 4.0));
        things::iron_Thing_setHealth(t, 100.0);
        let part = things::iron_Thing_part(t);
        let compiled_weight = things::iron_Part_weight(part);
        let js_weight: f64 = js(
            ctx,
            "const t = Thing.create(10); t.damage(4); t.health = 100; t.part.weight",
        );
        // The JS thing is the second one made (index 2), but its health is what is compared.
        assert_eq!(compiled_weight, js_weight);
        assert_eq!(compiled_weight, 200.0);
    });
}

// M9-API-107
#[test]
fn value_classes_stringify_as_arrays_and_parse_from_arrays_or_objects() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const v = new Vector3(1, 2, 3);
            const c = new Color(0.5, 0.25, 1, 0.75);
            const a = Vector3.fromJSON(JSON.parse(JSON.stringify(v)));
            const o = Vector3.fromJSON(JSON.parse('{"x":1,"z":3,"w":9}'));
            const col = Color.fromJSON({ r: 1, g: 0.5 });
            const bad = (f) => { try { f(); return 'no error'; } catch (e) { return e.name + ': ' + e.message; } };
            JSON.stringify([
                JSON.stringify(v), JSON.stringify(c), JSON.stringify({ at: v, n: [c] }),
                a instanceof Vector3, a.x, a.y, a.z, o.y, col.b, col.a,
                bad(() => Vector3.fromJSON([1, 2])), bad(() => Vector3.fromJSON(5)),
                // Functions take the array form as well.
                Things.centroid(JSON.parse('[[1,1,1],[3,3,3]]')).x,
                Things.tint(JSON.parse('[1,1,1,0.5]'), 2).r,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"["[1,2,3]","[0.5,0.25,1,0.75]","{\"at\":[1,2,3],\"n\":[[0.5,0.25,1,0.75]]}",true,1,2,3,0,0,1,"TypeError: the array has the wrong number of values","TypeError: expected an array of numbers or an object",2,2]"#
        );
    });
}
// M9-API-112
#[test]
fn a_namespace_and_a_class_with_one_name_merge_in_either_order() {
    for class_first in [true, false] {
        let rt = rquickjs::Runtime::new().unwrap();
        let ctx = rquickjs::Context::full(&rt).unwrap();
        ctx.with(|ctx| {
            if class_first {
                things::register_quickjs(&ctx).unwrap();
                part_ns::register_quickjs(&ctx).unwrap();
            } else {
                part_ns::register_quickjs(&ctx).unwrap();
                things::register_quickjs(&ctx).unwrap();
            }
            let out: String = js(
                &ctx,
                "const t = Thing.create(4); JSON.stringify([Part.kind(), t.part instanceof Part, t.part.weight, typeof Part])",
            );
            assert_eq!(out, r#"[7,true,8,"function"]"#, "class first: {class_first}");
        });
    }
}

// M9-API-121
#[test]
fn methods_with_one_script_name_dispatch_by_argument_count() {
    run(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const t = Thing.create(1);
            const bad = (() => { try { t.plus(); return 'no error'; } catch (e) { return e.name; } })();
            JSON.stringify([t.plus(5), t.plus(5, 2), bad])
            "#,
        );
        assert_eq!(out, r#"[6,26,"TypeError"]"#);
    });
}
