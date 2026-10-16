# Scripting

Crates and packages: [iron_script](../crates/iron_script/src) (Rust), [js/engine](../js/engine/src)
(`@iron/engine`), plus the loader in [iron_app/main.rs](../crates/iron_app/src/main.rs).
Design background: [DESIGN.md](../DESIGN.md) section 3 and 4.

## Values crossing the boundary

[iron_core::Value](../crates/iron_core/src/value.rs) is the only thing that crosses: `Null`,
`Bool`, `Number(f64)`, `String`, `Array`. There are no objects, no functions, no handles to JS.
Engine objects are numeric ids ([ids.rs](../crates/iron_core/src/ids.rs): `EntityId`,
`ComponentId`, `NodeId`). `from_js`/`to_js` in
[quickjs.rs](../crates/iron_script/src/quickjs.rs) convert; anything that is not bool, number,
string or array becomes `Null`.

## `ScriptRuntime`

Defined in [lib.rs](../crates/iron_script/src/lib.rs). Every backend provides the same globals.

| Method | Purpose |
|---|---|
| `load_bundle(name, code)` | Evaluate a Bun-built IIFE; `name` is used in error locations |
| `eval(code)` | Evaluate an expression to a `Value` (tests, console, feature checks) |
| `call_global(name, args)` | Call a global function: the way Rust calls into JS |
| `tick(now_ms)` | Fire due timers (see below); returns uncaught errors since last tick |
| `drain_ops()` / `drain_commits()` | Take what scripts queued since last call |
| `reset()` | Fresh context for hot reload; drops ops, commits, timers, errors; keeps the log sink |
| `set_log_sink(sink)` | Where `console.*` goes (default: `tracing`, target `script`) |

Difference from DESIGN.md section 4.1: the trait there has no `tick` or `drain_commits`; the code
added both (timers and UI commits are first-class). The shared behaviour is pinned by the
conformance suite ([conformance.rs](../crates/iron_script/src/conformance.rs)), which any new
backend must pass via `runtime_conformance_tests!`.

## QuickJS backend

[quickjs.rs](../crates/iron_script/src/quickjs.rs), `QuickJsRuntime` (rquickjs):

- Holds a `Context`, a `Runtime` and `Rc<RefCell<Shared>>`. `Shared` (ops, commits, errors, log
  sink) is what the native functions write into and survives `reset()`. Field order matters:
  the context must drop before its runtime (also handled in `reset`).
- `new_context` creates a runtime and context, runs `install_globals`, then evaluates
  [prelude.js](../crates/iron_script/src/prelude.js).
- **Native globals** installed by `install_globals`:
  - `console.log/info/debug/warn/error` -> the log sink.
  - `__iron_op(name, ...args)` -> pushes an `Op { name, args }` onto `Shared.ops`.
  - `__iron_commit(ops)` -> pushes a `Commit { ops }` onto `Shared.commits`.
  - `__iron_report_error(e)` -> pushes onto `Shared.errors` (used by the prelude).
- **Microtasks**: every entry point (`load_bundle`, `eval`, `call_global`) ends with `run_jobs()`,
  draining pending promise jobs. React 19 relies on microtasks, so "jobs run before returning to
  Rust" is part of the contract. A job that throws is kept in `Shared.errors` and returned by the
  next `tick`.
- Bundles are evaluated non-strict with the bundle name as the filename, so stack traces point at
  `game.js:line`. `thrown_to_error` builds a `ScriptError` ([error.rs](../crates/iron_script/src/error.rs))
  with message, stack and a file/line parsed from the stack.

## Timers (the prelude)

[prelude.js](../crates/iron_script/src/prelude.js) implements `setTimeout`/`clearTimeout` entirely
in JS so that Rust never holds a JS function (DESIGN rule). Timers live in a `Map` keyed by id,
with a due time on *engine time* and a sequence number for stable ordering.

`QuickJsRuntime::tick(now_ms)` repeatedly calls the global `__iron_fire_next_timer(now)`. Each
call fires the earliest due timer (ties broken by sequence) and returns `true`, so microtasks run
between timers because `call_global` drains jobs. The loop is capped at `MAX_TIMERS_PER_TICK`
(1000) so `setTimeout(f, 0)` loops cannot freeze a frame; timers that throw are reported through
`__iron_report_error` and the others still run. There is no `setInterval` or `requestAnimationFrame`.

## Ops and commits

| Channel | JS side | Rust side | Used for |
|---|---|---|---|
| Ops | `__iron_op(name, ...args)` (`op()` in `@iron/engine`) | `drain_ops()` -> `OpBuffer` of `Op {name, args}` | `loadFont`, `addStyleSheet`, `removeStyleSheet`, `focus`, `blur`, `scrollTo`... |
| Commits | `__iron_commit(ops)` | `drain_commits()` -> `Vec<Commit>` | React UI mutations |

