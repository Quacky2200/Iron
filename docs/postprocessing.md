# Post-processing

Passes over a camera's finished picture: chromatic aberration, blur, depth of field, ink outlines, vignette,
pixelate, film grain, colour grading and more, plus your own WGSL. Scripts use `PostProcess`; the renderer side
is [post.rs](../crates/platypus_render/src/world/post.rs); the built-ins (their WGSL and parameters) are in
[postfx.rs](../crates/platypus_render_api/src/postfx.rs) and `postfx/*.wgsl`. Examples: `examples/postfx` (every effect, a menu) and the small ones, each a copy of one scene (`modules/stage.ts`):
`postfx-look` (stacked looks), `postfx-aberration`, `postfx-focus`, `postfx-outline`, `postfx-film`. Tests
`M7-FX-01`..`09`.

## Using it

```ts
const v = PostProcess.create("vignette");     // a built-in effect, on, every camera, order 0
v.setFloat("intensity", 0.8);                  // parameters by name
v.order = 40;                                  // lower runs first, ties in creation order
v.camera = Camera.main;                        // only this camera (undefined: every camera)
v.enabled = false;                             // off (kept: turn it on again)
v.destroy();
PostProcess.all();                             // every pass
v.parameters;                                  // ["intensity", "smoothness", "roundness", "tint"]
```

| Member | |
|---|---|
| `PostProcess.create(effect)` | A built-in `PostEffect` (a string union): see the table below. |
| `PostProcess.createCustom(source, parameters?)` | Your own WGSL (next section). |
| `effect`, `enabled`, `order`, `camera`, `isValid`, `parameters` | Properties. `camera` is `Camera \| null`. |
| `setFloat` / `getFloat`, `setColor` / `getColor`, `setVector(name, x, y?, z?, w?)` | Parameters by name; a name the pass lacks is logged. A float uses the first component, a `vec2` / `vec3` the first two / three. |
| `passes`, `usesDepth` | For custom shaders: how many times it runs (1 to 4) and whether it reads depth. |

Passes run after bloom, exposure, tonemapping and FXAA, so on the colours the screen shows, and before any UI
is drawn over the world. A camera's passes are its own and those aimed at every camera; each runs over that
camera's part of the screen only (split screens and picture-in-picture work), including cameras that render
into a `RenderTexture`. With render scale below 1 they run at the scaled size, then the picture is stretched
(so `pixelate` and `blur` are cheaper). Passes are not saved with scenes: scripts make them at start.

## Built-in effects

| Effect | Parameters (defaults) | Notes |
|---|---|---|
| `chromaticAberration` | `amount` 0.015 | Red and blue slide apart towards the edges (eight spectral samples). |
| `blur` | `radius` 6 px | Gaussian, two passes (x then y), 33 taps spaced `radius / 16`. |
| `depthOfField` | `focus_distance` 8 m, `aperture` 2 (f-stop), `focal_length` 50 mm, `max_blur` 16 px, `blades` 0, `highlight` 1 | Thin-lens circle of confusion on a 24 mm sensor. Reads depth. Move `focus_distance` yourself to focus (the example eases it towards a chosen object). `blades` 0 is round bokeh, 3 or more a polygon. |
| `tiltShift` | `focus` 0.5, `width` 0.12, `max_blur` 14, `angle` 0, `blades`, `highlight` | A sharp band across the view, blurring away from it; no depth needed. |
| `outline` | `thickness` 1.5 px, `ink` (colour), `depth_threshold` 0.04, `normal_threshold` 0.5, `fade_distance` 0, `color_threshold` 0 | Ink along silhouettes (second difference of depth, relative to distance, so slanted floors are not edges) and creases (normals rebuilt from depth); `fade_distance` thins lines with distance; `color_threshold` (>0) inks brightness edges as well. Reads depth. |
| `vignette` | `intensity` 0.6, `smoothness` 0.5, `roundness` 1, `tint` (vec3, black) | |
| `pixelate` | `size` 6 px, `levels` 0 | Nearest-neighbour blocks; `levels` >= 2 cuts the colours. |
| `filmGrain` | `intensity` 0.15, `size` 1.5, `response` 0.6, `colored` 0.3 | New noise each frame, strongest in the midtones. |
| `colorGrading` | `exposure` (stops), `contrast`, `saturation`, `hue_shift` (degrees), `balance` (vec2: temperature, tint), `lift`, `gamma`, `gain` (vec3) | Exposure and balance in light, the rest in gamma space. |
| `crt` | `curvature` 0.3, `scanlines` 0.35, `mask` 0.25, `line_height` 3 | Bulging glass with cut corners, scanlines, a phosphor mask. |
| `posterize` | `levels` 4, `softness` 0.12 | Cel bands on brightness, hue kept. |

