# godrays-room

A derelict hall in late afternoon. Sun pours through a tall window with a cross mullion, the louvres of a half-shuttered window and a broken patch of the roof, drawing beams that land as bright shapes on floorboards, a table, a dust-sheeted armchair and crates. Five hundred dust motes drift and glow in the sunlight. Optionally a whole day passes: the sun swings from high in the south-west to a low orange sunset in the north-west.

Engine features used: a directional `Light` re-aimed per frame, `RenderSettings.volumetricFog` with `sharpDistance` / `sharpSamples` (per-pixel god rays near the camera), `autoExposure` with `compensation`, bloom, ACES, hard shadow cascades, `MeshRenderer.shadowCastingMode`, emissive `unlit` materials, `Float32Array` / `Int8Array` for per-mote state.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-room
cd .. && cargo run -p platypus_app -- examples/godrays-room
```

To package one exe: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-room`. Screenshot run (safest; the interactive window can crash some GPUs):

```bash
cargo run -p platypus_app -- examples/godrays-room --screenshot out.png --frames 60
```

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (601 lines) | Room constants, `daylight(u)`, `sunlit`, `pointInBeam`, the `Hall` component (room, furniture, ivy, dust, sun, camera), menu |
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Compiler settings |

No asset files; everything is primitives with colour materials.

## How it works

1. **Settings** (`apply()`): exponential fog, then the volume, with the sharp mode as an optional extra:

```ts
const GOD_RAYS = { intensity: 1.2, anisotropy: 0.5, steps: 4, distance: 30, albedo: new Color(1, 0.86, 0.68, 1), sharpSamples: 96 };
RenderSettings.volumetricFog = { ...GOD_RAYS, sharpDistance: state.sharp ? SHARP_DISTANCE : 0 };
```

   `SHARP_DISTANCE = 22` m: inside that range the sun's beams are computed per pixel, so the window edges and louvre stripes are crisp; beyond it the froxel volume takes over.
2. **Openings.** The west wall (x = -6) is cut into slabs around a tall window `WIN` and a shuttered one `SHUT`. The roof is a grid of planks (`PLANKS_X * PLANKS_Z` cells); a `missing[]` array marks cells removed in the `HOLE` rectangle, near its edges at random, and a few elsewhere. Rafters stay and cast stripes.
3. **Planks overlap.** Each roof plank is enlarged by `LAP = 0.02` and alternate ones raised 6 mm, because exactly abutting planks let shadow-map light leak through the seams and coplanar faces z-fight (stated in the source's comment).
4. **Sun path.** `daylight(u)` returns `dir`, `color` and `strength` for `u` in 0..1 (elevation 58 to 3 degrees, azimuth -35 to 40). `updateSun()` applies it to the light and also dims and tints `camera.backgroundColor`, the "overexposed outdoors" seen through the openings. With `state.day` false it holds `LATE_SUN`.
5. **Dust.** 500 sphere motes under a `Dust` parent with arrays for position, velocity, age, life, size, kind and level. `sunlit(x, y, z)` traces back towards the sun to the west wall (checking window, mullion, transom, louvres) and to the roof (checking missing cells and rafters), ignoring furniture. Motes spawn near beams 65 percent of the time via `pointInBeam()`. A mote is enabled only when lit, with a material precomputed per day step, kind and brightness:

```ts
this.motes[i]!.enabled = lit === 1;
if (lit) this.renderers[i]!.material = this.bright[this.step]![this.kind[i]!]![this.level[i]!]!;
```

   When the day step changes, `this.lit.fill(-1)` forces every mote to re-pick its material.
6. **Update order.** `onUpdate` checks keys, calls `updateSun()`, `updateDust(Math.min(dt, 0.05))` (capped for long frames), then moves the camera along the east wall with sines.
7. **Exposure.** `autoExposure = { compensation: ROOM_EXPOSURE (-2.2), ... }` keeps the hall dim while windows burn white; bloom `intensity: 0.85` spills glare over frames.

## Controls

| Key | Effect |
|---|---|
| T | Day passing vs fixed late afternoon |
| G | Sharp god rays (22 m) vs soft volume only |
| F | Fog (and so beams) on/off |
| D | Dust on/off (`dustRoot.enabled`) |

## Things to try

- Move the `HOLE` or widen `WIN` and watch both the beams and the dust test follow (the dust test reads the same constants).
- Lower `DAY_LENGTH` for a time-lapse, or remove the `strength` fade to see the sun swing under the horizon.
- Replace the shared `GOD_RAYS` constant with per-quality values using `QualitySettings`.
- Make a collapsed-building scene with this plank-grid trick for a destructible roof.
- Add real furniture occlusion to `sunlit` (a ray test) to stop motes glowing in shadow.

## Gotchas

- The dust "is it in the sun" test is analytic and ignores furniture, so a mote under the table can glow.
- `Math.random()` is used in `updateDust` for the wander, while the rest uses seeded `Random` (`Random.initState(7)`); dust motion is therefore not reproducible between runs.
- Typed arrays (`Float32Array`, `Int8Array`) hold per-mote state; indexed access needs `!` non-null assertions in the source.
- Beams cost more with `sharpDistance` on; the source leaves it off by default.

Related examples: godrays-cell, godrays-shed, godrays-sunset, godrays-forest, dark-room
