# godrays-shed

A dark garden shed at night, lit only by faint moonlight through the gaps between planks. Outside, an alien craft hovers by the window and sweeps a scanning beam (green or red) through the shed: a narrow spot cone that slices into blades at the plank gaps, crawls over a workbench, lawnmower and tools, and lights the dust it crosses. It can also scan the whole shed at once with a thin, wide pyramid-shaped spot light. A second menu page exposes most of the auto-exposure options.

Engine features used: `Light` `type: "spot"` with `spotShape: "pyramid"`, `aspectRatio`, `spotAngle`, `innerSpotAngle` and `volumetricIntensity`; the volume's quality settings (`resolution: "ultra"`, `denoising`, `historyWeight`, `samples`, `motionSamples`); a wide set of `autoExposure` fields (`metering`, `lowPercent`/`highPercent`, `darkAdaptation`, `darkBelow`, `curve`, `hold`) plus `RenderSettings.exposureTransition` and `fixedExposure`; `Camera.main?.exposure`; `QualitySettings.vSyncCount`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/godrays-shed
cd .. && cargo run -p platypus_app -- examples/godrays-shed
```

To package one exe: `cd tools && cargo run -p platypus_build -- package ../examples/godrays-shed`. Screenshot run (safest; the interactive window can crash some GPUs):

```bash
cargo run -p platypus_app -- examples/godrays-shed --screenshot out.png --frames 90
```

## Structure

| File | What it holds |
|---|---|
| `src/main.tsx` (662 lines) | Shed constants, option tables (`HAZE`, `SPEEDS`, `SCANS`), `apply()`, `inScan`, `scanAt`, the `Shed` component, key handlers, two-page menu |
| `project.json` | `name` and `title` ("Platypus - God rays: the alien scan") only |
| `tsconfig.json` | Compiler settings |

No asset files; geometry is boxes, cylinders and spheres.

## How it works

1. **Shed.** `buildShed` makes vertical boards of width `BOARD = 0.14` with `GAP = 0.012` between them, on all four walls, with the window cut out of the -z wall, a pitched roof of two rotated slabs and a frame with a cross. The gaps are literal: the moon and scanner shine through them.
2. **Moon and fog.** A faint blue directional `Moon` (`intensity: 0.3`), black dense fog (`fogDensity = 0.12`) and bloom (`threshold: 0.9`).
3. **The scanner** is a spot light under the craft, aimed through the window each frame:

```ts
this.scanner = eye.addComponent(Light, { type: "spot", spotAngle: 9, innerSpotAngle: 5, range: 14, intensity: 60, shadows: "hard" });
```

   `apply()` switches it to a full-shed sheet: `spotShape = "pyramid"`, `aspectRatio = 40`, `spotAngle = 1.5`, `innerSpotAngle = 1`. `scanAt(t)` returns a target and a `lock` value: a partial scan visits four stops (workbench, lawnmower, tools, shelves) holding 40 percent of each 6 s period (pulsing intensity on lock), a full scan eases there and back over `FULL_SCAN = 24` s. For the left-to-right sheet the light's up vector is `Vector3.right`, so its wide axis stands vertical.
4. **Volume tuning.** Because the plank blades are thin, noise is the hard case, so the haze quality is a menu choice:

```ts
resolution: "ultra", denoising: h.denoising, historyWeight: h.historyWeight,
samples: h.samples, motionSamples: h.motionSamples,
```

   `HAZE` lists five presets from "Smoothest (8 samples, 0.98)" to "None (steps)". `scanner.volumetricIntensity` can also be cut (`BEAM_HAZE`: 100/50/25/10 percent).
5. **Dust.** 260 tiny spheres under `dust`. `inScan(p, scan)` checks the point is in the cone (or the sheet's frustum) and that the line to the scanner passes through the window (between its bars) or a board gap (`(x + W) % (BOARD + GAP) > BOARD`). Lit motes get a material in the beam's colour, others a dim one. `setBeam()` rebuilds those materials, the craft's light ring materials and the scanner/glow colours when C is pressed.
6. **Craft.** A flattened sphere, a dome and 12 sphere lights whose materials chase round with `(i - t * 8) % 12`.
7. **Exposure page.** `holdExposure()` reads `Camera.main?.exposure`, rounds to 0.5 and turns auto off at that EV, so changing the rays' strength is not undone by adaptation. `stepEV` then adjusts `RenderSettings.fixedExposure`.
8. **Menu.** Two React pages (`ScanPage`, `ExposurePage`) behind `Tab`s; a live FPS/ms readout is measured over 0.5 s windows in `onUpdate`.

## Controls

| Key | Effect |
|---|---|
| 1 / 2 | Scan page / Exposure page |
| M | Scan mode (partial, full top to bottom, full left to right) |
| C | Beam colour (green / red) |
| R | Rays (fog) on/off |
| D | Dust on/off |
| G | Haze quality preset |
| V | Ray strength (`volumetricIntensity`) |
| Space | Pause |
| Q | VSync on/off |
| X | Auto / manual exposure |
| - and = | Manual EV down/up by 0.5 |
| N | Metering mode |
| H | Histogram band (40-90 percent) |
| F | Soft floor (dark adaptation) |
| K | Exposure curve |
| Y | Adaptation speed |
| L | Hold exposure |
| T | Transition time |

## Things to try

- Add a new stop to `scanAt`'s `stops` list for another prop and watch the beam lock on.
- Add a third beam colour to `BEAMS`; `setBeam` handles everything else.
- Use the pyramid sheet as a template for laser grids, searchlights or a security scanner.
- Save the best HAZE preset into a quality setting for your game and compare cost with `fps`.
- Drive `lock` from gameplay (e.g. the player moving) to make the scanner react.

## Gotchas

- Changing intensity under auto exposure is cancelled out within a second, hence `holdExposure()`.
- The pyramid light's `spotAngle` is 1.5 degrees with an `aspectRatio` of 40 (about 55 degrees wide), per the `SHEET` comment.
- `inScan` knows only the window, bars and boards; the dust ignores props.
- The `Stepper` component and `Tab` are local UI helpers; the menu is rebuilt by a `useState` tick (`refreshMenu`).

Related examples: godrays-lift, godrays-cell, godrays-room, dark-room, light-units
