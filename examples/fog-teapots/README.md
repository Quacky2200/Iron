# fog-teapots

The classic distance-fog picture: a row of eight shiny orange Utah teapots recedes into grey fog over a glossy
black-and-white checkerboard that reflects them, seen from low down. The far pots fade until the last are only
ghosts. A small menu (or keys) cycles the fog mode and moves the linear fog's end.

Engine features exercised:

- `RenderSettings`: `fog`, `fogMode` (`"linear" | "exponential" | "exponentialSquared"`), `fogColor`, `fogDensity`,
  `fogStartDistance`, `fogEndDistance`, `ambientMode`, `ambientLight`.
- `Model.load("assets/teapot.glb")`, `model.instantiate({ position, rotation })`, `model.whenLoaded()`.
- `Mirror` component on the floor (planar reflection), `Material.create` with `mainTexture`, `mainTextureScale`,
  `metallic`, `smoothness`.
- `Texture.create(n, n, pixels, { filterMode: "trilinear", anisoLevel: 16 })`.
- Directional `Light` with `shadows: "soft"`; `Camera` with `backgroundColor` and `farClipPlane`; React menu.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-teapots
cd .. && cargo run -p platypus_app -- examples/fog-teapots
```

Packaged: `platypus-build package ../examples/fog-teapots`. Screenshot run: `platypus_app examples/fog-teapots --screenshot out.png --frames 60`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 151 lines: fog `state`/`apply`, the `Teapots` component, `checker()`, `collect()`, `Menu`. |
| `assets/teapot.glb` | The Utah teapot (Newell teaset, 1975), converted to glTF by `examples/classics/tools/make_models.py` (per the header comment). |

## How it works

1. **Fog.** `apply()` sets grey fog (`GREY`) with start 5 m and an end the user moves (default 27, clamped 7 to 80).
   The same grey is the camera's `backgroundColor`, so the far pots dissolve into the background. `"off"` is not a
   `FogMode`, so it only turns `RenderSettings.fog` off.

   ```ts
   RenderSettings.fogStartDistance = FOG_START;
   RenderSettings.fogEndDistance = state.end;
   if (state.mode !== "off") RenderSettings.fogMode = state.mode;
   RenderSettings.fog = state.mode !== "off";
   ```

2. **Floor.** A `plane` scaled by 12 (the plane is 10 m, so 120 m), checker texture repeated 60 times with
   `mainTextureScale`, `metallic: 0.4`, `smoothness: 0.92`, plus a `Mirror` component for the reflections.
3. **Texture.** The checker is 128x128, 2x2 big squares, with trilinear filtering and 16x anisotropy: the comment says
   point sampling shimmers in the distance.
4. **Teapots.** One `Model` is instantiated eight times along `ROW` (a normalised diagonal) `SPACING = 3.2` m apart.
   A model loads asynchronously, so the shared orange material is assigned in `whenLoaded().then(...)` over every
   `MeshRenderer` found by the recursive helper `collect()`:

   ```ts
   model.whenLoaded().then(() => {
     for (const go of pots) for (const r of collect(go)) r.material = orange;
   });
   ```

5. **Camera.** `onUpdate` sways the camera very slowly (sines of `t`) at 0.6 m height and aims it along the row with
   `Quaternion.lookRotation`.

## Controls

| Input | Action |
|---|---|
| `F` or the "Fog" button | Cycle linear, exponential, exponential squared, off |
| `Up` / `Down` | Linear end +/- 2 m |
| "Linear end" button | End +2 m |

## Try / adapt

- Replace `teapot.glb` with your own model and keep the `whenLoaded()` + `collect()` pattern to retint it.
- Add a second `Mirror`, or change the floor's `smoothness` to see what the reflection needs to look good.
- Animate `FOG_START` too, or tie fog density to the time of day.
- Make the row a ring and spin the camera for a gallery scene.
- `fogDensity` only matters for the two exponential modes; try values between 0.02 and 0.15.

## Gotchas

- The material must be assigned after the model loads (`whenLoaded`), as instance renderers do not exist before then
  (the example's approach; I did not check engine internals).
- `fogDensity` is set but ignored in linear mode; start/end are ignored in exponential modes.
- The "Linear end" row is greyed when the mode is not linear but still reacts to clicks.

Related examples: fog, fog-park, classics, materials, godrays-room
