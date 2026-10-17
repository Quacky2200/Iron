# Scene and objects

Crates and packages: [iron_scene](../crates/iron_scene/src) (Rust),
[iron_script/components.rs](../crates/iron_script/src/components.rs) (component lifecycle),
[js/engine](../js/engine/src) (`@iron/engine`), plus the wiring in
[iron_app/main.rs](../crates/iron_app/src/main.rs). Design background:
[DESIGN.md](../DESIGN.md) section 7.5. Milestone M5.a. How cameras and planes reach the screen is in
[world-rendering.md](world-rendering.md).

## GameObjects in Rust

[lib.rs](../crates/iron_scene/src/lib.rs) has `Scene`, a generational arena of GameObjects. Each has a
name, an enabled flag, a parent, children and a local `Transform` (position, rotation, scale).
There is no ECS: parent and child links plus a dirty flag per object are enough for small scenes.

- `ObjectId` carries a generation, so the id of a destroyed object misses instead of finding whatever
  reused the slot.
- `create`, `destroy` (returns the object and its descendants, parent first), `set_parent(id, parent,
  keep_world)` (returns `false` for a cycle), `set_local`, `set_world`, `set_enabled`, `find`, `walk`
  (depth-first, in order).
- `active(id)` is true when the object and all its ancestors are enabled.
- World transforms are worked out lazily from the hierarchy. `update_world` refreshes the dirty ones
  (the app calls it before drawing); `world_matrix(id)` composes through the parents.
- `take_moved` returns the objects whose world transform changed since the last call, including new
  ones.

Components do not live in the scene. Script components are in `ComponentHost`, physics bodies in
`iron_physics`, cameras and panels in the app. They refer to objects by id.

## Script ids and engine ids

Scripts hold numbers, never engine references. [script.rs](../crates/iron_scene/src/script.rs) has
`ScriptScene`, which wraps a `Scene` and maps the script's object ids (`ScriptObject`, a `u32`) to
`ObjectId`s, like UI nodes and physics bodies.

| Range | Who picks the id |
|---|---|
| `1 ..` | Scripts (`new GameObject(...)` counts up from 1) |
| `1 << 30` (`ENGINE_IDS`) and up | The engine, for objects it creates for scripts (`create_engine_object`) |

Component ids split too: `ComponentHost` counts the engine's own up from 0, and scripts allocate
`addComponent` ids from `1 << 24`. Ids go through `f64` in `Args::id`, because `f32`
can't hold integers past 2^24.

Until scene files exist (M5.b), `spawn_starting_components` in `main.rs` makes one GameObject per
registered component class, named after it, holding that component. Games build the rest from
`onStart`.

## `scene.*` ops

Scripts change the scene with ops. `App::tick` matches the `scene.` prefix and calls
`ScriptScene::apply(name, args)`. A bad op logs `scene.<name>: ...` and changes nothing.

| Op | Arguments | Effect |
|---|---|---|
| `scene.create` | `id name parent\|null [p] [q] [s]` | New object. Reusing an id destroys the old object first. |
| `scene.destroy` | `id` | Destroys the subtree. Unknown ids are ignored. |
| `scene.parent` | `id parent\|null` | Re-parents and keeps the **local** transform. Scripts that keep the world position send `scene.local` after. A cycle is an error. |
| `scene.local` | `id [p] [q] [s]` | Sets the local transform. Missing parts are identity. A zero quaternion is an error. |
| `scene.enabled` | `id bool` | |
| `scene.name` | `id name` | |
| `scene.addComponent` | `object component class` | Returns `SceneEvent::ComponentAdded`. |
| `scene.removeComponent` | `component` | Returns `SceneEvent::ComponentRemoved`. |

`apply` returns an optional `SceneEvent` for what the rest of the engine must do, and `App::scene_op`
handles it:

- `ComponentAdded` calls `ComponentHost::adopt` (the script already created the instance).
- `ComponentRemoved` calls `ComponentHost::remove` (`onDestroy`, then `__iron_destroy`).
- `Destroyed(objects)` calls `remove_on_objects`, then `WorldViews::forget_objects` and
  `Panels::forget_objects`, so cameras, planes, panels and components on those objects go too.

