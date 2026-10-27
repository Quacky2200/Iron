# arcade

A small complete game: steer a ball around a walled arena to collect coins (1 point) and gems (5) before a
45-second round ends. Pickups in quick succession build a combo multiplier (up to x5). If you leave the
controls alone, the ball plays itself. The HUD (score, best, combo, timer, "+N" popups, a settings menu and a
round-over panel) is React; the game is components. It is the largest example (719 lines) and shows how
game code, UI, scenes, saved settings and hot reload fit together.

APIs used: `Component` with `@property`, `GameObject.findObjectOfType`, `GameObject.dontDestroyOnLoad`,
`SceneManager` (`createScene`, `setActiveScene`, `unloadScene`, `getSceneAt`), `input.getAxis` /
`getButtonDown`, `Time.timeScale`, `PlayerPrefs`, `Application.targetFrameRate`, `Screen`, `QualitySettings`,
`RenderSettings`, `ParticleSystem`, `Mirror`, `Texture.create`, `Random`/`Rng`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/arcade
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p platypus_app -- examples/arcade
```

Screenshot run: `cargo run -p platypus_app -- examples/arcade --screenshot out.png --frames 60`. The
interactive window may crash some GPUs; prefer the screenshot run. Package with
`platypus-build package ../examples/arcade`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only (the quality levels used by the Settings menu come from the engine defaults) |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | Game (`GameManager`, `Player`, `Pickup`, `Arcade`), arenas (`buildClassic`, `buildDeluxe`), procedural textures, HUD |

No asset files: the metal textures are generated in code.

## How it works

1. **Entry.** `register({ Arcade })` is spawned at start. `Arcade.onStart` restores the saved frame cap and
   style from `PlayerPrefs`, creates a `Game` object with `GameManager`, and marks it `GameObject.dontDestroyOnLoad`
   so the round survives scene changes. The other components are registered with `{ spawn: false }`.
2. **Round state.** `GameManager` holds score, combo, time left and `paused`; the combo window is 1 s, and
   points are `value * combo`. Esc is the `Cancel` button; pausing sets `Time.timeScale = 0`.
3. **Player.** Steering is `input.getAxis("Horizontal")` / `("Vertical")` (keys and stick). After 1.5 s idle,
   attract mode steers to the nearest pickup. Velocity is eased with `lerp`, position clamped to the arena and the
   ball is rolled with `Quaternion.angleAxis`.
4. **Pickups.** `Pickup` spins, optionally bobs, checks the distance to the ball (< 0.9) and, on collection, calls
   `game.collect(value)` and jumps to `randomSpot(height)`. There is no physics engine involved.
5. **Scenes as looks.** `switchStyle(style)` saves ball and pickup positions, creates a new scene
   (`SceneManager.createScene`), makes it active, builds the arena into it, restores positions, and unloads the
   old arena scenes:

```ts
const scene = SceneManager.createScene(`${style === "deluxe" ? "Deluxe" : "Classic"} ${++arenaCount}`);
SceneManager.setActiveScene(scene);
buildArena(style);
if (saved) restorePositions(scene, saved);
for (const s of old) SceneManager.unloadScene(s);
```

6. **Classic vs Deluxe.** Classic: default skybox, a plain floor, four cube walls. Deluxe: a black sky with a
   fixed ambient (`ambientMode = "color"`), bloom, a brushed-metal floor texture made by `brushedMetal()`, a nearly
   transparent `Mirror` plane just above the floor for reflections, riveted-steel beams from `rivetedMetal()`
   (a 256x256 `Texture.create` filled pixel by pixel) and a `ParticleSystem` of additive bubbles with colours above
   1 so bloom makes them glow.
7. **HUD.** The game calls `notify()` on changes; `useGame()` subscribes a React state setter to a listener set and
   returns a snapshot:

```tsx
useEffect(() => {
  const update = () => setView(read());
  listeners.add(update);
  update();
  return () => { listeners.delete(update); };
}, []);
```

   `THEMES` gives each style its panel look (the Deluxe one avoids `backdropFilter` for cost, per the
   source comment). Overlays use `pointerEvents: "none"` on the full-screen root and `"auto"` on buttons.
8. **Settings.** Window mode (`Screen.fullScreenMode`), resolution (`Screen.setResolution`), frame cap
   (`Application.targetFrameRate`, saved with `PlayerPrefs.setInt`) and quality level
   (`QualitySettings.setQualityLevel`). Window changes apply after the frame, so `refresh()` re-renders 100 ms later.
9. **Hot reload.** `onReload` calls `notify()`, and the classes are registered so `bun run dev`-style reloads
   can restore them with state (see CLAUDE.md "Hot reload"); I did not run this.

## Controls

| Input | Effect |
|---|---|
| WASD / arrow keys / left stick | Steer the ball (the "Horizontal" and "Vertical" axes) |
| Esc | Pause and open Settings (or close it) |
| Style button (top right) | Switch Classic / Deluxe |
| Settings (Esc) button | Open settings: style, window mode, resolution, frame cap, quality |
| Play again | After the round ends, restarts it |

## Things to try

- Change `roundLength`, `comboWindow` (both `@property`) or the `ARENA` constant.
- Add an obstacle type with a third `Pickup` value that subtracts points.
- Add sound with `AudioSource` on collect (see `docs/physics-and-audio.md`).
- Replace attract mode with a second player on the arrow keys.
- Save `best` with `PlayerPrefs.setInt` so it survives restarts.

## Gotchas

- Game and UI talk through module-level state and a listener set (`notify`), not through props: components
  are engine objects, and the React tree reads them with `GameManager.current`.
- `pickups()` uses `GameObject.roots.flatMap(...)`, so it finds only components on root objects (the coin's
  mesh is a child "Disc"; the `Pickup` is on the root).
- Textures and materials are made at runtime; for big textures the per-pixel loop in `Texture.create` is the
  startup cost.
- Quality names come from the engine's levels (`QualitySettings.names`), not from this example.

Related examples: platformer, scenes, quality, physics
