# godrays-lift

A dark hotel corridor at night with a green exit sign and, at its end, a lift. Lamps over the doors count the floors down, the doors slide open and a wide spot light from the car floods the dusty corridor with a hard-edged block of light, dazzling the (simulated) eye. The doors close, the corridor goes dark and the cycle repeats.

Engine features used: a `Light` of `type: "spot"` (`spotAngle: 120`, `innerSpotAngle: 90`) whose `intensity` is animated, `Light.volumetricIntensity`, `RenderSettings.volumetricFog` with light shafts from a local light, `RenderSettings.autoExposure` with slow adaptation to light, `postExposure` fallback, bloom, and emissive `unlit` materials.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-lift
cd .. && cargo run -p platypus_app -- examples/godrays-lift
```

To package one exe: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-lift`. A screenshot run (safest; the interactive window can crash some GPUs) needs enough frames to reach the open phase, since the doors open about 4 s in:

```bash
cargo run -p platypus_app -- examples/godrays-lift --screenshot out.png --frames 360
```

(The frame count assumes roughly 60 frames a second; the source does not say how a screenshot run advances time, so adjust if the doors are still shut.)

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (227 lines) | Constants, `apply()`, the `Corridor` component (corridor, lift, per-frame cycle), menu |
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Compiler settings |

No assets; the corridor is boxes (a `slab` helper takes two corners) and spheres.

## How it works

1. **Cycle timing.** Constants set the loop: `ARRIVE = 4`, `OPENING = 1.8`, `OPEN = 6`, `CLOSING = 1.8`, `DARK = 4`, summed as `CYCLE`. Time `t` wraps with `(this.t + dt) % CYCLE`.
2. **Air.** Black exponential fog and a light volume with modest settings; no albedo is given, so the beams take the light's own colour:

```ts
RenderSettings.fogColor = new Color(0, 0, 0, 1);
RenderSettings.fogDensity = 0.05;
RenderSettings.volumetricFog = { intensity: 0.6, anisotropy: 0.45, steps: 4, distance: 20 };
```

3. **Corridor.** `buildCorridor` makes the carpet, walls, skirting, dado rail and room doors (every 3.2 m on both sides), and the exit sign, an emissive `unlit` slab plus a small green point light with `volumetricIntensity: 0.4`.
4. **The lift.** `buildLift` builds the end wall with a doorway, a brass surround, a steel car behind it, a bright ceiling panel (emission 18) and two things that light the corridor: a `Car light` spot (starts at `intensity: 0`, `shadows: "hard"`) and a `Car fill` point light with `volumetricIntensity: 0` (no shafts, no shadows). Two door leaves overlap the doorway so no light gets past when shut; the spot's shadow from the leaves cuts the beam's edges as they part.
5. **Doors and light each frame.** `open` goes 0 to 1 and back using a smoothstep `ease`:

```ts
const open = ease((t - ARRIVE) / OPENING) - ease((t - ARRIVE - OPENING - OPEN) / CLOSING);
this.flood.intensity = FLOOD * Math.sqrt(open);
this.fill.intensity = 3 * open;
```

   The square root makes a sliver of gap already spill a visible blade of light. The light is off entirely while shut because, per the comment, its shadow through thin steel would leak.
6. **Floor lamps.** Five small spheres swap between two materials (`lampOn`/`lampOff`), counting down during `ARRIVE`; the last stays lit while the car is here. Swapping happens only when `lit` changes.
7. **Exposure.** Eyes used to the dark adapt slowly back down, so the flood dazzles:

```ts
RenderSettings.autoExposure = state.auto ? { minEV: -6, maxEV: 12, compensation: -2, speedDarkToLight: 0.9, speedLightToDark: 0.4 } : null;
```

   With auto exposure off, `postExposure` is fixed at 2.
8. **Camera.** Stands still near the corridor end, aimed back at the lift, with a small sine "breathing" motion.

## Controls

| Key | Effect |
|---|---|
| F | Dust in the air: `RenderSettings.fog` on/off (no fog, no beam) |
| A | Auto exposure vs fixed `postExposure` |
| Space | Pause the lift cycle |

## Things to try

- Make `OPEN` shorter and let the player trigger the cycle with a key instead of a timer.
- Replace the spot with a `pyramid` shaped spot (`spotShape`, `aspectRatio`; see godrays-shed) for a rectangular doorway-shaped beam.
- Put the corridor on a level with a second light source and fade between them with the same `ease` helper.
- Use the exposure settings (`speedDarkToLight` vs `speedLightToDark`) to build a "flash of light, eyes adjust" effect in any horror or sci-fi scene.
- Add a silhouette (a cube figure) in the doorway to see its shadow in the beam.

## Gotchas

- A light you intend to switch on and off is handled as intensity 0, not `enabled`; the comments say the spot would otherwise shadow-leak while the doors are shut.
- The fill point light disables its volume contribution (`volumetricIntensity: 0`) so only the spot creates shafts.
- Shadow quality matters here (4096 shadow map, `shadowDistance = 30`), because the door leaves and wall edges are thin.
- All timing runs from `dt` in `onUpdate`; there is no timer API used.

Related examples: godrays-shed, godrays-cell, dark-room, light-units, tunnel
