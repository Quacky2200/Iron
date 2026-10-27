// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Where compiling a program spends its time, by phase (`platypus --bench` reports it as start-up
//! parts). Process-wide, because lowering and code generation run on worker threads; a phase is
//! one clock read, and nothing is kept until [`enable`] is called.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ON: AtomicBool = AtomicBool::new(false);
static PHASES: Mutex<Vec<(&'static str, f64)>> = Mutex::new(Vec::new());

/// Turns the timers on (a bench run).
pub fn enable() {
    ON.store(true, Ordering::Relaxed);
}

/// The phases' milliseconds since the last call, in the order they were first seen.
pub fn take() -> Vec<(&'static str, f64)> {
    PHASES
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default()
}

/// Adds `ms` to a phase (for times measured another way).
pub fn add(phase: &'static str, ms: f64) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut p) = PHASES.lock() {
        match p.iter_mut().find(|(n, _)| *n == phase) {
            Some(e) => e.1 += ms,
            None => p.push((phase, ms)),
        }
    }
}

/// Times the stretches between marks.
pub struct Lap(Option<Instant>);

impl Lap {
    pub fn start() -> Self {
        Lap(ON.load(Ordering::Relaxed).then(Instant::now))
    }

    /// Adds the time since the last mark to `phase`.
    pub fn mark(&mut self, phase: &'static str) {
        let Some(t) = self.0 else {
            return;
        };
        let now = Instant::now();
        add(phase, (now - t).as_secs_f64() * 1000.0);
        self.0 = Some(now);
    }
}
