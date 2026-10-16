// Iron script prelude: timers on top of engine-driven time.
//
// Timers live entirely in JS so Rust never holds references to JS functions
// (DESIGN.md §3). Rust drives them by calling `__iron_fire_next_timer(now)`
// in a loop from `ScriptRuntime::tick`, draining microtasks between calls.
(() => {
  let now = 0;
  let nextId = 1;
  let nextSeq = 0;
  /** @type {Map<number, {due: number, seq: number, fn: Function, args: unknown[]}>} */
  const timers = new Map();

  globalThis.setTimeout = (fn, delay = 0, ...args) => {
    if (typeof fn !== "function") {
      throw new TypeError("setTimeout requires a function");
    }
    const id = nextId++;
    const ms = Number(delay);
    timers.set(id, { due: now + (ms > 0 ? ms : 0), seq: nextSeq++, fn, args });
    return id;
  };

  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };

  // Fires the earliest due timer. Returns false when none is due at `time`.
  globalThis.__iron_fire_next_timer = (time) => {
    now = time;
    let nextKey;
    let next;
    for (const [id, timer] of timers) {
      if (timer.due > now) continue;
      if (!next || timer.due < next.due || (timer.due === next.due && timer.seq < next.seq)) {
        nextKey = id;
        next = timer;
      }
    }
    if (!next) return false;
    timers.delete(nextKey);
    try {
      next.fn(...next.args);
    } catch (e) {
      __iron_report_error(e);
    }
    return true;
  };
})();
