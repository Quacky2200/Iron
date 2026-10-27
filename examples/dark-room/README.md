# dark-room

A test bench for eye adaptation (`RenderSettings.autoExposure`): a bedroom on a bright day behind closed louvred
shutters. Sun comes through the slats in crisp stripes; the shutters swing open (blinding, then the eyes settle) and close
again (near black, slowly recovering), on a 40 second loop while the camera looks around. Outside is a garden on its own layer.
A two-page menu exposes almost every exposure setting live, with a graph of the last 15 seconds of the view's metered
brightness against the exposure in use.

Engine features exercised:

- `RenderSettings.autoExposure` (`minEV`, `maxEV`, `compensation`, `speedDarkToLight`, `speedLightToDark`, `metering`
  `"centerWeighted" | "average" | "spot"`, `lowPercent`, `highPercent`, `darkAdaptation`, `darkBelow`, `curve`, `hold`),
  `fixedExposure`, `exposureTransition`, `RenderSettings.freezeExposure()`.
- `Camera.main`, `Camera.exposure`, `Camera.sceneBrightness` (EV100), read for the graph.
- `RenderSettings.volumetricFog` with `resolution: "ultra"`, `denoising`, `historyWeight`, `samples`; `fog`, `bloom`, `tonemapping`,
  `reflectionIntensity`, HDR `skybox`.
- Layers with a light's `cullingMask` (the sky fill light reaches only layer 9, the garden); `Light.volumetricIntensity`,
  point/spot lights with `innerSpotAngle`; emissive materials swapped at run time.
- `QualitySettings.setQualityLevel`, `names`, `vSyncCount`; React UI with an inline `<svg>` `polyline` graph.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/dark-room
cd .. && cargo run -p iron_app -- examples/dark-room
```

Packaged: `iron-build package ../examples/dark-room`. Screenshot run: `iron examples/dark-room --screenshot out.png --frames 300`
(the interactive window has crashed some GPUs; the screenshot run is safest). The shutters open 10 s into the cycle,
close at 25 s.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. The garden layer number (9) is a constant in code, not a named layer in `project.json`. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 767 lines: constants/tables, `State`, `apply()`, `freeze()`, helpers, the `Bedroom` component (room, shutters, furniture, garden, lights, per-frame update), key handlers, menu pages, `Graph`, `MeteringZone`. |

No assets.

## How it works

1. **Single `State` object.** Menu and keys change `state`; `apply()` recomputes `RenderSettings.autoExposure`:

   ```ts
   metering: METERING[state.metering],
   lowPercent: state.band ? 40 : 0,
   highPercent: state.band ? 90 : 100,
   darkAdaptation: state.soft ? 0.6 : 1,
   ```

   When `auto` is off, `fixedExposure` is the manual EV (or `frozenAt`) and `exposureTransition` (1 s, 3 s or 0) blends to it.
2. **Freeze.** `freeze()` calls `RenderSettings.freezeExposure()`, then stores `RenderSettings.fixedExposure` in `state.frozenAt`, so
   exposure switches to manual with nothing changing on screen.
3. **The room.** `buildRoom` cuts the -z wall around two windows; `buildShutters` makes two hinged leaves per window with
   slats tilted `Quaternion.euler(50, 0, 0)`, 5 cm apart; `setShutters(k)` swings every hinge's `localRotation` by up to
   `OPEN_ANGLE` (105 degrees). Open/close timing is two eased steps: `ease((t - OPEN_AT) / SWING) - ease((t - CLOSE_AT) / SWING)`.
4. **Two light worlds.** The room gets a very low ambient (0.035) and a sun of intensity 40; the garden is on layer 9 and gets an extra
   directional sky light whose `cullingMask = 1 << OUTSIDE` limits it to that layer. `reflectionIntensity = 0.02` keeps the bright sky
   from showing in the room's surfaces.
5. **Interior lights.** A bedside lamp (point light plus emissive shade) and a flickering television (random emissive screen
   materials and a spot light) give metering something to react to; `updateLights` also dims the sun with moving "clouds".
6. **Look-around.** `LOOKS` keyframes give a yaw over time, eased by `yawAt(t)`, so the camera faces the windows when
   the shutters open and close.
7. **Graph and readouts.** Ten samples a second of `Camera.main.exposure` and `sceneBrightness` go into `history` (15 s);
   `Graph` builds `polyline` points. `MeteringZone` draws the spot/centre-weighted area over the view.

## Controls

| Key | Action |
|---|---|
| `A` | Auto exposure on / off (blends over the transition) |
| `Z` | Freeze exposure where it is |
| `T` | Transition time: 1 s, 3 s, instant |
| `-` / `=` | Manual EV a stop darker / brighter |
| `L` | Hold the exposure |
| `Y` | Adaptation speed: game-fast or eye-slow |
| `M` / `H` | Metering mode / histogram band |
| `S` / `C` | Soft floor / compensation curve |
| `G` | Haze quality preset |
| `K`, `B`, `V` | Clouds, bedside lamp, television |
| `Q`, `U` | Next quality level, vsync |
| `1`, `2` | Menu page (exposure, scene) |
| `R`, `Space` | Restart cycle, pause |

## Try / adapt

- Take `apply()` as a ready-made exposure settings panel for your game.
- Hook `Camera.sceneBrightness` to gameplay (stealth: darkness meter).
- Reuse the shutter rig for blinds or doors: hinge GameObject plus `localRotation`.
- Put the garden-on-a-layer trick (fill light with a `cullingMask`) into any scene where inside and outside need separate lighting.
- Replace the scripted loop with player input.

## Gotchas

- `curve: undefined` is passed when the curve option is off (the object literal omits the value).
- Quality changes call `applyHaze()` again, because the comment notes Low turns the haze off.
- The panel root box has `pointerEvents: "none"` while the inner panel is `"auto"`.
- The `Graph` reads `Camera.main`; samples are skipped until the exposure is a finite number.
- The garden's layer is a bare number, not a name; compare layers, which names them in `project.json`.

Related examples: tunnel, light-units, layers, godrays-room, fog-raider
