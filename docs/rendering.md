# Rendering

Crate: [iron_render](../crates/iron_render/src) (wgpu). It consumes the GPU-free
`Vec<iron_ui::DrawCmd>` described in [ui-tree.md](ui-tree.md) and does not depend on winit. Design
background: [DESIGN.md](../DESIGN.md) section 5.2; milestone tests `M2-DRW-*`, `M3-TXT-*`,
`M3-EVT-05` in [TESTS.md](../TESTS.md).

## Files

| File | Role |
|---|---|
| [gpu.rs](../crates/iron_render/src/gpu.rs) | `Gpu` (instance, adapter, device, queue). `headless()` and `with_surface()` |
| [surface.rs](../crates/iron_render/src/surface.rs) | `WindowSurface`: swapchain, resize, `present_with` |
| [offscreen.rs](../crates/iron_render/src/offscreen.rs) | `OffscreenTarget`: texture + `read_pixels` (tests, screenshots) |
| [lib.rs](../crates/iron_render/src/lib.rs) | `Color`, re-exports, `encode_clear` |
| [ui/mod.rs](../crates/iron_render/src/ui/mod.rs) | `UiRenderer` and `render()` (the plan/record/submit pipeline) |
| [ui/quad.wgsl](../crates/iron_render/src/ui/quad.wgsl) | Instanced shader: boxes, shadows, glyphs, gradients, clips, images |
| [ui/effects.wgsl](../crates/iron_render/src/ui/effects.wgsl) | Fullscreen passes: blur, composite, blit |
| [ui/atlas.rs](../crates/iron_render/src/ui/atlas.rs) | Glyph atlas (swash) |
| [ui/gradient.rs](../crates/iron_render/src/ui/gradient.rs) | `iron_ui` gradients to the GPU struct |
| [ui/images.rs](../crates/iron_render/src/ui/images.rs) | `url()` image cache, `background-size/position` math |
| [testing.rs](../crates/iron_render/src/testing.rs) | Golden helpers and the shared test GPU |

## GPU setup

`Gpu::headless()` and `Gpu::with_surface(target, w, h)` request a `HighPerformance` adapter and
**fall back to a software adapter** (WARP / llvmpipe) if no hardware adapter exists, so CI and
`--screenshot` work without a GPU. Device limits are `downlevel_defaults` raised to the adapter's
resolution limits. `WindowSurface` prefers an sRGB format, uses `AutoVsync`, tolerates 0x0 sizes
(minimized: frames are skipped) and reconfigures on `Outdated`/`Lost`/`Suboptimal`.

Frame latency (`desired_maximum_frame_latency`) is 1 by default and 2 on DX12, where 1 made live
resizing jitter. `IRON_FRAME_LATENCY` (1 or 2 only; more just adds input lag) overrides it. On
Windows the window class background is set to black (in `iron_platform`), so any area exposed by a
resize before the next frame is black, not white.

## Colour and units

- Draw commands are in **logical pixels**; `RenderTarget { view, format, size (physical), scale }`
  says how to map them. Shaders receive `globals.viewport` and `globals.scale`, and every quad
  grows by one physical pixel for anti-aliasing.
- Colours are converted by `linear_premultiplied` (sRGB to linear, premultiplied alpha). The canvas
  (`CANVAS_FORMAT` = `Rgba8UnormSrgb`) stores sRGB but blends linearly, using
  `PREMULTIPLIED_ALPHA_BLENDING`.

## `UiRenderer::render`

One call per frame: `render(gpu, &RenderTarget, clear, &[DrawCmd]) -> RenderStats`. It has three
stages. Planning never touches the GPU encoder, which keeps the logic easy to reason about.

### 1. Plan: commands to instances and steps

Walk `cmds` once, producing:

- `instances: Vec<Instance>` (8 x `vec4`, matching `struct Instance` in quad.wgsl). `params.x` is
  the **kind**: `KIND_QUAD`, `OUTER_SHADOW`, `INSET_SHADOW`, `IMAGE`, `GLYPH`, `KIND_BORDER`,
  `KIND_GRADIENT_PIECE`. `rect`/`radii` (horizontal) and `radii_y` (location 7, last so earlier
  locations stay put) are the shape, `p0/p1` kind-specific data (border widths, shadow source box, atlas rect),
  `color0/color1` fill and border, `params.y/z/w` blur sigma / gradient index / clip index.
- `gradients: Vec<GpuGradient>` (up to 8 stops each, extras dropped with a warning).
- `steps: Vec<Step>`: the ordered render program, over a pool of full-size textures:

