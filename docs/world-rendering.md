# World rendering

Code: [iron_render/src/world](../crates/iron_render/src/world/mod.rs) (the GPU pass),
[iron_app/world_view.rs](../crates/iron_app/src/world_view.rs) (cameras and planes from scripts),
[iron_app/panels.rs](../crates/iron_app/src/panels.rs) (world panels), the drawing and input routing
in [iron_app/main.rs](../crates/iron_app/src/main.rs), and
[rendering.ts](../js/engine/src/rendering.ts) on the script side. Milestones M5.a and M5.a2. Objects and
components are in [scene-and-objects.md](scene-and-objects.md); the 2D UI renderer is in
[rendering.md](rendering.md). Textures, sprites and glTF models are in
[assets-and-models.md](assets-and-models.md); particles are in [particles.md](particles.md).

The world holds lit meshes (built-in primitives, script meshes, model meshes), quads (planes, sprites,
world panels, particles, halos) and lights. There is no ECS or scene graph on the GPU side: each frame
the app builds a flat `WorldView` per camera.

## The world pass

`WorldRenderer` ([world/mod.rs](../crates/iron_render/src/world/mod.rs), shader `world.wgsl`) draws
`WorldView`s into a `RenderTarget`:

- `WorldView { camera: WorldCamera, viewport: [x, y, w, h], quads: Vec<WorldQuad>, meshes:
  Vec<MeshInstance>, lights: Vec<Light>, ambient, skybox, clip_plane, probes, fxaa, target_texture }`.
  The viewport is normalized, from the top-left of the target. `target_texture` names a render texture
  to draw into instead of the screen (see Render textures).
- `WorldCamera { view_proj, position, forward, orthographic, clear }`. `WorldCamera::new(world,
  projection, near, far, size, clear)` builds it from the camera's world matrix and a viewport `size`
  in pixels (for the aspect). `Projection` is `Perspective { fov_y_degrees }` or `Orthographic {
  half_height }`.
- `WorldQuad { model, color, texture, uv, additive, order, layer }`. The unit quad (-0.5..0.5 in x and y,
  facing +z) is placed by `model`. `color` is linear straight-alpha RGBA that multiplies the texture.
  `uv` is the part of the texture shown. `additive` adds light instead of covering (glows). `order` is
  the sorting order (see below). `layer` (0 to 31) is the object's layer, used by culling masks.
- `MeshInstance { mesh, model, color, emission, smoothness, metallic, unlit, texture, tiling, glow,
  cast_shadows, mirror, layer, maps, specular, mode, alpha_cutoff, double_sided }`. `mesh` is a
  built-in primitive id or a mesh set with `set_mesh`. `color` is linear straight-alpha RGBA;
  `emission` is light the mesh gives off (it lights nothing else); `smoothness` 0..1 is
  1 - roughness and `metallic` 0..1 is Unity's metallic workflow (see Shading); `unlit` ignores lights; `tiling` is scale (xy) and offset (zw), Unity's
  `mainTextureScale`/`mainTextureOffset`. `glow` is `MeshGlow::None`, `Additive` or `Beam`. The material fields are under Materials.
- `WorldTexture` is how quads and meshes name a texture: `Frame(i)` indexes the `textures` slice given
  to the render call (world panels, drawn fresh each frame, premultiplied alpha), and `Asset(id)` is a
  texture the renderer keeps (`set_texture`, straight alpha). `None` is plain colour. `Reflection(i)` is
  set by the renderer itself for a mirror (see Planar mirrors).

`render_views(gpu, target, views, textures)` first updates reflection probes and renders mirrors (below),
then calls `render_views_with`, which does one render pass per target:

1. The target is cleared to black. Parts no view covers stay black.
2. **Cameras draw in order**, which the app sorts by `depth` (lower first), so a higher camera draws
   over a lower one, like Unity's camera depth.
3. For each view, the pass sets the viewport and scissor to its pixel rect (`viewport_px`), then
   draws a fullscreen triangle with the **background fill** (`fs_fill`): the camera's `clear` colour,
   or the sky along each pixel's view ray when the view has `skybox` and there is a sky, replaces the
   colour in the viewport (no blending, depth compare `Always`) and writes depth 1.0. So a later
   camera fully replaces what was under its viewport, and its depth starts fresh.
4. **Opaque meshes** (not `blended()`: no glow, and mode `Opaque` with colour alpha 1, or `Cutout`) are sorted by `(mesh, texture)` and drawn instanced:
   consecutive instances of one mesh and texture are one draw call. Depth test `Less`, depth writes on.
5. **See-through things** (blended meshes: `Fade` or `Transparent` mode, alpha below 1 or a glow; and all quads) are drawn after, back to
   front in one list: first by `order` (lower first; meshes are order 0), then by distance. In a
   perspective view distance is straight-line (squared) to the camera; in an orthographic view it is
   the distance along the view direction, so everything at one depth is equally far, like Unity's 2D
   sorting. Blending is premultiplied alpha.
   - Quads and see-through meshes depth test with `LessEqual` and write depth, so layered sprites at the
     same z do not fight. Culling is off: planes show from both sides.
   - **Glow** (additive quads, `MeshGlow::Additive`, `Beam`) uses its own pipelines with **no depth
     writes**: they light what is behind them and hide nothing. A quad's colour is stored
     premultiplied with zero coverage, so blending just adds it.

Consecutive items with the same mesh or texture and kind draw in one call.

Only the screen's last pass is the **final pass** (`render_views_with(..., final_pass)`): bloom, exposure,
tonemapping and FXAA run there. Mirror, probe and render texture renders are ungraded and unsmoothed.

