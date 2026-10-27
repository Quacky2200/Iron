# light-units

Real-world light units (as in Unity 7 and HDRP). A sunlit stone courtyard on a clear day (sun 100000 lux, sky 9000 nits
in the source) and, through a doorway, a hall lit by three 1500 lumen lamps: roughly ten stops darker, as in life.
The camera walks in from the sun and out again while auto exposure adapts; a readout shows the exposure in use, how
bright the view meters, and the light falling on the camera in lux.

Engine features exercised:

- `project.json` `"lightUnits": "physical"` and `GraphicsSettings.lightUnits` (shown in the menu).
- `Light` intensities in physical units: directional in lux (`SUN_LUX`), point in lumens (`LAMP_LUMENS`).
- `RenderSettings.autoExposure` (`minEV`, `maxEV`, `compensation`, `speedDarkToLight`, `speedLightToDark`, `metering`,
  `lowPercent`, `highPercent`), `RenderSettings.fixedExposure`, `RenderSettings.bloom`, `tonemapping`, and an HDR `skybox`
  (`zenith`, `horizon`, `ground`, `sunDirection`, `sunColor`, `sunSize`) in nits.
- Physical camera: `Camera.physicalExposure`, `aperture`, `shutterSpeed`, `iso`; readouts `Camera.exposure`, `sceneBrightness`.
- `Light.measureIlluminance(position, normal)` (async, returns lux; colliders stop its rays), `BoxCollider`.
- `Material.create({ occlusionMap })` with a 1x1 `Texture.create(..., { linear: true })`; `QualitySettings.shadowResolution`, `shadowCascades`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/light-units
cd .. && cargo run -p platypus_app -- examples/light-units
```

Packaged: `platypus-build package ../examples/light-units`. Screenshot run: `platypus examples/light-units --screenshot out.png --frames 120`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `"lightUnits": "physical"`: the setting that makes light intensities real units. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 206 lines: constants, `apply()`, helpers (`shape`, `slab`, `solid`, `mat`), the `Courtyard` component, the readout `Menu`. |

```json
{ "name": "light-units", "title": "Platypus - Real-world light units", "lightUnits": "physical" }
```

The header comment in the source says the setting lives in `package.json` (`"platypus": { "lightUnits": ... }`), but this
folder has no `package.json`; the setting is in `project.json`, so that comment is out of date.

## How it works

1. **Scale of the numbers.** Everything is in physical magnitudes: the sun is `intensity: 100000`, the ambient colour and
   sky are thousands of nits, the sun disc `sunColor` is 5e7, the lamp bulbs have `emission` of 20000. The tonemapper needs
   exposure to bring that into range.
2. **Exposure.** `apply()` switches between auto exposure and the physical camera's fixed exposure:

   ```ts
   RenderSettings.autoExposure = null;
   RenderSettings.fixedExposure = cameraRef.physicalExposure;
   ```

   and, in auto mode, `autoExposure = { minEV: -2, maxEV: 18, compensation: 0.5, speedDarkToLight: 4, speedLightToDark: 3.5, metering: "centerWeighted", ... }`
   (the comment says both ways take under a second, quicker than eyes).
3. **Building.** `slab()` makes a cube from two corners; `solid()` adds a `BoxCollider` so the light meter's rays stop at
   it. The hall's materials get an `occlusionMap` of 0.03, which cuts their ambient light to a few percent "as baked
   lighting would", so the hall is dark even though the sky is bright.
4. **Walk.** `WALK` is a list of `[time, z]` keyframes; `zAt(t)` smoothsteps between them in a 31 s loop. The camera faces
   -z when going in and +z when coming out.
5. **Lux meter.** Every 0.2 s:

   ```ts
   void Light.measureIlluminance(cam.position, Vector3.up).then((v) => (lux = v));
   ```

   This is an incident meter facing up with shadows counted: indoors the roof keeps the sun off it.
6. **Menu.** Shows `GraphicsSettings.lightUnits`, exposure EV, scene-brightness EV, lux, and the physical camera's
   `f/`, shutter and ISO when `P` is on.

## Controls

| Input | Action |
|---|---|
| `P` | Switch between auto exposure and the physical camera (f/16, 1/200 s, ISO 200 per the header comment) |
| `Space` | Pause/resume the walk |

The panel is display only (no buttons).

## Try / adapt

- Light a scene in lux/lumens from reference values (sun 100000, overcast 10000, 100 W bulb about 1500 lm) and use auto exposure.
- Use `Light.measureIlluminance` as a gameplay light meter (stealth: "am I in the light?").
- Tune the `autoExposure` speeds for a slower, more realistic eye.
- Change the physical camera's `aperture`/`iso` live and watch the EV readout.
- Switch `"lightUnits"` in `project.json` back to the default and see how the same numbers behave.

## Gotchas

- With physical units, every emissive and skybox value must be physical too (thousands of nits), or it is black.
- The hall goes black under the courtyard's physical camera exposure; that is the intended demonstration.
- `Light.measureIlluminance` returns a promise, so the lux value lags a little.
- The overlay panel here has no `pointerEvents: "auto"`, so it does not intercept clicks.

Related examples: tunnel, dark-room, quality, materials