| Step | Meaning |
|---|---|
| `Clear{target,color}` | Deferred (applied as the load op of the next pass on that target) |
| `Draw{target, range, image}` | One instanced draw of `instances[range]`; with `image`, the image pipeline for one background image |
| `Copy{src,dst}` | Snapshot a texture (for backdrop blur) |
| `Blur{tex,temp,area,sigma}` | Separable Gaussian: horizontal into `temp`, vertical back |
| `Composite{src,dst,area,opacity,clip,blend}` | Alpha-composite a layer, scissored, with clip; `blend: Option<(mode, backdrop)>` for a blend mode |

**Batching.** Consecutive boxes, shadows and glyphs stay in one `Step::Draw` regardless of draw
order, because the instance order is the paint order and a single pipeline draws all kinds
(the shader switches on `kind`). A batch only ends when something needs a different pass: a layer
push/pop, a backdrop blur, or a background **image** (needs its own texture bind group): colour is
drawn, the batch is flushed, each image layer is drawn alone (bottom first), then the border is
drawn on top. Image layers are clipped to `ImageFill.area`/`radii`. `images::repeat_tiles` expands
`round` and `space` into whole tiles (the gap goes to the image shader in `p1.xy`).

**Blend modes.** A `PushLayer` with a blend mode other than normal composites with
`Step::Composite.blend`: the parent is copied first as the backdrop, then `composite_blend_pipeline`
(`fs_composite_blend` in effects.wgsl, binding 5 is the backdrop) mixes source and backdrop itself
in sRGB with a replace blend state.

**Layers.** `PushLayer` flushes, takes a pooled texture, clears it to transparent and makes it
`current`. `PopLayer` flushes, optionally adds `Blur` (`filter: blur`, sigma = blur x scale), then
`Composite` into the parent with `opacity`, scissored to the part of `bounds` the surrounding clip
allows. `take_texture`/`pool` reuse textures between frames. Unbalanced pushes/pops log a warning
and are repaired.

**Backdrop blur.** `Copy` the current target, blur the copy over `rect.expand(3 * blur)`, then
composite it back using a **rounded-rect clip** of the box. The result is frosted glass of
whatever was already drawn.

**Clip table.** `PushClip` appends a `GpuClip { rect, rx, ry, bounds, parent }` to a per-frame
table and pushes its index on `clip_stack`; `bounds` is the intersection of the whole chain.
Instances do **not** break batches for clips: each carries its clip index in `params.w`
(`tag()` stamps the pending instances when the clip stack changes). In the shader,
`clip_coverage` multiplies coverage along the parent chain (up to 32 deep) using an approximate
elliptical rounded-box signed distance. A polygon clip (`clip-path: polygon()`/`path()`) stores its
points after the entry, one `vec4` each (x, y, contour start, contour end); `info.y` is the kind,
`info.z` the first point vec4 and `info.w` the count. `clip_word` and `sd_polygon` (in both
shaders) evaluate it. Consequences:

- Anything whose bounds do not intersect the active clip's bounds is **culled** in the plan.
- The vertex shader shrinks each quad to the clip bounds, so overdraw outside the clip costs nothing.
- A layer starts with an empty clip stack (its content is unclipped) and is clipped **when
  composited** (`Composite.clip`), so a blur inside a clipped layer is cut off correctly.
- `BackdropBlur` registers its own rounded clip entry (which is why `RenderStats.clips` counts it).

**Text.** For each `GlyphRun`, glyph positions are snapped to whole pixels vertically and to
quarter pixels horizontally (`SUBPIXEL_BINS` = 4; the sub-pixel offset is baked into the cached
raster). `GlyphAtlas::glyph` returns an atlas entry; each glyph becomes a `KIND_GLYPH` instance whose
`p0` is the atlas rect. Underline/strike decorations are plain quads.

### 2. Upload

`globals`, `instances`, `gradients` and `clips` buffers are written with `write_buffer`; each grows
to the next power of two when too small. Empty storage buffers get a dummy element (wgpu forbids
zero-sized bindings).

### 3. Record and submit

Steps are encoded in order into one command encoder, then a final **blit** copies the canvas
(texture 0, always `CANVAS_FORMAT`) to the real target, whose format can differ (`blit_pipeline` is
cached per format). Blur and composite parameters use small per-pass uniform buffers. Textures are
returned to the pool afterwards. `RenderStats` (`draw_calls`, `passes`, `layers`, `instances`,
`clips`) lets tests assert batching behaviour (for example that plain boxes share a draw call and
no layer is created).

