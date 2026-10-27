# godrays-forest

Morning sun slants through a dense canopy of tall trees. The canopy is hundreds of overlapping leaf-clump spheres, so the sun's shadow map is full of holes and long diagonal beams cross a lilac haze and land on a mossy floor with ferns, rocks, logs and flowers. The camera walks slowly round an elliptical path, looking mostly towards the sun, where forward scattering makes the beams brightest.

Engine features used: `RenderSettings.volumetricFog` including `sharpDistance` and `sharpSamples` (per-pixel god rays near the camera), the `VolumetricFogSettings` type, `RenderSettings.heightFog`, `Light.volumetricIntensity`, `ParticleSystem` (additive dust motes with `colorOverLifetime`), shadow cascades, `Quaternion.fromToRotation`/`angleAxis`, `Debug.log`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-forest
cd .. && cargo run -p platypus_app -- examples/godrays-forest
```

To package one exe: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-forest`. Screenshot run (safest; the live window can crash some GPUs):

```bash
cargo run -p platypus_app -- examples/godrays-forest --screenshot out.png --frames 60
```

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (376 lines) | Light and air constants, materials, path maths, tree/fern/bush/rock/log/flower builders, `buildForest`, the `Forest` component, menu |
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Compiler settings |

No assets: every object is a primitive (`cube`, `sphere`, `cylinder`, `capsule`, `plane`) with a flat-colour material.

## How it works

1. **Light and air.** `SUN_DIR = v(0.5, -0.62, 0.6).normalized` (light travel direction, about 38 degrees up). The haze colour `HAZE` doubles as the camera background. The volume settings are a typed constant:

```ts
const SHAFTS: VolumetricFogSettings = {
  intensity: 0.25, anisotropy: 0.45, albedo: hex("#ffe8c6"),
  steps: 4, distance: 60, sharpSamples: 64,
};
```

   `apply()` writes fog colour, exponential fog, `heightFog` (`density: 0.025, falloff: 0.4`) and spreads the constant into `RenderSettings.volumetricFog`, adding `sharpDistance: state.sharp ? 35 : 0`. With `sharpDistance` > 0 the sun's beams within that range are computed per pixel for crisper edges; the volume alone is the default.
2. **The sun** is created with `volumetricIntensity: 1` and `shadows: "hard"`, with `shadowDistance = 55` and 4 cascades. Setting `RenderSettings.fog = false` removes the beams too, because they are lit fog.
3. **Path.** The camera follows `pathAt(t) = (12 sin t, 0, 20 cos t)`. A table of 128 points (`PATH_POINTS`) lets `pathDistance(x, z)` keep trees, rocks and logs off the path.
4. **Procedural wood.** `Random.initState(23)` then `buildForest` places up to 170 trees with `farFromTrunks` rejection sampling. A `tree` is a `rod` (cylinder from point to point, using `Quaternion.fromToRotation(Vector3.up, d)`) for trunk and boughs, plus 6-9 squashed sphere clumps; upper clumps use a lighter leaf material:

```ts
const shade = oy > 0.2 * R ? g + 1 : oy < -0.1 * R ? g - 1 : g;
```

   Then up to 190 ferns, 36 bushes, 30 rocks, 6 logs and 18 flower patches. The count of renderers is logged with `Debug.log`.
5. **Dust.** A `ParticleSystem` with a 16 x 5 x 16 box shape, 500 max particles, `blendMode: "additive"`, `simulationSpace: "world"` and a fade-in/out `colorOverLifetime`. Each frame it is moved to 4 m in front of the camera.
6. **Walk (`place()`).** Advances `t` by `dt * WALK_SPEED` (0.036 rad/s, a lap in about three minutes), adds a small head bob, and aims at `TO_SUN + ahead * 0.55` with a small upward tilt, so the camera always looks partly into the sun.

## Controls

| Key | Effect |
|---|---|
| G | Sharp god rays (`sharpDistance` 35 m) vs soft volume only |
| F | Fog on/off (off removes the beams) |
| Space | Pause the walk |

The same switches appear in the top-left menu.

## Things to try

- Change `sharpSamples` (4..128 per the docs) and `SHARP_DISTANCE` and compare G on/off by eye.
- Make the canopy sparser (fewer clumps `n`) to see beams get wider and more numerous.
- Swap the lilac `HAZE` for a green or blue-grey for a different time of day, or pull `SUN_DIR.y` towards 0 for a low sun.
- Reuse `pathAt`/`pathDistance` for an NPC patrol route or a first-person controller start.
- Replace the leaf spheres with a model, keeping the "many overlapping blobs" idea for shadow holes.

## Gotchas

- All 170 trees are built from separate `MeshRenderer` objects (the source counts them in `meshes`); there is no instancing in this example.
- Leaf clumps have no alpha cutout: the dappled light comes entirely from gaps between spheres.
- `pick` is a local generic helper (`<T,>`) instead of `Random.pick`, indexing with `Random.range`.
- Beams only reach `distance: 60`, so the volume ends a bit past the fog's visible range.

Related examples: godrays-clouds, fog-park, godrays-room, fog-world, fog-raider
