# Scene, components and scene files in Rust: design note

Status: **decided by the plan, built in steps; step 1 (members, lookups, removal, lifecycle hooks) done, `instantiate` and the scenes loader next** (2026-10-19). The user asked for `scene.ts` and the component model, then
`scenes.ts`. Most of the object model is native already (`iron_scene`, `NativeHost`, `getComponent`/`addComponent`, `@property`
save and load, the animation components). This note says what is left, how `scenes.ts` fits when most engine components are not
ported yet, and the order.

## 1. What is native today

| Part | Where |
|---|---|
| Objects, hierarchy, names, layers, enabled/active, local and world transforms (floats) | `iron_scene::Scene` |
| `GameObject`/`Transform` API: create, destroy, name, enabled, layer, set parent, local/world position/rotation, local scale | `iron_scene::api::scene_api` |
| Compiled components: lifecycle (`onAwake`, `onStart`, `onUpdate`, `onLateUpdate`, `onFixedUpdate`, `onPreRender`, `onDestroy`), `getComponent`/`addComponent` (exact class), `@property` get/set/save/load | `iron_ts_rt::NativeHost` |
| Engine components with their own registries | `Camera`, `Animation`, `Animator` (by object) |

## 2. What `scene.ts` has that is not native

1. **Transform**: `eulerAngles`, `localEulerAngles`, `lossyScale`, `forward/right/up`, `translate`, `rotate`, `lookAt`,
   `transformPoint`, `inverseTransformPoint`, `transformDirection`. Pure functions of the transform: Rust, in `scene_api`, using
   the `iron_math` types. The scene stores floats, so results differ from the TypeScript's doubles in the last digits; the tests
   compare rounded values.
2. **GameObject**: `children`, `findChild`, `root`, `destroyed`, `hideFlags`, `id`, `byId`, `roots`, `GameObject.find`,
   `getComponents`, `getComponentInChildren`, `removeComponent`, `findObjectOfType`, `instantiate`, `dontDestroyOnLoad`,
   `createPrimitive` (needs rendering and physics components: later). Lists come as a count and an indexer (`childCount`,
   `childAt(i)`) until arrays cross the boundary; for compiled component classes `getComponents`/`findObjectOfType` are compiler
   intrinsics that build the array from the runtime registry.
3. **Component base**: `transform`, `destroy()`, `getComponent`, and the hooks `onApplicationFocus(bool)`, `onApplicationPause(bool)`,
   `onReload()`.
4. **`getComponent(Base)` finds subclasses** (`instanceof`): the class table carries each class's ancestors.
5. **Destroy**: destroying an object removes its components from every registry (compiled ones with `onDestroy`, engine ones).
   One function does it (`iron_engine_api::objects::destroyed(&[ObjectId])`) and the host calls it with what `Scene::destroy`
   returned.
6. **`instantiate`**: copies an object with its children and components; references inside the copy point at the copies. Compiled
   components are copied through their reflection table (save, spawn, load, then remap references); engine components through a
   `clone` hook they register.

## 3. Scene files (`scenes.ts`)

The file format is `SceneData` (`iron.scene` version 1: objects with transform, children, components with `@property` values,
render settings, shared assets); `examples/scenes` has real files, so Rust reads and writes the same JSON.

`scenes.ts` is coupled to every engine component (rendering, physics, audio, particles, cloth, models, textures), and most are not
native yet. So the loader is generic:

- A **component kind** registers with `iron_scenes::register_kind(name, KindFns { create(object) -> id, save(id) -> JSON,
  load(id, JSON, ctx), clone(id, object, ctx), destroy(id) })`. `Camera`, `Animation` and `Animator` register now; every
  later port registers its components the same way and scene files with them start to load. An unknown type in a file is a warning
  and the component is skipped, as the TypeScript does for unregistered names.
- **Compiled components** are one kind per registered class, through `NativeHost` (create by name, `@property` list, set and get
  as `PropValue`). Values of type object (`GameObject`, components) are saved as `{ "$ref": id }` and resolved after the whole
  file is built, as in the original; `Vector3`, `Color` and the other value classes are saved as arrays or objects exactly as the
  original does.
- **SceneManager** state (scenes, the active one, `dontDestroyOnLoad`, load modes `single`/`additive`, events) lives in Rust; the
  script API takes scene data as text (`SceneManager.loadScene(json)`); `loadSceneAsync(path)` becomes a host request (the host
  reads the file and calls back), since file access belongs to the app.
- **Prefabs** are scene data for one object; `Prefab.instantiate` is `instantiate` over loaded data.
- **Hot-reload state** (`__iron_save_state` / restore) is the same save and load with `reload` mode, done after the loader.

## 4. Order

1. Transform members, GameObject members, Component base (`transform`, `destroy`), subclass-aware `getComponent`,
   `getComponents` and friends for compiled classes, destroy aggregation. Bun-compared (rounded) against `scene.ts`.
2. `instantiate` for objects with compiled and animation components.
3. `iron_scenes`: the generic loader/saver and `SceneManager`, with the three kinds above, tested against the real example
   scene files and the original saver's output.
4. Hot-reload state, `dontDestroyOnLoad`, async load requests.

## 5. Differences to expect

Positions are floats in the scene; lists are count and indexer (compiled classes get real arrays through intrinsics); `getComponent`
takes a class (compiled or engine) and engine classes arrive as they are ported; `loadSceneAsync` is a host request; scenes with
components of a kind not registered yet load with those components skipped (and a warning).
