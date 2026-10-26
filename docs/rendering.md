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
| [ui/mod.rs](../crates/iron_render/src/ui/mod.rs) | `UiRenderer`, the **wgpu executor**: uploads a `FramePlan`, records and submits it |
| [ui/plan.rs](../crates/iron_render/src/ui/plan.rs) | The **backend-neutral plan**: `plan()` turns a draw list into a `FramePlan` (no GPU object); `PlanResources` |
| [ui/backend.rs](../crates/iron_render/src/ui/backend.rs) | The `UiBackend` trait the app-facing renderers implement (`UiRenderer` today) |
| [iron_render_gles](../crates/iron_render_gles/src/lib.rs) | `GlesRenderer`: executes a `FramePlan` with GLES 3 through EGL / ANGLE (`egl.rs`, `shaders.rs`) |
| [ui/glyphs.rs](../crates/iron_render/src/ui/glyphs.rs) | `GlyphCache`: GPU-free glyph rasterizing and packing, with a queue of pixel uploads |
| [ui/quad.wgsl](../crates/iron_render/src/ui/quad.wgsl) | Instanced shader: boxes, shadows, glyphs, gradients, clips, images |
| [ui/effects.wgsl](../crates/iron_render/src/ui/effects.wgsl) | Fullscreen passes: blur, composite, blit |
| [ui/atlas.rs](../crates/iron_render/src/ui/atlas.rs) | The wgpu glyph atlas: the page textures a `GlyphCache` fills |
| [ui/gradient.rs](../crates/iron_render/src/ui/gradient.rs) | `iron_ui` gradients to the GPU struct |
| [ui/image_cache.rs](../crates/iron_render/src/ui/image_cache.rs) | `ImageCache`, GPU-free: `url()` images decoded, SVGs rasterized at a size, keys, eviction, a queue of `ImageUpload`s |
| [ui/images.rs](../crates/iron_render/src/ui/images.rs) | The wgpu side: `ImageTextures` (a texture and both samplers' bind groups per key) |
| [testing.rs](../crates/iron_render/src/testing.rs) | Golden helpers and the shared test GPU |

## GPU setup

`Gpu::headless()` and `Gpu::with_surface(target, w, h)` request a `HighPerformance` adapter and
**fall back to a software adapter** (WARP / llvmpipe) if no hardware adapter exists, so CI and
`--screenshot` work without a GPU. Device limits are `downlevel_defaults` raised to the adapter's
resolution limits. `WindowSurface` prefers an sRGB format, starts with `AutoNoVsync` (the app sets it per frame from `vSyncCount`), tolerates 0x0 sizes
(minimized: frames are skipped) and reconfigures on `Outdated`/`Lost`/`Suboptimal`.

`iron-app` (`project.json` `"type": "app"`, `docs/iron-build.md`) is the host for apps that are only UI. It asks the GPU allocator
for `MemoryHints::MemoryUsage` (small allocation blocks, `iron_render::set_low_memory`) instead of the default `Performance`, and its
3D `WorldRenderer` is built only if something asks for it (`lazy_world.rs`; `iron-game` builds it at once). For a clock that took private
bytes from ~306 MB to ~135 MB on Vulkan (NVIDIA, Windows); the working set stays ~122 MB. What remains is mostly the driver (wgpu's
own allocator holds ~28 MB, the Rust heap ~4 MB). The renderer for apps is planned to be GLES through ANGLE (a bare ANGLE/D3D11 context
measured ~61 MB private against ~138 MB for NVIDIA's OpenGL driver and ~135 MB for wgpu on Vulkan); wgpu cannot use ANGLE on Windows
(its Windows GL backend is WGL only), so that is a native GLES renderer behind the same draw list (`iron_ui::DrawCmd`).

### The GLES executor (`crates/iron_render_gles`)

`GlesRenderer` executes the same `FramePlan` with GLES 3 (`glow`), for `iron-app`. Its context comes
from EGL (`egl.rs`, `khronos-egl` loaded at run time): ANGLE's `libEGL.dll` on Windows, which draws
through Direct3D 11 (a bare context measured ~61 MB private against ~138 MB for NVIDIA's OpenGL
driver); Mesa's EGL elsewhere. `EglContext::headless(Some(dir))` loads ANGLE from `dir`
(`SetDllDirectoryW`), else the usual search (next to the program).

What differs from the wgpu executor (`shaders.rs` is a GLSL ES 3.00 port of `quad.wgsl`):

- **Tables are float data textures.** GLES 3.0 has no storage buffers, so the gradient table
  (`u_grads`, 12 texels per `GpuGradient`) and the clip/transform table (`u_clips`, 5 per `GpuClip`) are
  `RGBA32F` textures read with `texelFetch`, one entry per row (polygon clips read their points as a flat
  `vec4` array across rows, as in WGSL).
- **Instances** are instanced vertex attributes re-pointed at the draw's byte range (ES 3.0 has no
  base-instance draw); six vertices per instance, positions from `gl_VertexID`.
- **Canvases** are `SRGB8_ALPHA8` textures, so blending is in linear light like wgpu's; the glyph pages are
  `R8` and `SRGB8_ALPHA8`, filled from the shared `GlyphCache`'s upload queue. The final blit applies
  the sRGB transfer itself (`Target::srgb`) for a default framebuffer that doesn't encode on write.
- **`gl_FragCoord` is y up**; the fragment shader turns it into the plan's y-down logical pixels.

Done: the quad pass (boxes, borders and their styles, gradients, outer and inset shadows, glyphs, wavy
decorations, clips and polygon clips, transforms), the glyph atlas, images (`fs_image`: repeat, space,
`cover`/`contain`, rounded clips, `image-rendering: pixelated`, tints, border-image pieces; PNG and JPEG
from the shared `ImageCache`, SVGs rasterized at their drawn size; `SRGB8_ALPHA8` textures) and layers
(`effects.rs`, a port of `effects.wgsl`): `Copy` is a `glBlitFramebuffer`, `Blur` two passes of the
separable Gaussian (reads clamp at the edge), `Composite` opacity, clip, the `filter` colour matrix, drop
shadow tint and shift, the mask texture and the 15 blend modes (a separate program that mixes the layer with a
copy of its parent), so opacity groups, `filter`, drop shadows, masks, `mix-blend-mode` and `backdrop-filter`
work. Scissor rectangles are flipped to GL's bottom-left origin; a clear wipes the whole canvas as a wgpu load op
does; canvases filter linearly (a shifted drop shadow samples between texels). `render_overlay` clears
the canvas transparent and blends it onto what the target holds (premultiplied, `ONE`/`ONE_MINUS_SRC_ALPHA`).
`EglContext::window` makes a context on a winit window (Win32 HWND, Xlib; anything else is an error). Its
surface is plain, not ANGLE's sRGB one (reading that back returned unencoded bytes, so colours were off), so
`Target::srgb` is false and the blit encodes itself. `iron-app` (`gles_gfx.rs`) draws its one window this way
(no 3D world, world panels or secondary windows), swaps with vsync, skips a frame whose draw lists equal the
last one's, and falls back to wgpu when EGL is missing (`IRON_ANGLE_DIR` names ANGLE's folder; `IRON_GLES=0`
forces wgpu). `tests/vs_wgpu.rs` draws scenes through both executors and compares pixels
(`M2-GLES-02..13`: identical so far, 0 to 1 in 255); it and `tests/context.rs` need an EGL driver
(`IRON_ANGLE_DIR` on Windows) and skip without one.

