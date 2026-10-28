# fog

A misty clearing at dusk. A low sun shines through a slatted pergola and between ~90 trees, a lamp post's spot light
makes a cone in the mist, height fog lies in the valley and the distance fog swallows the horizon. The camera drifts
slowly; a small React menu (or keys) switches the fog modes and the light-shaft quality at run time.

Engine features exercised (all real names from `src/main.tsx`):

- `RenderSettings`: `fog`, `fogMode` (`"exponentialSquared" | "exponential" | "linear"`), `fogColor`, `fogDensity`,
  `fogStartDistance` / `fogEndDistance` (linear mode), `heightFog` (`{ density, baseHeight, falloff }` or `null`),
  `volumetricFog` (`{ intensity, anisotropy, steps, distance }` or `null`), `ambientMode`, `ambientLight`, `tonemapping`.
- `Light` with `type: "directional"` / `"spot"`, `shadows: "hard"`, and the per-light `volumetricIntensity` (the cone in the mist).
- `Material.create` (colour, smoothness, metallic, `emission`, `unlit`), `MeshRenderer` (`mesh` primitives, `shadowCastingMode`).
- `QualitySettings.shadowDistance`, `Random.initState` / `Random.range`, `input.getKeyDown`, React UI via `createRoot`.

## Build and run

From the repo root:

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog
cd .. && cargo run -p platypus_app -- examples/fog
```

Or make one executable with `platypus-build package ../examples/fog` (see `docs/platypus-build.md`). To check it without
opening the interactive window, render a screenshot: `platypus-app examples/fog --screenshot out.png --frames 60`.
The interactive window has crashed some GPUs; the screenshot run is the safest check.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and window `title` only. No quality, layer or light-unit settings. |
| `tsconfig.json` | Compiler options for `tsc` checking. |
| `src/main.tsx` | Everything (162 lines): the `Clearing` component, the fog `state` and `apply()`, key handling, the `Menu` UI. |

No assets: every mesh is a primitive and every material is made in code.

## How it works

1. **State and `apply()`.** One plain `state` object holds what the menu shows; `apply()` pushes it to `RenderSettings`
   and calls `refreshMenu()` so the UI re-renders.

   ```ts
   const state = { mode: MODES[0]!, height: true, shafts: true, steps: STEPS[0]! };
   let refreshMenu = () => {};
   ```

2. **Fog.** Height fog and light shafts ride on the fog, so with the distance fog "off" the example keeps
   `RenderSettings.fog` on and sets the density to 0.

   ```ts
   RenderSettings.heightFog = state.height ? { density: 0.05, baseHeight: 0, falloff: 0.35 } : null;
   RenderSettings.volumetricFog = state.shafts ? { intensity: 0.2, anisotropy: 0.3, steps: state.steps, distance: 45 } : null;
   RenderSettings.fog = state.mode !== "off" || state.height || state.shafts;
   ```

3. **`Clearing.onStart`** builds the scene: ambient colour mode with tonemapping `"aces"`, a warm directional sun with
   hard shadows, a ground plane, the pergola (cylinders, beams and 12 slats), 90 trees placed with seeded `Random`
   (skipping a corridor in front of the camera), the lamp post, and the `Camera` with `backgroundColor` set to the
   mist colour so the sky matches the fog.
4. **The lamp.** The globe is `unlit` with an `emission` above 1 and `shadowCastingMode = "off"`, or it would shadow
   its own light. The light is a separate GameObject pointing down:

   ```ts
   lamp.addComponent(Light, { type: "spot", color: new Color(1, 0.8, 0.55, 1), intensity: 80, range: 10, spotAngle: 50, shadows: "hard", volumetricIntensity: 3 });
   ```

5. **`onUpdate`** reads the keys, then moves the camera on slow sines and aims it at the pergola with
   `Quaternion.lookRotation`.
6. **Menu.** A function component with a `useState` tick; `refreshMenu` is reassigned each render so `apply()` can
   force a redraw. The outer box has `pointerEvents: "none"` so the 3D view stays unobstructed, the panel inside is
   `"auto"`.

## Controls

| Input | Action |
|---|---|
| `F` or the "Distance fog" button | Cycle exponential squared, exponential, linear (4 to 70 m), off |
| `H` | Height fog on/off |
| `L` | Light shafts (volumetric fog) on/off |
| `Q` | Shaft samples per slice: 4, 8, 1 |

## Try / adapt

- Raise `volumetricFog.steps` and `distance` for quality, lower them for speed; compare the three `STEPS`.
- Put `volumetricIntensity` on more lights (torches, windows) to get cones in mist anywhere.
- Switch `tonemapping` between `"aces"` and `"none"` (fog-blocks uses `"none"`) and watch the mist colour change.
- Replace the pergola with your own geometry: slats make the most visible shafts because the sun is low.
- Copy the `state` + `apply()` + `refreshMenu` pattern for any settings panel.

## Gotchas

- Fog colour and camera `backgroundColor` must match, or the horizon shows a seam. The example uses one constant (`MIST`).
- The mode "off" is not a `FogMode`; the example models it as `FogMode | "off"` and zeroes the density.
- `refreshMenu` is a module-level variable replaced on each render of `Menu`: a workaround for there being no store;
  fine for one menu instance.

Related examples: fog-blocks, fog-park, fog-teapots, godrays-forest, light-units
