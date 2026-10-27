# tunnel

Auto exposure demonstrated with a road tunnel through a hill. The camera drives in from a bright summer day: the tunnel
mouth looks black at first and only slowly opens up as the "eyes" adapt to the sodium lamps (slow from dark to light).
Deep inside, the exit ahead is a white blaze glowing through the haze; driving out into the sun dazzles for a moment,
then the landscape settles into view. Then the drive starts over.

Engine features exercised:

- `RenderSettings.autoExposure` (`minEV`, `maxEV`, `compensation`, `speedDarkToLight`, `speedLightToDark`), switched to
  `null` plus `RenderSettings.postExposure` for the fixed comparison.
- HDR `RenderSettings.skybox` (`zenith`, `horizon`, `ground`, `sunDirection`, `sunColor`, `sunSize`), `tonemapping = "aces"`,
  `RenderSettings.bloom` (`threshold`, `intensity`, `scatter`), changed every frame while driving.
- Fog and light shafts: `fogMode = "exponential"`, `fogColor`, `fogDensity`, `RenderSettings.fog`, and
  `volumetricFog` (`intensity`, `anisotropy`, `steps`, `distance`, `albedo`).
- `Material.create({ occlusionMap })` with a 1x1 linear `Texture` as "baked" ambient occlusion; `mainTexture` + `mainTextureScale`
  for tiling tarmac; `ReflectionProbe` with `boxProjection: true`; point `Light`s; `QualitySettings.shadowCascades`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/tunnel
cd .. && cargo run -p platypus_app -- examples/tunnel
```

Packaged: `platypus-build package ../examples/tunnel`. Screenshot run: `platypus examples/tunnel --screenshot out.png --frames 200`
(the interactive window has crashed some GPUs; the screenshot run is safest). The car starts 45 m before the entrance at
11 m/s, so a longer `--frames` count shows the inside.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only (no `lightUnits` setting, so light values are not physical units). |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 307 lines: constants, `apply()`, helpers (`shape`, `slab`, `shut`, `tarmac`), the `Drive` component (road, tunnel, hill, country, bloom, driving), `Menu`. |

No assets; the tarmac texture is generated.

## How it works

1. **Exposure.** `apply()` picks auto exposure or a fixed daylight exposure of -5 stops. The comments explain the values:
   slow to adapt to daylight on the way out (a few seconds of glare), quicker to the dark going in.

   ```ts
   { minEV: -4, maxEV: 16, compensation: -0.7, speedDarkToLight: 0.7, speedLightToDark: 1.3 }
   ```

2. **Scene brightness.** The sun `Light` has `intensity: 40`, ambient is only `(0.9, 1.0, 1.2)`, and the skybox and sun disc
   are tens to hundreds of times above 1. The tunnel's air, walls and road use `occlusionMap` textures made with `shut(share)`
   so the sky's light barely gets in:

   ```ts
   const shut = (share: number) => Texture.create(1, 1, [new Color(share, share, share, 1)], { linear: true });
   ```

3. **Geometry.** `slab()` builds boxes from two corners: road in three stretches (so the middle can be damp and shut off),
   walls with tiled lower halves, ceiling, portal frames, lamp housings with emissive 8x sodium colour and a point light every
   14 m. The hill is rock slabs plus overlapping spheres, with boulders by the portals; fields, hedges and 140 trees are the country.
4. **Haze.** Fog colour is nearly black; the volumetric fog glows only where light reaches it (sun shafts outside, lamps inside).
5. **Bloom while driving.** `setBloom(k)` rounds `k` to 1/100 steps and only updates `RenderSettings.bloom` when the step changes:
   full bloom until 70 m from the exit, fading to a trace within 12 m, so the exit does not swell into an orb.
6. **Driving.** `onUpdate` advances `z` by `SPEED * dt`, loops from `END_Z` to `START_Z`, and sways the camera slightly while
   looking down the road with `Quaternion.lookRotation`.

## Controls

| Input | Action |
|---|---|
| `A` | Auto exposure on / off (off: fixed exposure for daylight, tunnel near black) |
| `F` | Haze (`RenderSettings.fog`) on / off |
| `Space` | Pause / resume the drive |

The menu buttons do the same.

## Try / adapt

- Use the `autoExposure` speeds and `minEV`/`maxEV` as a "how long does the player stay blind" gameplay knob.
- Reuse the `shut()` occlusion trick for any interior in a bright outdoor scene.
- Drive the camera from input (a vehicle controller) instead of the loop.
- Tie `setBloom` to distance from any bright portal.
- Try the physical units setup of light-units for the same scene with real lux and lumens.

## Gotchas

- With auto exposure off, the fixed `postExposure = -5` is chosen for daylight: the tunnel is near black on purpose.
- Light units here are artistic (sun 40, lamps 20), not physical; light-units shows the physical variant.
- Coplanar surfaces are avoided by hand (portals stand 0.9 m proud, the hill is 0.6 m out) to prevent flicker.
- Volumetric fog settings are applied once in `onStart`; `F` toggles the whole fog, not just the shafts.

Related examples: dark-room, light-units, fog, godrays-lift, godrays-room
