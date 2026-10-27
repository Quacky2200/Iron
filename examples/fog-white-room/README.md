# fog-white-room

The Animus's loading room from Assassin's Creed: a white mannequin with pale blue glowing joints on an endless floor of
white tiles under a grey, cloudy sky, everything past a clear bubble a few tiles wide melting into white fog. The floor
follows the player like a treadmill, so there is no wall and no edge. Tiles light up under footsteps and here and
there on their own, pale fragments drift up out of the floor, and a deck of grey cloud drifts overhead.

Engine features exercised:

- `RenderSettings.cloudLayer` (HDRP's cloud layer, drawn with the sky behind the fog: `altitude`, `size`, `coverage`,
  `opacity`, `softness`, `tint`, `windSpeed`, `windDirection`, `fadeDistance`), linear `fog`, `heightFog`, `skybox`
  (no sun), `bloom`, `tonemapping`, `postExposure`, `ambientMode: "color"`.
- `Model.load(..).whenLoaded()`, `model.instantiate({ parent, position })`, `model.materials`, `model.animations`
  (the glTF's clips, their rotation keys made sign-continuous), the `Animation` a copy gets (destroyed here).
- `Animator` with an `AnimatorController`: a 1D blend tree by speed (idle, walk, jog, sprint, each with a
  `timeScale` matching its stride), one-shot states (jump, air, land, roll), triggers, bools, exit times and offsets.
  Blend trees keep their normalized time, so a walk breaking into a run carries on mid-stride.
- `SkinnedMeshRenderer` materials swapped, `CharacterController.move`, a `BoxCollider` floor that is moved only when
  the player crosses a block.
- `Texture.create` (floor pane and an emission map for the grid lines), `Material.create` with
  `mainTextureScale: { x, y }`, `emissionMap`, `unlit`; a pool of lit-tile renderers switched between ten materials.
- `input` keys, mouse buttons, `mouseDelta`, `mouseScrollDelta`; a React HUD.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-white-room
cd .. && cargo run --release -p platypus_app -- examples/fog-white-room
```

Screenshot run: `platypus examples/fog-white-room --screenshot out.png --frames 120` (the interactive window has
crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options (`src/modules` for bare imports; there are none). |
| `src/main.tsx` | About 520 lines: settings (`TILE`, `SPEEDS`, `ROLL`, `LAND`, `FOGS`), `floorTextures()`, the `WhiteRoom` component, the HUD. |
| `assets/mannequin.glb` | Quaternius's mannequin with eight of its animations (CC0, see `assets/CREDITS.md`). |

## How it works

1. **The bubble.** Linear fog starts a few tiles out (`FOGS`: 7 to 26 m by default), white like the floor and sky;
   a thin height fog hugging the floor thickens only over the long look to the horizon.
2. **The floor.** One 96 m cube whose texture is an 8 x 8 block of tiles, each its own shade, with bevels and grid
   lines (also the emission map). It moves with the player, snapped to whole blocks, so the tiles never seem to move;
   past `RECENTRE` metres everything shifts back to keep floats precise.
3. **Lit tiles.** A pool of 48 thin panels: one lights under each new tile stepped on, others on their own nearby,
   each fading through ten materials (quick to light, slow to fade).
4. **The figure.** Loaded with `await Model.load(..).whenLoaded()`, hung below the `CharacterController`'s capsule,
   repainted (main material white, joints glowing blue) and driven by an `Animator` whose `speed` and `grounded`
   parameters follow the movement. Rolls move as their root motion did (`ROLL`); a slow landing holds the player still
   while the deep landing plays (`LAND`), so the feet don't slide.
5. **The sky.** A grey gradient sky with no sun, and the cloud layer 40 m up, drifting at 1.5 m/s.

## Controls

| Input | Action |
|---|---|
| WASD / arrows | Move (relative to the camera) |
| Shift / Alt | Sprint / walk |
| Space / C | Jump / roll |
| Mouse drag, Q / E | Turn the camera |
| Wheel | Zoom |
| F / H / T | Fog distance / height fog / lit tiles |

## Try / adapt

- Change `cloudLayer.coverage` and `softness` for a clear or overcast sky; raise `altitude` and `size` together.
- Add more of the pack's animations to the blend tree (strafes, a crouch walk) with their own thresholds.
- Tint the fog and sky for a different "memory" (the Animus's sepia, a night blue).

## Gotchas

- The mannequin's parts are skinned: their renderers are `SkinnedMeshRenderer`s, so the repaint looks for those
  (natively `getComponent(MeshRenderer)` does not find a skinned renderer, as in Unity).
- The cloud layer is drawn with the background: distance fog doesn't cover it, height fog does.

Related examples: fog, fog-teapots, fog-blocks, animation, godrays-clouds