`crates/iron_render_cpu` is a second consumer of that draw list: `vello_cpu` + `softbuffer`, no GPU API at all (the clock: ~16 MB
private). It draws boxes, borders (also per side), gradients, images and SVGs (decoded lazily, downscaled to the size drawn,
dropped when unused), shadows, text, clips, layers, blur and backdrop blur, and affine and perspective transforms (a perspective
subtree is drawn flat, cached and warped); not `border-image`, masks, layer colour matrices and drop shadows, nor the 3D world. It is
not wired into either host: apps are meant to be accelerated.

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
stages, and the first is **backend-neutral**: `plan::plan(cmds, &PlanParams, &mut dyn PlanResources)`
returns a `FramePlan` (`instances`, `gradients`, the clip/transform `clips` table, `steps`, and the
count of frame-sized `textures`) and holds no GPU object. What planning needs from a renderer
comes through `PlanResources`: `glyph` (place a glyph in the atlas), `image` (an image's natural
size) and `svg_at` (an SVG rasterized for a size). `WgpuResources` in mod.rs is the wgpu one; a
second renderer (the GLES one for `iron-app`, through ANGLE) supplies its own, uses the same
`GlyphCache` and `ImageCache` (their pixels are queued as uploads, written by the renderer after planning;
`ImageInfo::key` names an image's texture in `Step::Draw`), and executes the same `Step`s over its own textures. `UiBackend`
(backend.rs) is the app-facing trait; the wgpu `UiRenderer` implements it. The planner is tested
without a GPU (`tests/plan.rs`, `M2-PLN-01..07`); the golden tests (`tests/ui.rs`) check the wgpu
executor.

### 1. Plan: commands to instances and steps (`plan.rs`)

Walk `cmds` once, producing:

- `instances: Vec<Instance>` (9 x `vec4`, matching `struct Instance` in quad.wgsl). `params.x` is
  the **kind**: `KIND_QUAD`, `OUTER_SHADOW`, `INSET_SHADOW`, `IMAGE`, `GLYPH`, `KIND_BORDER`,
  `KIND_GRADIENT_PIECE`, `KIND_WAVY`. `rect`/`radii` (horizontal) and `radii_y` (location 7) are the shape, `p0/p1` kind-specific data (border widths, shadow source box, atlas rect),
  `color0/color1` fill and border, `params.y/z/w` blur sigma / gradient index / clip index, and
  `xf.x` (location 8) the transform index (1-based, 0 = none): the instance's rect and data are in
  that transform's plane.
- `gradients: Vec<GpuGradient>` (up to 8 stops each, extras dropped with a warning).
- `steps: Vec<Step>`: the ordered render program, over a pool of full-size textures:

| Step | Meaning |
|---|---|
| `Clear{target,color}` | Deferred (applied as the load op of the next pass on that target) |
| `Draw{target, range, image}` | One instanced draw of `instances[range]`; with `image: Some((url, pixelated))`, the image pipeline for one background image (nearest-neighbour bind group when `pixelated`) |
| `Copy{src,dst}` | Snapshot a texture (for backdrop blur) |
| `Blur{tex,temp,area,sigma}` | Separable Gaussian: horizontal into `temp`, vertical back |
| `Composite{src,dst,area,opacity,clip,blend,matrix,tint,shift,mask}` | Alpha-composite a layer, scissored, with clip; `blend: Option<(mode, backdrop)>` for a blend mode; `matrix` is the `filter` colour matrix, `tint`/`shift` make a drop-shadow copy, `mask: Option<(texture, MaskMode)>` multiplies by a mask |

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
and are repaired. Open layers live on `layer_stack` as `OpenLayer` (own texture, mask, parent, bounds,
blur, opacity, blend, matrix, shadows, outer clips).

**Filters.** The order is fixed: blur, then drop shadows, then the colour matrix (at composite),
whatever order the CSS lists them. `PopLayer` blurs the layer, then for each `LayerShadow` (last
first, so the first listed ends on top) copies the layer, blurs the copy by the shadow's sigma and
composites it under the layer with `tint` (the source's alpha in the shadow colour, linear
premultiplied) and `shift` (offset x scale, physical px). The layer itself then composites with
`matrix`. `filtered_source` in effects.wgsl does the tint and the matrix: it un-premultiplies, goes to
sRGB, applies the 4x5 matrix (`CompositeParams.matrix` as WGSL columns plus `offset`), clamps and
converts back, like CSS filter functions in browsers. `shift.z`/`shift.w` switch matrix and tint on.

**Masks.** `BeginMask(mode)` (inside a layer, after its content) flushes, takes a second pooled
texture, clears it and makes it `current`, so the following quad (the mask layers) draws there;
`OpenLayer.texture` keeps the content texture. `PopLayer` then passes `mask: Some((mask, mode))` to
every `Composite` of that layer, shadows included. The composite bind group gets the mask at
binding 6 (the source texture when there is no mask); `mask_at` returns 1 with no mask, the mask's
alpha for `Alpha`, or its sRGB luminance times alpha for `Luminance`, and `fs_composite` and
`fs_composite_blend` multiply the colour by it.

**Backdrop filter.** `Copy` the current target, blur the copy over `rect.expand(3 * blur)` (skipped
when `blur` is 0, a colour-only filter), then composite it back using a **rounded-rect clip** of the
box, with `BackdropBlur.matrix` as the composite's `matrix`. The result is frosted glass of
whatever was already drawn.

**Clip table.** `PushClip` appends a `GpuClip { rect, rx, ry, bounds, parent }` to a per-frame
table and pushes its index on `clip_stack`; `bounds` is the intersection of the whole chain.
Instances do **not** break batches for clips: each carries its clip index in `params.w`
(`tag()` stamps the pending instances when the clip stack changes). In the shader,
`clip_coverage` multiplies coverage along the parent chain (up to 32 deep) using an approximate
elliptical rounded-box signed distance. A polygon clip (`clip-path: polygon()`/`path()`) stores its
points after the entry, one `vec4` each (x, y, contour start, contour end); `info.y` is the kind,
`info.z` the first point vec4 and `info.w` the count. `clip_word` and `sd_polygon` (in both
shaders) evaluate it. `info.y` also carries the transform the shape is in, as `kind + 4 x transform
index` (kind 0 box, 1 nonzero, 2 evenodd); `clip_coverage_with` splits it with `% 4` and `/ 4`.
Consequences:

- Anything whose bounds do not intersect the active clip's bounds is **culled** in the plan.
- The vertex shader shrinks each quad to the clip bounds, so overdraw outside the clip costs nothing.
- A layer starts with an empty clip stack (its content is unclipped) and is clipped **when
  composited** (`Composite.clip`), so a blur inside a clipped layer is cut off correctly.
- `BackdropBlur` registers its own rounded clip entry (which is why `RenderStats.clips` counts it).

**Transforms.** `PushTransform(h)` stores the homography and its inverse as an entry in the same table
(`Clips::push_transform`, packed into the `GpuClip` fields; nothing chains to it and its bounds are
unlimited) and pushes `(index, h)` on `xf_stack`; `PopTransform` pops it. `tag()` stamps pending
instances with the clip index and `xf.x` together, so transforms do not break batches either. A
`PushClip` inside a transform stores its shape in the element's plane (`Clips::push` with the
current transform; its `bounds` are `Homography::map_rect` of the shape) and the shader tests it in
that plane. In the plan, culling bounds and `PushLayer`/`BackdropBlur` bounds go through `world()`
(`map_rect` with the current homography). Shaders:

