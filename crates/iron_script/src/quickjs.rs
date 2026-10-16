use std::cell::RefCell;
use std::rc::Rc;

use iron_core::Op;
use rquickjs::context::EvalOptions;
use rquickjs::function::Rest;
use rquickjs::{Array, Context, Ctx, Exception, Function, Object, Runtime, Type};

use crate::{LogLevel, LogSink, OpBuffer, ScriptError, ScriptRuntime, Value, tracing_log_sink};

/// The QuickJS backend (via `rquickjs`). Interpreter only, so it also works on iOS.
pub struct QuickJsRuntime {
    // Field order matters: the context must drop before its runtime.
    context: Context,
    _runtime: Runtime,
    ops: Rc<RefCell<OpBuffer>>,
    log: Rc<RefCell<LogSink>>,
}

impl QuickJsRuntime {
    pub fn new() -> Self {
        let ops = Rc::new(RefCell::new(OpBuffer::default()));
        let log = Rc::new(RefCell::new(tracing_log_sink()));
        let (runtime, context) = new_context(&ops, &log);
        Self {
            context,
            _runtime: runtime,
            ops,
            log,
        }
    }
}

impl Default for QuickJsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRuntime for QuickJsRuntime {
    fn load_bundle(&mut self, name: &str, code: &str) -> Result<(), ScriptError> {
        self.context.with(|ctx| {
            let mut opts = EvalOptions::default();
            opts.filename = Some(name.to_owned());
            opts.strict = false;
            ctx.eval_with_options::<rquickjs::Value, _>(code, opts)
                .map(|_| ())
                .map_err(|e| to_script_error(&ctx, e))
        })
    }

    fn eval(&mut self, code: &str) -> Result<Value, ScriptError> {
        self.context.with(|ctx| {
            let v = ctx
                .eval::<rquickjs::Value, _>(code)
                .map_err(|e| to_script_error(&ctx, e))?;
            Ok(from_js(&v))
        })
    }

    fn call_global(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError> {
        self.context.with(|ctx| {
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
        })
    }

    fn drain_ops(&mut self) -> OpBuffer {
        self.ops.borrow_mut().take()
    }

    fn reset(&mut self) {
        self.ops.borrow_mut().take();
        let (runtime, context) = new_context(&self.ops, &self.log);
        // Assign the context first so the old context drops before the old runtime.
        self.context = context;
        self._runtime = runtime;
    }

    fn set_log_sink(&mut self, sink: LogSink) {
        *self.log.borrow_mut() = sink;
    }
}

fn new_context(ops: &Rc<RefCell<OpBuffer>>, log: &Rc<RefCell<LogSink>>) -> (Runtime, Context) {
    let runtime = Runtime::new().expect("QuickJS runtime");
    let context = Context::full(&runtime).expect("QuickJS context");
    context.with(|ctx| {
        install_globals(&ctx, ops.clone(), log.clone()).expect("install Iron globals");
    });
    (runtime, context)
}

fn install_globals<'js>(
    ctx: &Ctx<'js>,
    ops: Rc<RefCell<OpBuffer>>,
    log: Rc<RefCell<LogSink>>,
) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;
    for (name, level) in [
        ("log", LogLevel::Info),
        ("info", LogLevel::Info),
        ("debug", LogLevel::Debug),
        ("warn", LogLevel::Warn),
        ("error", LogLevel::Error),
    ] {
        let log = log.clone();
        let f = Function::new(ctx.clone(), move |args: Rest<rquickjs::Value>| {
            let msg = args.0.iter().map(display).collect::<Vec<_>>().join(" ");
            let sink = log.borrow().clone();
            sink(level, &msg);
        })?;
        console.set(name, f)?;
    }
    globals.set("console", console)?;

    let op = Function::new(
        ctx.clone(),
        move |name: String, args: Rest<rquickjs::Value>| {
            ops.borrow_mut().push(Op {
                name,
                args: args.0.iter().map(from_js).collect(),
            });
        },
    )?;
    globals.set("__iron_op", op)?;
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
        _ => Value::Null,
    }
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
    let thrown = ctx.catch();
    let Some(exception) = thrown
        .as_object()
        .and_then(|o| Exception::from_object(o.clone()))
    else {
        // `throw "text"` or another non-Error value.
        return ScriptError::new(display(&thrown));
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
