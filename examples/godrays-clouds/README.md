# godrays-clouds

Late afternoon on a hillside looking towards the sun over a patchwork of fields. The sun sits behind a broken layer of cumulus built from flattened spheres; the clouds cast shadows, and the light volume turns the gaps into crepuscular rays fanning down across the haze. The cloud layer sways slowly so the beams shift and open and close.

Engine features used: `RenderSettings.skybox` (procedural sky with `sunDirection`, `sunColor`, `sunSize`), `RenderSettings.volumetricFog` at kilometre scale, `RenderSettings.fog` / `fogDensity` / `heightFog`, `RenderSettings.autoExposure` and `postExposure`, `RenderSettings.bloom` with a high threshold, 4 shadow cascades over 2600 m, a directional `Light`, and a `Camera` with `clearFlags: "skybox"`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-clouds
cd .. && cargo run -p platypus_app -- examples/godrays-clouds
```

Or package a single exe with `cd tools && cargo run -p platypus_build -- package ../examples/godrays-clouds`. A screenshot run is safest, since the interactive window can crash some GPUs:

```bash
cargo run -p platypus_app -- examples/godrays-clouds --screenshot out.png --frames 60
```

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (218 lines) | Constants, `apply()`, the `Clouds` component (land and cloud builders), React menu |
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Compiler settings |

No assets: fields are flat cubes, trees and hills are spheres, clouds are spheres.

## How it works

1. **Sun and gap.** `SUN = new Vector3(-0.224, -0.5, 0.837).normalized` is the direction the light travels. The camera looks along -z, so the sun is ahead, slightly right, 30 degrees up. `SUN_GAP` (293, 0, -766) is where the camera-to-sun line crosses the cloud layer at `CLOUD_BASE = 700`; clouds are not placed within `GAP_RADIUS` of it, so the sun disc peers through.
2. **Sky.** The skybox is HDR values, and the sun colour is huge so that only the disc is far above white:

```ts
sunDirection: SUN.scale(-1),
sunColor: new Color(900, 830, 720, 1),
sunSize: 0.9,
```

   Note `sunDirection` points towards the sun, the opposite of the light's travel direction. Bloom `threshold: 4` means only the disc and cloud rims glow.
3. **Haze.** The god rays need something to scatter in. Three layers are combined: distance fog (`fogDensity = 0.00018`), height fog (`{ density: 0.0006, baseHeight: 0, falloff: 0.006 }`, thicker low down) and the light volume:

```ts
RenderSettings.volumetricFog = { intensity: 0.9, anisotropy: 0.55, steps: 4, distance: 2400, albedo: new Color(1, 0.96, 0.9, 1) };
```

   `fogColor` is the sky's light colour so distance fades to pale blue, not grey. `distance: 2400` stretches the volume over the plain.
4. **Land.** `buildLand` loops over a 25 x 29 grid of `CELL = 150` m fields, each a flat cube rotated by `Quaternion.euler(0, 12, 0)` (via a local `place` helper) with 80 percent-likely hedges on two sides, trees, 40 copses and hills of stretched spheres. `Random.initState(17)` fixes the layout.
5. **Clouds.** `buildClouds` makes 60 clusters of 5-11 flattened spheres under one parent `Cloud layer`. The material has both a colour and an `emission`, because scattering inside a cloud keeps its underside grey, not black:

```ts
const white = Material.create({ color: new Color(0.95, 0.95, 0.97, 1), smoothness: 0, emission: new Color(1.1, 1.15, 1.3, 1) });
```

   The cast shadows are what the rays run between.
6. **Animation (`onUpdate`).** The layer position is a Lissajous-ish sway (`SWAY = 220` m, `SWAY_TIME = 50` s) and the camera only turns a little with sines of `t`. Pausing stops `t`.
7. **Exposure.** `apply()` either turns on auto exposure (`{ minEV: 0, maxEV: 16, compensation: 0.9, ... }`) or sets it to `null` and uses a fixed `postExposure = -4.5`, so the two modes can be compared.

## Controls

| Key | Effect |
|---|---|
| F | Haze (and so the beams) on/off: `RenderSettings.fog` |
| A | Auto exposure vs fixed `postExposure` |
| Space | Pause the cloud drift |

The same actions are buttons in the top-left React menu.

## Things to try

- Move `SUN` lower to lengthen the shadows and lower the fan of rays; update `SUN_GAP` so the hole stays in line with the disc.
- Raise `volumetricFog.distance` or lower `intensity` and see how far the beams stretch before the fog colour takes over.
- Add a third cloud layer at a different height with its own sway speed.
- Use the `sunColor`/`bloom.threshold` pairing for any scene where a sun disc must glare without blooming the whole frame.
- Replace the sphere hills with a terrain model and keep the same fog set-up.

## Gotchas

- Scale matters: shadow distance 2600 with 4 cascades and a 6000 m far plane; the volume only covers `distance` metres from the camera.
- The land is 725 field cubes plus hedges, trees and copses, and the clouds add a few hundred spheres (60 clusters of 5-11 puffs, minus those skipped near the gap), all built at start-up; the source uses no instancing or LOD.
- Grid rotation is done by hand (`grid.rotate(...)`) because only the field cubes carry their own rotation.
- Beams are not tied to a cloud: they come from the sun's shadow map, so the cloud spheres must actually be in the shadow cascades.

Related examples: godrays-sunset, godrays-forest, fog-park, fog-world, world
