//! QuickJS side of the boundary: opaque handle objects and value-class argument reading.
//!
//! An engine handle (a `GameObject`, a `Body`) reaches QuickJS as an object of the class the
//! manifest names (`GameObject`), made by [`handle_new`]. The numeric handle sits in the object's
//! internal slot (a Rust class instance, like a V8 internal field), not in a property: the object
//! has no own properties, `JSON.stringify` gives `{}`, and every member lives on the class's
//! prototype. Id 0 is "no object" and travels as `null`.

use rquickjs::{
    Class, Ctx, Exception, Function, JsLifetime, Object, Result, Value, class::Trace,
    function::Args,
};

/// What a handle object holds: the id and the class it was made as (`Transform` and `GameObject`
/// can share an id but are not interchangeable).
#[derive(Trace, JsLifetime)]
#[rquickjs::class(rename = "IronHandle")]
pub struct HandleBox {
    #[qjs(skip_trace)]
    pub raw: u64,
    #[qjs(skip_trace)]
    pub class: &'static str,
}

/// The JS handle object for `raw` as class `class` (a global class made by `register_quickjs`);
/// `null` for the id 0.
pub fn handle_new<'js>(ctx: &Ctx<'js>, class: &'static str, raw: u64) -> Result<Value<'js>> {
    if raw == 0 {
        return Ok(Value::new_null(ctx.clone()));
    }
    let ctor: Object = ctx.globals().get(class)?;
    let proto: Object = ctor.get("prototype")?;
    Ok(Class::instance_proto(HandleBox { raw, class }, proto)?.into_value())
}

/// The id inside a handle object of `class`; `null` and `undefined` are the id 0. Anything else is
/// a `TypeError`.
pub fn handle_arg<'js>(ctx: &Ctx<'js>, v: &Value<'js>, class: &'static str) -> Result<u64> {
    if v.is_null() || v.is_undefined() {
        return Ok(0);
    }
    if let Ok(c) = Class::<HandleBox>::from_value(v) {
        let b = c.borrow();
        if b.class == class {
            return Ok(b.raw);
        }
        return Err(Exception::throw_type(
            ctx,
            &format!("expected a {class}, got a {}", b.class),
        ));
    }
    Err(Exception::throw_type(
        ctx,
        &format!("expected a {class} or null"),
    ))
}

/// The value as an options object (`Window.create({ title: "T" })`): a plain object whose prototype
/// is `Object.prototype` (or none), not an array, not a function, not a handle or value-class
/// instance (those have their class's prototype).
pub fn options_object<'js>(ctx: &Ctx<'js>, v: &Value<'js>) -> Option<Object<'js>> {
    let o = v.as_object()?;
    if o.is_array() || o.is_function() {
        return None;
    }
    let Some(proto) = o.get_prototype() else {
        return Some(o.clone());
    };
    let object_proto: Object = ctx
        .globals()
        .get::<_, Object>("Object")
        .ok()?
        .get("prototype")
        .ok()?;
    (proto.as_value() == object_proto.as_value()).then(|| o.clone())
}

/// Like [`handle_arg`] for the receiver of a method, which must be an object of the class.
pub fn recv_arg<'js>(ctx: &Ctx<'js>, v: &Value<'js>, class: &'static str) -> Result<u64> {
    if v.is_null() || v.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            &format!("this is not a {class} (was the method detached?)"),
        ));
    }
    handle_arg(ctx, v, class)
}

/// The fields of a value class from a script value: an array of exactly `N` numbers (what
/// `JSON.parse` of `[x,y,z]` gives, and what `toJSON` writes) or an object with the fields'
/// names (a missing field is an exception).
pub fn value_parts<'js, const N: usize>(
    ctx: &Ctx<'js>,
    o: &Object<'js>,
    names: [&str; N],
) -> Result<[f64; N]> {
    let mut out = [0.0; N];
    if o.is_array() {
        let n: usize = o.get("length")?;
        if n != N {
            return Err(Exception::throw_type(
                ctx,
                "the array has the wrong number of values",
            ));
        }
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = o.get::<_, f64>(i as u32)?;
        }
    } else {
        for (slot, name) in out.iter_mut().zip(names) {
            *slot = o.get::<_, f64>(name)?;
        }
    }
    Ok(out)
}
/// Makes the namespace object the global `name`. When a class (a handle class registered by another
/// module) already holds the name, the namespace's members that the class lacks are added to the
/// class instead, so `Scene` can be both the scene class and the scene helper functions.
pub fn install_namespace<'js>(ctx: &Ctx<'js>, name: &str, ns: Object<'js>) -> Result<()> {
    let globals = ctx.globals();
    let existing: Value = globals.get(name)?;
    if existing.is_function() && existing.as_object() != Some(&ns) {
        let merge: rquickjs::Function = ctx.eval(
            "(function(cls, ns) { for (const k of Object.getOwnPropertyNames(ns)) { if (!Object.prototype.hasOwnProperty.call(cls, k)) Object.defineProperty(cls, k, Object.getOwnPropertyDescriptor(ns, k)); } })",
        )?;
        merge.call::<_, ()>((existing, ns))?;
        return Ok(());
    }
    globals.set(name, ns)
}

/// Keeps a script function for as long as the engine holds it. The reference count of the
/// function keeps it (and what it closes over) alive and visible to the collector as a root; drop
/// every `Callback` (clear the engine's listeners) before the runtime is dropped.
pub fn keep_function<'js>(f: Function<'js>) -> Function<'static> {
    // SAFETY: the function owns a reference to its context; only the borrow's lifetime is erased.
    // It is used on the thread that runs the scripts, while its runtime lives.
    unsafe { std::mem::transmute::<Function<'js>, Function<'static>>(f) }
}

/// Calls a kept function with `n` arguments made by `fill`. An exception (or a conversion error)
/// is not propagated: the engine called, not a script, so it is recorded for
/// `iron_api::callback::take_errors`.
pub fn call_function(
    f: &Function<'static>,
    n: usize,
    fill: impl FnOnce(&Ctx<'static>, &mut Args<'static>) -> Result<()>,
) {
    let ctx = f.ctx().clone();
    let mut args = Args::new(ctx.clone(), n);
    let r = fill(&ctx, &mut args).and_then(|()| f.call_arg::<Value>(args).map(|_| ()));
    if r.is_err() {
        let thrown = ctx.catch();
        let text = if let Some(o) = thrown.as_object() {
            o.get::<_, String>("message").unwrap_or_default()
        } else {
            String::new()
        };
        crate::callback::push_error(format!(
            "a callback given to the engine threw an exception: {text}"
        ));
    }
}