- `vs_main` maps each corner of the (grown) quad with `xf_apply` (a perspective divide, so straight
  edges stay straight). The shrink-to-clip-bounds step is skipped when `xf` is set, as those bounds
  are in window space.
- Fragments compute the window point, then `local_point` maps it back with `xf_unapply` to the
  instance's plane for the shape, glyph atlas lookup and gradients (`shade`, `glyph`, `fs_image`).
  `clip_coverage` runs on the window point and un-applies each clip's own transform.
- Glyph masks of text under a rotation, skew, uneven scale, mirror or perspective are sampled
  nearest-neighbour in the plane, so such text is slightly jagged ([differences.md](differences.md)).
  Text under a map that only scales evenly and moves takes the path under **Text** instead.

**Text.** For each `GlyphRun`, glyph positions are snapped to whole pixels vertically and to
quarter pixels horizontally (`SUBPIXEL_BINS` = 4; the sub-pixel offset is baked into the cached
raster). `GlyphAtlas::glyph` returns an atlas entry; each glyph becomes a `KIND_GLYPH` instance whose
`p0` is the atlas rect. `params.y` is 0 for a normal mask, 1 for a colour glyph, 2 for a crisp mask
(`font-smooth: never`; the shader thresholds coverage at 0.5). A `GlyphRun.stroke` draws a second
glyph per glyph from an outline-stroked raster (`atlas.glyph(..., stroke_px)`) in the stroke colour
over the fill.

