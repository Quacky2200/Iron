// SPDX-License-Identifier: Apache-2.0

//! Script runtimes behind one trait, so each platform can use the best JS engine
//! (DESIGN.md §4.1), plus the component host that drives script components.

mod components;
pub mod conformance;
mod error;
mod quickjs;

use std::rc::Rc;

pub use components::{ComponentHost, load_game, reload, spawn_each};
pub use error::ScriptError;
pub use iron_core::{Commit, OpBuffer, Value};
pub use quickjs::{QuickJsRuntime, take_op_time, time_ops};

/// Script-side support code every backend evaluates in a fresh context (timers).
pub const PRELUDE: &str = include_str!("prelude.js");

/// Caps timer callbacks per tick so `setTimeout(f, 0)` loops can't freeze a frame.
pub const MAX_TIMERS_PER_TICK: usize = 1000;

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
/// Every backend provides the same globals to scripts:
/// - `console.log/info/warn/error/debug(...)` → the log sink
/// - `__iron_op(name, ...args)` → queues an op, collected with [`ScriptRuntime::drain_ops`]
/// - `__iron_commit(ops)` → one UI commit, collected with [`ScriptRuntime::drain_commits`]
/// - `setTimeout` / `clearTimeout` (from [`PRELUDE`]) driven by [`ScriptRuntime::tick`]
/// - `queueMicrotask` and promises; pending jobs run before every call returns to Rust
pub trait ScriptRuntime {
    /// Runs a script bundle (an IIFE built by Bun). `name` is used in error locations.
    fn load_bundle(&mut self, name: &str, code: &str) -> Result<(), ScriptError>;

    /// Evaluates an expression and returns its value. Mostly for tests and the console.
    fn eval(&mut self, code: &str) -> Result<Value, ScriptError>;

    /// Calls a global function by name.
    fn call_global(&mut self, name: &str, args: &[Value]) -> Result<Value, ScriptError>;

    /// Advances script time to `now_ms` and fires every timer that is due, in due-time
    /// order, running microtasks after each one. Timers set with a zero delay during the
    /// tick fire in the same tick (up to [`MAX_TIMERS_PER_TICK`]).
    ///
    /// Returns uncaught errors from timers and async jobs since the last tick.
    fn tick(&mut self, now_ms: f64) -> Vec<ScriptError>;

    /// Takes every op scripts queued since the last drain, in order.
    fn drain_ops(&mut self) -> OpBuffer;

    /// Takes every UI commit since the last drain, in order.
    fn drain_commits(&mut self) -> Vec<Commit>;

    /// Takes every UI commit since the last drain, in order, with the UI tree each is for
    /// (`__iron_commit(ops, target)`: 0 is the screen, others are world panels).
    fn drain_targeted_commits(&mut self) -> Vec<(u32, Commit)> {
        self.drain_commits().into_iter().map(|c| (0, c)).collect()
    }

    /// Throws away all script state and starts a fresh context (used by hot reload).
    /// Pending ops, commits, timers and errors are dropped; the log sink is kept.
    fn reset(&mut self);

    fn set_log_sink(&mut self, sink: LogSink);
}
