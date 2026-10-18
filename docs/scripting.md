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
| `load_bundle(name, code)` | Evaluate a Bun-built IIFE (or, for a dev build's game code, the CommonJS wrapper `load_game` makes); `name` is used in error locations |
| `eval(code)` | Evaluate an expression to a `Value` (tests, console, feature checks) |
| `call_global(name, args)` | Call a global function: the way Rust calls into JS |
| `tick(now_ms)` | Fire due timers (see below); returns uncaught errors since last tick |
| `drain_ops()` / `drain_commits()` | Take what scripts queued since last call |
| `drain_targeted_commits()` | Like `drain_commits`, with the UI tree each commit is for (the default method says 0 for all) |
| `reset()` | Fresh context; drops ops, commits, timers, errors; keeps the log sink |
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
  - `__iron_commit(ops, target?)` -> pushes a `Commit { ops }` onto `Shared.commits` with its UI tree
    `target` (default 0, the screen; others are world panels). `drain_commits()` drops the targets;
    `drain_targeted_commits()` returns `(target, Commit)` pairs.
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
| Ops | `__iron_op(name, ...args)` (`op()` in `@iron/engine`) | `drain_ops()` -> `OpBuffer` of `Op {name, args}` | `loadFont`, `uiScale`, `addStyleSheet`, `removeStyleSheet`, `focus`, `blur`, `setPointerCapture`, `releasePointerCapture`, `scrollTo`..., and the `physics.*` and `audio.*` families |
| Commits | `__iron_commit(ops, target)` | `drain_targeted_commits()` -> `Vec<(u32, Commit)>` | React UI mutations, for the screen or a world panel |

Ops are generic (a name plus arguments, see [ops.rs](../crates/iron_core/src/ops.rs)); the doc
comment notes a binary encoding is planned once the op set settles (TESTS.md `M2-BEN-04`).
There is no registry: the **app** matches on `op.name` in `App::tick`
([main.rs](../crates/iron_app/src/main.rs)) and logs `unknown script op` otherwise. `physics.*` and `audio.*` ops are matched by prefix and
forwarded to `Physics::apply` and the mixer; `cloth.*` goes to `ClothSystem`
([physics-and-audio.md](physics-and-audio.md#cloth)); `app.runInBackground` and `app.targetFrameRate` are the
app's own; `scene.*` goes to `ScriptScene::apply` and `render.*` to
`WorldViews`/`Panels` (see [scene-and-objects.md](scene-and-objects.md) and
[world-rendering.md](world-rendering.md)).

A `Commit` is a `Vec<Value>`, each op `[opcode, ...args]`
([opcodes.rs](../crates/iron_ui/src/opcodes.rs), mirrored by
[opcodes.ts](../js/react/src/opcodes.ts)). Applied as a unit by `UiTree::apply_commit`; see
[ui-tree.md](ui-tree.md).

## `ComponentHost` and `@iron/engine`

Rust side: [components.rs](../crates/iron_script/src/components.rs). `ComponentHost` keeps a
`Vec<Entry { id, class, object, hooks, started, enabled }>` and talks to JS only by calling globals:

| Rust call | JS global (installed by `@iron/engine`) |
|---|---|
| `registered_classes` | `__iron_registered()` -> names of the starting classes (registered without `spawn: false`) |
| `spawn_on(class, object, name)` (`spawn` has no object) | `__iron_create(class, id, object, name)` |
| first run of a component | `__iron_hooks(id)` -> names of its lifecycle methods, kept in the `Entry` |
| `update_active(dt, active)` | `__iron_tick(dt)` (script `Time`; returns the scaled `dt`), then `__iron_dispatch(id, "onStart"/"onUpdate"/"onLateUpdate", ...args)` |
| `fixed_update(dt, active)` | `__iron_dispatch(id, "onFixedUpdate", dt)` |
| `pre_render(active)` | `__iron_dispatch(id, "onPreRender")` |
| `remove`, `remove_on_objects`, and on error | `onDestroy` (not after an error), then `__iron_destroy(id)` |
| `adopt(id, class, object)` | none: takes over a component a script made with `addComponent` |

`onStart` runs once before any other hook. Each hook only runs on components that define it and whose
GameObject is active (the `active` closure). A component that throws is
logged, disabled and destroyed; others keep running. The order within a frame is in
[scene-and-objects.md](scene-and-objects.md#components-and-the-lifecycle). Callbacks are looked up by
`(component id, method name)` at call time, never stored, so a reloaded bundle's code is picked up.

JS side: [js/engine/src/index.ts](../js/engine/src/index.ts).

- `Component` base class (`id`, `gameObject`, `transform`, optional `onStart`, `onUpdate`,
  `onLateUpdate`, `onFixedUpdate`, `onPreRender`, `onDestroy`, `onReload`), `register({ Name: Class }, { spawn })`
  (an object, so names survive minification; `spawn: false` names a class for scene files without
  starting one). `GameObject`, math, `Time`, cameras and the rest of the
  M5.a API are in [scene-and-objects.md](scene-and-objects.md) and [world-rendering.md](world-rendering.md).
- `op(name, ...args)`, `loadFont(face)`, `addStyleSheet(css)` / `removeStyleSheet(id)`. JS allocates
  stylesheet ids; the app maps them to tree ids (`App.sheets`).
- `setUiScale(scale)`: sends the `uiScale` op (`"none"`, `"fixed" f`, `"height" h`, `"fit" w h`).
  Takes `{ height: 1080 }`, `{ width, height }`, a number (fixed zoom) or `null` (the default). The
  app handles it in `ui_scale.rs` (see [input-and-events.md](input-and-events.md#ui-scale-m3d-step-9b)).
- `input` ([input.ts](../js/engine/src/input.ts)): keyboard and gamepad state and events for
  gameplay code, fed by `__iron_input(type, ...args)` from the app (see
  [input-and-events.md](input-and-events.md#gamepads-and-game-input-m3c-step-6)).
- `physics` ([physics.ts](../js/engine/src/physics.ts)) and `audio` ([audio.ts](../js/engine/src/audio.ts)):
  bodies, colliders, sounds, buses. They queue `physics.*`/`audio.*` ops and receive results through
  `__iron_physics` and `__iron_audio` (see [physics-and-audio.md](physics-and-audio.md)).
- The `__iron_*` host functions are assigned on `globalThis` at import time; instances live in a
  `Map<number, Component>`.
- [globals.ts](../js/engine/src/globals.ts) declares the ambient types (`__iron_op`, timers,
  `process.env.NODE_ENV`, `__iron_ui_event`, `__iron_input`, `__iron_physics`, `__iron_audio`, and the scene hooks below). It is hand-written for now (a generated `.d.ts` is planned).

The scene globals, installed by `@iron/engine` and called by Rust: `__iron_registered`,
`__iron_create(class, id, object, name)`, `__iron_hooks(id)`, `__iron_dispatch`, `__iron_destroy`,
`__iron_tick(dt)` (script `Time`, once per frame), `__iron_scene(moved)` (engine moves of objects, see
[scene-and-objects.md](scene-and-objects.md#the-move-batch)) and `__iron_screen(width, height, scale)`
(the screen size in UI pixels, when it changes). `__iron_animate(dt)` poses animated objects between `onUpdate`
and `onLateUpdate` ([animation-3d.md](animation-3d.md#when-it-runs)); `__iron_focus(focused, paused)` reports
focus and background pausing ([platform.md](platform.md#focus-and-pausing)); `__iron_assets_changed(paths)`
reports reloaded asset files ([assets-and-models.md](assets-and-models.md#hot-reload-of-assets)). `__iron_asset_text([id, text | null, ...])` answers
`asset.readText` (files scenes load, see [scene-and-objects.md](scene-and-objects.md#scenes)). Like `__iron_ui_event`, the optional ones
(`__iron_tick`, `__iron_animate`, `__iron_focus`, `__iron_assets_changed`, `__iron_scene`, `__iron_screen`,
`__iron_physics`, `__iron_audio`) are probed or
called tolerantly, so games that don't import the engine still run.

`iron_app` makes **one GameObject per starting class** (registered without `spawn: false`) with that
component (`spawn_starting_components`; a start scene is loaded by the scripts themselves, see
[scene-and-objects.md](scene-and-objects.md#scenes)) and ticks at a fixed 60 Hz (`TICK`, max 5 catch-up ticks per
frame). Per frame the order is:

1. For each fixed tick: `fixed_update` (`onFixedUpdate`), `apply_script_output` (commits, then ops in
   order), `step_physics` (calls `__iron_physics(moved, contacts, rays, characters)` if defined), the
   cloth step, then `send_scene_moves` (`__iron_scene`).
2. `ScriptRuntime::tick` (timers and microtasks).
3. `apply_script_output`, `update_active` (`__iron_tick`, then `onUpdate`, `__iron_animate` and
   `onLateUpdate`, `dt` capped at 0.1 s), `apply_script_output`, then `update_audio`, which calls
   `__iron_audio(ended)`.

Before all that, `App::tick` polls the script and asset watchers. While the game is paused in the
background (see [platform.md](platform.md#focus-and-pausing)) it stops there: none of the steps run.
4. At draw time: `pre_render` (`onPreRender`), `apply_script_output`, `__iron_screen` if the size
   changed, then panels and the world are drawn ([world-rendering.md](world-rendering.md)).

See [physics-and-audio.md](physics-and-audio.md#per-fixed-tick).

## From TypeScript to a running game

1. A game folder (e.g. [examples/css](../examples/css)) has `src/main.tsx` and a `build` script:
   `bun build src/main.tsx --outdir dist --entry-naming game.[ext] --format iife --target browser
   --define process.env.NODE_ENV='"development"'`. Bun strips types, bundles React and the `@iron/*`
   workspace packages into one IIFE, and (when CSS is imported) writes `dist/game.css` next to it.
   Examples build with `NODE_ENV="production"` (smaller, faster React); `bun run build:dev` in an
   example keeps the development build.
   The `--define` is required because React reads `process.env.NODE_ENV` and QuickJS has no `process`.
2. `iron <folder>` ([main.rs](../crates/iron_app/src/main.rs)) runs the current folder when no game is
   given (`parse_args_from`; the folder may come before or after the options, `M5-APP-03`). It calls
   `resolve_bundle`: a directory
   means `<dir>/dist/game.js` with the directory as the asset root; a `.js` path uses its parent.
3. `QuickJsRuntime::new()`, `load_bundles`, then `ComponentHost::registered_classes` + `spawn` each.
   `load_bundles` loads `game.js` alone, or for a dev build (below) `vendor.js` and then the game's
   code through `iron_script::load_game`.
4. `ui_events` is probed once with `typeof __iron_ui_event === 'function'` (set by `@iron/react`);
   games without React UI skip event dispatch.
5. `dist/game.css`, if present, is added with `add_stylesheet_with_base(css, "dist")` so its relative
   `url(...)`s are rebased onto the game folder (see [css.md](css.md)). `@font-face` faces found in it
   are loaded through the same `load_font` helper as `loadFont()`.
6. Assets (`url()` images, fonts) are resolved with
   [resolve_asset](../crates/iron_core/src/assets.rs): relative paths only, no `..`, no schemes, no
   drive letters. `data:` URLs (Bun inlines small assets) go through `decode_data_url`.
   Game scripts therefore cannot read arbitrary files.

**Dev builds.** `bun run dev` in an example runs [scripts/dev.ts](../scripts/dev.ts) instead of the
`build` script. It writes two files to `dist/`:

- `vendor.js`: an IIFE with React, the reconciler, the `react-refresh` runtime, `@iron/engine` and
  `@iron/react` ([vendor.ts](../js/react/src/vendor.ts)), built once. It keeps a table of those
  modules behind `globalThis.__iron_require`.
- `game.js`: the game's own code as CommonJS (`--format cjs`), with `react`, `react/*` and `@iron/*`
  left external, so its `require` calls get the vendor bundle's copies. Bun's `--react-fast-refresh`
  adds the `$RefreshReg$` / `$RefreshSig$` calls. It starts with the banner `// @iron-dev`.

The split exists because React, the reconciler and the engine must stay in one runtime for Fast
Refresh: the game's code can run again while their module state (the fiber tree, the scene's
classes and instances) stays. `bun run dev --once` builds once without watching. A production
`bun run build` still makes one self-contained `game.js`.

The app detects a dev build by the banner (`dev_vendor` in [main.rs](../crates/iron_app/src/main.rs)):
if `game.js` starts with `// @iron-dev`, it reads `vendor.js` from the same folder (a missing one is
logged as an error and the code loads as a plain bundle).

Test bundles are built the same way into `tests/fixtures/*.js` (`bun run build:fixtures`) and
embedded in Rust tests with `include_str!`; see [testing.md](testing.md).

An experimental ahead-of-time compiler for a TypeScript subset exists in `tools/`; see
[iron-ts.md](iron-ts.md). The engine does not use it yet.

## Hot reload

Flow, from an edit to the new code running:

1. `bun run dev` in an example runs `scripts/dev.ts` (see "Dev builds" above), which builds
   `dist/vendor.js` once and runs `bun build --watch` for the game's code, so every save
   rewrites `dist/game.js` and, when CSS is imported, `dist/game.css`.
2. `iron <folder>` starts a `BundleWatcher` ([reload.rs](../crates/iron_app/src/reload.rs)). It uses
   `notify` on the `dist` folder (not the files: Bun may replace them) and only counts events for
   the bundle, its CSS and `vendor.js` by file name. Access events are ignored.
3. `App::tick` polls the watcher first thing. `poll` reports a change once the files have been quiet
   for `SETTLE` (150 ms), because Bun writes the JS and the CSS separately, and big files in
   several writes.
4. `App::reload_scripts` reads the bundle. If it is a dev build and `vendor.js` is the same text as
   the loaded one, it takes the in-place path (next). Otherwise it saves the old code's state
   (below) and calls `iron_script::reload`.

Screenshot runs (`--screenshot`) have no watcher. If the watch can't start, the app logs a warning
and runs without hot reload.

Other files in the game folder (textures, models, sounds) have their own `AssetWatcher` in the same file.
Those reload in place without touching the scripts: see
[assets-and-models.md](assets-and-models.md#hot-reload-of-assets). `bun run dev` also writes `.meta` files
for new assets before building ([assets-and-models.md](assets-and-models.md#the-asset-database)).

**`--cook`.** `iron <folder> --cook` doesn't load any scripts: it prepares textures and models into
`.iron/cooked/` and exits ([assets-and-models.md](assets-and-models.md#cooking)).

**In place (dev builds, Fast Refresh).** When only `game.js` changed, `reload_in_place` runs the new
code in the same runtime, so React state, the world, physics and sounds all stay. The sequence is
`iron_script::load_game` ([components.rs](../crates/iron_script/src/components.rs)):

1. `__iron_hot_begin()` ([hot.ts](../js/engine/src/hot.ts)) runs the undo callbacks the old top level
   registered with `_undoOnReload` (only `addStyleSheet` does: it removes its sheet), then sets
   `_loadingGame()`.
2. The code is evaluated inside a wrapper, `(function (require, module, exports) { <code>\n})(__iron_require, ...)`.
   The wrapper shares the code's first line, so error line numbers stay right. The top level runs
   again: `register` replaces the classes by name, and `createRoot()` returns the live root while
   `_loadingGame()` is set, and that root's `render` does nothing then ([react.md](react.md#createroot)).
3. `__iron_hot_end()` runs, even if the code threw. It calls `_patchClasses` ([scene.ts](../js/engine/src/scene.ts)):
   every live component whose class is now registered as a different class under its name gets the
   new prototype (`Object.setPrototypeOf`), keeps its fields and gets defaults for fields the new
   class added. Then the after-load callbacks run: `vendor.ts` registers `performReactRefresh`, which
   re-renders the components whose code changed and keeps their state where the hooks still match.
4. Back in Rust: `ComponentHost::refresh_hooks` forgets which lifecycle methods each component has
   (the new classes may add or drop some), `game.css` is removed and loaded again, and
   `Input::Refresh` is sent. The log line is `reloaded <bundle> in place in N ms`.

A syntax error stops before anything ran. A throwing top level may have run partly. Either way the
error is logged as `reload failed: ...` and the runtime stays (there is no fresh runtime to fall
back on, unlike the full reload).

It falls back to the full reload below when the build isn't a dev build, when `vendor.js` changed
(or can't be read). That path keeps state through scenes, but React state is lost.

Limits of the in-place path:

- Only style sheets are undone from the top level. Fonts (`loadFont`), `input` and scene listeners and
  timers registered at the top level are not, so they pile up on each reload. Register them in
  components instead.
- Class statics start fresh, since the class is a new object. Top-level objects (constants,
  `new GameObject(...)`) are created again, and module-level variables are new.
- A live component whose class is no longer registered keeps its old code. `__iron_hot_end` warns
  `reload: X isn't registered, so its components keep the old code (register({ X }, { spawn: false }))`.
- Fast Refresh needs the dev build's transform. Production builds don't refresh; they use the full
  reload.

M4-HOT-06 covers the in-place path ([testing.md](testing.md#fixture-bundles)).

**New runtime first, then swap.** `iron_script::reload(rt, fresh, host, load, bootstrap)` runs the
`load` closure on a fresh `QuickJsRuntime` (one `load_bundle`, or `load_bundles` for a dev build: vendor, then
game code), reads `registered_classes` and calls the `bootstrap` closure, all
in `fresh`. The app's closure restores the saved state, or, with nothing to restore, calls
`spawn_starting_components`, which makes the objects and components in a new `ScriptScene`;
`spawn_each` is the plain one-component-per-class bootstrap. Only when every step works are `rt` and the `ComponentHost` replaced (the old context
is dropped). A syntax error, a throwing top level or a throwing constructor returns an error, and
both stay as they were. `reload_scripts` logs `reload failed, keeping the old code` and returns
before touching the UI, physics or audio. The game keeps running and the next save tries again.
M4-HOT-01 and M4-HOT-04 cover the success and failure paths.

**State is saved first.** `reload_scripts` asks the old runtime for its state before building the new one.
If `__iron_save_state` exists (it does when the bundle imports `@iron/engine`'s scenes), the old code
returns one JSON string; a bundle without it, or a save that throws (logged as `saving state failed,
starting over`), just starts over. The JSON is a `ReloadState` ([scenes.ts](../js/engine/src/scenes.ts)):

- Every scene, saved with the scene format ([scene-and-objects.md](scene-and-objects.md#scenes)):
  objects, hierarchy, transforms, components and `@property` values, and which scene is active.
  Default transforms are left out. The `DontDestroyOnLoad` scene too.
- In this reload mode a script component's other plain fields are saved too (counters, timers,
  state), with references to objects and components remapped. A field that can't be saved
  (functions, engine handles, class instances) is left out quietly instead of warning.
- The quality level and settings (`_qualityState`) and the clock (`Time`, frame count, time scale).

**Restoring.** The fresh runtime gets `__iron_reloading = true` before the bundle loads, so the start
scene is not loaded on top of the saved ones. After the bundle's top level and registration, the
bootstrap sets the flag back and calls `__iron_restore_state(json)`. If it returns true, the scenes are
rebuilt with `loadObjects` in reload mode and `spawn_starting_components` is skipped; otherwise it runs
as on a first start.

- Script components come back marked as restored: no `onAwake`, no `onStart` (they started already;
  `__iron_hooks` leaves `onStart` out), and `onReload()` is called once everything is built. Put
  re-attaching event listeners, timers and callbacks there. A throwing `onReload` is logged.
- Engine components (`Rigidbody`, `MeshRenderer`, `Light`, ...) are rebuilt like a scene load:
  `onAwake` and `onStart` run, so bodies and renderers are made again.
- A property that no longer exists on the class (renamed or removed) is dropped with a warning; new
  properties keep their defaults.
- **Fallback.** If a saved component class isn't registered any more, `restoreState` warns with what to
  register (`register({ X }, { spawn: false })`), loads the start scene and returns false, so the game
  starts over.

The log line `reloaded <bundle> in N ms` has a `restored` field that says which way it went.
`IRON_RELOAD_AT_FRAME=N` makes a `--screenshot` run call `reload_scripts` after frame N, so a reload can
be checked without a window or a file edit (the bundle on disk is reloaded as it is).

**What is reset.** After a successful swap `reload_scripts` wipes the world the old scripts
built, and the restored scenes rebuild their part of it as ops:

- The UI tree: `UiTree::clear_script_nodes` (see [ui-tree.md](ui-tree.md#node-lifetime)), because
  the new bundle's reconciler starts its IDs again.
- Script stylesheets (`addStyleSheet`) and `game.css`, which is removed and loaded again from disk
  with `load_game_css`.
- `uiScale` goes back to unset.
- Physics: a new `Physics::new()`. Bodies and their IDs are gone; `Rigidbody` and colliders are made
  again from their saved properties.
- Audio: the mixer is dropped, which stops every sound; it is created again on first use.
- The scene, cameras, planes and world panels: a new `ScriptScene` and `WorldViews`, no panels, and
  pointer and key focus back on the screen. The screen size is sent again on the next frame.

Then it sends `Input::Refresh` and probes `__iron_ui_event`, `__iron_input`, `__iron_physics`, `__iron_audio` and
`__iron_scene` again. The new bundle's top level has already queued its UI and ops, and they apply
after the wipe.

M4-HOT-02, 03 and 05 cover the saved state, the rebuilt objects and the fallback. M4-HOT-07 is the
benchmark ([testing.md](testing.md#benchmarks)): about 146 ms for 1,000 components, over the 100 ms target.

**Limits.**

- Not kept: physics velocities and other body state beyond the saved properties, playing sounds,
  particles in flight, module-level variables in the bundle (they start fresh; keep state in
  component fields), React state (the new bundle renders its UI from scratch; only the in-place
  path above keeps it), and fields that can't be saved.
- `bun build --watch` doesn't watch `@iron/engine` or `@iron/react` sources outside the example
  folder, so editing those doesn't trigger a rebuild. Touch a file in the example or rebuild
  (in a dev build, restart `bun run dev`: `vendor.js` is built once).
- Keys held at the time of the reload are forgotten.