## The move batch

Scripts keep a mirror of every object's name, parent and local transform, so reads need no round
trip. Only changes the **engine** makes (physics moving a body) go back. `set_world_from_engine`
records the object, and `take_batch` returns a flat array, 11 numbers per object:

```
id, px, py, pz, qx, qy, qz, qw, sx, sy, sz      (local transform)
```

`App::send_scene_moves` calls `__iron_scene(moved)` once per fixed tick, after physics steps. It
skips the call when the batch is empty or the bundle has no `__iron_scene` (probed at startup and
after a reload as `scene_events`). Script-made changes are never echoed back. The JS side updates the
mirror without queueing ops.

## Components and the lifecycle

`ComponentHost` keeps `Entry { id, class, object, hooks, started, enabled }`.

| Call | Purpose |
|---|---|
| `spawn_on(rt, class, object, name)` | Creates the instance with `__iron_create(class, id, object, name)`. `spawn` is the same without an object. |
| `adopt(id, class, object)` | Takes over a component a script created with `addComponent`. |
| `remove(rt, id)` | `onDestroy` if it had started and has the hook, then `__iron_destroy`. |
| `remove_on_objects(rt, objects)` | `remove` for every component on destroyed objects. |
| `fixed_update`, `update_active`, `pre_render` | The per-tick and per-frame hooks below. Each takes an `active(object)` closure. |

**Hooks are looked up once.** On a component's first run the host calls `__iron_hooks(id)`, which
returns the names of its lifecycle methods, and stores them. A missing hook costs nothing. If the
bundle has no `__iron_hooks`, every hook is assumed present. Components on inactive objects are
skipped. A component that throws is logged, disabled and destroyed; the others keep running.

**Order.** `App::tick` runs, per frame:

1. For each fixed 60 Hz tick (at most 5 catch-up ticks): `onStart` for new components, then
   `onFixedUpdate(dt)` for all; `apply_script_output`; `step_physics`; `send_scene_moves`.
2. `ScriptRuntime::tick` (timers and microtasks).
3. `apply_script_output`, then `update_active`: `__iron_tick(dt)` advances script `Time` and returns the
   scaled `dt`; then `onStart` for new components and `onUpdate(dt)` for all; then `onLateUpdate(dt)`
   for all. `dt` is the frame length, capped at `MAX_FRAME_DT` (0.1 s).
4. `apply_script_output`, `update_audio`, `Input::Refresh` for the screen and every panel.
5. Drawing: `pre_render` runs `onPreRender()`, then `apply_script_output`, then
   `__iron_screen(width, height, scale)` if the size changed, then the world and UI are drawn.

`onDestroy` runs when the component or its object is removed. `onStart` always runs before the first
other hook, in whichever step reaches the component first.

## TypeScript API overview

All exported from [js/engine/src/index.ts](../js/engine/src/index.ts). The names follow Unity and
s&box.

**Scene** ([scene.ts](../js/engine/src/scene.ts)).

- `GameObject`: `new GameObject(name, { parent, position, rotation, scale })`, `name`, `parent`,
  `children`, `components`, `setParent(parent, keepWorld = true)`, `addComponent(Class, props)`,
  `getComponent`, `removeComponent`, `destroy`, `findChild`. Statics: `find(name)`, `roots`, `byId`,
  `findObjectOfType`.
- `Transform`: `position`, `rotation`, `scale` (world) and local versions, `forward`, `right`, `up`,
  `eulerAngles`, `translate`, `rotate`, `lookAt`, `transformPoint`, `inverseTransformPoint`,
  `transformDirection`.
- `Component`: `id`, `gameObject`, `transform`, `getComponent`, `destroy`, and the optional hooks
  `onStart`, `onUpdate(dt)`, `onLateUpdate(dt)`, `onFixedUpdate(dt)`, `onPreRender()`, `onDestroy()`.
- `register({ Name: Class })`. Registered classes are the starting components. Engine classes such as
  `Camera` are not registered and go by class name.

