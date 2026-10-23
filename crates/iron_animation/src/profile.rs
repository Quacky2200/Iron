//! Where the animation update spends its time, for `--bench` (the host merges these into its
//! `syncParts`). Off unless the host turns it on: a timer is then one clock read.

use std::cell::{Cell, RefCell};
use std::time::Instant;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    /// Milliseconds per part since the last [`take`], in the order parts were first seen.
    static PARTS: RefCell<Vec<(&'static str, f64)>> = const { RefCell::new(Vec::new()) };
}

/// Turns the timers on (a bench run).
pub fn enable() {
    ON.with(|o| o.set(true));
}

/// Whether the timers are on.
pub fn enabled() -> bool {
    ON.with(Cell::get)
}

/// The parts' times since the last call, and starts over.
pub fn take() -> Vec<(&'static str, f64)> {
    PARTS.with(|p| {
        let mut p = p.borrow_mut();
        let out = p.clone();
        for part in p.iter_mut() {
            part.1 = 0.0;
        }
        out
    })
}

/// A clock read when the timers are on.
pub fn start() -> Option<Instant> {
    enabled().then(Instant::now)
}

/// Adds the time since `since` to `part`.
pub fn stop(part: &'static str, since: Option<Instant>) {
    if let Some(t) = since {
        add(part, t.elapsed().as_secs_f64() * 1000.0);
    }
}

/// Adds `ms` to `part`.
pub fn add(part: &'static str, ms: f64) {
    PARTS.with(|p| {
        let mut p = p.borrow_mut();
        match p.iter_mut().find(|(n, _)| *n == part) {
            Some(e) => e.1 += ms,
            None => p.push((part, ms)),
        }
    });
}
