# Textures, sprites and models

Code: [textures.ts](../js/engine/src/textures.ts), [models.ts](../js/engine/src/models.ts) and
[rendering.ts](../js/engine/src/rendering.ts) (`Mesh`, `Material`) on the script side;
[world_view.rs](../crates/iron_app/src/world_view.rs) (`texture_op`, `model_op`),
[models.rs](../crates/iron_app/src/models.rs) (glTF) and the `__iron_textures`/`__iron_models` calls in
[main.rs](../crates/iron_app/src/main.rs) in the app;
[world/texture.rs](../crates/iron_render/src/world/texture.rs) and
[world/mod.rs](../crates/iron_render/src/world/mod.rs) (`set_texture`, `set_mesh`) in the renderer.
How these are drawn is in [world-rendering.md](world-rendering.md).

## Textures

A `Texture` is an image the world draws with (not the UI's `image` element, which has its own path in
`iron_ui`). Scripts make one two ways:

- `Texture.load(src, { filterMode, wrapMode, mipmaps, anisoLevel, linear })` reads a PNG or JPEG from the game folder.
- `Texture.create(width, height, colors, options)` takes linear `Color`s, bottom row first like Unity's
  `SetPixels`. They are converted to sRGB bytes, top row first, before they cross.

Both send `render.texture id src filter wrap mipmaps [width height [pixels...]]`. `src` is `null` for
pixel data. Defaults: bilinear, repeat, mipmaps on, `anisoLevel` 1, `linear` false. `anisoLevel` (1 to 16,
trilinear only) sharpens grazing angles within what `QualitySettings.anisotropicFiltering` allows.
`linear: true` marks data (normal maps, masks) rather than colours: the renderer reads it without sRGB
decoding and averages its mips in linear space. `Texture.whenLoaded()` returns a promise.
A `RenderTexture` is a texture cameras draw into (see [world-rendering.md](world-rendering.md#render-textures)).

**Ids.** Scripts count textures up from 1. The engine's own (a model's) start at `1 << 30`
(`ENGINE_IDS`), so they never meet.

**Loading is synchronous in the app.** `WorldViews::texture_op` reads the file (a relative path, no
`..`, through `resolve_asset`; `data:` URLs work too), decodes it with the `image` crate to RGBA8 and
queues a `TextureData` in `texture_uploads`. `apply_script_output` collects `(id, width, height)` and
calls `__iron_textures([[id, w, h], ...])` after the batch of ops, which sets `width`, `height` and
`isLoaded` and resolves `whenLoaded`. A failure logs a warning and reports 0 x 0 (`failed`). The size
is also kept in `texture_sizes`, which sprites need before they can be laid out. `Texture.destroy()`
sends `render.textureRemove`, and things still using it draw the checker.

**Upload.** Before each draw the app moves `texture_uploads` into the renderer
(`WorldRenderer::set_texture`). Textures are `Rgba8UnormSrgb`, so sampling decodes to linear; a second
`Rgba8Unorm` view of the same data (`AssetTexture` keeps both) is used when a material reads the texture as
a map. With
`mipmaps` on, the CPU builds every level (`texture::half`): each texel averages up to 2 x 2 texels **in
linear light** (not encoded values), weighted by alpha so see-through texels do not darken their
neighbours. A texture over the GPU's maximum size is refused with a warning. Samplers are shared per
`(filter, wrap, anisotropy)` (`SamplerKey`): `Point` is nearest and crisp, `Bilinear` blends texels but not mip levels,
`Trilinear` blends both; wrap is `Repeat`, `Clamp` or `Mirror`.

**Alpha.** Kept textures hold **straight** alpha; the frame textures of world panels hold premultiplied.
Quads carry a flag (`params.x`) so the shader treats each right.

### The missing-texture checker and the default spot

Two textures exist in every `WorldRenderer`, made in `new`:

- `MISSING_TEXTURE` (`u32::MAX - 1`): a magenta and black 4 x 4 checker (`texture::missing`), crisp and
  repeating. A `WorldTexture::Asset(id)` whose texture was never set, failed to load or was removed is
  drawn with it, so missing assets stand out instead of passing as white. The check is in `render_views`
  (`self.textures.get(&id).map_or(missing, ...)`). Sprites of a texture that failed also show it: the
  app's `mark_missing` gives the failed id an 8 x 8 size so the sprite still lays out.
- `DEFAULT_PARTICLE_TEXTURE` (`u32::MAX`): a white spot fading smoothly to clear at its edge
  (`texture::soft_spot`). Particles and light halos use it when they have no texture of their own.

Neither can be removed.

## Materials and meshes

`Material(color, emission, smoothness, unlit, mainTexture, tiling, offset, metallic)` is a plain value,
Unity's Standard material (metallic workflow). `smoothness` 0..1 is 1 - roughness; `metallic` 0..1 (default
0) makes a metal: tinted reflections and no diffuse (the shading is in
[world-rendering.md](world-rendering.md#meshes-and-lights)). A `MeshRenderer` sends it with its mesh in
`render.meshRenderer` (the last arguments are `castShadows` and `metallic`); the texture goes by id (`null`:
plain colour).

`Material.create({ color, emission, smoothness, metallic, unlit, mainTexture, mainTextureScale,
mainTextureOffset })` builds one from named settings, and `material.with({...})` copies with some changed;
anything left out keeps its default or current value. `withColor` and `withTexture(texture, scale, offset)`
are shortcuts for `with`.

**Maps and modes.** `create` and `with` also take Unity's Standard shader settings: `normalMap` and
`normalScale`, `metallicGlossMap` (metallic in red, smoothness in alpha), `specularColor` (switches to the
specular workflow, and the map then holds specular colour and smoothness), `occlusionMap` and
`occlusionStrength`, `emissionMap`, `detailAlbedoMap` with its own `detailAlbedoMapScale` and
`detailAlbedoMapOffset`, `renderingMode` (`"opaque"`, `"cutout"`, `"fade"`, `"transparent"`, type
`RenderingMode`), `alphaCutoff` and `doubleSided`. Load normal and metallic maps with `linear: true`. They
travel in the last argument of `render.meshRenderer` (`_extras`, an array that is only sent when a value
differs from the default); the app reads it in `material_extras` into `MaterialMaps`, `RenderMode` and the
rest of `MeshInstance`. How they are shaded is in
[world-rendering.md](world-rendering.md#materials). Scenes save a material's extra settings only when
they differ from the defaults, and a texture's `linear` and `anisoLevel`.

`Mesh.create({ positions, normals, uvs, indices })` sends `render.mesh id ...`. `WorldViews::apply`
builds a `MeshData`, completes it (`complete`: smooth normals, zero UVs and tangents when missing), checks it
(`is_valid`) and keeps a copy in `mesh_data` (for mesh colliders) as well as queueing the upload.
`Mesh.set` replaces the data and `Mesh.destroy` removes it (`render.mesh id null`); renderers using a
removed mesh draw nothing. The renderer also keeps a mesh's buffers by id (`set_mesh`, `remove_mesh`).

**Mesh colliders from render meshes.** A `MeshCollider` with `sharedMesh` sends a `physics.collider` op
whose shape is `renderMesh [mesh id, convex 0/1, scale x y z]`. The app rewrites it in
`WorldViews::resolve_mesh_collider` into a `mesh` or `convex` shape holding that mesh's (scaled)
positions and indices from `mesh_data`, then hands it to physics. So scripts never send big arrays
twice.

## Sprites

`Sprite.create(texture, rect, pivot, pixelsPerUnit)` names part of a texture: `rect` in pixels from the
texture's bottom-left (all of it when null), `pivot` 0..1 across the rect from its bottom-left (where
the object's origin sits), `pixelsPerUnit` texels per metre (default 100). `Sprite.grid(texture,
columns, rows)` cuts a sheet left to right, top to bottom.

`SpriteRenderer` has `sprite`, `color`, `flipX`, `flipY` and `sortingOrder`, and sends `render.sprite`.
In `WorldViews::views`, a sprite becomes a `WorldQuad` once its texture's size is known
(`texture_sizes`). `sprite_quad` scales the unit quad to `rect / pixelsPerUnit`, shifts it so the pivot
is at the object's origin, and works out the UV rectangle in top-down space (a flip makes the UV
size negative). The quad faces the object's +z, is unlit, and sorts with the other see-through things
by `order`, then distance (see [world-rendering.md](world-rendering.md#the-world-pass)): in an
orthographic 2D game `sortingOrder` decides what is on top.

## glTF models

`Model.load(src)` (cached per path) sends `render.model id src` and returns at once. `Model` has
`isLoaded`, `failed`, `whenLoaded()`, `materials` and `meshes` (after loading), and
`instantiate({ position, rotation, scale, parent, colliders })`.

**In the app.** `model_op` calls `models::load` ([models.rs](../crates/iron_app/src/models.rs)) with
the game folder:

- The `.glb`/`.gltf` file and anything it names (buffers, images) must be inside the game folder
  (`resolve_asset`; `data:` URIs work; URIs are percent-decoded).
- Each material's textures (base colour, emissive, normal, metallic-roughness, occlusion) become textures
  under engine ids, with the sampler's filter and wrap mapped (`Nearest` mag filter is `Point`;
  otherwise trilinear when the min filter has mips). Colours (base, emissive) are sRGB and data (normal,
  metallic-roughness, occlusion) is linear, so one image used both ways is two textures. Textures are
  shared between materials that use the same image and role. An image that cannot be read gives the
  material `MISSING_TEXTURE`.
- Each triangle primitive becomes its own mesh under an engine id (`EngineIds`, counting up from
  `1 << 30`). Other primitive modes and malformed meshes are skipped with a warning. Missing normals and
  UVs are made up. Tangents are read from the file, or made from the UVs (glTF's convention matches ours).
- The result is a description: `{ nodes, roots, materials }`. Nodes carry a decomposed transform,
  children and primitives (`mesh`, `material`, `bounds`); materials carry colour, emission, smoothness
  (1 - roughness), `metallic` (glTF's `metallicFactor`), `unlit` and the texture id and size. Also `alpha` mode (`alphaMode`: opaque, mask or blend),
  `alphaCutoff`, `doubleSided`, and the map ids: `normal` (id, scale), `metallicRoughness`, `occlusion`
  (id, strength) and `emissive`. Only `blend` materials keep their colour alpha.
- Meshes go to `mesh_uploads` and `mesh_data`, textures to `texture_uploads`, and the description is
  sent to `__iron_models([id, json | null, ...])` (null: it failed) after the ops batch.

**In scripts.** `__iron_models` calls `Model._loaded`. `instantiate` makes a GameObject named after the
file with the model's nodes as children (each a `GameObject` with the node's transform), and a
`MeshRenderer` per primitive with the right `Material`. In scripts, `mask` becomes `cutout`, `blend` becomes `fade`, and the metallic-roughness map is
read with `gltfPacking` (occlusion red, roughness green, metallic blue). A node with several primitives gets one child
object per part. With `colliders: "mesh"` (exact triangles, for static scenery) or `"convex"` each
renderer also gets a `MeshCollider` with `sharedMesh`. `instantiate` before loading finishes works: the
nodes appear when the model arrives. The root remembers its source (`_model`) and the nodes are marked
`_modelPart`, so a saved scene stores the model path and rebuilds the nodes on load
([scene-and-objects.md](scene-and-objects.md#scenes)).

## Limits

- Textures are 2D only, RGBA8, and all mip levels are built on the CPU.
- Loading blocks the frame it happens in; there is no async or streaming load.
- glTF: base colour, emissive, normal, metallic-roughness and occlusion textures, factors, alpha modes and
  double-sided. No extensions, skins, animations or cameras, and
  only triangle meshes.
- Hot reload makes a new `WorldViews`, and scripts load their textures and models again (see
  [scripting.md](scripting.md#hot-reload)).

## Tests

`M5-TEX-01` and `M5-TEX-02` (texture ops, sprite sizing and pivot) are unit tests in `world_view.rs`;
`M5-GLB-01` (mesh colliders from a render mesh) is there too. Goldens `M5-RND-08` cover textured meshes
and sprites and the missing checker. JS units are `js/engine/test/{textures,models,meshes}.test.ts`; the
fixture model is `tests/fixtures/assets/house.glb`.
