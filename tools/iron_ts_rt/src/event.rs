//! The microtask queue and the timers behind `Promise`, `queueMicrotask`, `setTimeout` and
//! `setInterval`. No thread and no OS event loop: the host decides when script runs.
//!
//! The contract with the host: after every call into script it calls `ts_rt_microtasks_run()`, and
//! once per frame it calls `ts_rt_timers_advance(now_ms)` with the game clock (the timers' time is
//! whatever the host passes, so a paused game pauses its timers). `ts_rt_run_loop()` runs both until
//! nothing is left, for tests and tools. See `docs/research/iron-ts-generators-and-proxy.md`.

use super::*;
use std::collections::VecDeque;

struct Timer {
    due: f64,
    seq: u64,
    id: u64,
    /// Repeat period, or negative for a one-shot timer.
    interval: f64,
    closure: *mut u8,
}

pub(crate) struct Events {
    micro: VecDeque<*mut u8>,
    timers: Vec<Timer>,
    now: f64,
    next_id: u64,
    next_seq: u64,
}

impl Events {
    pub(crate) const fn new() -> Self {
        Events {
            micro: VecDeque::new(),
            timers: Vec::new(),
            now: 0.0,
            next_id: 1,
            next_seq: 0,
        }
    }

    /// The closures the collector must keep alive.
    pub(crate) fn roots(&self) -> impl Iterator<Item = &*mut u8> {
        self.micro
            .iter()
            .chain(self.timers.iter().map(|t| &t.closure))
    }
}

/// Runs a `() => void` closure; an exception it leaves behind is dropped (the browser reports it
/// and goes on with the next task).
///
/// # Safety
/// `closure` must be a live `() => void` closure.
unsafe fn call(closure: *mut u8) {
    let c = closure as *mut Closure;
    // SAFETY: by contract; compiled closures are `extern "C" fn(env)`.
    unsafe {
        let f: unsafe extern "C" fn(*mut u8) = std::mem::transmute((*c).func);
        f((*c).env);
    }
    with_heap(|h| h.exc = std::ptr::null_mut());
}

/// `queueMicrotask(callback)`.
///
/// # Safety
/// `closure` must be a live `() => void` closure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_microtask_push(closure: *mut Closure) {
    with_heap(|h| h.events.micro.push_back(closure as *mut u8));
}

/// Runs the microtasks until the queue is empty (jobs may queue more).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_microtasks_run() {
    while let Some(job) = with_heap(|h| h.events.micro.pop_front()) {
        // The job is no longer in the queue, so keep it reachable while it runs.
        with_heap(|h| h.temps.push(job));
        // SAFETY: queued closures are live `() => void` closures.
        unsafe { call(job) };
        with_heap(|h| {
            h.temps.pop();
        });
    }
}

/// `setTimeout` / `setInterval`: returns the timer's id.
///
/// # Safety
/// `closure` must be a live `() => void` closure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_timer_add(closure: *mut Closure, delay: f64, repeat: bool) -> f64 {
    // Like the browser: at least 1 ms (so a 0 ms timer runs after already queued 0 ms timers).
    let delay = if delay.is_nan() || delay < 1.0 {
        1.0
    } else {
        delay.floor()
    };
    with_heap(|h| {
        let ev = &mut h.events;
        let id = ev.next_id;
        ev.next_id += 1;
        let seq = ev.next_seq;
        ev.next_seq += 1;
        ev.timers.push(Timer {
            due: ev.now + delay,
            seq,
            id,
            interval: if repeat { delay } else { -1.0 },
            closure: closure as *mut u8,
        });
        id as f64
    })
}

/// `clearTimeout` / `clearInterval`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_timer_clear(id: f64) {
    with_heap(|h| h.events.timers.retain(|t| t.id as f64 != id));
}

/// The next timer due at or before `limit`, taken off the list (a repeating timer is put back
/// for its next period).
fn next_due(limit: f64) -> Option<(*mut u8, f64)> {
    with_heap(|h| {
        let ev = &mut h.events;
        let mut best: Option<usize> = None;
        for (i, t) in ev.timers.iter().enumerate() {
            if t.due <= limit
                && best.is_none_or(|b| (t.due, t.seq) < (ev.timers[b].due, ev.timers[b].seq))
            {
                best = Some(i);
            }
        }
        let i = best?;
        let (closure, due) = (ev.timers[i].closure, ev.timers[i].due);
        ev.now = ev.now.max(due);
        if ev.timers[i].interval >= 0.0 {
            let seq = ev.next_seq;
            ev.next_seq += 1;
            let t = &mut ev.timers[i];
            t.due = due + t.interval;
            t.seq = seq;
        } else {
            ev.timers.remove(i);
        }
        Some((closure, due))
    })
}

/// Runs every timer due by `now_ms` (the host's clock), each followed by its microtasks.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_timers_advance(now_ms: f64) {
    while let Some((closure, _)) = next_due(now_ms) {
        with_heap(|h| h.temps.push(closure));
        // SAFETY: timer closures are live `() => void` closures.
        unsafe { call(closure) };
        with_heap(|h| {
            h.temps.pop();
        });
        ts_rt_microtasks_run();
    }
    with_heap(|h| h.events.now = h.events.now.max(now_ms));
}

/// Microtasks and timers until nothing is left (a test or a tool, not a game loop). A repeating
/// timer nobody clears stops it after a million rounds.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_run_loop() {
    ts_rt_microtasks_run();
    for _ in 0..1_000_000 {
        let next = with_heap(|h| {
            h.events
                .timers
                .iter()
                .map(|t| t.due)
                .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
        });
        let Some(due) = next else { break };
        ts_rt_timers_advance(due);
    }
}

/// How many timers and microtasks are waiting.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_events_pending() -> f64 {
    with_heap(|h| (h.events.micro.len() + h.events.timers.len()) as f64)
}
