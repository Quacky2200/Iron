# World rendering

Code: [iron_render/src/world](../crates/iron_render/src/world/mod.rs) (the GPU pass),
[iron_app/world_view.rs](../crates/iron_app/src/world_view.rs) (cameras and planes from scripts),
[iron_app/panels.rs](../crates/iron_app/src/panels.rs) (world panels), the drawing and input routing
in [iron_app/main.rs](../crates/iron_app/src/main.rs), and
[rendering.ts](../js/engine/src/rendering.ts) on the script side. Milestone M5.a. Objects and
components are in [scene-and-objects.md](scene-and-objects.md); the 2D UI renderer is in
[rendering.md](rendering.md).

The world is quads only for now: coloured or textured planes. Meshes and lights come later (M5.a2).

## The world pass

`WorldRenderer` ([world/mod.rs](../crates/iron_render/src/world/mod.rs), shader `world.wgsl`) draws
`WorldView`s into a `RenderTarget`:

- `WorldView { camera: WorldCamera, viewport: [x, y, w, h], quads: Vec<WorldQuad> }`. The viewport is
  normalized, from the top-left of the target.
- `WorldCamera { view_proj, position, clear }`. `WorldCamera::new(world, projection, near, far, size,
  clear)` builds it from the camera's world matrix and a viewport `size` in pixels (for the aspect).
  `Projection` is `Perspective { fov_y_degrees }` or `Orthographic { half_height }`.
- `WorldQuad { model, color, texture, uv }`. The unit quad (-0.5..0.5 in x and y, facing +z) is placed
  by `model`. `color` is linear straight-alpha RGBA that multiplies the texture. `texture` indexes the
  `textures` slice given to the render call, and `None` is plain colour.

`render_views(gpu, target, views, textures)` does one render pass:

1. The target is cleared to black. Parts no view covers stay black.
2. **Cameras draw in order**, which the app sorts by `depth` (lower first), so a higher camera draws
   over a lower one, like Unity's camera depth.
3. For each view, the pass sets the viewport and scissor to its pixel rect (`viewport_px`), then
   draws a fullscreen triangle with the **background fill**: the camera's `clear` colour replaces the
   colour in the viewport (no blending, depth compare `Always`) and writes depth 1.0. So a later
   camera fully replaces what was under its viewport, and its depth starts fresh.
4. Quads are sorted back to front by distance from that view's camera and drawn with depth testing
   (`Less`) and premultiplied alpha blending, so see-through quads blend over what is behind them.
   Culling is off: planes show from both sides. Consecutive quads with the same texture draw in one
   call.

One quad storage buffer holds every view's quads, and one uniform buffer holds a `GpuGlobals` per view
(`view_proj` and the background), picked with a dynamic offset. Both grow to a power of two. The depth
texture is `Depth32Float` and is recreated when the target size changes. `viewport_size(rect, size)`
gives a viewport's pixel size, for a camera's aspect. `render(...)` is the single-camera shortcut.

A 0x0 target returns early.

## Screen UI over the world

`UiRenderer::render_overlay(gpu, target, cmds)` ([ui/mod.rs](../crates/iron_render/src/ui/mod.rs)) is
`render` with two changes: the canvas starts **transparent** instead of cleared, and its final blit
**blends** (premultiplied alpha) onto what the target holds, with `LoadOp::Load`. The blit pipeline is
cached by `(format, overlay)`.

`draw_frame` in `main.rs` picks the path:

- No active camera: `UiRenderer::render` over the plain `CLEAR` colour, as before M5.a.
- Otherwise: `WorldRenderer::render_views`, then `render_overlay`.

Screenshot runs and window frames both go through `draw_frame`.

## Cameras and planes from scripts

`Camera` and `PlaneRenderer` are components in `@iron/engine`
([rendering.ts](../js/engine/src/rendering.ts)). They send `render.*` ops keyed by the component id,
and `WorldViews` ([world_view.rs](../crates/iron_app/src/world_view.rs)) stores them.

| Op | Arguments |
|---|---|
| `render.camera` | `component object fov near far [r g b a] depth orthographic orthographicSize [x y w h]` |
| `render.plane` | `component object [w h] [r g b a]` |
| `render.panel` | `component object [w h] pixelsPerMeter resolution [r g b a] interactive` (handled by `Panels`) |
| `render.remove` | `component` (removes a camera, plane or panel) |

The component re-sends its op when a property changes. The defaults are a dark clear colour, `depth`
0 and a full-screen `rect`.

`WorldViews::views(scene, size, extra)` runs every frame, after `update_world`:

- Keeps cameras whose object is active, sorted by `depth` (a stable sort, so ties keep the order they
  were added).
- Takes only the position and rotation from a camera's world matrix. Scale is ignored.
- Gives every view the same quads: the active planes, then `extra` (the panel quads). A plane lies
  flat in its object's XZ plane facing +y, like Unity's Plane (`lay_flat`).
- With no cameras it returns an empty list, so only screen UI is drawn.

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

## Limits

- Quads only. No meshes, lights or shadows yet.
- Panels re-render every frame while visible, with no dirty tracking.
- Panels are not occluded by other world quads for hit testing: the nearest interactive panel wins.
- Plane quads are not hit tested.
- Pointer events on a panel use UI pixels of the panel, not the screen scale.

## Tests

Goldens in [iron_render/tests/world.rs](../crates/iron_render/tests/world.rs): `M5-RND-05` (a floor
in perspective, a plane seen through a half-transparent one, a textured plane, and screen UI blended
over the view) and `M5-RND-06` (a perspective view, an orthographic top-down view and an inset
drawn in depth order). `M5-PNL-02` (rays and panel quads) is a unit test in `panels.rs`, `M5-PNL-01`
(per-root commits) is in `js/react/test/reconciler.test.tsx`, and `M5-CAM-01` (camera conversions) is
in `js/engine/test/camera.test.ts`. `examples/world` is the manual check (`M5-PNL-03`).
