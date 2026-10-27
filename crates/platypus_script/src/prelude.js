// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Platypus script prelude: timers on top of engine-driven time.
//
// Timers live entirely in JS so Rust never holds references to JS functions
// (DESIGN.md §3). Rust drives them by calling `__platypus_fire_next_timer(now, domain)`
// in a loop from `ScriptRuntime::tick`, draining microtasks between calls.
//
// Event domains (one per window family): a timer remembers the domain current when it was set
// (`__platypus_domain()`, installed by the host; 1 without it) and fires with that domain entered
// (`__platypus_enter_domain(d)` / `__platypus_leave_domain()`). `__platypus_fire_next_timer(time, domain)`
// only fires that domain's timers.
(() => {
  let now = 0;
  let nextId = 1;
  let nextSeq = 0;
  /** @type {Map<number, {due: number, seq: number, fn: Function, args: unknown[], domain: number}>} */
  const timers = new Map();
  const currentDomain = () =>
    typeof globalThis.__platypus_domain === "function" ? globalThis.__platypus_domain() : 1;

  globalThis.setTimeout = (fn, delay = 0, ...args) => {
    if (typeof fn !== "function") {
      throw new TypeError("setTimeout requires a function");
    }
    const id = nextId++;
    const ms = Number(delay);
    timers.set(id, {
      due: now + (ms > 0 ? ms : 0),
      seq: nextSeq++,
      fn,
      args,
      domain: currentDomain(),
    });
    return id;
  };

  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };

  // Fires the earliest due timer of `domain`. Returns false when none is due at `time`.
  globalThis.__platypus_fire_next_timer = (time, domain = 1) => {
    now = time;
    let nextKey;
    let next;
    for (const [id, timer] of timers) {
      if (timer.due > now || timer.domain !== domain) continue;
      if (!next || timer.due < next.due || (timer.due === next.due && timer.seq < next.seq)) {
        nextKey = id;
        next = timer;
      }
    }
    if (!next) return false;
    timers.delete(nextKey);
    const enter = globalThis.__platypus_enter_domain;
    if (typeof enter === "function") enter(next.domain);
    try {
      next.fn(...next.args);
    } catch (e) {
      __platypus_report_error(e);
    } finally {
      if (typeof enter === "function") globalThis.__platypus_leave_domain();
    }
    return true;
  };
})();
