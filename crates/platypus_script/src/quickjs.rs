// SPDX-License-Identifier: Apache-2.0

use std::cell::RefCell;
use std::rc::Rc;

use iron_core::Op;
use rquickjs::context::EvalOptions;
use rquickjs::function::Rest;
use rquickjs::{Array, Context, Ctx, Exception, Function, Object, Runtime, Type};

use crate::{
    Commit, LogLevel, LogSink, MAX_TIMERS_PER_TICK, OpBuffer, PRELUDE, ScriptError, ScriptRuntime,
    Value, tracing_log_sink,
};

/// Installs a native API into a context: what `#[iron_api(.., quickjs)]` generates as
/// `register_quickjs` (for example `iron_scene::api::scene_api::register_quickjs`).
pub type NativeInit = for<'js> fn(&Ctx<'js>) -> rquickjs::Result<()>;

thread_local! {
    /// Whether `__iron_op` calls are timed (a bench run), and their milliseconds so far.
    static OP_TIME: std::cell::Cell<(bool, f64)> = const { std::cell::Cell::new((false, 0.0)) };
}

/// Times the `__iron_op` calls from now on (`iron --bench`: the part of script time spent
/// copying op arguments out of JavaScript).
pub fn time_ops() {
    OP_TIME.with(|o| o.set((true, o.get().1)));
}

/// The milliseconds `__iron_op` calls took since the last call.
pub fn take_op_time() -> f64 {
    OP_TIME.with(|o| {
        let (on, ms) = o.get();
        o.set((on, 0.0));
        ms
    })
}

/// The QuickJS backend (via `rquickjs`). Interpreter only, so it also works on iOS.
pub struct QuickJsRuntime {
    // Field order matters: the context must drop before its runtime.
    context: Context,
    runtime: Runtime,
    shared: Rc<RefCell<Shared>>,
    /// Native APIs to install again in every new context (`reset()`).
    natives: Vec<NativeInit>,
}

/// State the native functions write into; survives `reset()` (but is cleared by it).
struct Shared {
    ops: OpBuffer,
    /// Each with the UI tree it's for (0: the screen; else a world panel's id).
    commits: Vec<(u32, Commit)>,
    errors: Vec<ScriptError>,
    log: LogSink,
}

impl QuickJsRuntime {
    pub fn new() -> Self {
        let shared = Rc::new(RefCell::new(Shared {
            ops: OpBuffer::default(),
            commits: Vec::new(),
            errors: Vec::new(),
            log: tracing_log_sink(),
        }));
        let (runtime, context) = new_context(&shared, &[]);
        Self {
            context,
            runtime,
            shared,
            natives: Vec::new(),
        }
    }

    /// Installs a native API (`register_quickjs` of an `#[iron_api(.., quickjs)]` module) in the
    /// current context, and again in every context `reset()` makes. Engine handles in these APIs
    /// are opaque objects (see `iron_api::qjs`). Register the value classes' modules
    /// (`iron_math`) before the ones that use them.
    pub fn add_native(&mut self, init: NativeInit) -> Result<(), ScriptError> {
        self.natives.push(init);
        self.context
            .with(|ctx| init(&ctx).map_err(|e| to_script_error(&ctx, e)))
    }

    /// [`ScriptRuntime::tick`] for one event domain: fires the timers that domain set (each with
    /// that domain entered, through the host's `__iron_enter_domain` / `__iron_leave_domain`).
    pub fn tick_domain(&mut self, now_ms: f64, domain: u32) -> Vec<ScriptError> {
        for _ in 0..MAX_TIMERS_PER_TICK {
            match self.call_global(
                "__iron_fire_next_timer",
                &[now_ms.into(), f64::from(domain).into()],
            ) {
                Ok(Value::Bool(true)) => continue,
                Ok(_) => break,
                Err(e) => {
                    self.shared.borrow_mut().errors.push(e);
                    break;
                }
            }
        }
        std::mem::take(&mut self.shared.borrow_mut().errors)
    }

    /// Runs pending promise jobs and microtasks until the queue is empty.
    /// Exceptions thrown by jobs are kept and returned by the next `tick`.
    fn run_jobs(&mut self) {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => break,
                Err(job) => job.0.with(|ctx| {
                    let error = thrown_to_error(&ctx.catch());
                    self.shared.borrow_mut().errors.push(error);
                }),
            }
        }
        // The script's turn is over: what it queued to send in one op (its objects'
        // transforms, `__iron_flush` from @iron/engine) goes out before Rust reads anything.
        self.context.with(|ctx| {
            if let Ok(flush) = ctx.globals().get::<_, Function>("__iron_flush")
                && let Err(e) = flush.call::<_, ()>(())
            {
                let error = to_script_error(&ctx, e);
                self.shared.borrow_mut().errors.push(error);
            }
        });
    }
}