The table is `EFFECTS` in `postfx.rs`: name, parameter slots (a `vec4` each, at most 8) with defaults, passes,
whether it reads depth, and the WGSL. The enum `PostEffectKind` follows its order (a test, `M7-FX-01`, keeps
the script's names and the table in step).

## Your own effect

```ts
const wobble = PostProcess.createCustom(`
  fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let w = sin(view_uv(uv).y * 60.0 + post_time() * 4.0) * strength * 0.004;
    return scene(uv + vec2<f32>(w, 0.0)) * tint;
  }`, ["strength", "tint:color"]);
wobble.setFloat("strength", 1);
```

`source` defines `effect(uv, color) -> vec4<f32>`: `uv` is across the whole target, `color` the picture there.
`parameters` declares up to 8 values (`"name"` is a float; `"name:float|vec2|vec3|vec4|color"`), each a
private WGSL variable set before `effect` runs. The renderer adds the bindings and these helpers
([post.rs](../crates/platypus_render/src/world/post.rs) `PRELUDE`):

| Helper | |
|---|---|
| `scene(uv)`, `scene_px(pixel)` | The picture before this pass (bilinear, or one pixel); edges clamp. |
| `scene_depth(uv)` | Distance along the view to what is drawn, in metres (far plane where nothing is). |
| `scene_normal(uv)` | World-space normal, zero on the background. Both need `usesDepth` (set when the source mentions them). |
| `view_uv(uv)`, `aspect()`, `pixel()` | The camera's own 0..1 across its part of the screen; its width over height; one pixel in uv. |
| `post_time()`, `frame_index()`, `pass_index()` | Seconds (the sum of frame times), the frame, which of `passes`. |
| `luma`, `hash21`, `to_gamma`, `to_linear`, `fx.rect`, `fx.size` | Utilities and the raw uniform. |

A shader that does not compile (no `effect`, a type error) is logged once (`post-process shader skipped`) and
left out; the other passes still run.

## How it runs

- **Where.** `render_views_with` ([world/mod.rs](../crates/platypus_render/src/world/mod.rs)): if any view has
  `WorldView::post`, the frame goes through the post path (the HDR texture, as with bloom or grading), the
  graded (and FXAA'd) picture lands in `PostChain::input`'s texture (target format), `PostChain::encode` runs
  the passes and the last one draws into the target.
- **Ping-pong.** Two textures of the target's size and format. A step reads one and draws into the other; a
  view that is only part of the target first copies the picture across and draws into its rectangle (scissor).
  The last step draws into the target itself when it covers all of it, else a copy follows. Uniforms (parameters,
  rectangle, size, time, pass) are one slot per step in a dynamic-offset buffer.
- **Depth and normals.** The world pass keeps its depth when a pass `depth` (`StoreOp::Store`; the texture is
  bindable). A resolve pass per view reads it (`texture_depth_2d`, or `texture_depth_multisampled_2d` sample 0
  with MSAA) and writes `R32Float` distance along the view direction and `Rgba16Float` world normals, rebuilt
  from the neighbour pixels on the nearer side of each axis (so silhouettes do not bend them). Perspective and
  orthographic both work. Normals are of the depth buffer, so they have no normal-map detail.
- **Pipelines** are cached by `PostShader::id` (a hash of source and parameters) and target format; the app
  keeps one `Arc<PostShader>` per script shader (`platypus_app/src/postfx.rs`) so they stay cached.
- **Sync.** The registry (`platypus_render_api::post_version`, `post_passes`) changes on any pass edit; `render_sync`
  sends the enabled passes to `WorldViews::post`, and `WorldViews::views` gives each camera's view its own.

## Limits

- Effects see the screen-referred picture (after tonemapping), not HDR: no bloom-style effect over HDR yet.
- Depth is the opaque (and see-through) meshes' depth buffer: see-through surfaces write none, so the glass
  and fog volumes are not in `scene_depth`.
- Outlines have no per-object opt-out (layers) yet, and there is no toon material.
- Depth of field is a single gather pass of 48 taps (round or polygon); very large blur radii show the spiral.
- Mirrors, reflection probes and shadow renders get no passes.