Under a transform that only scales evenly (`s > 0`) and translates (`Homography::as_scale_translate`),
a `DrawCmd::Text` is drawn like a browser does: glyphs are rasterized at `font_size * s * scale`, their
positions mapped (`g.x * s + tx`, `g.y * s + ty`), the stroke width scaled by `s`, and the instances
placed in window space. They are marked `xf[0] = -1`, which `tag()` turns into transform 0 instead of
the current one. `background-clip: text` fill rects are mapped with `world()` too. Rasters are cached
by size in 0.25 px steps (the atlas key), so a short scale animation rasterizes only a few sizes.
(`M3-XF-03`, golden `M3-XF-02`.)

**Text decorations** (`TextCmd.decorations`): `solid` is one quad, `double` two (the second
`2 x thickness` below), `dotted`/`dashed` a `KIND_BORDER` instance with a one-sided styled-border
gradient entry, and `wavy` a `KIND_WAVY` instance (a sine wave of the line's thickness, drawn by the
shader inside a band 3 x thickness tall).

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
(`filtered_source`, opacity, `mask_at` and clip coverage), `fs_composite_blend` (the same against a backdrop copy), `fs_blit`. The same `Clip` struct and coverage function are duplicated
in both shader files; keep them in sync with `GpuClip` in mod.rs.