impl Default for QuickJsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRuntime for QuickJsRuntime {
    fn load_bundle(&mut self, name: &str, code: &str) -> Result<(), ScriptError> {
        let result = self.context.with(|ctx| {
            let mut opts = EvalOptions::default();
            opts.filename = Some(name.to_owned());
            opts.strict = false;
            ctx.eval_with_options::<rquickjs::Value, _>(code, opts)
                .map(|_| ())
                .map_err(|e| to_script_error(&ctx, e))
        });
        self.run_jobs();
        result
    }

    fn eval(&mut self, code: &str) -> Result<Value, ScriptError> {
        let result = self.context.with(|ctx| {
            let v = ctx
                .eval::<rquickjs::Value, _>(code)
                .map_err(|e| to_script_error(&ctx, e))?;
            Ok(from_js(&v))
        });
        self.run_jobs();
        result
    }

    fn call_global(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError> {
        let result = self.context.with(|ctx| {
            let f: Function = ctx
                .globals()
                .get(name)
                .map_err(|_| ScriptError::new(format!("`{name}` is not a function")))?;
            let args = args
                .iter()
                .map(|a| to_js(&ctx, a))
                .collect::<rquickjs::Result<Vec<_>>>()
                .map_err(|e| to_script_error(&ctx, e))?;
            let v: rquickjs::Value = f
                .call((Rest(args),))
                .map_err(|e| to_script_error(&ctx, e))?;
            Ok(from_js(&v))
        });
        self.run_jobs();
        result
    }

    fn tick(&mut self, now_ms: f64) -> Vec<ScriptError> {
        self.tick_domain(now_ms, 1)
    }

    fn drain_ops(&mut self) -> OpBuffer {
        self.shared.borrow_mut().ops.take()
    }

    fn drain_commits(&mut self) -> Vec<Commit> {
        std::mem::take(&mut self.shared.borrow_mut().commits)
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    fn drain_targeted_commits(&mut self) -> Vec<(u32, Commit)> {
        std::mem::take(&mut self.shared.borrow_mut().commits)
    }

    fn reset(&mut self) {
        {
            let mut shared = self.shared.borrow_mut();
            shared.ops.take();
            shared.commits.clear();
            shared.errors.clear();
        }
        let (runtime, context) = new_context(&self.shared, &self.natives);
        // Assign the context first so the old context drops before the old runtime.
        self.context = context;
        self.runtime = runtime;
    }

    fn set_log_sink(&mut self, sink: LogSink) {
        self.shared.borrow_mut().log = sink;
    }
}

fn new_context(shared: &Rc<RefCell<Shared>>, natives: &[NativeInit]) -> (Runtime, Context) {
    let runtime = Runtime::new().expect("QuickJS runtime");
    let context = Context::full(&runtime).expect("QuickJS context");
    context.with(|ctx| {
        install_globals(&ctx, shared).expect("install Iron globals");
        let mut opts = EvalOptions::default();
        opts.filename = Some("iron:prelude.js".into());
        ctx.eval_with_options::<(), _>(PRELUDE, opts)
            .map_err(|e| to_script_error(&ctx, e))
            .expect("Iron prelude");
        for init in natives {
            init(&ctx)
                .map_err(|e| to_script_error(&ctx, e))
                .expect("native API");
        }
    });
    (runtime, context)
}

fn install_globals<'js>(ctx: &Ctx<'js>, shared: &Rc<RefCell<Shared>>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;
    for (name, level) in [
        ("log", LogLevel::Info),
        ("info", LogLevel::Info),
        ("debug", LogLevel::Debug),
        ("warn", LogLevel::Warn),
        ("error", LogLevel::Error),
    ] {
        let shared = shared.clone();
        let f = Function::new(ctx.clone(), move |args: Rest<rquickjs::Value>| {
            let msg = args.0.iter().map(display).collect::<Vec<_>>().join(" ");
            // Clone the sink so it can't observe the RefCell borrowed.
            let sink = shared.borrow().log.clone();
            sink(level, &msg);
        })?;
        console.set(name, f)?;
    }
    globals.set("console", console)?;

    let s = shared.clone();
    let op = Function::new(
        ctx.clone(),
        move |name: String, args: Rest<rquickjs::Value>| {
            let t = OP_TIME.with(|o| o.get().0.then(std::time::Instant::now));
            s.borrow_mut().ops.push(Op {
                name,
                args: args.0.iter().map(from_js).collect(),
            });
            if let Some(t) = t {
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                OP_TIME.with(|o| o.set((true, o.get().1 + ms)));
            }
        },
    )?;
    globals.set("__iron_op", op)?;

    let s = shared.clone();
    let commit = Function::new(
        ctx.clone(),
        move |ops: rquickjs::Value, target: rquickjs::function::Opt<f64>| {
            let ops = match from_js(&ops) {
                Value::Array(ops) => ops,
                other => vec![other],
            };
            let target = target
                .0
                .filter(|t| t.is_finite() && *t >= 0.0)
                .unwrap_or(0.0) as u32;
            s.borrow_mut().commits.push((target, Commit { ops }));
        },
    )?;
    globals.set("__iron_commit", commit)?;

    let s = shared.clone();
    let report = Function::new(ctx.clone(), move |thrown: rquickjs::Value| {
        s.borrow_mut().errors.push(thrown_to_error(&thrown));
    })?;
    globals.set("__iron_report_error", report)?;
    Ok(())
}

/// Turns a JS value into the text `console.log` prints.
fn display(v: &rquickjs::Value) -> String {
    if let Some(s) = v.as_string() {
        return s.to_string().unwrap_or_default();
    }
    if let Some(e) = v
        .as_object()
        .and_then(|o| Exception::from_object(o.clone()))
    {
        return e.message().unwrap_or_else(|| "Error".into());
    }
    match from_js(v) {
        Value::Null if v.is_undefined() => "undefined".into(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => format_number(n),
        Value::String(s) => s,
        Value::Array(_) => {
            let ctx = v.ctx();
            ctx.json_stringify(v.clone())
                .ok()
                .flatten()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_else(|| "[array]".into())
        }
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

fn from_js(v: &rquickjs::Value) -> Value {
    match v.type_of() {
        Type::Bool => Value::Bool(v.as_bool().unwrap_or_default()),
        Type::Int | Type::Float => Value::Number(v.as_number().unwrap_or_default()),
        Type::String => Value::String(
            v.as_string()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_default(),
        ),
        Type::Array => Value::Array(
            v.as_array()
                .map(|a| {
                    a.iter::<rquickjs::Value>()
                        .filter_map(Result::ok)
                        .map(|x| from_js(&x))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        // Typed arrays (big meshes): read straight from their bytes, not element by element.
        Type::Object => v
            .as_object()
            .and_then(typed_numbers)
            .map_or(Value::Null, Value::Array),
        _ => Value::Null,
    }
}

/// A typed array's numbers (`None` for other objects).
fn typed_numbers(o: &rquickjs::Object) -> Option<Vec<Value>> {
    // Read before any more script runs: the pointer is only good until then.
    macro_rules! read {
        ($t:ty) => {
            if let Some(a) = o.as_typed_array::<$t>() {
                let raw = a.as_raw()?;
                let bytes = unsafe { raw.as_ref() };
                return Some(
                    bytes
                        .chunks_exact(std::mem::size_of::<$t>())
                        .map(|b| Value::Number(<$t>::from_le_bytes(b.try_into().unwrap()) as f64))
                        .collect(),
                );
            }
        };
    }
    read!(f32);
    read!(f64);
    read!(u32);
    read!(i32);
    read!(u16);
    read!(i16);
    read!(u8);
    read!(i8);
    None
}

fn to_js<'js>(ctx: &Ctx<'js>, v: &Value) -> rquickjs::Result<rquickjs::Value<'js>> {
    Ok(match v {
        Value::Null => rquickjs::Value::new_null(ctx.clone()),
        Value::Bool(b) => rquickjs::Value::new_bool(ctx.clone(), *b),
        Value::Number(n) => rquickjs::Value::new_number(ctx.clone(), *n),
        Value::String(s) => rquickjs::String::from_str(ctx.clone(), s)?.into_value(),
        Value::Array(items) => {
            let arr = Array::new(ctx.clone())?;
            for (i, item) in items.iter().enumerate() {
                arr.set(i, to_js(ctx, item)?)?;
            }
            arr.into_value()
        }
    })
}

fn to_script_error(ctx: &Ctx<'_>, err: rquickjs::Error) -> ScriptError {
    if !err.is_exception() {
        return ScriptError::new(err.to_string());
    }
    thrown_to_error(&ctx.catch())
}

/// Converts a thrown JS value (an `Error` or anything else) into a `ScriptError`.
fn thrown_to_error(thrown: &rquickjs::Value) -> ScriptError {
    let Some(exception) = thrown
        .as_object()
        .and_then(|o| Exception::from_object(o.clone()))
    else {
        // `throw "text"` or another non-Error value.
        return ScriptError::new(display(thrown));
    };
    let name: Option<String> = exception.get("name").ok();
    let message = exception.message().unwrap_or_default();
    let mut error = ScriptError::new(match name {
        Some(name) if !name.is_empty() => format!("{name}: {message}"),
        _ => message,
    });
    error.stack = exception.stack().filter(|s| !s.is_empty());
    error = error.with_location_from_stack();
    // Syntax errors may carry their location as properties instead of in the stack.
    if error.file.is_none() {
        error.file = exception.get::<_, String>("fileName").ok();
        error.line = exception.get::<_, u32>("lineNumber").ok();
    }
    error
}