One quad storage buffer and one mesh-instance buffer hold every view's data. A uniform buffer holds a
`GpuGlobals` per view (`view_proj`, background, shadow matrix and settings), picked with a dynamic
offset, and a lights buffer is bound per view. Group 1 is the material (`material_group`, one bind group
per `MaterialKey`): the main texture and sampler, then the normal, metallic, occlusion, emission, detail
and reflection textures (8 bindings; missing maps bind a 1 texel default). Group 2 holds the shadow
arrays, the sky cube and the shadow matrices. Buffers grow to a power of two. `GpuGlobals` also holds
`inv_view_proj` and the viewport (for the skybox), `env` (sky present, intensity, last mip, ambient from
sky), `flags` (draw the sky; target size) and `clip` (the clip plane). Per mesh instance, `GpuMesh`
carries `metallic` and the mirror flag (in `params`) and the probe it uses. The depth textures are
`Depth32Float`, kept per size in a map (the screen, mirrors and probe faces; cleared when it holds
more than 8). `viewport_size(rect, size)` gives a
viewport's pixel size, for a camera's aspect. `render(...)` is the single-camera shortcut.

A 0x0 target returns early.

## Meshes and lights

**Meshes.** [mesh.rs](../crates/iron_render/src/world/mesh.rs): `MeshData { positions, normals, uvs,
tangents, indices }`. Meshes are right-handed, y up, counter-clockwise front faces, with Unity's sizes (a 1 m
cube, a sphere of radius 0.5, a 2 m capsule, a 10 m plane). `primitive(name)` builds `cube`, `sphere`,
`capsule`, `cylinder`, `plane` and `quad`; every renderer registers them at `PRIMITIVE_BASE +
index` (`primitive_id`). Script and model meshes use ids below `PRIMITIVE_BASE`. `MeshData::complete`
makes up smooth normals, zero UVs and tangents when they are missing (tangents point along increasing u,
`w` gives the bitangent's side, `M5-MAT-01`), and `is_valid` refuses indices that are out
of range or not whole triangles. `beam()` is the open cone for spot-light beams (`BEAM_MESH`). Mesh UVs
run top-down; the shader flips them so tiling works bottom-up like Unity's.

**Shading.** `vs_mesh`/`fs_mesh` in `world.wgsl`. The surface colour is `texture * colour`; opaque meshes
ignore the texture's alpha, see-through ones use it. Shading is metallic-roughness PBR, Cook-Torrance
like Unity's Standard shader:

- `roughness = 1 - smoothness` (clamped to at least 0.045), `f0 = mix(0.04, colour, metallic)`, and the
  diffuse albedo is `colour * (1 - metallic)`. So a metal has tinted reflections and no diffuse.
- Every light in the view adds `(kd + spec * PI) * light colour * N.L`. `spec` is `ggx` (normal
  distribution) times `smith` (Schlick geometry, Unreal's k for direct light) times `fresnel`
  (Schlick) over `4 N.V N.L`; `kd = (1 - fresnel) * albedo`. Diffuse is not divided by pi (light
  intensities are Unity's), so the specular is scaled up by pi to match.
- `LightKind::Directional` (parallel rays along `direction`), `Point` (fades out at `range`) and `Spot`
  (a cone with soft edges between an inner and outer angle) feed that sum.
- Light from all around comes from the environment (below): `albedo * ambient_light(n)` for the diffuse
  and `env_brdf_approx(f0, roughness, N.V) * reflected` for the specular. `env_brdf_approx` is Karis's
  analytic fit of the split-sum environment BRDF, so there is no lookup texture.
- `emission` is added after lighting. Back faces are lit with the flipped normal.
- Shadows scale each light's `N.L` term (see Shadows).

### Skinned meshes

`MeshData` can also hold `joints` and `weights` (four per vertex). A mesh with them gets a second vertex
buffer of `SkinVertex { joints: [u32; 4], weights: [f32; 4] }` (attributes 4 and 5). `MeshInstance::bones`
is `Some((first, count))` for a skinned renderer, pointing into `WorldView::bones`: one matrix per bone,
the bone's world matrix times its inverse bind pose, made each frame by `WorldViews::views` from the
renderer's `Skin` ([animation-3d.md](animation-3d.md#skinned-meshes) has the script side).

- The main pass has four extra pipelines (`skinned`: opaque or see-through, one- or two-sided) using
  `vs_mesh_skinned` with the normal `fs_mesh`. Bone matrices go in a storage buffer (binding 4 of the
  mesh group). Every view's bones are appended in one list, each view after the earlier ones; the
  instance's `maps.w` carries the first bone plus one (0: a rigid mesh).
- `skin_matrix` blends the four bone matrices by weight. The object's own `model` matrix is ignored:
  bones are already in world space.
- The shadow pass has `vs_shadow_skinned` and its own bones buffer, so skinned casters bend the shadow.
  Skinned cut-outs cast solid shadows. Skinned and rigid casters are drawn in separate runs.
- Skinned meshes are never frustum culled: they move away from their mesh bounds.
- The golden is `M5-RND-21`.

### Materials

`MaterialMaps` and the other `MeshInstance` fields follow Unity's Standard shader. Maps are
`WorldTexture`s, each optional:

- **Normal map** (`normal`, `normal_scale`): tangent space, green up, read as linear data. `vs_mesh` builds
  the tangent frame from the mesh's tangents; `fs_mesh` perturbs the normal.
- **Metallic and smoothness** (`metallic_gloss`): metallic in red and smoothness in alpha. The material's
  metallic is replaced by the map's and its smoothness scales the map's. With `gltf_packing` it is read
  the glTF way: occlusion in red, roughness in green, metallic in blue, multiplying the factors.
- **Occlusion** (`occlusion`, `occlusion_strength`): darkens sky light and reflections only, not direct light.
- **Emission** (`emission`) multiplies the emission colour.
- **Detail** (`detail`, `detail_tiling`) is multiplied over the albedo (x2 in gamma space, so mid grey
  changes nothing) with its own tiling.
- **Specular workflow**: `specular: Some(rgb)` is the head-on reflectance, `metallic` is ignored, the
  albedo is darkened by the specular's strength, and the metallic map is read as sRGB colour (rgb) plus
  smoothness (a). `MaterialKey.metallic_srgb` picks that view.
- **Rendering mode** (`RenderMode`): `Opaque` (a colour alpha below 1 still draws as `Fade`), `Cutout`
  (holes where alpha is below `alpha_cutoff`; the shadow pass has its own entry points,
  `vs_shadow_cutout` / `fs_shadow_cutout`, so shadows get the holes), `Fade` (everything fades) and
  `Transparent` (highlights and reflections stay, for glass). `double_sided` draws back faces too.

Textures keep two views: `Rgba8UnormSrgb` for colours and a linear `Rgba8Unorm` view of the same data
for maps, chosen per binding. A texture's `srgb` flag (false for `linear` textures) also makes the CPU
mip chain average in linear space. Samplers are keyed by filter, wrap and anisotropy (`SamplerKey`).

**Lights from scripts.** `Light` sends `render.light component object type [r g b] intensity range
spotAngle innerSpotAngle haloSize beamIntensity shadows(none|hard|soft) shadowStrength`. Colour is
multiplied by intensity before it reaches `Light`. A light shines along its object's -z, like
Unity's. `RenderSettings.ambientLight` sends `render.ambient` (the default is a dim blue-grey).

**Halos.** With `haloSize` above 0 the app adds an additive quad at the light that faces each camera,
using the soft spot texture (`DEFAULT_PARTICLE_TEXTURE`) in the light's hue at steady brightness. It is
pulled half its size towards the camera, so nearby walls do not slice it in two.

**Beams.** With `beamIntensity` above 0 on a spot light, the app adds a `BEAM_MESH` instance with
`MeshGlow::Beam`: a cone as long as the range and as wide as the outer angle, unlit and additive. The
shader (`m.params.y > 1.5`) fades it where the surface turns away from the eye and towards the open end.

## Sky and environment

`WorldRenderer::set_sky(Option<Sky>)` and [environment.rs](../crates/iron_render/src/world/environment.rs)
with `env.wgsl`. `Sky { zenith, horizon, ground, sun_direction, sun_color, sun_size }` is a procedural
sky in linear HDR colours (the sun is usually brighter than 1). Scripts set it with
`RenderSettings.skybox` (`ProceduralSky`, all fields optional), which sends `render.sky`; `null` removes it.

- **The map.** When the sky changes, the next render redraws it (`redraw_sky`) into a fresh `EnvMap`: a
  128-texel `Rgba16Float` cube with a full mip chain. `EnvPass::draw_sky` draws `fs_sky` into each face
  (`sky_color`: horizon-to-zenith gradient, a soft step down to the ground, a sun disc with a soft edge and
  a glow) and `blur_mips` fills mips 1.. with a 4-tap box from the mip above, each face on its own (seams
  blur a little differently at face edges). Without a sky the renderer holds a 1-texel black cube, and
  `env.x` is 0.
- **Background.** A camera with `clearFlags: "skybox"` (`render.camera` last argument `skybox`; the
  default `solidColor` sends `color`) sets `WorldView.skybox`. `fs_fill` unprojects each pixel with
  `inv_view_proj` and samples mip 0 along that ray. With no sky it falls back to the camera colour.
- **Reflections.** `environment(dir, roughness, ...)` samples the cube along the reflected view
  direction at `lod = sqrt(roughness) * last_mip`, so rough surfaces see the blurry mips. Without a sky it
  returns the view's ambient colour. Everything is multiplied by `environment_intensity`
  (`RenderSettings.reflectionIntensity`).
- **Sky light.** `ambient_light(n)` samples the blurriest mip (about the average sky) along the normal
  instead of the flat `ambient` colour, when there is a sky and `ambient_from_sky` is on
  (`RenderSettings.ambientMode`, `"skybox"` by default; `"color"` keeps `ambientLight`). The app copies
  `sky` and `environment` from `WorldViews` to `WorldRenderer` before each draw (`main.rs`).
- The cube is bound in group 2 with the shadow map; only a probe changes it (see Reflection probes).

## Planar mirrors

A `Mirror` component on an object with a `MeshRenderer` sends `render.mirror component object [nx ny nz]`
(the local normal: +y for planes, +z for quads). `WorldViews::views` turns it into
`MeshInstance.mirror = Some(Mirror { normal, point })` in world space (the object's matrix applied to the
normal and its origin).

`render_views` handles mirrors before the real pass. The **main view** (biggest viewport) is the only one
reflected:

1. For each mirror mesh in it, `reflection_view` clones the view with: the camera moved by the
   reflection matrix `x - 2 (n . x + d) n`; a clip plane `[n, d + 1e-3]` so only what is in front of the
   mirror is drawn (the hair hides the seam); the projection flipped left to right (`flip * view_proj *
   reflect`) because a reflection reverses triangle winding and the flip reverses it back, so culling
   still works; and all mirror meshes removed (no mirrors in mirrors).
2. `render_views_with` draws that view into its own `Rgba16Float` texture (`reflection_target(k)`,
   one per mirror, the size of the target; bloom is paused). The clip is `clipped()` in the shader, a
   `discard` for any fragment with `dot(clip.xyz, p) + clip.w < 0`.
3. The mirror mesh gets `WorldTexture::Reflection(k)` and `params.w = 1`. In `fs_mesh` its colour is
   white times the mesh colour (the tint), and the specular `reflected` term is read from that texture at
   the **screen position**, mirrored in x within the viewport (`viewport.x * 2 + viewport.z - pos.x`),
   because the reflection was rendered flipped. The usual Fresnel and metallic terms still apply.
4. Other views have `mirror` cleared, so they see the sky (or probe) in the mirror instead.

Each mirror costs one more render of the scene. The mirror must be a flat plane through the object's origin.

## Reflection probes

A `ReflectionProbe` component sends `render.probe component object [sx sy sz] resolution boxProjection
everyFrame` (the size is halved into `half_extents`; resolution is clamped to 16..1024). `WorldViews::views`
gives every camera the active probes as `ReflectionProbe { id, position, half_extents, resolution,
box_projection, render }`. `render` is true when `everyFrame` is on or the probe was asked for:
adding or re-sending the op and `renderProbe()` (`render.probeRender`) put it in `probe_requests`, and
the app clears that set after each draw (`probes_rendered`). So `refreshMode: "viaScripting"` renders
when added and on `renderProbe()`.

- **Capture.** `update_probes` runs first in `render_views`, using the main view's probe list. Probes no
  longer listed lose their cubemap. A probe is captured when it has no map of the right size or `render`
  is set: six views of the main view's world from `position` (90 degree perspective, near 0.05, far 1000,
  `face_camera(face)` for direction and up) are each drawn into one shared `Rgba16Float` 2D target per
  face size. `EnvPass::copy_face` copies it into the cube face, flipping x (`fs_copy_flip`: a camera's
  image and a cube face are mirror images). Then `blur_mips` fills the chain. Probe views have no probes
  and no mirrors, so they see the sky there.
- **Which probe a mesh uses.** `probe_for`: the smallest-volume probe with a captured map whose box holds
  the mesh's centre (`model.w_axis`). No probe: the sky. The choice goes into `GpuMesh.probe` and
  `probe_box` (centre and mip count, half extents and box-projection flag). Runs of opaque meshes are
  split by probe id, so each run binds that probe's cube in group 2 (`lighting_group`, one bind group per
  probe map).
- **Box projection.** With `box_projection`, `environment` intersects the reflected ray with the probe's
  box from the shaded point and samples the cube towards that hit as seen from the probe centre, so walls
  line up and floors look grounded. Without it the probe acts as infinitely far.
- **Sampling.** `lod = sqrt(roughness) * (mips - 1)` of the probe's cube, times `environment_intensity`.
  A probe replaces only the specular reflection; diffuse sky light still comes from the sky.

## Shadows

[shadows.rs](../crates/iron_render/src/world/shadows.rs). `WorldRenderer::draw_shadows` plans and draws
all shadow maps **once a frame**, from the main screen view (the biggest viewport, not a render
texture's). Every view drawn that frame reads the same maps: other cameras, mirrors, probe faces and
render textures. Casters are the main view's opaque and cutout meshes with `cast_shadows` on (cutout
casters keep their holes).

`shadows::plan(camera, lights, settings)` returns a `ShadowPlan`: a `ShadowSlot` per casting light (a light
with `Shadows { soft, strength }`, strength above 0), a matrix per map, and texel sizes for the normal
offset.

- **Sun.** The first directional light with shadows gets `shadow_cascades` (1 to 4) cascades in a
  `Depth32Float` array of `shadow_resolution` texels. With 2 or more, the main camera's view out to
  `shadow_distance` is split into slices that grow with distance, each fitted with a sphere and an
  orthographic map, moved in whole texels so edges do not crawl. One cascade is a single sphere that
  distance across in front of the camera.
- **Spot and point lights.** A spot light gets one perspective map (a little wider than its cone), a
  point light a cube of six 90 degree maps (faces +x, -x, +y, -y, +z, -z). They share a second array at
  half the sun's resolution with at most `MAX_LOCAL_LAYERS` (24) layers. Lights are taken nearest the
  camera first and a light that no longer fits goes without. The maps reach the light's `range`.
- `ShadowArray` holds each texture array: one view to sample, one per layer to draw into. Arrays are
  recreated when the size or layer count changes. `GpuLight.shadow` carries the slot's first matrix
  index, strength, filter radius and texel size; `shadow_matrices` is a storage buffer in group 2.
- **Shader.** `sun_shadow` takes the first cascade whose map holds the point (offset along the normal by
  1.5 texels); `local_shadow` picks the cube face from the direction to the light (the offset grows with
  distance). Both use `pcf`, a (2r + 1)^2 box of comparison samples, and mix the result with 1 by the
  strength. Outside every map a point is lit.
- `WorldStats::shadow_maps` counts the maps drawn in the frame (`M5-RND-20` checks 4 + 1 + 6).
- Scripts set the distance, size and cascades with `QualitySettings.shadowDistance` (default 25 m),
  `shadowResolution` (default 2048, 256 to 8192) and `shadowCascades` (1, 2 or 4; Low 1, Medium 2, High and
  Ultra 4): `render.shadowSettings 0 distance resolution cascades`. The app copies them to `WorldRenderer`
  before each draw.

Each `MeshInstance` has `cast_shadows` (`MeshRenderer.shadowCastingMode`, `on` / `off`): opaque runs are
grouped by it too, and the shadow pass draws only the casting ones.

## Bloom and post-processing

[bloom.rs](../crates/iron_render/src/world/bloom.rs) and `bloom.wgsl`, [fxaa.rs](../crates/iron_render/src/world/fxaa.rs).
Post effects run only in the screen's **final pass**.

**Bloom.** `RenderSettings.bloom` (`{ threshold, intensity, scatter }`, default off) sends `render.bloom`; the
app sets `WorldRenderer::bloom`. With bloom on (`intensity` above 0), `render_views` draws the world into an
`Rgba16Float` texture (`HDR_FORMAT`), so emission and hot lights can be brighter than white.
`BloomPass::encode` picks the bright parts above `threshold` (with a soft knee), blurs them down a chain
of half-size textures (at most 7 levels, fewer for small targets) and back up. `scatter` sets how far the
blur spreads. Pipelines are cached per colour format and the textures per size.

**Exposure and tonemapping.** `Grading { exposure, tonemapping }` is applied after bloom by `fs_grade`
as the HDR texture is copied to the target. `Tonemapping` is `None` (clipped at white), `Neutral`
(Khronos PBR Neutral: colours kept until a soft shoulder) or `Aces` (filmic). Exposure multiplies the light
by 2 to the stops. `RenderSettings.tonemapping` (`"none"`, `"neutral"`, `"aces"`) and `postExposure` (stops,
-10 to 10) send `render.post 0 postExposure tonemapping`; the app sets `WorldRenderer::grading`. A
non-identity grading also draws through the HDR texture.

**FXAA.** `Camera.antialiasing = "fxaa"` (camera op argument 14) sets `WorldView.fxaa`. If the main view
has it, `FxaaPass` runs over the finished, tonemapped image: it finds edges by gamma-space luma contrast
and blends along them. It is cheaper than MSAA and smooths texture and shading edges too, a little
blurrier. Its textures are cached per size.

## Render textures

A `RenderTexture(width, height)` ([textures.ts](../js/engine/src/textures.ts)) is a `Texture` that
cameras draw into. It sends `render.renderTexture id width height` (`setSize` sends it again; up to 16384
texels); the app calls `WorldRenderer::set_render_texture`, which makes a `RENDER_TEXTURE_FORMAT`
(`Rgba8UnormSrgb`) texture kept under that id. Materials, sprites and quads show it like any asset
texture (`WorldTexture::Asset(id)`). `Texture.destroy()` removes it.

`Camera.targetTexture` (camera op argument 15, `null` clears it) sets `WorldView.target_texture`. A camera
with a target texture is not `Camera.main` and is not a screen view.

`render_views` draws in this order: shadows, then the render textures (`render_into_texture`, each
texture's cameras in depth order, with the world's lights and sky but no bloom, grading or FXAA), then the
screen. So the screen sees this frame's picture. A camera cannot sample its own texture: in its own
views, things showing it draw plain colour; other render textures show their latest picture. Scenes save
`RenderTexture` (size) like other textures (`M5-RTX-01`).

## Screen UI over the world

`UiRenderer::render_overlay(gpu, target, cmds)` ([ui/mod.rs](../crates/iron_render/src/ui/mod.rs)) is
`render` with two changes: the canvas starts **transparent** instead of cleared, and its final blit
**blends** (premultiplied alpha) onto what the target holds, with `LoadOp::Load`. The blit pipeline is
cached by `(format, overlay)`.

That canvas holds only UI, so backdrop filters and blend modes there would see no world. When the
draw list has either (`needs_backdrop`: a `BackdropBlur`, or a `PushLayer` with a blend mode),
`UiRenderer::render_over(gpu, target, backdrop, cmds)` is used instead: the world is drawn into
`backdrop_target` (a cached texture of the target's size and format that can be sampled, unlike a
swapchain image), the canvas is **seeded** with a copy of it (a blit into canvas 0 in place of its
clear), the UI is drawn as usual, and the final blit **replaces** the target's pixels. Frames
without those effects keep the cheaper overlay path.

`draw_frame` in `main.rs` picks the path:

- No active camera: `UiRenderer::render` over the plain `CLEAR` colour, as before M5.a.
- UI that reads the world: `render_views` into `backdrop_target`, then `render_over`.
- Otherwise: `WorldRenderer::render_views`, then `render_overlay`.

Screenshot runs and window frames both go through `draw_frame`.

## Components from scripts

`Camera`, `PlaneRenderer`, `MeshRenderer`, `Light`, `SpriteRenderer`, `ParticleSystem`, `Mirror`,
`ReflectionProbe`, `LODGroup` and `WorldPanel` are components in `@iron/engine` ([rendering.ts](../js/engine/src/rendering.ts),
[textures.ts](../js/engine/src/textures.ts), [particles.ts](../js/engine/src/particles.ts)). They send
`render.*` ops keyed by the component id, and `WorldViews`
([world_view.rs](../crates/iron_app/src/world_view.rs)) stores them in one map per kind.

| Op | Arguments |
|---|---|
| `render.camera` | `component object fov near far [r g b a] depth orthographic orthographicSize [x y w h] clearFlags(skybox\|color) cullingMask [32 cull distances]\|null cullSpherical` |
| `render.plane` | `component object [w h] [r g b a]` |
| `render.mesh` | `id [positions] [normals] [uvs] [indices] [joints, 4 per vertex] [weights, 4 per vertex]` (a script mesh; `id null` removes it) |
| `render.meshRenderer` | `component object mesh(name \| id) [r g b a] [er eg eb] smoothness unlit texture\|null [scale x y, offset x y] castShadows metallic ... [bone objects]\|null [bind poses, 16 each]` (the last two for `SkinnedMeshRenderer`) |
| `render.light` | `component object type [r g b] intensity range spotAngle innerSpotAngle haloSize beamIntensity shadows shadowStrength cullingMask` |
| `render.sprite` | `component object texture [x y w h]\|null [pivot x y] pixelsPerUnit [r g b a] flipX flipY sortingOrder` |
| `render.particles`, `render.particleControl` | see [particles.md](particles.md) |
| `render.texture`, `render.textureRemove`, `render.model` | see [assets-and-models.md](assets-and-models.md) |
| `render.ambient` | `0 [r g b]` |
| `render.sky` | `0 [zenith rgb] [horizon rgb] [ground rgb] [sun dir xyz]\|null [sun rgb] sunSize intensity ambientFromSky`, or `0 null` for none |
| `render.mirror` | `component object [nx ny nz] cullingMask` (local plane normal) |
| `render.probe`, `render.probeRender` | `component object [sx sy sz] resolution boxProjection everyFrame cullingMask`; `component` (render again next frame) |
| `render.bloom` | `0 threshold intensity scatter`, or `0 null` for off |
| `render.quality` | `0 antiAliasing renderScale shadows(disable\|hardOnly\|all) softRadius lodBias vSyncCount` (see Quality settings) |
| `render.lodGroup` | `component object size [heights per level] [[renderer ids] per level] forcedLevel(-1: none)` |
| `render.shadowSettings` | `0 distance resolution` |
| `render.panel` | `component object [w h] pixelsPerMeter resolution [r g b a] interactive` (handled by `Panels`) |
| `render.remove` | `component` (removes a camera, plane, mesh renderer, light, sprite, particle system, mirror, probe or panel) |

The component re-sends its op when a property changes. The defaults are a dark clear colour, `depth`
0 and a full-screen `rect`. A bad op (unknown primitive, mesh with bad indices) logs a warning.

`MeshRenderer` takes a primitive name or a `Mesh` and a `Material` (`color`, `emission`, `smoothness`,
`metallic`, `unlit`, `mainTexture`, tiling; `Material.create` and `with` take named settings, and
`withColor` and `withTexture` make copies; see [assets-and-models.md](assets-and-models.md#materials-and-meshes)). `Mesh.create` sends script
data. `GameObject.createPrimitive(type)` ([primitives.ts](../js/engine/src/primitives.ts)) makes an
object with the matching mesh and collider.

`WorldViews::views(scene, size, extra)` runs every frame, after `update_world`:

- Keeps cameras whose object is active, sorted by `depth` (a stable sort, so ties keep the order they
  were added).
- Takes only the position and rotation from a camera's world matrix. Scale is ignored.
- Builds the shared lists once: planes, then sprites (only once their texture size is known), then
  `extra` (the panel quads) become quads; mesh renderers plus spot-light beams become `MeshInstance`s;
  lights become `Light`s; halos are kept as positions.
- Builds the probe list (active probe objects) and each mirror's world plane, and sets `skybox` from
  the camera's `clearFlags`.
- Gives each camera a `WorldView` with clones of those lists, plus per-camera quads: the particles'
  quads and the halos, built facing that camera's rotation.
- A plane lies flat in its object's XZ plane facing +y, like Unity's Plane (`lay_flat`).
- With no cameras it returns an empty list, so only screen UI is drawn.

Components on destroyed objects are dropped by `forget_objects`.

`CameraFollow` and `CameraOrbit` are script components that move a camera object in `onLateUpdate`.
They have no Rust side.

## Coordinates

- Right-handed, +y up. A camera looks down its **-z**. Projections have depth 0 to 1
  (`glam::camera::rh::proj::directx`), and `Matrix4x4` in `@iron/engine` matches, so a camera's matrices
  in TypeScript equal what the renderer uses.
- A quad faces +z. A panel's top-left corner is at (-w/2, +h/2) in its object's space.
- **Screen points are top-left UI pixels**: the window's pixels divided by the DPI scale and the UI
  scale. These are the units of `Screen.width` and `height` and of pointer event `clientX/Y`.
  Viewport points are 0 to 1 from the top-left, and NDC y is flipped (`1 - v * 2`).
- `Camera` conversions in TypeScript: `screenToViewportPoint`, `viewportToScreenPoint`,
  `viewportPointToRay`, `screenPointToRay`, `worldToViewportPoint`, `worldToScreenPoint`,
  `viewportToWorldPoint`, `screenToWorldPoint`. `z` is the distance in front of the camera. A camera's
  `rect` maps the screen to its viewport. `pixelRect` and `aspect` follow `Screen`. `Camera.main` is the
  first active camera added, and `Camera.allCameras` lists them in draw order.

## World panels

A `WorldPanel` component puts React UI on a plane. In a script:

```ts
const panel = menu.addComponent(WorldPanel, { width: 640, height: 400, pixelsPerMeter: 170 });
createRoot(panel).render(<Menu />);
```

**Commit targets.** `createRoot(target)` ([react.md](react.md#createroot)) gives the panel its own React
root whose commits carry the component id: `__iron_commit(ops, target)`. QuickJS stores each commit
with its target, and `ScriptRuntime::drain_targeted_commits` returns `(target, Commit)` pairs. Target 0
is the screen. `App::apply_script_output` routes each commit to the screen tree or to
`Panels::get_or_create(target, ...)`, which makes the panel's tree on the first commit (commits can
arrive before `WorldPanel.onStart` sends `render.panel`).

**Panel trees.** Each `Panel` has its own `UiTree` and `Interaction` (hover, focus, scroll state):

- `new_panel_tree` builds the tree with the system fonts, replays every font the scripts loaded
  (`font_sources`) and adds every stylesheet (`sheet_sources`: `game.css` under the key `GAME_CSS`, and
  `addStyleSheet` sheets under their script ids). Later `loadFont`, `addStyleSheet` and
  `removeStyleSheet` calls update the screen and all existing panels, so every tree has the same CSS.
- Size, `pixelsPerMeter` (default 500), `resolution` (default 2, clamped 0.25 to 4) and `tint` come
  from `render.panel`. Size is clamped to at least 1 UI pixel.

**Textures.** `Panels::render` runs every frame before the world pass. For each active panel it sets
the tree size and clock, loads images, builds the draw list and draws it with
`UiRenderer::render` into the panel's own `Rgba8UnormSrgb` texture (cleared transparent). The texture
is `size * resolution` pixels, capped at 4096, and is recreated when that changes. It returns one
`WorldQuad` per panel (model = the object's world matrix scaled to the size in metres, `color` = tint)
and the texture views. The quads go to `WorldViews::views` as `extra`, so every camera sees them.
Panels on inactive objects are skipped, and `forget_objects` drops panels on destroyed objects.

**Input routing.** `Surface` is `Screen` or `Panel(id)`. `App` keeps `pointer_on`, `keys_on`,
`buttons_down` and `pointer_px`, and `handle_on(surface, input)` runs one `Input` on that surface's
tree and interaction.

- `route_pointer` runs on mouse move. With no button down it asks `surface_at(px, py)`; the surface
  that is left gets `PointerLeave`, the new one gets `PointerMove`.
- `surface_at`: if the **screen UI** has an element at the point, it wins. Otherwise
  `camera_ray(px, py)` casts a ray, `Panels::hit` picks the nearest interactive panel it meets, and
  the point becomes the hit in that panel's UI pixels. Otherwise it is the screen.
- **`pointerEvents: none` rule.** Screen UI wins wherever `UiTree::hit_test` finds an element other
  than the root. A full-screen layout box (a HUD wrapper) would therefore hide every panel behind it.
  Layout boxes that cover the screen must set `pointerEvents: "none"`; children with their own
  handlers still get events.
- A drag stays with the surface it started on. While a button is down, the position is projected onto
  that panel's plane with `Panels::project` (outside its bounds too), or `(-1e6, -1e6)` if the ray
  misses.
- Press, release, wheel and cursor go to `pointer_on`. The cursor shape and image come from that
  surface's tree. A press sets `keys_on`, so keys, text and IME go to the surface clicked last (and the
  `focus` op also sets it, through `surface_of(node)`). `Modifiers` and `WindowBlur` go to every
  surface. Each panel gets `Input::Refresh` per frame.
- IME is allowed when the focused surface has an editor. For a panel the candidate-window area is not
  set.
- `camera_ray` takes the last view whose viewport contains the point (the camera drawn on top), then
  unprojects the near and far NDC points through the inverse `view_proj`. `ray_quad` intersects the
  ray with the quad plane in the quad's own space and returns `(u, v, distance)` with (0, 0) at the
  top-left. `None` means parallel or behind.

## Layers and culling

Every GameObject has a `layer` from 0 to 31 (`GameObject.layer`, op `scene.layer id n`, default 0;
`ScriptScene::layer` reads it, 0 for unknown ids). `LayerMask`
([layers.ts](../js/engine/src/layers.ts)) names layers: Unity's built-ins (0 Default, 1 TransparentFX,
2 Ignore Raycast, 4 Water, 5 UI) plus the game's `package.json` `iron.layers` (`{ "8": "Player" }`,
passed on by `info.rs`). A mask is a `u32`, a bit per layer; scripts may use signed masks (`~mask`),
which are made unsigned before they cross the op boundary. Missing or null masks mean every layer
(`ALL_LAYERS`).

**What a mask filters.**
- **Camera** `cullingMask`: the camera draws only meshes, planes, sprites, panels, particles and halos
  on those layers. `layerCullDistances` (32 numbers, 0: out to the far plane) hides a layer's things
  beyond a distance, measured along the view, or straight from the camera with `layerCullSpherical`.
  The app does this in `WorldViews::views` with `Camera::shows(layer, at, position, forward)`, using
  each object's origin. Every list is held as `Tagged { layer, at, item }` and filtered per camera,
  so the `WorldView`s already hold only what that camera draws. Panels return `(object, quad)` pairs
  so their quads carry the panel object's layer.
- **Light** `cullingMask`: `fs_mesh` checks the mask against each instance's `layer` (`in_mask`), so a
  light lights only those layers. The shadow-casting light's mask also picks which meshes cast its
  shadows.
- **Mirror** and **ReflectionProbe** `cullingMask`: the layers drawn into the reflection or cubemap.
  Set them before the component starts.

**Frustum culling** ([cull.rs](../crates/iron_render/src/world/cull.rs)) runs in `render_views_with`,
per view. `Frustum::from_view_proj` takes the six planes from the matrix (wgpu depth 0 to 1; works for
perspective and orthographic) and tests spheres and boxes. Meshes use the box of their mesh data
(`bounds`) moved by the instance matrix (`transform_aabb`); quads use `QUAD_BOUNDS`.
- Opaque runs get a `visible` flag. Opaque meshes out of view are skipped in the colour pass but
  **still drawn into the shadow map**, so a plank above the camera still shadows the floor in view.
  Runs are split by `visible` as well as by mesh and texture.
- See-through meshes and quads out of view are dropped before sorting.
- Point and spot lights whose range sphere misses the frustum are dropped; directional lights never
  are. The shadow light's index is looked up again in the kept list for each view.
- `WorldRenderer::stats()` returns `WorldStats` (meshes, quads and lights drawn and culled, summed
  over the views of the last `render_views` call), which the tests use.

**LOD groups.** `LODGroup` (`rendering.ts`: `size`, `setLODs([{ screenRelativeTransitionHeight,
renderers }])`, `forceLOD(i)`) sends `render.lodGroup`. Levels go from most to least detailed, and the
heights must go down. For each camera `views()` computes how tall the group looks: `Camera::screen_height(size, distance)`
is `size / (2 · distance · tan(fov / 2))` (perspective) or `size / (2 · half_height)` (orthographic),
where `size` is the group's size times the largest world scale of its object. That is multiplied by
`QualitySettings.lodBias` (`Quality.lod_bias`; Low 0.4, Medium 0.7, High 1, Ultra 2). The first
level whose height is at most that value is shown; below the last, nothing. The renderers of the other
levels are left out of that camera's `meshes` (`Tagged.renderer` holds the MeshRenderer id). A forced
level wins at any distance. Groups are removed with `render.remove` or when their object is destroyed.

`examples/layers` shows camera masks and cull distances (with a minimap camera that sees only map
markers), field of view and render distance, and LOD groups with `lodBias`. Light, probe and mirror
masks are covered by the `M5-RND-15` golden.

## Quality settings

`WorldRenderer` has the knobs; scripts set them through `QualitySettings`
(`js/engine/src/quality.ts`), whose levels (Low / Medium / High / Ultra, overridable in the game's
`package.json` under `iron.quality`) send `render.quality` (anti-aliasing, render scale, shadow mode,
soft filter, `lodBias`, `vSyncCount`, anisotropic filtering, texture mip limit) and `render.shadowSettings`
(distance, map size, cascades). The default level is applied when the
engine loads; the app copies the values onto the renderer before each frame.

- **MSAA** (`msaa`): the main world pass draws into a multisampled colour target (cached by size,
  format and count) with a multisampled depth buffer, resolving into the target (or the bloom HDR
  texture). Pipelines are cached by format and sample count. `supported_samples` lowers the count to
  what the GPU allows for the format and depth: without `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`
  (requested at device creation when the adapter has it) only WebGPU's guaranteed 1 and 4. Mirror
  and probe renders stay single-sampled.
- **Render scale** (`render_scale`, 0.25 to 2): `render_views` draws the whole world (probes,
  mirrors, bloom) into a texture of the scaled size (`upscale.rs`) and stretches it over the target
  with a bilinear blit. Screen UI is drawn afterwards at full resolution.
- **Vsync** (`vSyncCount`, 0 or 1; `Quality.vsync`): the app calls `WindowSurface::set_vsync` before each frame,
  which switches between `AutoVsync` and `AutoNoVsync`. With it off, `Application.targetFrameRate` caps the
  frame rate (see [platform.md](platform.md)). The quality choice is saved in `PlayerPrefs` as `iron.quality`
  and restored at startup.
- **Shadow quality** (`shadow_quality`: `Disable`, `HardOnly`, `All`; `soft_shadow_radius` 1 or 2):
  the shadow pass is skipped when disabled, soft shadows become one sample under `HardOnly`, and the
  soft filter is a (2r + 1)^2 box of comparison samples (the radius is in `GpuLight.shadow`).
- **Texture quality** (`anisotropicFiltering`: `disable`, `enable`, `forceEnable`; `globalTextureMipmapLimit`
  0 to 3): `QualitySettings` sends them in `render.quality` (`M5-QUA-02`). `Anisotropic` sets how
  each texture's `anisoLevel` (a Texture option, 1 to 16, trilinear only) is honoured: `Disable` makes it 1,
  `Enable` uses the texture's level, `ForceEnable` uses at least 9. The mip limit drops that many top
  levels of every texture. Samplers are cached per filter, wrap and anisotropy.

## Cooked texture uploads

`WorldRenderer::set_texture` builds the mip chain (`mip_chain`) and calls the shared upload with a
`CookedTexture { width, height, format, srgb, mips, filter, wrap, anisotropy }`. `TexelFormat` is `Rgba8`,
`Bc1` (8 bytes per 4 x 4 block) or `Bc3` (16). `set_cooked_texture` uploads such levels as they are, with
the matching wgpu format (the sRGB variant when `srgb`; a linear view format for the sampling path) and
whole-block row sizes. It returns false for data that fails `is_valid`, is too big, or is BC when the GPU
lacks `TEXTURE_COMPRESSION_BC` (`supports_bc`; `gpu.rs` requests it when the adapter has it). The app
then decodes the cooked texture and uses `set_texture`. How cooking makes them:
[assets-and-models.md](assets-and-models.md#cooking).

## Limits

- Only the first directional light with shadows casts sun shadows; spot and point lights share 24 layers,
  nearest first. Casters come from the main screen view's opaque and cutout meshes (no transparent casters).
- Bloom, exposure and tonemapping are one setting for every camera; FXAA follows the main view's camera.
  Mirrors, probes and render textures are ungraded.
- Mirrors are flat, rendered only for the main view, show no mirrors inside mirrors, and cost a full
  extra scene render each. Other views see the sky in them.
- Probes are captured from the main view's world without mirrors or other probes. A mesh uses one probe
  (by its centre), with no blending between probes; the cube mips are a box blur, not a GGX
  prefilter. The sky is one fixed 128-texel map, and only specular reflections use probes.
- There is no screen-space reflection.
- Skinning is four bones per vertex, in the vertex shader. Skinned meshes are never culled and a skinned
  renderer's bones are rebuilt every frame for each view.
- Frustum culling is conservative: meshes are tested by the bounding box of their mesh, quads by the unit
  quad, and nothing is occlusion culled. Cull distances are per layer, not per object.
- Panels re-render every frame while visible, with no dirty tracking.
- Panels are not occluded by other world quads for hit testing: the nearest interactive panel wins.
- Plane quads are not hit tested.
- Pointer events on a panel use UI pixels of the panel, not the screen scale.

## Tests

Goldens in [iron_render/tests/world.rs](../crates/iron_render/tests/world.rs): `M5-RND-05` (a floor
in perspective, a plane seen through a half-transparent one, a textured plane, and screen UI blended
over the view) and `M5-RND-06` (a perspective view, an orthographic top-down view and an inset
drawn in depth order). `M5-RND-07` (lit primitives), `-08` (textured meshes and sprites), `-09` (bloom, additive glows and a
beam) and `-10` (directional shadows) cover the M5.a2 work. The materials work adds `M5-RND-11` (a smoothness by metallic sphere grid under the
procedural sky, with the skybox as background), `M5-RND-12` (a planar mirror: reflected camera, clip plane
and screen-space sampling) and `M5-RND-13` (a box-projected reflection probe in a room). Later goldens: `M5-RND-17` (material maps and
rendering modes under a sky), `-18` (tonemapping, exposure and FXAA), `-19` (a render texture shown on a
monitor quad) and `-20` (cascades, a spot light and a point light; it also checks the shadow map count), and `-21`
(a tube skinned to two bones, bent, with a bent shadow). Cooked textures are tested with cooking
(`M5-AST-06`, see [assets-and-models.md](assets-and-models.md#tests)).
Unit tests: `M5-MAT-01` (tangents, `mesh.rs`) and `M5-SHD-01` (shadow planning, `shadows.rs`); JS units
`M5-MAT-02` (`Material` ops and scene saves), `M5-RTX-01` (render textures) and `M5-SHD-02` (cascades) in
`js/engine/test`.
`examples/materials` is the manual check. `M5-PNL-02` (rays and panel quads) is a unit test in `panels.rs`, `M5-PNL-01`
(per-root commits) is in `js/react/test/reconciler.test.tsx`, and `M5-CAM-01` (camera conversions) is
in `js/engine/test/camera.test.ts`. `examples/world` is the manual check (`M5-PNL-03`).
