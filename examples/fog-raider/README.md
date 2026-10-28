# fog-raider

A blocky tomb in the style of the first Tomb Raider games on the PS1. A corridor of 1 m sandstone blocks opens into a
hall with pillars, a pool and steps up to a dais, lit by six flickering orange torches. Black linear fog sits very
close to the camera, and the render scale is low with anti-aliasing off for the chunky look. A small explorer walks
a loop on her own and the camera follows.

Engine features exercised:

- `RenderSettings.fog`, `fogMode = "linear"`, `fogStartDistance`, `fogEndDistance`, `fogColor`; ambient colour.
- `QualitySettings.renderScale` and `QualitySettings.antiAliasing` for the low-res look.
- `Texture.create(16, 16, pixels, { filterMode: "point" })`, `Material.create` (`mainTexture`, `emission`, `unlit`).
- Point `Light`s whose `intensity` is changed every frame (flicker); GameObject hierarchies and `localRotation`.
- `Quaternion.lookRotation`, `Quaternion.euler`, `Vector3.lerp`, `input.getKeyDown`, React HUD.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-raider
cd .. && cargo run -p platypus_app -- examples/fog-raider
```

Packaged: `platypus-build package ../examples/fog-raider`. Screenshot run: `platypus-app examples/fog-raider --screenshot out.png --frames 60`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 326 lines: fog `state`/`apply`, texture and palette helpers, `buildTomb`, `PATH`, the `Tomb` component (walker, camera, flicker), `Hud`. |

No assets: all geometry is scaled cubes, spheres and capsules.

## How it works

1. **Fog as a style.** `apply()` sets linear fog with start = 20% of end and quality for the chunky look:

   ```ts
   RenderSettings.fogStartDistance = state.end * START_RATIO;
   RenderSettings.fogEndDistance = state.end;
   QualitySettings.renderScale = state.lowRes ? LOW_RES : 1;
   QualitySettings.antiAliasing = state.lowRes ? 1 : 4;
   ```

2. **Textures and materials.** A 16x16 grey speckle texture (point filtered) is tinted per block by `palette()`, which
   makes six slightly mismatched materials per base colour; `pick()` chooses one at random (seeded with
   `Random.initState(1996)`).
3. **`buildTomb()`** lays out the corridor (z 0 to 16), the hall (z 16 to 30), pool, pillars, steps and dais from
   one `block()` helper. A comment records a bug fix: building the corridor's last metre twice made coplanar faces
   that flickered.
4. **Torches.** Each is a bracket block, an unlit emissive flame block and a point light with `range: 7`; the lights are
   collected in `flickers`.
5. **The walker** (`buildWalker`) is a GameObject tree of primitives: torso, head, pack, and `Hip`/`Shoulder`
   pivots whose `localRotation` is swung with `Math.sin(this.stride)`. She faces -z, the engine's forward.
6. **`onUpdate`** steers toward the next `PATH` waypoint with limited turn rate, slows on sharp turns, follows the floor
   height from `groundAt()`, and moves the camera behind her with exponential smoothing, clamped by `keepInside()` so it never
   leaves the rooms:

   ```ts
   cam.position = cam.position.lerp(wanted, 1 - Math.exp(-dt * 3));
   cam.rotation = Quaternion.lookRotation(next.add(new Vector3(0, 1.1, 0)).sub(cam.position));
   ```

7. **Flicker:** `l.intensity = 5 + sin(t*9 + i*1.7)*0.5 + sin(t*23 + i)*0.35`.

## Controls

| Input | Action |
|---|---|
| `Up` / `Down` (or `+` / `-`, numpad) | Fog end +/- 1 m, clamped 3 to 40 (start follows at 20%) |
| `P` or click the row | Toggle the low-res look (0.4 render scale, no AA) |
| `F` or click the row | Fog on/off |

## Try / adapt

- Replace the scripted `PATH` walk with input-driven movement and a character controller.
- Make the fog end shorter in dark areas (a horror feel) or tie it to a gameplay value.
- Raise `LOW_RES` toward 0.6 for a softer retro look.
- Use the `palette()` + `pick()` trick for any modular blocky level.
- Add more `PATH` waypoints or a second room.

## Gotchas

- The explorer is not physics-driven; the floor height comes from a hand-written `groundAt()`.
- Coplanar faces flicker (z-fighting): the source works around it by not building overlapping blocks.
- The HUD box uses `pointerEvents: "none"` on the outer box and `"auto"` on the panel so clicks reach only the panel.

Related examples: fog, fog-park, fog-world, dark-room, tunnel
