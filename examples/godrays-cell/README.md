# godrays-cell

A medieval stone prison cell lit only by sunlight through a small barred window high in the back wall. The sun is a single shadow-casting directional light; its shadow map is striped by the bars, and the engine's froxel light volume turns the dusty air into visible beams. About 260 dust motes drift through the cell and flare up only where the sun reaches them.

Engine features used (all in `src/main.tsx`): `RenderSettings.volumetricFog` (with `resolution: "ultra" | "high"`), `RenderSettings.fog` / `fogMode` / `fogColor` / `fogDensity`, `RenderSettings.autoExposure` (with `compensation`), `RenderSettings.bloom`, `RenderSettings.tonemapping`, `QualitySettings.shadow*`, `Light` (directional with hard shadows, point with `volumetricIntensity: 0`), `Camera`, `Material.create` (including `unlit` and `emission`), `MeshRenderer.shadowCastingMode`, `Random.initState`, and a React overlay menu via `createRoot()`.

## Build and run

From the repo root:

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-cell
cd .. && cargo run -p platypus_app -- examples/godrays-cell
```

A packaged single exe is also possible: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-cell`. For a screenshot instead of a window (the safest option, since the interactive window can crash some GPUs):

```bash
cargo run -p platypus_app -- examples/godrays-cell --screenshot out.png --frames 60
```

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (556 lines) | Everything: constants, the `Cell` component, helpers, the menu |
| `project.json` | Only `name` and `title` ("Platypus - God rays: the cell"); no quality overrides, no scenes |
| `tsconfig.json` | Compiler settings |

No assets: all geometry is primitives (`cube`, `cylinder`, `capsule`, `sphere`) and colours; there are no textures or models.

## How it works

1. **Geometry constants.** The cell is x -3..3, z -4..4, floor y 0, ceiling y 4, walls 0.5 thick. The window (`WIN`), bar positions (`BAR_XS`, `BAR_YS`) and door (`DOOR`) are plain constants, because the dust code reuses them to decide whether a point is sunlit.
2. **Sun direction.** `SUN_DIR = new Vector3(0.25, -0.55, -0.8).normalized` is the direction the light travels (in through the window, down and towards the camera).
3. **`apply()`** pushes the menu state into the engine. The volume is configured here:

```ts
RenderSettings.volumetricFog = {
  intensity: 0.8,
  anisotropy: 0.5,
  steps: 8,
  distance: 20,
  albedo: tint.albedo,
  resolution: state.ultra ? "ultra" : "high",
};
```

   `anisotropy` 0.5 makes the fog scatter forward, so the beams are brightest when the camera looks towards the window. `resolution` is the volume's detail (finer = sharper beams, dearer fill).
4. **`Cell.onStart()`** sets quality (`shadowDistance = 16`, 4096 shadow map, 2 cascades), a near-black ambient, ACES tonemapping and bloom with `threshold: 1`, then auto exposure with a very low compensation so the cell stays dim:

```ts
RenderSettings.autoExposure = { compensation: CELL_EXPOSURE, speedDarkToLight: 1, speedLightToDark: 0.6 };
```

   It creates the `Sun` (`type: "directional"`, `intensity: 6`, `shadows: "hard"`), a `Bounce` point light with `volumetricIntensity: 0` (it lights the floor but stays out of the fog so the beams are the sun's alone), builds the room, then the camera with `backgroundColor` set to a very bright value (the sky seen through the window burns white).
5. **Building the room.** `buildShell` makes thick slabs; the back wall is four slabs around the window hole, so the sun can only enter there. `stoneWall` lays rows of random-sized stone blocks over the inside faces, skipping `holes` for the window and door. `buildWindow` adds `cylinder` bars (`BAR_R = 0.028`) that cast the stripes. `buildFloor`, `buildCeiling` (a drain grate over a recess), `buildDoor`, `buildProps` (pallet, bucket, chains built from capsule `link`s and `ring`s) fill in the rest. Everything is seeded with `Random.initState(1347)` so the cell is identical each run.
6. **Dust.** `inSun(x, y, z)` casts a ray from a mote back towards the sun and checks it passes through the window opening (front and back of the wall) and misses the bars. Each frame a mote drifts, and only when `inSun` is true is it enabled and given an emissive, unlit material:

```ts
m.go.enabled = sunny;
if (sunny) m.go.getComponent(MeshRenderer)!.material = lit[m.kind]![m.level]!;
```

   Motes in shadow are hidden rather than drawn dark, because a dark speck against a lit beam looked wrong. Lit materials are pre-built per tint, kind and brightness level, so no materials are created per frame.
7. **Camera.** Set every frame in `onUpdate` from sines of time: a slow sway of position and look target, `Quaternion.lookRotation(target.sub(cam.position))`.
8. **Tint.** `applyTint` swaps the sun colour, the bounce colour, the camera background and the lit mote materials between a warm and a teal set.

## Controls

| Key | Menu button | Effect |
|---|---|---|
| G | Beam detail | Volume resolution: Ultra / High |
| R | Rays | `RenderSettings.fog` on/off (no fog = no beams) |
| D | Dust | Motes on/off |
| T | Sunlight | Warm / teal |

The menu is a React `Menu` component in the top left; its panel uses `pointerEvents: "auto"` while the full-screen wrapper uses `"none"`, so clicks outside the panel are not captured.

## Things to try

- Change `steps` or `anisotropy` in `apply()` and compare with the Ultra/High switch.
- Move the window (`WIN`) or add a bar to `BAR_XS`; the dust test `inSun` follows the same constants, so mote lighting stays consistent with the shadows.
- Replace the barred window with a grate of many thin bars to stress the volume (the engine has a golden test for this case).
- Reuse `inSun` plus an emissive-mote pool in any scene with a fixed shaft of light; it is cheap (260 spheres, no per-frame allocation of materials).
- Raise the sun `intensity` and `CELL_EXPOSURE` together to see how auto exposure and bloom interact with a bright window.

## Gotchas

- Motes are toggled with `go.enabled` and swapped materials; shadow casting is disabled per mote (`shadowCastingMode = "off"`) so they do not shadow the beams.
- The dust sunlight test is a hand-written analytic check, not a query of the shadow map; it ignores props.
- `refreshMenu` is a module-level function that the `Menu` component overwrites on render, the pattern used to drive React state from keyboard handlers in the compiled subset.
- Colours like `sky: new Color(19, 17.4, 14.4, 1)` exceed 1 on purpose: the renderer is HDR.

Related examples: godrays-room, godrays-shed, godrays-lift, dark-room, fog