## Shaders

[quad.wgsl](../crates/iron_render/src/ui/quad.wgsl):

- `vs_main`: six vertices per instance (two triangles), no vertex buffer data besides the instance;
  growth for AA and for outer shadows (`3 * sigma`); clipped to the clip bounds.
- `fs_main` -> `shade(kind)`:
  - **Box**: signed-distance rounded rectangle with per-corner elliptical radii (`sd_rounded_box`)
    for the outer edge and for the inner (padding) edge (`inner_rx`/`inner_ry`, reduced by border
    widths); gradient composited over the fill; border
    is `outer - inner` coverage. Backgrounds extend under the border (`background-clip: border-box`).
  - **Styled border** (`KIND_BORDER`, `styled_border`): picks the side by nearest edge and width,
    then draws solid, dashed, dotted, double, groove, ridge, inset or outset with dashes and dots
    fitted per side. Side data lives in the gradient table entry.
  - **Shadows**: closed-form Gaussian blurred rounded box (`erf`-based, `blurred_box`), no offscreen
    pass. Shadows use circular mean radii (`circular()`). Outer shadows are masked out of the casting box; inset shadows are drawn inside the padding
    box minus the blurred hole.
  - **Glyph**: coverage from the R8 atlas page, or the RGBA page for colour emoji.
  - **Gradients**: `gradient_color` for linear (angle/corner), radial and conic (kind 2: angle from up,
    clockwise, centre in `info2.yz`), evaluated per pixel from `GpuGradient`; `info2.w` wraps stops
    for `repeating-*`. `KIND_GRADIENT_PIECE` draws a stretched gradient border-image piece.
- `fs_image`: separate entry point/pipeline; samples one texture tile with repeat flags, clipped to
  the box shape. For `border-image` it draws pieces as `KIND_IMAGE` with a texture sub-rect in
  `color0` (`images::border_image_axis` tiles an edge); piece edges are snapped to pixels.

[effects.wgsl](../crates/iron_render/src/ui/effects.wgsl): `vs_fullscreen` (one oversized triangle),
`fs_blur` (1D Gaussian with `radius = ceil(3 sigma)`, max `MAX_BLUR_RADIUS` = 128), `fs_composite`
(opacity and clip coverage), `fs_composite_blend` (blend mode against a backdrop copy), `fs_blit`. The same `Clip` struct and coverage function are duplicated
in both shader files; keep them in sync with `GpuClip` in mod.rs.

## Glyph atlas and images

- [atlas.rs](../crates/iron_render/src/ui/atlas.rs): two pages (2048 R8 coverage, 1024 RGBA colour),
  `etagere` bucketed allocation, 1 px padding. Keys include font blob id, glyph id, size x 4,
  subpixel bin, variable-font coordinates, synthetic bold/skew. Rasterized with `swash`. When a page
  fills, the whole atlas is cleared at the start of the next frame and visible glyphs re-rasterize.
- [images.rs](../crates/iron_render/src/ui/images.rs): `ImageCache` keyed by URL; failures are
  cached too so they are reported once. `url()` values are `data:` URLs (decoded with
  `decode_data_url`) or paths resolved by `iron_core::resolve_asset` against `set_asset_root` (the
  app sets it to the game folder). `UiRenderer::image_size(gpu, url)` loads through this cache so the app can report natural sizes for `image` elements. `tile_rect` implements `background-size`
  (`auto`/`cover`/`contain`/lengths) and `background-position`.

## Offscreen and screenshot path

`OffscreenTarget::with_format(gpu, w, h, format)` creates a `RENDER_ATTACHMENT | COPY_SRC` texture;
`read_pixels` copies to a buffer with 256-byte row padding, maps it, and returns tightly packed RGBA8
(top row first). The app's `--screenshot` path ([main.rs](../crates/iron_app/src/main.rs)
`save_screenshot`) renders the draw list again into an `Rgba8UnormSrgb` offscreen target and writes
a PNG with `iron_render::testing::write_png`. It still opens a window (frames run until the target
count), unlike tests, which are fully headless.

## Golden testing

See [testing.md](testing.md). In short: `assert_golden(id, w, h, &pixels, tolerance)` in
[testing.rs](../crates/iron_render/src/testing.rs) compares against `tests/golden/<id>.png`;
`IRON_UPDATE_GOLDEN=1` rewrites references. `shared_gpu()` returns one `Gpu` per test binary behind a
mutex because creating devices from parallel threads can deadlock some Vulkan layers.
