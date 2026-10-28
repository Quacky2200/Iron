# Iron developer docs

These pages explain **how Iron is implemented today**: data flow, key types, algorithms and where
the code lives. They complement, and do not repeat:

| File | Role |
|---|---|
| [DESIGN.md](../DESIGN.md) | The intended design and decisions (may be ahead of the code) |
| [TODO.md](../TODO.md) | Plan and progress by milestone |
| [TESTS.md](../TESTS.md) | Test plan with IDs such as `M3-EVT-01` |
| [CLAUDE.md](../CLAUDE.md) | Short overview, commands, conventions, gotchas |

Where the code differs from DESIGN.md the pages say so ("Difference from DESIGN.md").

## Index

| Page | Covers |
|---|---|
| [scripting.md](scripting.md) | `iron_script`, `@iron/engine`, ops and commits, `ComponentHost`, bundles and how `iron_app` loads them, dev builds (vendor + game), hot reload (state kept, or in place with Fast Refresh) |
| [physics-and-audio.md](physics-and-audio.md) | `iron_physics` (rapier) and `iron_audio` (kira): ops and script IDs, per-tick order, batch formats, TS handle APIs, GameObject components (`Rigidbody`, colliders, joints, `Rope`, `CharacterController`, 2D physics, `AudioSource`/`AudioListener`), body-object sync, `Cloth` (position-based solver, `cloth.*` ops), offline mixer and muting, limits |
| [ui-tree.md](ui-tree.md) | `iron_ui` tree, opcodes, styles, Taffy layout, Parley text, text fields (`editor.rs`), draw list |
| [css.md](css.md) | Stylesheet parsing, selector matching, cascade, invalidation, variables, `::before`/`::after`/`::marker`/`::backdrop`/`::selection`/`::placeholder`, `@font-face` (incl. WOFF/WOFF2), `@media`, URL rebasing |
| [react.md](react.md) | `@iron/react`: reconciler host config, style/attr diffing, `createRoot`, `Text`/`Span`/`Input`/`TextArea`, events, refs, Fast Refresh |
| [input-and-events.md](input-and-events.md) | Platform input (incl. raw mouse motion), `Pointer`, `Interaction`, hit testing, focus, dispatch to scripts, cursor, script `Cursor` (hide, lock, image), scrolling, text fields (keys, clipboard, IME), custom cursors, dialogs, gamepads, game `input` API and mouse, gamepad UI navigation, drag and drop (elements and files from the system) |
| [animation.md](animation.md) | Transitions, `@keyframes` animations, interpolation, `@starting-style`, `:outro` deferred removal, `transition*`/`animation*` events |
| [animation-3d.md](animation-3d.md) | Animation of GameObjects: `AnimationCurve`/`AnimationClip`, blending (`Pose`), `Animation`, `Animator` with `AnimatorController` (blend trees, transitions), `__iron_animate`, skinned meshes (bones, bind poses, GPU skinning), glTF skins and animations |
| [svg.md](svg.md) | SVG files as images, inline `<svg>` (serialized and rasterized with resvg), CSS on SVG content, caching and eviction, limits |
| [scene-and-objects.md](scene-and-objects.md) | `iron_scene` GameObjects, script and engine ids, `scene.*` ops, the move batch, component lifecycle order (incl. `onAwake` and the application focus and pause hooks), `HideFlags`, scenes (file format, `@property`, `SceneManager`, `instantiate`, prefabs, start scene), the TS API (math, `Transform`, `Time`, dates, logging, `Random`, input, `Application`/`SystemInfo`), limits |
| [world-rendering.md](world-rendering.md) | The 3D world pass (cameras in depth order, viewports, background fill, opaque and see-through sorting, glow), lit meshes (PBR) and lights, skinned meshes, cooked (BC) texture uploads, halos and beams, the procedural sky and environment map, the cloud layer drawn with the background (HDRP's, `RenderSettings.cloudLayer`), planar mirrors, reflection probes, cascaded sun, spot and point shadows, layers and culling masks, frustum culling, LOD groups, material maps and rendering modes, bloom, exposure, tonemapping and FXAA, render textures, the editor ground grid and gizmo lines, thick 2D lines with fills and `Shapes`, `render.*` ops, `render_overlay` for screen UI, `WorldPanel` and `TextMesh` (commit targets, panel trees and textures, pointer and key routing), coordinates |
| [postprocessing.md](postprocessing.md) | `PostProcess`: passes over a camera's finished picture (script API, the built-in effects and their parameters, your own WGSL and its helpers, depth and normals for depth of field and outlines, per-camera passes, how the chain ping-pongs and what it can't do), `examples/postfx` |
| [assets-and-models.md](assets-and-models.md) | `Texture` (load, create, filters, CPU mipmaps), the missing-texture checker, `Material` (metallic, maps, rendering modes, `create`/`with`)/`RenderTexture`/`Mesh`, mesh colliders from render meshes, `Sprite`/`SpriteRenderer`, glTF `Model.load`/`instantiate` (skins, animations), engine ids, the asset database (`.meta` GUIDs and importers), asset hot reload, cooking (`iron --cook`, BC1/BC3) |
| [particles.md](particles.md) | `ParticleSystem`: settings as key/value pairs, the CPU simulation in `particles.rs`, reports to scripts, drawing as camera-facing quads |
| [platform.md](platform.md) | Window modes (`FullScreenMode`, platform fallbacks), how display settings flow from manifest and saved prefs, `screen.set`, the log file, vsync and the frame cap, `PlayerPrefs` (file, keys, when written), focus and background pausing (`runInBackground`, `Lifecycle`, `FixedClock`), limits |
| [windows.md](windows.md) | User guide to `Window`: opening tool windows, dialogs and isolated windows from TypeScript, options and styles, parent/child, events, keys and hot reload, messages (`postMessage`, `request`, `broadcast`) and their limits, families and event domains, how `Window` relates to `Screen` and `Application` (and Unity and Godot), platform support table, screenshots |
| [crashes.md](crashes.md) | User guide to crashes: what a player sees (Iron's crash dialog, the plain fallback), where crash reports go and what is in them, `Application.crash(message?, plain?)`, trying the dialogs, when no dialog shows, reading a report and chasing a crash that only happens sometimes, flags and platforms |
| [rendering.md](rendering.md) | `iron_render`: GPU setup, `UiRenderer` planning and passes, shaders, offscreen path |
| [editor.md](editor.md) | `iron_editor` (the second binary), the project folder, the log feed and `editor.*` ops (code links, project files), `@iron/editor-ui` (menubar, docking, virtual list, tree view, console, project browser), the Scene view (editor camera, orientation gizmo, SceneIcons, wire gizmos, transform handles, `onEditorFrame`) and the editor UI in `editor/` (demo scene, live Hierarchy, Game tab) |
| [iron-build.md](iron-build.md) | `tools/iron_build`, the build tool for games compiled by iron_ts: config, module graph, `game.irbundle`, CSS bundling, assets and `.meta` files, `types/iron.d.ts`, `src/modules` imports, the dev loop protocol |
| [iron-ts.md](iron-ts.md) | `tools/iron_ts`, the experimental ahead-of-time TypeScript compiler (oxc to typed IR to Cranelift object): how to run it and its tests (incl. GC stress mode), the pipeline, source files, the supported language, `iron_ts_rt` (mark-sweep GC, shadow-stack roots, `ts_rt_*`), adding a test module |
| [iron-ts-native.md](iron-ts-native.md) | The move from QuickJS (bun-bundled JavaScript) to native iron_ts: why, what changed (compiler and GC, `#[iron_api]` and generated declarations, `iron-build`, native UI, scenes, physics, animation, render sync, windows, packs, QuickJS deprecated), how games differ, benchmark summary (45 examples, stages, micro-benchmark, crowds, start-up), limits and what is next |
| [benchmarks.md](benchmarks.md) | Frame times of every example, native against the QuickJS originals from `main` (generated by `iron-build bench`, raw data in `benchmarks/results.json`), the spread of re-runs, the stages, the Animator and crowd numbers |
| [ui-native.md](ui-native.md) | `iron_ui_native`: the native reconciler (element descriptions, hooks, diff, effects in React's order, portals, events) |
| [animation-native.md](animation-native.md) | `iron_animation`: curves, clips, pose blending, the `Animation` and `Animator` components and their script API |
| [iron-api.md](iron-api.md) | `iron_api` and `iron_api_macros`: `#[iron_api]` on a Rust module generates the manifest, the TypeScript `.d.ts` and JSON, `extern "C"` shims and typed QuickJS bindings; supported types, naming, status (not yet used by engine crates), measured call cost |
| [testing.md](testing.md) | Test layout per crate, fixtures, goldens, mapping to TESTS.md |
| [difference-to-bevy.md](difference-to-bevy.md) | How Iron relates to Bevy: what each project is for, where the overlap comes from (shared libraries, references), where Iron differs, and what Bevy has that Iron doesn't yet |
| [difference-to-unity.md](difference-to-unity.md) | Iron vs Unity: TypeScript instead of C# with the same component model, portability and popularity, licensing (per-install fees and why we avoid them), API stability |
| [difference-to-sbox.md](difference-to-sbox.md) | Iron vs s&box: what we took (Unity-style APIs, Razor-like panels with CSS, editor-first), the stack and language changes, publishing approval, stability |
| [difference-to-electron.md](difference-to-electron.md) | Iron vs Electron: bundled Chromium and V8 cost, the Rust and React app-shell plan, and the web platform APIs we plan (files, fetch, WebSocket, canvas, audio, wasm, storage) |
| [differences.md](differences.md) | **Where Iron knowingly differs from browsers** (filter order, mask modes, linear-light fades, defaults, gaps). Read before changing those areas |

## Crate map

```
iron_core      Value, ids, Commit/Op, asset path safety          (no deps on other Iron crates)
iron_platform  winit window + event loop, InputEvent (keys, pointer, IME, gamepads), gilrs,
               window modes and resolutions (display.rs, see platform.md)
iron_script    ScriptRuntime trait, QuickJS backend, prelude.js, ComponentHost
iron_scene     GameObject hierarchy (Scene), ScriptScene: scene.* ops and the move batch
iron_ui        UiTree, styles, CSS, layout, text, draw list, events/interaction
iron_physics   rapier3d world driven by `physics.*` ops (bodies, colliders, characters, joints), per-tick batches (no deps on other Iron crates but iron_core)
iron_audio     kira mixer driven by `audio.*` ops, offline backend
iron_render    wgpu: Gpu, surfaces, UiRenderer (consumes iron_ui::DrawCmd), WorldRenderer (lit meshes, quads, lights, shadows, bloom)
iron_app       the `iron` binary that wires everything together (world_view, particles, models, physics_sync, cloth_sync, lifecycle, asset_db, cook, info)
js/engine      @iron/engine   (GameObject, Component, register, op, loadFont, stylesheets, input, math, Time, cameras, meshes, lights, textures, particles, models, physics and audio components, Cursor, Application, WorldPanel, TextMesh, LineRenderer2D, Shapes, Spline, SplineAnimate)
js/react       @iron/react    (reconciler, events)
js/editor-ui   @iron/editor-ui (MenuBar, Dock, VirtualList, TreeView, Console, SceneView; see editor.md)
editor/        the editor's UI, run by `iron_editor`
```

## The frame pipeline

The binary [main.rs](../crates/iron_app/src/main.rs) is driven by winit's redraw callback
(`PlatformApp::frame`). Input arrives separately, between frames, through `PlatformApp::input`.
The window is created hidden and shown after its first frame; a resize also draws a frame right away.

```
 winit WindowEvent
      |  iron_platform::convert_window_event
      v
 InputEvent --(scale to logical px)--> iron_ui::Input --> Interaction::handle
                                                              |  UiEvent (hover, click, key, ...)
                                                              v
                                       __iron_ui_event(type, path, data) in QuickJS
                                                              |  React handlers run, setState
 frame():                                                     v
   0. sync_screen: UiTree::set_size(window / scale), __iron_screen if the size changed (first,
      so the first frame's script output lays out and scrolls at the window's size, not 1x1)
   1. per fixed 60 Hz tick: ComponentHost::fixed_update -> apply_script_output -> step_physics
      -> cloth step -> send_scene_moves (onFixedUpdate, then commits and ops, then bodies follow
      moved objects, rapier, objects follow bodies, __iron_physics, cloth, then __iron_scene)
      (while paused in the background none of this runs, see platform.md)
   2. ScriptRuntime::tick(now_ms)                 -> timers + microtasks (React scheduling)
   2b. apply_script_output, ComponentHost::update_active (__iron_tick, onUpdate, __iron_animate, onLateUpdate),
      apply_script_output, update_particles (+ __iron_particles), update_audio
   3. drain_targeted_commits -> UiTree::apply_commit of the screen tree or a world panel's tree
   4. drain_ops (loadFont, addStyleSheet, focus, scroll, cursor.*, render.texture/model, app.quit...)
      handled by the app; __iron_textures / __iron_models report loads
   5. Interaction::handle(Input::Refresh)         -> re-hit-test under a still pointer
   5b. ComponentHost::pre_render (onPreRender) -> apply_script_output
   6. sync_screen again (a UI scale set this frame)
      load_images: take_image_requests -> image_size -> set_image_size (before each draw_list)
   7. UiTree::draw_list()                         -> layout_if_needed: restyle -> text shaping
                                                     -> Taffy -> absolute rects -> Vec<DrawCmd>
   7b. Panels::render: each world panel's tree is drawn into its own texture
   7c. mesh, texture, bloom and shadow settings queued by scripts are handed to WorldRenderer
   8. draw_frame: with a camera, WorldRenderer::render_views (cameras in depth order) then
      UiRenderer::render_overlay; without one, UiRenderer::render -> instances, layers, clips
      -> wgpu passes
   9. present (or --screenshot: offscreen target, PNG, exit)
```

Key ideas to keep in mind:

- **Rust owns the world, JS owns behaviour.** JS only holds numeric node ids. React sends a
  *commit* (list of `[opcode, ...args]`) per React commit; Rust applies it as a unit.
- **Lazy, dirty-driven work.** `apply_commit` only records what changed. The expensive work
  (restyle, text shaping, layout) happens once in `UiTree::layout_if_needed`, called by
  `draw_list()` and `hit_test()`.
- **Logical pixels everywhere above the renderer.** The app divides window sizes and cursor
  positions by the scale factor; `RenderTarget::scale` converts back.
- **The draw list is GPU-free.** `iron_ui` never touches wgpu; tests can inspect `Vec<DrawCmd>`.

## Where to start reading

| I want to... | Read |
|---|---|
| Add a CSS property | `apply_style` in [style.rs](../crates/iron_ui/src/style.rs), then [ui-tree.md](ui-tree.md) |
| Add a selector / pseudo-class | [css.rs](../crates/iron_ui/src/css.rs), [cascade.rs](../crates/iron_ui/src/cascade.rs), [css.md](css.md) |
| Change text field editing | [editor.rs](../crates/iron_ui/src/editor.rs), [input-and-events.md](input-and-events.md#text-fields-m3c-step-5) |
| Add a UI event | [events.rs](../crates/iron_ui/src/events.rs), [events.ts](../js/react/src/events.ts), [input-and-events.md](input-and-events.md) |
| Change gamepad / game input | [gamepad.rs](../crates/iron_platform/src/gamepad.rs), [game_input.rs](../crates/iron_app/src/game_input.rs), [input.ts](../js/engine/src/input.ts), [input-and-events.md](input-and-events.md#gamepads-and-game-input-m3c-step-6) |
| Add a physics or audio op | [physics-and-audio.md](physics-and-audio.md), [iron_physics](../crates/iron_physics/src/lib.rs), [iron_audio](../crates/iron_audio/src/lib.rs) |
| Add a script op | [main.rs](../crates/iron_app/src/main.rs) `tick()`, [scripting.md](scripting.md) |
| Change script hot reload | [reload.rs](../crates/iron_app/src/reload.rs), `App::reload_scripts` in [main.rs](../crates/iron_app/src/main.rs), [scripting.md](scripting.md#hot-reload) |
| Add a scene op or component hook | [script.rs](../crates/iron_scene/src/script.rs), [components.rs](../crates/iron_script/src/components.rs), [scene-and-objects.md](scene-and-objects.md) |
| Change meshes, lights, shadows, bloom, sky, mirrors or probes | [world/mod.rs](../crates/iron_render/src/world/mod.rs), [world.wgsl](../crates/iron_render/src/world/world.wgsl), [world_view.rs](../crates/iron_app/src/world_view.rs), [world-rendering.md](world-rendering.md) |
| Change textures, sprites or glTF loading | [texture.rs](../crates/iron_render/src/world/texture.rs), [models.rs](../crates/iron_app/src/models.rs), [assets-and-models.md](assets-and-models.md) |
| Change animation, `Animator` or skinning | [animation.ts](../js/engine/src/animation.ts), [world/mod.rs](../crates/iron_render/src/world/mod.rs), [world_view.rs](../crates/iron_app/src/world_view.rs), [animation-3d.md](animation-3d.md) |
| Change asset reload, `.meta` files or cooking | [reload.rs](../crates/iron_app/src/reload.rs), [asset_db.rs](../crates/iron_app/src/asset_db.rs), [cook.rs](../crates/iron_app/src/cook.rs), [assets.ts](../scripts/assets.ts), [assets-and-models.md](assets-and-models.md) |
| Change focus, pausing or the frame cap | [lifecycle.rs](../crates/iron_app/src/lifecycle.rs), [platform.md](platform.md#focus-and-pausing) |
| Change cloth | [cloth.rs](../crates/iron_physics/src/cloth.rs), [cloth_sync.rs](../crates/iron_app/src/cloth_sync.rs), [physics-and-audio.md](physics-and-audio.md#cloth) |
| Change particles | [particles.rs](../crates/iron_app/src/particles.rs), [particles.ts](../js/engine/src/particles.ts), [particles.md](particles.md) |
| Change cameras, planes, the editor grid, gizmo lines or world panels | [world/mod.rs](../crates/iron_render/src/world/mod.rs), [world_view.rs](../crates/iron_app/src/world_view.rs), [panels.rs](../crates/iron_app/src/panels.rs), [world-rendering.md](world-rendering.md) |
| Add a draw command | [draw.rs](../crates/iron_ui/src/draw.rs), [ui/mod.rs](../crates/iron_render/src/ui/mod.rs), [rendering.md](rendering.md) |
