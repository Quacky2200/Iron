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
    /// The event domain current when it was set (see [`ts_rt_set_domain_hooks`]).
    domain: u32,
    due: f64,
    seq: u64,
    id: u64,
    /// Repeat period, or negative for a one-shot timer.
    interval: f64,
    closure: *mut u8,
}

pub(crate) struct Events {
    /// Closure and the domain current when it was queued.
    micro: VecDeque<(*mut u8, u32)>,
    /// Run when the microtask queue has drained (the unhandled-rejection check).
    after: VecDeque<(*mut u8, u32)>,
    timers: Vec<Timer>,
    now: f64,
    next_id: u64,
    next_seq: u64,
}

impl Events {
    pub(crate) const fn new() -> Self {
        Events {
            micro: VecDeque::new(),
            after: VecDeque::new(),
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
            .map(|(c, _)| c)
            .chain(self.after.iter().map(|(c, _)| c))
            .chain(self.timers.iter().map(|t| &t.closure))
    }
}

/// The host's view of event domains (one per window family): which one is current, and entering and
/// leaving one around a callback. Timers and microtasks remember the domain current when they were
/// set and run with it entered again. Without hooks every callback is in domain 1.
#[derive(Clone, Copy)]
struct DomainHooks {
    current: extern "C" fn() -> u32,
    enter: extern "C" fn(u32),
    leave: extern "C" fn(),
}

extern "C" fn main_domain() -> u32 {
    1
}
extern "C" fn no_enter(_: u32) {}
extern "C" fn no_leave() {}

thread_local! {
    static HOOKS: std::cell::Cell<DomainHooks> = const {
        std::cell::Cell::new(DomainHooks { current: main_domain, enter: no_enter, leave: no_leave })
    };
}

/// Installs this thread's domain hooks; `None` for a hook restores its default (domain 1, no-ops).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_set_domain_hooks(
    current: Option<extern "C" fn() -> u32>,
    enter: Option<extern "C" fn(u32)>,
    leave: Option<extern "C" fn()>,
) {
    HOOKS.with(|h| {
        h.set(DomainHooks {
            current: current.unwrap_or(main_domain),
            enter: enter.unwrap_or(no_enter),
            leave: leave.unwrap_or(no_leave),
        })
    });
}

fn hooks() -> DomainHooks {
    HOOKS.with(|h| h.get())
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
    let domain = (hooks().current)();
    with_heap(|h| h.events.micro.push_back((closure as *mut u8, domain)));
}

/// Runs `closure` once the microtask queue has drained (before `ts_rt_microtasks_run` returns).
///
/// # Safety
/// `closure` must be a live `() => void` closure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_after_microtasks(closure: *mut Closure) {
    let domain = (hooks().current)();
    with_heap(|h| h.events.after.push_back((closure as *mut u8, domain)));
}

/// Runs the microtasks until the queue is empty (jobs may queue more), then the closures
/// registered with [`ts_rt_after_microtasks`], which may queue more jobs.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_microtasks_run() {
    loop {
        let next = with_heap(|h| {
            h.events
                .micro
                .pop_front()
                .or_else(|| h.events.after.pop_front())
        });
        let Some((job, domain)) = next else { break };
        // The job is no longer in the queue, so keep it reachable while it runs.
        with_heap(|h| h.temps.push(job));
        let hk = hooks();
        (hk.enter)(domain);
        // SAFETY: queued closures are live `() => void` closures.
        unsafe { call(job) };
        (hk.leave)();
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
    let domain = (hooks().current)();
    with_heap(|h| {
        let ev = &mut h.events;
        let id = ev.next_id;
        ev.next_id += 1;
        let seq = ev.next_seq;
        ev.next_seq += 1;
        ev.timers.push(Timer {
            domain,
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

/// The next timer of `domain` (any when `None`) due at or before `limit`, taken off the list (a
/// repeating timer is put back for its next period; with `coalesce` it skips the periods already
/// missed, so it runs once per advance).
fn next_due(limit: f64, domain: Option<u32>, coalesce: bool) -> Option<(*mut u8, u32)> {
    with_heap(|h| {
        let ev = &mut h.events;
        let mut best: Option<usize> = None;
        for (i, t) in ev.timers.iter().enumerate() {
            if t.due <= limit
                && domain.is_none_or(|d| t.domain == d)
                && best.is_none_or(|b| (t.due, t.seq) < (ev.timers[b].due, ev.timers[b].seq))
            {
                best = Some(i);
            }
        }
        let i = best?;
        let (closure, due, owner) = (ev.timers[i].closure, ev.timers[i].due, ev.timers[i].domain);
        ev.now = ev.now.max(due);
        if ev.timers[i].interval >= 0.0 {
            let seq = ev.next_seq;
            ev.next_seq += 1;
            let t = &mut ev.timers[i];
            t.due = due + t.interval;
            if coalesce && t.due <= limit {
                t.due = limit + t.interval;
            }
            t.seq = seq;
        } else {
            ev.timers.remove(i);
        }
        Some((closure, owner))
    })
}

fn advance(now_ms: f64, domain: Option<u32>, coalesce: bool) {
    while let Some((closure, owner)) = next_due(now_ms, domain, coalesce) {
        with_heap(|h| h.temps.push(closure));
        let hk = hooks();
        (hk.enter)(owner);
        // SAFETY: timer closures are live `() => void` closures.
        unsafe { call(closure) };
        (hk.leave)();
        with_heap(|h| {
            h.temps.pop();
        });
        ts_rt_microtasks_run();
    }
    with_heap(|h| h.events.now = h.events.now.max(now_ms));
}

/// Runs every timer of domain 1 (the main game: every timer, unless the host installed domain
/// hooks) due by `now_ms` (the host's clock), each followed by its microtasks.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_timers_advance(now_ms: f64) {
    advance(now_ms, Some(1), false);
}

/// Runs the timers of one event domain due by `now_ms`, each with that domain entered. A repeating
/// timer that missed several periods (a paused domain) runs once, not once per missed period. Other
/// domains' timers wait for their own advance.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_timers_advance_domain(now_ms: f64, domain: u32) {
    advance(now_ms, Some(domain), true);
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
        advance(due, None, false);
    }
}

/// Forgets every waiting microtask, after-hook and timer (a hot reload: they hold the closures of
/// the program being replaced, and must not run its code against the new world). Suspended
/// coroutines that only they referenced are collected as garbage.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_events_clear() {
    with_heap(|h| {
        h.events.micro.clear();
        h.events.after.clear();
        h.events.timers.clear();
    });
}

/// How many timers and microtasks are waiting.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_events_pending() -> f64 {
    with_heap(|h| (h.events.micro.len() + h.events.timers.len()) as f64)
}
