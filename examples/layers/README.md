# layers

A forest with a walker, showing layers, per-layer culling distances, a second camera and levels of detail. The main
camera hides the "Minimap" layer (bright map markers), draws the "Detail" layer (rocks and flowers) only out to 14 m,
and a top-down orthographic minimap camera in the corner draws only the ground, the walker and the markers. Trees use
`LODGroup`: round crowns up close, blocky ones farther out, nothing when tiny.

Engine features exercised:

- `project.json` `layers` names, `LayerMask.nameToLayer`, `LayerMask.getMask`, `LayerMask.contains`, `LayerMask.everything`,
  `GameObject.layer`.
- `Camera`: `cullingMask`, `layerCullDistances`, `fieldOfView`, `horizontalFieldOfView`, `farClipPlane`, `clearFlags: "skybox"`,
  and a second camera with `orthographic`, `orthographicSize`, `depth`, `rect: new Rect(...)`.
- `LODGroup` with `setLODs([{ screenRelativeTransitionHeight, renderers }])`, `QualitySettings.lodBias`.
- `RenderSettings.skybox = { sunDirection }`, soft shadows, `Random`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/layers
cd .. && cargo run -p platypus_app -- examples/layers
```

Packaged: `platypus-build package ../examples/layers`. Screenshot run: `platypus-game examples/layers --screenshot out.png --frames 30`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | Declares the layer names. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 188 lines: `shape()`, `marker()`, the `Walker` and `LayersDemo` components, and the menu (`Toggle`, `Row`, `Cycle`, `Menu`). |

```json
"layers": { "8": "Detail", "9": "Player", "10": "Minimap" }
```

The built-in `Default` layer needs no entry. No assets.

## How it works

1. **`shape()`** creates a primitive and sets its layer by name: `go.layer = LayerMask.nameToLayer(layer)`.
2. **Scene.** 40 trees at radius 10 to 28 (skipping a spot in front of the camera), each with a trunk, a sphere crown, a
   cube crown (LOD 1) and a flat disc marker 8 m up on the `Minimap` layer; 160 rocks and flowers on `Detail`; and
   a capsule walker on `Player` circling at radius 7 (`Walker.onUpdate`) with an orange marker.
3. **LODs.** Each tree gets a `LODGroup` of size 3.7 and two levels; below 6% of screen height nothing draws:

   ```ts
   tree.addComponent(LODGroup, { size: 3.7 }).setLODs([
     { screenRelativeTransitionHeight: 0.25, renderers: [renderer(trunk), renderer(crown)] },
     { screenRelativeTransitionHeight: 0.06, renderers: [renderer(trunk), renderer(block)] },
   ]);
   ```

4. **Main camera.** The mask removes only `Minimap`; the per-layer distance array is read, edited and assigned back:

   ```ts
   main.cullingMask = LayerMask.everything & ~LayerMask.getMask("Minimap");
   const far = main.layerCullDistances;
   far[LayerMask.nameToLayer("Detail")] = 14;
   main.layerCullDistances = far;
   ```

5. **Minimap camera.** Orthographic, size 30, 30 m up and looking down (`Quaternion.euler(-90, 0, 0)`), `depth: 1` so it
   draws after the main camera, and `rect` is the lower-right 23% x 31% of the screen. Its mask is
   `getMask("Default", "Player", "Minimap")`, so detail objects are not drawn.
6. **Menu.** `Toggle` flips a layer's bit in the main camera's mask with `main.cullingMask ^= 1 << layer`. `Cycle`
   steps through field of view (55, 30, 75, 100; also shows `horizontalFieldOfView`), `QualitySettings.lodBias` (1, 0.4,
   2) and `farClipPlane` (1000, 30, 20).

## Controls

Mouse only: the buttons for Default / Detail / Player / Minimap layers, Field of view, LOD bias and Render distance.
There are no key bindings in the source.

## Try / adapt

- Use layers for gameplay filters: a "Ghost" layer visible only to a second camera, or a first-person weapon camera.
- Make the minimap a render texture (see `Camera.targetTexture` in the docs) instead of a viewport rectangle.
- Add a third LOD level that is a billboard.
- Hide an object class from shadows or lights by using the lights' own culling masks.
- Put the Minimap `rect` and layer names into a real HUD.

## Gotchas

- `layerCullDistances` is a copy: edit it and assign it back, as above.
- Layer names must be in `project.json` before `nameToLayer` can find them.
- The markers are 8 m above their objects so the ortho camera sees them over the tree tops; the main camera
  never draws them because their layer is masked out.
- `Toggle` reads `main` which only exists after `onStart`; the menu is refreshed from `onStart` for that reason
  (`refreshMenu()` at the end).

Related examples: quality, materials, light-units, fog-park
