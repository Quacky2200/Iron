//! Where a compiled game's render sync (`sync_native_render`) spends its time, for `--bench`
//! (`syncParts` in the report). Off unless a bench run turns it on: a lap is then one clock read.

use std::cell::{Cell, RefCell};
use std::time::Instant;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    /// Milliseconds per part since the last [`take`], in the order parts were first seen.
    static PARTS: RefCell<Vec<(&'static str, f64)>> = const { RefCell::new(Vec::new()) };
}

/// Turns the timers on (a bench run).
pub(crate) fn enable() {
    ON.with(|o| o.set(true));
}

/// The parts' times since the last call, and starts over.
pub(crate) fn take() -> Vec<(&'static str, f64)> {
    PARTS.with(|p| {
        let mut p = p.borrow_mut();
        let out = p.clone();
        for part in p.iter_mut() {
            part.1 = 0.0;
        }
        out
    })
}

/// Times the stretches between marks.
pub(crate) struct Lap(Option<Instant>);

impl Lap {
    pub(crate) fn start() -> Self {
        Lap(ON.with(Cell::get).then(Instant::now))
    }

    /// Adds the time since the last mark to `part`.
    pub(crate) fn mark(&mut self, part: &'static str) {
        let Some(t) = self.0 else {
            return;
        };
        let now = Instant::now();
        let ms = (now - t).as_secs_f64() * 1000.0;
        self.0 = Some(now);
        PARTS.with(|p| {
            let mut p = p.borrow_mut();
            match p.iter_mut().find(|(n, _)| *n == part) {
                Some(e) => e.1 += ms,
                None => p.push((part, ms)),
            }
        });
    }
}
