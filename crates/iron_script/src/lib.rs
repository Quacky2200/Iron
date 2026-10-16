//! Script runtimes behind one trait, so each platform can use the best JS engine
//! (DESIGN.md §4.1), plus the component host that drives script components.

mod components;
pub mod conformance;
mod error;
mod quickjs;

use std::rc::Rc;

pub use components::ComponentHost;
pub use error::ScriptError;
pub use iron_core::{OpBuffer, Value};
pub use quickjs::QuickJsRuntime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Receives `console.*` output from scripts.
pub type LogSink = Rc<dyn Fn(LogLevel, &str)>;

/// The default sink: forwards script logs to `tracing` with target `script`.
pub fn tracing_log_sink() -> LogSink {
    Rc::new(|level, msg| match level {
        LogLevel::Debug => tracing::debug!(target: "script", "{msg}"),
        LogLevel::Info => tracing::info!(target: "script", "{msg}"),
        LogLevel::Warn => tracing::warn!(target: "script", "{msg}"),
        LogLevel::Error => tracing::error!(target: "script", "{msg}"),
    })
}

/// A JavaScript engine embedded in Iron.
///
/// Scripts talk to the engine through two native globals every backend installs:
/// - `console.log/info/warn/error/debug(...)` → the log sink
/// - `__iron_op(name, ...args)` → queues an op, collected with [`ScriptRuntime::drain_ops`]
pub trait ScriptRuntime {
    /// Runs a script bundle (an IIFE built by Bun). `name` is used in error locations.
    fn load_bundle(&mut self, name: &str, code: &str) -> Result<(), ScriptError>;

    /// Evaluates an expression and returns its value. Mostly for tests and the console.
    fn eval(&mut self, code: &str) -> Result<Value, ScriptError>;

    /// Calls a global function by name.
    fn call_global(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError>;

    /// Takes every op scripts queued since the last drain, in order.
    fn drain_ops(&mut self) -> OpBuffer;

    /// Throws away all script state and starts a fresh context (used by hot reload).
    /// The log sink is kept.
    fn reset(&mut self);

    fn set_log_sink(&mut self, sink: LogSink);
}
