# scenes

Shows how a game is split into JSON scene files and how code moves between them. The game starts in
`scenes/level1.scene.json` (the Courtyard). A React menu loads the Night garden (replacing everything), adds or
removes a bonus room (additively), copies a spinner object with `GameObject.instantiate`, and saves the active
scene to the log. A `Keeper` object survives every load and counts them. The comment in the source says hot
reload keeps all of it.

Engine features used: `SceneManager` (`loadSceneAsync`, `unloadScene`, `getActiveScene`, `getSceneAt`,
`sceneCount`, `onSceneLoaded`, `onSceneUnloaded`), `Scene` (`getRootGameObjects`, `serialize`, `rootCount`),
`GameObject.dontDestroyOnLoad`, `GameObject.instantiate`, `GameObject.findObjectOfType`, the `@property`
decorator, `register(..., { spawn: false })`, `onReload`, `Log.info`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/scenes
```

Then, from the repo root:

```bash
cargo run -p iron_app -- examples/scenes
```

Package with `iron-build package ../examples/scenes`. A screenshot run is
`iron examples/scenes --screenshot out.png --frames N`; it is the safest check, since the window has crashed
some GPUs.

## Structure

| File | What |
|---|---|
| `src/main.tsx` | `Spinner`, `Bobber`, `Keeper` components, scene helpers, and the `Menu` UI |
| `scenes/level1.scene.json` | "Courtyard": camera, sun, objects (642 lines) |
| `scenes/level2.scene.json` | "Night garden": camera, a directional light, 6 point lights, 7 mesh renderers, 6 `Bobber`s |
| `scenes/bonus.scene.json` | "Bonus": a platform, trophy and spot, loaded additively |
| `project.json` | `name: scenes`, `title: Iron - Scenes`, `startScene: scenes/level1.scene.json` |

Scene files have `"format": "iron.scene"`, a `version`, a `name`, optional `renderSettings` (level1 sets a skybox
sun direction and `ambientMode`) and `objects`, each with `id`, `name`, `position`, `rotation` (quaternion),
`scale` and `components` (a `type` plus `properties`). There are no assets: shapes are primitive meshes.

## How it works

**Start.** `project.json`'s `startScene` names the scene the engine loads first (the exact order against startup
components is not checked here). The `Menu` is a plain
screen UI (`createRoot().render(<Menu />)`).

**Scene-only components.** `Spinner` and `Bobber` are registered so scene files can name them, but not spawned
by themselves:

```ts
register({ Spinner, Bobber }, { spawn: false });
register({ Keeper });
```

`@property` marks fields that are saved to and loaded from the scene file (`speed`, `axis`, `height`).

**Keeper.** In `onAwake` it calls `GameObject.dontDestroyOnLoad(this.gameObject)` and registers
`SceneManager.onSceneLoaded` / `onSceneUnloaded` callbacks. After a hot reload `onReload` re-attaches the
listeners, because fields survive but callbacks do not.

**Loading.** `loadLevel` calls `await SceneManager.loadSceneAsync(path)` (single mode: other scenes go); the bonus
room passes `"additive"` and keeps the returned `Scene` so it can be unloaded:

```ts
bonus = await SceneManager.loadSceneAsync("scenes/bonus.scene.json", "additive");
```

**Copy and save.** `copySpinner` picks a random root object that has a `Spinner`, instantiates a copy at a
random offset and reverses and doubles its speed (`speed *= -2`). `save` calls
`SceneManager.getActiveScene().serialize()` and logs the JSON, which is the format of the files in `scenes/`.

**Menu refresh.** The menu keeps a module-level `refresh` function that bumps a state counter; the scene
callbacks and buttons call it. It lists each scene with its root count and shows the `Keeper.loads` number.
The outer box has `pointerEvents: "none"` and the panel inside `"auto"`, so the world stays clickable.

## Controls

Mouse only, on the menu: Courtyard, Night garden, Add/Remove bonus room, Copy a spinner, Save to log.

## Things to try

- Make a third level by copying a scene file, then add a button that loads it.
- Add a door object that calls `SceneManager.loadSceneAsync` when the player touches it.
- Put persistent state (score, settings) in a `dontDestroyOnLoad` component like `Keeper`.
- Save a scene, edit objects at run time, and write the result into a new file.
- Use additive scenes for streaming parts of a large world.

## Gotchas

- Components named in a scene file must be registered or the load cannot create them; `spawn: false` keeps them
  from being added by the startup logic.
- `@property` is the only decorator supported by the compiled subset and takes no options.
- Single loads destroy everything not marked `dontDestroyOnLoad`; `bonus` is reset to `null` in `loadLevel`
  because loading single drops an additive scene.
- Callbacks are not kept across hot reload (see `onReload`).
- Quaternions in scene files are stored as x, y, z, w arrays.

Related examples: world, platformer, arcade, animation
