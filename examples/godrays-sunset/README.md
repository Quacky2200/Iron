# godrays-sunset

A living room on the 25th floor of a tower (floor at y = 80) with floor-to-ceiling west-facing windows over a city of 300 box towers. Over a 70 second cycle the sun sinks from afternoon to the horizon; its beams through the steel mullions and the venetian blinds' slats swing round, stretch across the room and go from gold to deep orange. The sky and haze glow, the towers' windows light up, a lamp comes on at dusk, and the loop restarts.

Engine features used: a directional `Light` re-aimed every frame, `RenderSettings.skybox` (procedural sky rewritten in 700 steps over the cycle), `RenderSettings.volumetricFog`, `autoExposure`, `Texture.create` from a pixel array plus `Material` `mainTexture`, `mainTextureScale` and `emissionMap`, `clearFlags: "skybox"`, bloom, ACES tonemapping.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-sunset
cd .. && cargo run -p platypus_app -- examples/godrays-sunset
```

To package one exe: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-sunset`. Screenshot run (safest; the interactive window can crash some GPUs):

```bash
cargo run -p platypus_app -- examples/godrays-sunset --screenshot out.png --frames 60
```

With the day passing, a screenshot shows only the start of the cycle (afternoon); pressing T in the window holds golden hour, which is the better still.

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (360 lines) | Flat dimensions, `sky(u)`, `apply()`, `Flat` component (flat, blinds, furniture, city, `update(u)`), menu |
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Compiler settings |

No asset files. The city facades are textures made in code.

## How it works

1. **`sky(u)`** is a pure function from cycle position `u` (0..1) to the sun: elevation 38 degrees down to -2 by `u = 0.72`, a small azimuth swing, a colour that warms with `warm = s ** 1.6`, a `strength` that fades in and out, and a `night` factor for the city lights. It returns `dir` (travel direction) and `toSun` (for the skybox).
2. **`Flat.update(u)`** applies it: rotates the sun with `Quaternion.lookRotation(s.dir)`, sets colour and `intensity = 10 * s.strength`, recomputes ambient light, lamp intensity, and city materials. The expensive parts are quantised, because each assignment sends ops to the engine:

```ts
const skyStep = Math.round(u * SKY_STEPS);
if (skyStep !== this.skyStep) {
  this.skyStep = skyStep;
  ...
  RenderSettings.skybox = { zenith: ..., horizon: ..., sunDirection: s.toSun, sunSize: 1.4 };
```

3. **Haze.** `fogDensity = 0.0009` plus a low-intensity volume (`intensity: 0.35, anisotropy: 0.4, steps: 4, distance: 40`). The comment says looking west into the sun lights the haze strongly, so it is kept low to avoid washing out the city while room beams still show.
4. **The flat.** `slab()` positions boxes relative to `FLOOR`. The window wall is a sill, head, 9 mullions every `BAY = 1.25` m and a transom, all casting shadow-map stripes. `buildBlinds` adds about 33 thin cubes rotated `Quaternion.euler(35, 0, 0)` that cover the left side (`BLINDS_TO = -0.6`); a parent `blindsRoot` is toggled with `enabled`.
5. **The city.** `buildCity` creates one 256 x 256 facade texture (a 16 x 16 block of storeys and bays, dark glass in grey concrete) and a second, emissive texture of lit windows chosen in clusters. Each of 7 tower kinds gets 6 materials, one per light level, sharing those textures and differing by `mainTextureScale` and `emission` strength:

```ts
mainTexture: pane,
mainTextureScale: new Vector2(bays / BLOCK, floors / BLOCK),
emissionMap: lit,
emission: new Color(2.6 * e, 2.1 * e, 1.6 * e, 1),
```

   Towers are then swapped between those materials as `night` rises, with a per-tower `shift` so they do not all light up at once.
6. **Camera.** At (3.6, FLOOR + 1.5, 2.6), swaying and slowly panning across the window wall.

## Controls

| Key | Effect |
|---|---|
| T | Day passing vs held golden hour (`GOLDEN = 0.55`) |
| B | Blinds down/up |
| F | Haze (and shafts) on/off |
| A | Auto exposure vs fixed `postExposure` of -3.5 |

## Things to try

- Add a second light bounce (a point light with `volumetricIntensity: 0`) near the floor pool, like godrays-cell does.
- Tie `DAY_LENGTH` to real time, or drive `u` from a slider in the React menu.
- Reuse `sky(u)` and the quantised skybox update for any day/night cycle.
- Use the generated-texture + `emissionMap` trick for cheap lit windows on any large set of buildings.
- Tilt the slats (`Quaternion.euler(35, ...)`) to see how blind angle changes the stripe spacing on the floor.

## Gotchas

- Steps matter: `SKY_STEPS = 700` and `LIGHT_LEVELS = 6` exist because each change is an op sent to the engine; per-frame writes were avoided.
- The skybox, `fogColor` and `volumetricFog` are all reassigned together at each sky step (the volume values are constant; the source does not say why they sit in that block).
- The "Flat" has no real walls outside the window; a facade slab fills the space above and below so the city is not seen through the floor.
- Colours exceed 1 (HDR): sky and sun use values like `80 * s.strength`.

Related examples: godrays-room, godrays-clouds, godrays-cell, light-units, fog-world
