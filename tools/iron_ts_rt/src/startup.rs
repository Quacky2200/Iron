//! Startup of a compiled program: which component classes `register(..)` named (in call order) and
//! whether every module of the graph has finished evaluating (a module with a top-level `await`
//! finishes later). The host starts the registered classes only when the startup is done.
//!
//! Both are per thread, like the heap. [`startup_reset`] clears them before a program's top level
//! runs (`iron_ts::jit::Game::init` does it, so a hot reload starts clean).

use super::*;
use std::cell::RefCell;

/// How the program's startup stands.
#[derive(Debug, Clone, PartialEq)]
pub enum StartupState {
    /// Every module has finished evaluating (also the state of a program with no top-level `await`).
    Done,
    /// Modules that started and have not finished, in the order they started. The first one is
    /// the one the others wait for.
    Pending(Vec<String>),
    /// A module's evaluation threw (a rejected top-level `await`): its path and the error text.
    Failed { module: String, message: String },
}

#[derive(Default)]
struct Startup {
    /// `(class, spawn)` in the order of the `register(..)` calls.
    registered: Vec<(String, bool)>,
    pending: Vec<String>,
    failed: Option<(String, String)>,
}

thread_local! {
    static STARTUP: RefCell<Startup> = RefCell::new(Startup::default());
}

/// `register(Class)`: `names` is the class name, or several separated by commas
/// (`register({ A, B })`); `spawn` is non-zero unless the call said `{ spawn: false }`. Calling it
/// again for a class keeps its first place and takes the new `spawn`.
///
/// # Safety
/// `names` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_register_class(names: *const Str, spawn: f64) {
    // SAFETY: the caller passes a live string.
    let text = unsafe { &*names }.to_string_lossy();
    STARTUP.with(|s| {
        let mut s = s.borrow_mut();
        for name in text.split(',').filter(|n| !n.is_empty()) {
            match s.registered.iter_mut().find(|(n, _)| n == name) {
                Some(entry) => entry.1 = spawn != 0.0,
                None => s.registered.push((name.to_string(), spawn != 0.0)),
            }
        }
    });
}

/// The classes `register(..)` named so far, in call order, with their `spawn` option.
pub fn registered_classes() -> Vec<(String, bool)> {
    STARTUP.with(|s| s.borrow().registered.clone())
}

/// Was `class` named by a `register(..)` call that has run?
pub fn is_registered(class: &str) -> bool {
    STARTUP.with(|s| s.borrow().registered.iter().any(|(n, _)| n == class))
}

/// Forgets the registrations and the startup state (before a program's top level runs).
pub fn startup_reset() {
    STARTUP.with(|s| *s.borrow_mut() = Startup::default());
}

/// Startup events from the module wrappers the compiler writes: `kind` 0 is a module starting
/// evaluation (`name` is its path), 1 the module finishing, 2 the module failing (`message` is the
/// error text; the first failure is the one kept).
///
/// # Safety
/// `name` and `message` must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_startup(kind: f64, name: *const Str, message: *const Str) {
    // SAFETY: the caller passes live strings.
    let (name, message) = unsafe { ((&*name).to_string_lossy(), (&*message).to_string_lossy()) };
    STARTUP.with(|s| {
        let mut s = s.borrow_mut();
        match kind as i32 {
            0 => s.pending.push(name),
            1 => s.pending.retain(|n| *n != name),
            _ => {
                s.pending.retain(|n| *n != name);
                if s.failed.is_none() {
                    s.failed = Some((name, message));
                }
            }
        }
    });
}

/// How the startup stands now. Microtasks must have run for a module that finished to show.
pub fn startup_state() -> StartupState {
    STARTUP.with(|s| {
        let s = s.borrow();
        if let Some((module, message)) = &s.failed {
            StartupState::Failed {
                module: module.clone(),
                message: message.clone(),
            }
        } else if s.pending.is_empty() {
            StartupState::Done
        } else {
            StartupState::Pending(s.pending.clone())
        }
    })
}
