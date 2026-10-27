# iron_ts: Symbol, iterators, generators, async and Proxy (design note, for review)

Both topics are **STOP** items in `TODO.md` M9.b2: nothing here is built. The questions are at the end.
What exists today: `for...of` over arrays, strings, `Map`, `Set` and typed arrays is compiled to loops
by the compiler (`lower/arrays.rs`, `lower/maps.rs`); a user class cannot be iterated; there is no
`Symbol`, no `function*`, no `Promise`/`async`.

## 1. `Symbol`

Symbols are values with identity, used as property keys. Iron objects are fixed-layout structs, so
only keys known at compile time can work.

- `Symbol.iterator`, `asyncIterator`, `toPrimitive`, `hasInstance`, `toStringTag`, `species` ...: compile
  time constants. A class member written `[Symbol.iterator]() { ... }` is an ordinary method with a
  reserved internal name (`@@iterator`); the compiler looks it up by that name (closed world).
- `Symbol("desc")` as a value: an `f64`-sized id (type `symbol`), a table of descriptions, `===` by id,
  `description`, `toString()`, `Symbol.for` / `keyFor` through a registry. `typeof` is `"symbol"`.
- A symbol as a *dynamic* property key on an object (`obj[sym] = 1`) is a compile error (no dynamic
  shapes). A `Map<symbol, V>` works (keys by id).

## 2. Iterators

`for (const x of obj)`, spread and destructuring on a value whose static type has `[Symbol.iterator]()`:
call it, then call `next()` per step; the result `IteratorResult<T>` is a prelude struct
`{ done: boolean; value: T }`. A fast path avoids the allocation when the iterator is a compiled
generator or one of the built-in collections (their loops stay as they are). `Iterable<T>` /
`Iterator<T>` annotations are the interface types the checker uses. Array methods that take an
iterable (`Array.from(iter)`, `new Map(iter)`, `Promise.all`) accept them the same way. Built-in
iterators (`map.entries()`, `arr.values()`) stay eager arrays/loops unless stored in a variable, in
which case they become real iterator objects.

## 3. Generators: two ways to build them

`function*`, `yield`, `yield*`, `next(sent)`, `return(v)`, `throw(e)`, `try/finally` across a yield, a
generator that is never finished.

**A. State machine (what TypeScript's downlevel emit does).** The body is turned into a flat set of
states; locals that live across a `yield` move into a heap frame object; `resume(frame, sent)` is a
`switch(state)` in a loop. Our IR is structured (blocks, loops, `if`), so resuming in the middle of a
nested loop needs the body in control-flow-graph form first: a new lowering path for generator bodies
only, with its own tests for every statement kind (`for`, `switch`, `try/finally`, labelled jumps).
Cheapest at run time (no extra stack), but it is the largest compiler change in the plan.

**B. Stackful coroutines.** A generator is a coroutine with its own native stack (`corosensei` crate:
x86-64, aarch64, ARM, RISC-V; Windows, Linux, macOS; **not** WebAssembly). `yield` is a runtime call that
switches back; `next()` switches in. This is cheap because our GC roots live on the **shadow stack**,
not the native stack: each coroutine gets its own shadow-stack segment, the collector scans the
segments of every suspended coroutine it can reach, and nothing on a native stack is a GC reference.
`try/finally`, checked-return exceptions and loops need no special lowering at all, and `async`/`await`
(section 4) is the same mechanism. Costs: a native stack per live generator (64 KB reserved,
committed lazily), a context switch per `yield` (tens of ns), and no WebAssembly target (A would be
needed there).

Recommendation: **B first**, because it is small (days, not weeks) and gives `yield*`, `finally` and
`await` for free; keep **A** as the later optimisation for hot generators and for WebAssembly.

## 4. `Promise`, `async`/`await`, timers

`Promise` as a prelude class (state, value, reaction list); `async function` is a coroutine whose
`await` suspends until the promise settles; `.then/.catch/.finally`, `Promise.all/allSettled/race/any/
resolve/reject`, `queueMicrotask`, `setTimeout/setInterval/clearTimeout`, `async` iterators and
`for await`. Needs one contract with the host: **who runs the microtask queue and the timers**. Proposal:
the runtime owns a microtask queue drained after every call into script (`ts_rt_run_microtasks`) and a
timer heap the host advances with `ts_rt_timers_advance(now_ms)` once per frame (game time), so no
thread and no OS event loop is involved; engine calls that complete later (file reads, network) resolve
a promise through a handle (`ts_rt_promise_resolve(handle, value)`).

## 5. `Proxy` and `Reflect`

A proxy intercepts property access by name at run time (`get`, `set`, `has`, `deleteProperty`,
`ownKeys`, `apply`, `construct`). In a typed ahead-of-time compiler the static type of a proxy is the
type of its target, so every `p.x` on that type could be a proxy access:

1. **Not supported** (compile error with a reason). Honest default: gameplay code almost always wants
   getters/setters, which already work.
2. **Proxyable classes** (closed world): `new Proxy(target, handler)` marks the target's class; property
   reads and writes on values of that class compile to `if (obj.proxy) slow path else direct`, so only
   those classes pay. The handler's `get(target, key, receiver)` gets the key as a string (the class's
   reflection table maps names to slots, which `@property` already generates). Function proxies
   (`apply`, `construct`) work on function-typed values the same way. `Reflect.get/set/has/
   ownKeys/getPrototypeOf` with literal keys are plain accesses; with dynamic keys they use the
   reflection table. `Proxy.revocable`, symbols as keys and index-signature targets are out.

Recommendation: **1 now, 2 when a program needs it** (reactive state, validation wrappers).

## 6. The rest of the tier-4 list (no design needed)

`Temporal` (the model is `jiff`'s; a large surface, build PlainDate/PlainTime/PlainDateTime/Instant/
ZonedDateTime/Duration in that order), `BigInt` (a runtime bignum with `bigint` literals and the
mixing errors), `WeakRef`/`FinalizationRegistry` (needs collector hooks), `ArrayBuffer`/`DataView` and
typed-array views that share memory (an owner pointer in the array header), `structuredClone`,
`globalThis`, `Error.stack`.

## Questions for review

1. Generators: B (stackful coroutines, no WebAssembly) first, A later? Or A up front?
2. `async`/`await` and `Promise` on the same coroutines, with the microtask/timer contract above?
3. `Proxy`: option 1 (not supported) for now, option 2 when needed?
4. Is WebAssembly a target in the next year? (It decides whether A is needed early.)

## Answers (user, 2026-10-19)

1. Generators: B, stackful coroutines first. 2. `async`/`await` and `Promise` on the same coroutines with
the microtask and timer contract above. 3. `Proxy`: not supported until needed. 4. WebAssembly is not a
target in the next year. Build `Symbol`, iterators and generators now (async follows on the same
mechanism).
