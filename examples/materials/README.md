# materials

A showroom for physically based shading under a procedural sky. It has a grid of spheres from rough to smooth and
plastic to metal, a mirror wall, a gold bunny and a chrome teapot, a reflection probe so shiny things reflect each
other, and a front row of maps and modes: normal map, cut-out leaf, glass, faded sphere, specular workflow.

Engine features exercised:

- `Material.create({ color, smoothness, metallic, normalMap, normalScale, mainTexture, renderingMode, alphaCutoff, doubleSided, specularColor })`
  with `renderingMode` values `"cutout"`, `"transparent"`, `"fade"`.
- `Texture.create(w, h, pixels, { linear: true, filterMode: "trilinear", anisoLevel: 8 })` for code-made normal map and alpha mask.
- `Mirror` (planar reflection, on a quad), `ReflectionProbe` (`size`, `resolution`, `refreshMode: "everyFrame"`).
- `RenderSettings.skybox = { sunDirection }`, `Camera` with `clearFlags: "skybox"`, the `CameraOrbit` component
  (`center`, `distance`, `pitch`, `yaw`, `autoRotate`).
- `Model.load`, `model.instantiate`, `model.whenLoaded()`; `transform.rotate`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/materials
cd .. && cargo run -p platypus_app -- examples/materials
```

Packaged: `platypus-build package ../examples/materials`. Screenshot run: `platypus-app examples/materials --screenshot out.png --frames 60`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 140 lines: `Spin`, `Showroom`, `bumps()`, `leafTexture()`, `collect()`, `Caption`. |
| `assets/bunny.glb` | Stanford Bunny model. |
| `assets/teapot.glb` | Utah teapot model. |

(Both models are the same kind of test models as in the `classics` example.)

## How it works

1. **Camera and sky.** The camera is a `Camera` plus a `CameraOrbit` that slowly auto-rotates around the grid. The
   `skybox` is the procedural sky; a matching directional soft-shadow `Light` points opposite the sky's
   `sunDirection`.
2. **Grid.** 6 x 3 spheres; smoothness is `x / 5` across and metallic `z / 2` front to back, one tint colour:

   ```ts
   Material.create({ color: tint, smoothness: x / 5, metallic: z / 2 })
   ```

3. **Front row.** A local `row()` helper places five items:
   - a cube with a code-generated tangent-space normal map of round bumps (`normalScale: 1.2`);
   - a quad using an alpha-masked leaf texture with `renderingMode: "cutout"`, `alphaCutoff: 0.5` and `doubleSided: true`,
     so its shadow has holes;
   - two spheres with the same translucent colour (alpha 0.25), one `"transparent"` (the specular highlight stays) and one
     `"fade"`;
   - a sphere using the specular workflow (`specularColor`).
4. **Mirror wall.** A quad with a metallic, smoothness 1 material and a `Mirror` component, plus a thin dark cube as a frame.
5. **Probe.** A `ReflectionProbe` of size 14x8x10 at 128 px, refreshed every frame because the models turn.
6. **Models.** Bunny (gold: `metallic: 1`, `smoothness: 0.85`) and teapot (chrome). The material is applied after load:

   ```ts
   model.whenLoaded().then(() => {
     for (const r of collect(go)) r.material = p.material;
   });
   ```

   A `Spin` component rotates each by 25 degrees per second with `this.transform.rotate(...)`.
7. **UI.** A caption (bottom left) explains the layout; no interaction.

## Controls

None in the source: the camera orbits by itself (`autoRotate: 8`), and the UI is a caption. Whether `CameraOrbit` also
accepts mouse input is not shown in this file.

## Try / adapt

- Swap the tint and sweep another pair of parameters (`metallic` vs `smoothness`) to build a material reference chart.
- Replace the generated normal map with a texture asset.
- Add a second `Mirror` or a probe per room for a house with reflective rooms.
- Use `"cutout"` for foliage cards and `"fade"` for UI-like world objects that dissolve.
- Make material swaps interactive (click to change `metallic`).

## Gotchas

- Imported models have their own materials; this example overwrites them after `whenLoaded()` using a recursive search for `MeshRenderer`s.
- Both the `Mirror` and the probe re-render the scene: the probe is set to `everyFrame` here, which costs more than an on-demand refresh.
- Texture data from code has the bottom row first.
- Normal maps and the alpha-mask are created with `linear: true` where they hold data, not colour (the leaf mask uses the default).

Related examples: classics, fog-teapots, cornell, quality, layers
