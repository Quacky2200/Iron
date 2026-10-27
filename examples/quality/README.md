# quality

The same scene at Low, Medium, High and Ultra. Thin things expose anti-aliasing (a picket fence, a turning fan of
12 thin blades, four catenary cables), their shadows expose shadow quality, and Low's render scale softens everything
but the UI. The menu switches levels or single settings; until it is used, the levels cycle every two seconds.

Engine features exercised:

- `QualitySettings`: `setQualityLevel`, `getQualityLevel`, `names`, `onLevelChanged(cb)` (returns an unsubscribe function),
  `antiAliasing`, `renderScale`, `shadows` (`"disable" | "hardOnly" | "all"`), `softShadowQuality` (`"low" | "medium" | "high"`),
  `shadowResolution`.
- `project.json` `quality.default` to pick the starting level.
- `LineRenderer` (`positions`, `width`, `material`), `Material.create`, `Time.time`, `transform.rotate`.
- React `useState`/`useEffect` in a menu that subscribes to engine events.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/quality
cd .. && cargo run -p platypus_app -- examples/quality
```

Packaged: `platypus-build package ../examples/quality`. Screenshot run: `platypus_app examples/quality --screenshot out.png --frames 30`
(the interactive window has crashed some GPUs; the screenshot run is safest). Screenshots after the first two
seconds of frames come from the cycled levels, so compare by level with the menu, not by frame count.

## Structure

| File | What |
|---|---|
| `project.json` | `"quality": { "default": "High" }`. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 186 lines: `Fan`, `Demo` (auto cycling), `QualityDemo` (scene), `useQuality`, `Button`, `Menu`. |

No assets. Only the `default` level is configured in `project.json`; the level names (Low, Medium, High, Ultra) come
from the engine (`QualitySettings.names`).

## How it works

1. **Scene.** `QualityDemo.onStart` builds a floor, a 25-picket fence with two rails (cubes via `box()`), a hub with 12
   blades rotated by `Quaternion.euler(0, 0, i * 15)` and spun by `Fan`, two posts with four `LineRenderer` cables, a chrome
   and a clay sphere, under a procedural skybox and a soft-shadowed sun.
2. **Auto cycling.** A `Demo` component advances the level every 2 s until `demo` is turned off by any button:

   ```ts
   QualitySettings.setQualityLevel((QualitySettings.getQualityLevel() + 1) % QualitySettings.names.length);
   ```

3. **Menu stays in sync.** `useQuality()` subscribes to `QualitySettings.onLevelChanged` in `useEffect` and returns a
   `refresh` function for changes the engine does not announce (single settings):

   ```ts
   useEffect(() => QualitySettings.onLevelChanged(() => setTick((t) => t + 1)), []);
   ```

4. **Single settings.** Each row cycles a value with a generic `next(values, current)` helper: anti-aliasing 1/2/4/8,
   render scale 50/75/100/150%, shadows, soft shadow quality, shadow map size 512 to 4096. Changing one does not change the
   level name shown, only the values.

## Controls

Mouse only: the four level buttons, and rows for Anti-aliasing, Render scale, Shadows, Soft shadows, Shadow map. The first
click anywhere in the menu stops the 2 s auto cycling. No key bindings.

## Try / adapt

- Build a settings screen for your game from this menu: write `QualitySettings.getQualityLevel()` into `PlayerPrefs` and restore it.
- Define your own levels in `project.json` (the source shows only `default`; see `docs/` for the full schema).
- Add a frame-time readout (see fog-world's `RenderStats` and `QualitySettings.vSyncCount`).
- Use `renderScale` as a dynamic resolution knob: lower it when frame time rises.
- Keep a thin-geometry test scene like this around for regression screenshots.

## Gotchas

- `onLevelChanged` fires on level changes only; after setting one property you must re-render yourself (`refresh()`).
- `next()` and the `as const` array are needed so the type of `QualitySettings.shadows` is kept by the compiled-TS typing.
- Render scale affects the 3D view; the UI is drawn at native resolution.
- `softShadowName` and `nextSoftShadows` take a `string` and return literal types, a workaround so the union type checks.

Related examples: layers, fog-raider (uses renderScale and antiAliasing), light-units, materials