## Glyph atlas and images

- [glyphs.rs](../crates/iron_render/src/ui/glyphs.rs) (GPU-free) and [atlas.rs](../crates/iron_render/src/ui/atlas.rs) (wgpu): `GlyphCache` places glyphs and queues their pixels as `GlyphUpload`s, `GlyphAtlas` writes them into two page textures before the frame's draws. Two pages (2048 R8 coverage, 1024 RGBA colour),
  `etagere` bucketed allocation, 1 px padding. Keys include font blob id, glyph id, size x 4,
  subpixel bin, variable-font coordinates, synthetic bold/skew, stroke width (x4; 0 for the fill). Strokes use swash's `Stroke`. Rasterized with `swash`. When a page
  fills, the whole atlas is cleared at the start of the next frame and visible glyphs re-rasterize.
- [images.rs](../crates/iron_render/src/ui/images.rs): `ImageCache` keyed by URL; failures are
  cached too so they are reported once. `url()` values are `data:` URLs (decoded with
  `decode_data_url`) or paths resolved by `iron_core::resolve_asset` against `set_asset_root` (the
  app sets it to the game folder). `UiRenderer::image_size(gpu, url)` loads through this cache so the app can report natural sizes for `image` elements. `tile_rect` implements `background-size`
  (`auto`/`cover`/`contain`/lengths) and `background-position`. `ImageSamplers` holds two repeating
  samplers, `smooth` (linear) and `nearest` (`image-rendering: pixelated`); each `LoadedImage` has a
  bind group for each (`bind_group`, `bind_group_nearest`), and `Step::Draw` picks one by its flag.

**SVG.** `.svg` URLs and `data:image/svg+xml` URLs (inline `<svg>` elements) are rasterized by resvg at the tile's physical size (`ImageCache::svg_at`, sizes bucketed to 8 px) and cached; `data:` SVGs are evicted after 120 unused frames (`begin_frame`). See [svg.md](svg.md).

**Gradient tiling.** A gradient fill is laid out like an image: `tile_rect` (natural size = the box)
gives one tile from `background-size`/`position`, `repeat_tiles` its repeat axes (`round` rescales;
`space` gaps are not kept), and `gradient::build` is built for that tile. The instance carries the
tile in `p1` and the repeat axes in `params.y` (bit 1 = x, bit 2 = y); `shade` in quad.wgsl wraps the
pixel into the tile on repeating axes, skips the gradient outside the tile on others, and evaluates
`gradient_color` against the tile instead of the box.

## Offscreen and screenshot path

`OffscreenTarget::with_format(gpu, w, h, format)` creates a `RENDER_ATTACHMENT | COPY_SRC` texture;
`read_pixels` copies to a buffer with 256-byte row padding, maps it, and returns tightly packed RGBA8
(top row first). The app's `--screenshot` path ([main.rs](../crates/iron_app/src/main.rs)
`save_screenshot`) renders the draw list again into an `Rgba8UnormSrgb` offscreen target and writes
a PNG with `iron_render::testing::write_png`. It still opens a window (frames run until the target
count), unlike tests, which are fully headless.

## Image tint

`ImageFill.tint` and `DrawCmd::BorderImage.tint` (`Option<Rgba>`) are passed in the image
instance's `color1` as linear premultiplied RGBA; all zeros means no tint. `image_tint` in
quad.wgsl returns white for zeros, and `fs_image` multiplies the sampled premultiplied pixel by it
on both the tiled and the repeating path. Golden M3-GAME-01.

## Golden testing

See [testing.md](testing.md). In short: `assert_golden(id, w, h, &pixels, tolerance)` in
[testing.rs](../crates/iron_render/src/testing.rs) compares against `tests/golden/<id>.png`;
`IRON_UPDATE_GOLDEN=1` rewrites references. `shared_gpu()` returns one `Gpu` per test binary behind a
mutex because creating devices from parallel threads can deadlock some Vulkan layers.