**Math** ([math.ts](../js/engine/src/math.ts)). Immutable `Vector2`, `Vector3`, `Vector4`,
`Vector2Int`, `Vector3Int`, `Quaternion` (Euler degrees in Unity's order, `lookRotation`, `slerp`),
`Matrix4x4`, `Ray`, `Plane`, `Rect`, `RectInt`, `Bounds`, `Pose`, `Color` (`Color.hex` converts sRGB to
linear), and `Mathf`. Coordinates are right-handed, with -z as `Vector3.forward` for cameras
([world-rendering.md](world-rendering.md#coordinates)).

**Dates and spans** ([time.ts](../js/engine/src/time.ts)). C#-style `DateTime` and `TimeSpan` on JS
`Date`. **Resolution is one millisecond.** There are no ticks, microseconds or nanoseconds, a
`TimeSpan` rounds to whole milliseconds, and `f` format digits past three print as zeros. Don't use
them to profile.

**Time and timers** ([clock.ts](../js/engine/src/clock.ts)). `Time` (`time`, `deltaTime`,
`unscaledDeltaTime`, `fixedDeltaTime`, `frameCount`, `timeScale`, `realtimeSinceStartup`) advances in
`__iron_tick`. `timeScale` scales the `dt` components get; physics is not scaled yet. `TimeSince`
(`now`, `ago`, `seconds`, `over`), `TimeUntil` (`in`, `seconds`, `done`, `fraction`) and awaitable
`Task.delay`, `delaySeconds` and `frame` (resolves at the next tick). `Task.delay` uses `setTimeout`,
so it runs on engine time.

**Logging** ([log.ts](../js/engine/src/log.ts)). `Debug`, `Log` and `Console` all end in `console.*`,
so they reach the app's log sink. Values print readably: vectors via `toString`, errors with their
stack, other objects as JSON (cycles marked). `Log.trace` is `console.debug`.

**Random** ([random.ts](../js/engine/src/random.ts)). `Random` (`value`, `range`, `rangeInt`, `bool`,
`insideUnitCircle`, `insideUnitSphere`, `onUnitSphere`, `rotation`, `initState`) and a seedable `Rng`
(mulberry32, C#'s `Next` and `NextDouble`, `pick`, `shuffle`). Not for security.

**Input** ([input.ts](../js/engine/src/input.ts)). On top of the keyboard and gamepad state
([input-and-events.md](input-and-events.md#gamepads-and-game-input-m3c-step-6)): `getKey`,
`getKeyDown` and `getKeyUp` (true for one tick; key repeats are not presses), `getAxis("Horizontal" |
"Vertical")` (keys and left stick, the larger wins), `getButton` and `getButtonDown` (`Jump`, `Fire1`,
`Fire2`, `Submit`, `Cancel`; `getButtonDown` is keys only for now). Press sets roll over in an
`onTick` listener, so they follow `Time`.

**Screen** ([screen.ts](../js/engine/src/screen.ts)). `Screen.width`, `height` (UI pixels, what
pointer events use), `scale` and `aspect`, set by `__iron_screen`.

Camera, plane and panel components are in [world-rendering.md](world-rendering.md).

## Limits

- No scene files yet. Scenes are built from script (M5.b adds loading and saving).
- State is not kept on hot reload. `reload_scripts` builds a new `ScriptScene` and
  `WorldViews`, drops all panels and starts components again; `reload` takes a bootstrap closure
  (`spawn_starting_components`) instead of spawning plain components. See
  [scripting.md](scripting.md#hot-reload).
- Only local transforms cross to scripts, one batch per fixed tick.
- Physics bodies are separate from GameObjects. `set_world_from_engine` and the move batch are the
  hook for syncing them.
- `Time.timeScale` doesn't scale physics.

## Tests

`M5-OBJ-01..03` are in [tests/scene.rs](../crates/iron_scene/tests/scene.rs) (hierarchy, world
transforms, moved sets). `M5-OBJ-06..08` are in [tests/scripts.rs](../crates/iron_scene/tests/scripts.rs)
(ops, move batch, lifecycle order, running `tests/fixtures/src/scene.ts` in QuickJS). JS units are
`js/engine/test/{scene,math,time,helpers,camera}.test.ts`. See [testing.md](testing.md).