Ops are generic (a name plus arguments, see [ops.rs](../crates/iron_core/src/ops.rs)); the doc
comment notes a binary encoding is planned once the op set settles (TESTS.md `M2-BEN-04`).
There is no registry: the **app** matches on `op.name` in `App::tick`
([main.rs](../crates/iron_app/src/main.rs)) and logs `unknown script op` otherwise.

A `Commit` is a `Vec<Value>`, each op `[opcode, ...args]`
([opcodes.rs](../crates/iron_ui/src/opcodes.rs), mirrored by
[opcodes.ts](../js/react/src/opcodes.ts)). Applied as a unit by `UiTree::apply_commit`; see
[ui-tree.md](ui-tree.md).

## `ComponentHost` and `@iron/engine`

Rust side: [components.rs](../crates/iron_script/src/components.rs). `ComponentHost` keeps a
`Vec<Entry { id, class, started, enabled }>` and talks to JS only by calling globals:

| Rust call | JS global (installed by `@iron/engine`) |
|---|---|
| `registered_classes` | `__iron_registered()` -> class names |
| `spawn(class)` | `__iron_create(class, id)` |
| `update(dt)` | `__iron_dispatch(id, "onStart"/"onUpdate", ...args)` |
| on error | `__iron_destroy(id)` |

`update` runs `onStart` once, then `onUpdate(dt)`, per component. A component that throws is
logged, disabled and destroyed; others keep running. Callbacks are looked up by
`(component id, method name)` at call time, never stored, so a reloaded bundle's code is picked up.

JS side: [js/engine/src/index.ts](../js/engine/src/index.ts).

- `Component` base class (`id`, optional `onStart`, `onUpdate`), `register({ Name: Class })`
  (an object, so names survive minification).
- `op(name, ...args)`, `loadFont(face)`, `addStyleSheet(css)` / `removeStyleSheet(id)`. JS allocates
  stylesheet ids; the app maps them to tree ids (`App.sheets`).
- The `__iron_*` host functions are assigned on `globalThis` at import time; instances live in a
  `Map<number, Component>`.
- [globals.ts](../js/engine/src/globals.ts) declares the ambient types (`__iron_op`, timers,
  `process.env.NODE_ENV`, `__iron_ui_event`). It is hand-written for now (a generated `.d.ts` is planned).

Today `iron_app` has no scene system (M5): it spawns **one instance of every registered class**
at startup and ticks them at a fixed 60 Hz (`TICK`, max 5 catch-up ticks per frame).

## From TypeScript to a running game

1. A game folder (e.g. [examples/css](../examples/css)) has `src/main.tsx` and a `build` script:
   `bun build src/main.tsx --outdir dist --entry-naming game.[ext] --format iife --target browser
   --define process.env.NODE_ENV='"development"'`. Bun strips types, bundles React and the `@iron/*`
   workspace packages into one IIFE, and (when CSS is imported) writes `dist/game.css` next to it.
   The `--define` is required because React reads `process.env.NODE_ENV` and QuickJS has no `process`.
2. `iron <folder>` ([main.rs](../crates/iron_app/src/main.rs)) calls `resolve_bundle`: a directory
   means `<dir>/dist/game.js` with the directory as the asset root; a `.js` path uses its parent.
3. `QuickJsRuntime::new()`, `load_bundle`, then `ComponentHost::registered_classes` + `spawn` each.
4. `ui_events` is probed once with `typeof __iron_ui_event === 'function'` (set by `@iron/react`);
   games without React UI skip event dispatch.
5. `dist/game.css`, if present, is added with `add_stylesheet_with_base(css, "dist")` so its relative
   `url(...)`s are rebased onto the game folder (see [css.md](css.md)). `@font-face` faces found in it
   are loaded through the same `load_font` helper as `loadFont()`.
6. Assets (`url()` images, fonts) are resolved with
   [resolve_asset](../crates/iron_core/src/assets.rs): relative paths only, no `..`, no schemes, no
   drive letters. `data:` URLs (Bun inlines small assets) go through `decode_data_url`.
   Game scripts therefore cannot read arbitrary files.

Test bundles are built the same way into `tests/fixtures/*.js` (`bun run build:fixtures`) and
embedded in Rust tests with `include_str!`; see [testing.md](testing.md).

## Hot reload

`reset()` exists and is tested, but hot reload is not wired into `iron_app` yet (milestone M4).
Because the Rust tree owns all UI state and JS holds only ids, the planned flow is: `reset()`,
reload the bundle, re-render.
