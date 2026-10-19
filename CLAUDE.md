# Iron

An s&box-like game engine. The core is Rust. Gameplay is TypeScript running in QuickJS, bundled by Bun.
UI is React through a custom reconciler, drawn by our own renderer. The editor will be built the same
way. Full design: `DESIGN.md`. Plan and progress: `TODO.md`. Test IDs (e.g. `M3-EVT-01`): `TESTS.md`.

**Core rule: Rust owns the world, JS owns behaviour.**
- Scripts never hold engine references, only numeric IDs.
- Scripts talk to Rust through ops (`__iron_op(name, ...args)`) and batched UI commits (`__iron_commit(ops)`).
- Rust calls into JS through `__iron_*` globals.

## Layout

| Path | What |
|---|---|
| `crates/iron_core` | Shared types: `Value` (crosses the JS boundary), `NodeId`, `Commit`, ops, asset path safety (`resolve_asset`, `decode_data_url`) |
| `crates/iron_platform` | winit window + event loop (`Platform`, `PlatformApp`), `InputEvent` conversion, gilrs gamepads |
| `crates/iron_script` | `ScriptRuntime` trait + QuickJS backend (rquickjs 0.14), timers prelude, `ComponentHost` (component lifecycle) |
| `crates/iron_ui` | UI tree built from React commits (opcodes in `opcodes.rs`), styles (`style.rs`, `text_style.rs`, `paint.rs`), Taffy layout (`layout.rs`), Parley text (`text.rs`), CSS (`css.rs` parser, `cascade.rs` matching/cascade/invalidation), draw list (`draw.rs`), hit testing (`tree.rs`), pointer events + `:hover`/`:active` (`events.rs`), focus/keys/default actions (`interaction.rs`), drag and drop (`interaction/dnd.rs`) |
| `crates/iron_render` | wgpu renderer for the draw list: quads, SDF rounded boxes, shadows, blur layers, gradients, images, glyph atlas (swash); `render_overlay` blends UI over the 3D world. `world/` is the 3D pass (`WorldRenderer`, `WorldCamera`, `WorldQuad`). `testing.rs` has golden helpers and a shared test GPU |
| `crates/iron_scene` | GameObjects: generational `ObjectId`s, names, enabled/active, parent/children, local `Transform` (glam) and cached world matrices, `take_moved()`; `ScriptScene` runs `scene.*` ops (script ids, engine ids from 2^30) and batches engine moves for `__iron_scene` |
| `crates/iron_physics` | rapier world driven by `physics.*` ops (script-chosen body/collider IDs); `step()` per fixed tick, `take_batch()` → `__iron_physics(moved, contacts, rays, characters)`. Plane locks (a locked axis plus both turns across it, e.g. every Rigidbody2D) are pinned by Iron (`set_constraints`, `hold_pins`), not rapier: see Gotchas |
| `crates/iron_audio` | kira mixer driven by `audio.*` ops: sounds (decoded or streamed), playbacks, buses, spatial emitters; `Mixer` trait over a device or `OfflineBackend`; finished sounds → `__iron_audio(ended)` |
| `crates/iron_app` (panels) | `panels.rs`: world panels (a UI tree + interaction + texture each), camera rays and panel hits; `world_view.rs`: cameras and planes from `render.*` ops; `main.rs` routes the pointer and keys between the screen and panels (`Surface`) |
| `crates/iron_app` | The `iron` binary: `iron [game folder] [--screenshot out.png] [--frames N]` (no folder: the current one). Loads `dist/game.js` + `dist/game.css` and wires scripts → UI → renderer. `iron_editor` is a second binary of the same code that runs `editor/` |
| `js/engine` | `@iron/engine`: `Component`, `register`, `op`, `loadFont`, `addStyleSheet`/`removeStyleSheet`, `input` (keys, gamepads), `physics` (`RigidBody`, `Collider`, raycasts, character controller), `audio` (`Sound`, `Bus`, `Emitter`), `GameObject`/`Transform`/`Component` (scene.ts), Unity-style math (math.ts), `DateTime`/`TimeSpan` (time.ts), `Time`/`TimeSince`/`Task` (clock.ts), `Debug`/`Log`/`Console`, `Random`; 2D shapes: `LineRenderer2D` (SDF strokes, fills, `sortingOrder`) and `Shapes` (lines.ts), `TextMesh`, `Spline`/`SplineAnimate` (spline.ts), `Gizmos` (gizmos.ts). Transforms reach Rust batched (`flushTransforms` → one `scene.locals` op per script turn) |
| `js/editor-ui` | `@iron/editor-ui`: editor components drawn by Iron's UI (`MenuBar`, `Dock`, `VirtualList`, `TreeView`, `Console`), logic in plain modules (`menu.ts`, `dock.ts`, `virtual.ts`, `tree.ts`, `console.ts`) |
| `editor/` | The editor's UI, run by `iron_editor` (`crates/iron_app/src/bin/iron_editor.rs`, `editor.rs`); see `docs/editor.md` |
| `js/react` | `@iron/react`: reconciler (`host.ts`), UI event dispatch (`events.ts`), style/attr diffing, `createRoot`, `Text`/`Span` |
| `examples/*` | `hello`, `boxes`, `text`, `css`, `form`, `physics`, `world` (3D), `platformer` (2D), `classics` (test models), `cornell`, `materials`, `quality`, `layers`, `scenes`, `arcade` (a small game with a React HUD), `animation`, `cloth`, `sine-circle-demo` and `raindrop-demo` (MiniGameEngine ports), `shapes`, `splines`, `clock` (twelve faces), `fishtank`, `emoji` (a spinner wheel picks one of ~100 Noto SVG emoji that enter, pose, get a crowd reaction and leave, with synthesised sound effects from `tools/make_sfx.py`) games (each has `bun run build`). `examples/topdown-zombie-game` is not in this repo: it's a git worktree of the user's zombie-horders clone (branch `iron`), ignored via `.git/info/exclude` |
| `docs/` | Implementation docs: how the code works today (pipeline, crates, events, rendering, testing). Start at `docs/README.md`. `docs/upstream/` holds reports for upstream projects (the rapier friction issue and PR) |
| `tests/fixtures` | JS fixtures for Rust integration tests; `tests/layout` JSON layout fixtures; `tests/golden` PNG goldens |

### UI model
- **Elements:** `box`, `text`, `span`, `input`, `textarea`, `image`, `list`, `dialog`, `svg` (with SVG
  elements inside) and `#text` nodes. `ROOT` is node 0. Generated `::before`/`::after`/`::marker`/
  `::backdrop` ids start at `0x8000_0000`.
- **Styles:** inline `style` props (camelCase) go through `apply_style`. Stylesheets go through `cssparser` + `selectors`, with cascade (specificity, inline, then `!important`), custom properties and `var()`, and a rule index plus dependency-aware restyle.
- **Units:** layout is in logical pixels. The app divides by the window scale factor.

## Commands

On Windows the shell needs `export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`.

```bash
cargo fmt --all
```
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
```bash
cargo test --workspace
```
```bash
bun test
```
```bash
bun run typecheck
```
```bash
bun run build
```
```bash
bun run build:fixtures
```
```bash
cargo run -p iron_app -- examples/css --screenshot out.png --frames 30
```
```bash
cargo run -p iron_app -- examples/world --cook
```

- `bun run build` builds all the examples.
- `bun run build:fixtures` rebuilds `tests/fixtures`. Run it after changing js/.
- `bun run dev` in an example (`scripts/dev.ts`) makes a dev build (vendor.js + game.js) and rebuilds
  game.js on every edit; a running `iron` window reloads it in place (React Fast Refresh, components
  swap classes, the world stays). Other reloads keep state through scene serialization.
- The `cargo run` line renders a screenshot and exits; check it by eye. `--cook` prepares a
  game's textures and models into `.iron/cooked/` (git-ignored) and exits.
- `bun scripts/assets.ts <game folder>` writes `.meta` files (GUIDs, import settings) for new
  assets; `bun run dev` runs it.
- `bun run screenshots` renders every example to `tests/output/` (git-ignored), next to the
  golden tests' renders.
- Benchmarks: `cargo bench -p iron_ui --bench restyle` (and the other benches in `crates/iron_ui/benches`).

## Working agreements
- **Commits:** work milestone by milestone, one commit per step, without asking.
  - Run every check above before each commit.
  - End commit messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - Update `TODO.md` checkboxes, and `TESTS.md` when adding test IDs.
  - After each finished step, update `docs/` to match the code. Delegate this to a subagent on the
    Sonnet model, and commit the docs with the step or right after it. Give the subagent only
    what changed since the last docs commit (the commits and files from `git diff`), and have it
    read just that code and the affected doc pages, not the whole codebase.
- **Charm font:** never commit it. It lives in `/Charm/` with local copies under `examples/*/assets/fonts/`. It's excluded via `.git/info/exclude` and `dist/` is ignored. Check `git status` before committing.
- **Code style:** match the surrounding code. Comments are short and say why. Tests are named after behaviour and tagged with their `TESTS.md` ID in a comment.
- **CSS:** prefer standard CSS over s&box's custom extensions. The CSS parity backlog is TODO.md "M3d".
- **Known differences from browsers** (filter order, mask modes, linear-light fades, defaults, gaps)
  are listed in `docs/differences.md`. Check it before touching those areas, and keep it current.

## Gotchas
- **Shell edits:** bash heredocs mangle apostrophes and backslashes on this machine. For multi-line edits use the Edit/Write tools, or a Python script written to the scratchpad.
- **Interrupted builds:** if cargo was killed mid-build, delete the corrupt `.pdb` files under `target/` before rebuilding.
- **Stale app binary:** `cargo clippy`/`cargo test` don't rebuild `target/debug/iron_app.exe`.
  Run `cargo build -p iron_app` before any manual check in the window. A running window locks the
  exe, so close it before rebuilding.
- **Logs:** `iron` is a GUI-subsystem program. Its log goes to `Player.log` (last run:
  `Player-prev.log`) next to the game's `prefs.json`, with ANSI colours, and to stderr when
  started from a terminal or piped. Screenshot runs write no file unless `IRON_LOG_FILE` is set.
- **Graphics backend:** Vulkan by default on Windows; `WGPU_BACKEND=dx12` switches.
  `IRON_FRAME_LATENCY` (1 or 2) sets the queued frames. The default is 1, or 2 on DX12, where 1 made
  resizing jitter.
- **GPU tests:** use `iron_render::testing::shared_gpu()`. Creating devices in parallel deadlocks.
- **Bun CSS:** importing CSS needs `--outdir dist --entry-naming "game.[ext]"`.
  - It emits `game.js` + `game.css`.
  - Small assets are inlined as `data:` URLs.
  - Larger ones are copied with hashed names, and their `url()` is relative to the CSS file. The engine rebases them.
- **Library versions:** cssparser 0.38, selectors 0.41, parley 0.12, taffy 0.15, rquickjs 0.14, edition 2024, Rust 1.99. APIs differ from older docs.
- **Parley fonts:** after registering a font, Parley needs a fresh `LayoutContext`, or its resolver cache keeps the old fonts.
- **Screenshot runs open a real window** for a moment, and it takes the user's clicks (a probe in
  `examples/physics` once got 60 extra bodies). Tell the user when a batch of runs is going.
- **rapier plane locks:** rapier 0.36.1 drops all friction for a body that can't move along an
  axis nor turn across it (its 3D friction solve inverts a singular 2×2 matrix). Iron pins such
  axes itself instead (`iron_physics` `set_constraints` / `hold_pins`, `M4-PHY-07`). Reported as
  dimforge/rapier#1047, fixed by #1048; drop the pins once a rapier release has it.
- **The zombie game's worktree** (`examples/topdown-zombie-game`) makes `bun install` add it to
  `bun.lock`: leave that `bun.lock` change out of Iron's commits. Its commits go to its own repo.
- **Fixed-size windows:** a game's manifest can set `iron.display.resizable: false`; its window
  then opens at the manifest's size, ignores saved sizes, and can't be resized or maximized.
- **Hot reload and module state:** restored components get `onReload`, not `onAwake`, and the new
  code's module-level variables start empty: rebuild caches (textures, singletons) in `onReload`.

## Status (2026-10-19)
- **Done:**
  - M1 scaffold
  - M2 React → Taffy → boxes
  - M3a text and fonts
  - M3b CSS stylesheets
  - M3c step 1: hit testing (`UiTree::hit_test`, `event_path`)
  - M3c step 2: pointer events. `iron_ui::Pointer` turns input into DOM-style events and sets
    `:hover`/`:active`; the app sends them to `__iron_ui_event(type, path, data)`, which
    `@iron/react` (`events.ts`) runs through capture/bubble over `on*` props. `cursor` → OS cursor
  - M3c step 3: keyboard and focus. `iron_ui::Interaction` wraps `Pointer`, owns focus and runs
    default actions (Tab, focus on press) using the "prevented" flag `dispatch` returns;
    `ref.focus()`/`blur()` arrive as `focus`/`blur` ops
  - M3c step 4: `overflow`/`clipPath` clipping (clip table in the renderer) and scrolling
    (offsets, wheel/keys/scroll-into-view, draggable overlay scrollbars, `onScroll`)
  - M3c step 5: text fields (`editor.rs`: Parley `PlainEditor` per `input`/`textarea`), clipboard
    (arboard in the app), IME (pre-edit/commit, `set_ime_cursor_area`)
  - M3c step 6: gamepads (`iron_platform::gamepad`) and keys to scripts through `@iron/engine`'s
    `input`; d-pad UI navigation (`Input::Navigate`/`Activate`) while UI has focus
  - M3c step 7: `examples/form`. **M3c is done.**
  - M3d (CSS parity) is done: groups 1–10 and step 5.1 (inline SVG and SVG images via resvg),
    (`sound-in`/`sound-out` skipped: s&box-specific, CSS stays about display)
  - M4 (2026-10-17): physics (`iron_physics`, rapier), audio (`iron_audio`, kira) and hot
    reload plumbing (watch `dist/`, new runtime then swap, old code kept on error, world reset).
    State-keeping reload and Fast Refresh moved after M5.a/b ("Hot reload, part 2" in TODO.md).
  - M5.a done (2026-10-17): `iron_scene`, GameObjects/components with Unity/s&box-style TS APIs
    (math, DateTime/TimeSpan, Time, Debug/Log, Random, input), lifecycle hooks, 3D world pass,
    camera system, `WorldPanel` (React UI on planes).
  - M5.a2 done (2026-10-17): Rigidbody/colliders (3D and 2D), CharacterController, joints and
    rope (`LineRenderer` tube), colliders following scale; MeshRenderer/Light/primitives;
    AudioSource/AudioListener; textures, sprites (`sortingOrder`), the missing-texture
    checker; particles; glows (halos, bloom, spot beams); Cursor and game mouse input;
    Application/SystemInfo (from the game's `package.json`); glTF models (with mesh
    colliders); directional shadows (`shadowCastingMode`); examples `platformer`, `classics`
    (Stanford Bunny, Newell teaset), `cornell` (bounce lights). Docs refreshed.
  - M5.c materials, first part (2026-10-17): PBR (Cook-Torrance metallic/roughness), a
    procedural sky with reflections and skybox, planar mirrors (`Mirror`), reflection probes,
    `examples/materials`. Docs refreshed (`8bac39e`).
  - Quality settings (2026-10-17, `4f796b8`): MSAA, render scale, shadow quality,
    `QualitySettings` levels with manifest defaults, `examples/quality`.
  - Camera render groups and culling (2026-10-17): `GameObject.layer`, `LayerMask`
    (manifest names), `Camera.cullingMask`, `layerCullDistances` / `layerCullSpherical`,
    `fieldOfView` / `horizontalFieldOfView`; `Light` / `ReflectionProbe` / `Mirror`
    culling masks (per instance in the shader); frustum culling of meshes, quads and
    lights (`world/cull.rs`, `WorldRenderer::stats()`); `LODGroup` with
    `QualitySettings.lodBias`; `examples/layers`.
  - M5.b scenes (2026-10-17): JSON scene files (`scenes.ts`, `@property` in
    `property.ts`), `SceneManager` (single / additive / async from files, unload,
    active scene, events), `dontDestroyOnLoad`, `GameObject.instantiate`,
    `iron.startScene`, `register(..., { spawn: false })`, `examples/scenes`.
  - Hot reload part 2, state (2026-10-17): the old code saves every scene
    (`__iron_save_state`), the new code restores it (`__iron_restore_state`); restored
    script components get `onReload`; unregistered classes make it start over.
    `IRON_RELOAD_AT_FRAME=N` reloads during screenshot runs. Benchmark `M4-HOT-07` at
    ~146 ms against 100 ms (follow-ups in TODO.md).
  - React Fast Refresh (2026-10-17): dev builds reload game code in place (`hot.ts`,
    `js/react/src/vendor.ts` / `refresh.ts`, `scripts/dev.ts`, `react-refresh` 0.19).
  - Display settings and PlayerPrefs (2026-10-17): window modes / resolution
    (`iron_platform::display`, `screen.set`), vsync, `targetFrameRate`, `PlayerPrefs`
    (`prefs.rs`, per-OS folder), saved display and quality choices; the arcade has a
    settings menu (Esc).
  - Fixes (2026-10-17): UI backdrop blur and blend modes over the 3D world
    (`render_over`, `M5-RND-16`); a layout panic when UI with absolute boxes was removed
    (`hoisted_by`, `M2-LAY-06`).
  - `examples/arcade` (2026-10-17): a coin collector with a React HUD, FPS counter and
    settings menu; Classic and Deluxe arenas as separate scenes (Style button), the round
    and positions kept across the switch.
  - App (2026-10-17): `iron` with no folder runs the current one; a GUI-subsystem program
    logging to `Player.log`; exclusive fullscreen at the monitor's resolution;
    `Application.runInBackground`, focus events and pause hooks (`lifecycle.rs`).
  - M5.c done (2026-10-17): material maps (normal, metallic, occlusion, emission,
    detail), the specular workflow, rendering modes (cutout / fade / transparent),
    double-sided, texture quality (anisotropy, mip limit); tonemapping (neutral, ACES),
    exposure and FXAA; render textures (`Camera.targetTexture`); shadows drawn once a
    frame with sun cascades and spot / point light shadows. Goldens `M5-RND-17`..`20`.
  - M5.d done (2026-10-17): animation (`animation.ts`: curves, clips, `Animation`,
    `Animator` with blend trees and transitions), skinned meshes and glTF animations
    (GPU skinning), prefabs, asset hot reload, GUID `.meta` files (`scripts/assets.ts`,
    `asset_db.rs`), cooking (`iron --cook`, BC1/BC3), cloth (`iron_physics::cloth`);
    `examples/animation`, `examples/cloth`.
  - M5.e started (2026-10-17, built by a background agent, cherry-picked here: work from
    agents' worktrees comes in as linear commits, no merges): `iron_editor` (a second
    binary over main.rs, `editor.rs`), `@iron/editor-ui` (menubar, docking, virtual
    list, tree view, console with code links, project browser) and the `editor/` app;
    UI drag and drop (`dnd.rs`) with files dropped from the OS. See `docs/editor.md`.
  - The editor's Scene view (2026-10-17/18, another session): an editor camera in the panel,
    the renderer's ground grid (`M5-RND-22`), the x/y/z orientation gizmo, SceneIcons, wire
    gizmos on the selection (`Gizmos`, `M5-RND-23`), move / rotate / scale handles
    (`M5-EDT-24`). The controls still need a manual check in the window (`M5-EDT-23`).
  - 2D drawing (2026-10-18): thick 2D lines as instanced SDF capsules (`LineRenderer2D`),
    filled shapes (lyon), GDI-style `Shapes` helpers, per-shape draw order (`sortingOrder`),
    `TextMesh` (world text without a UI panel, redrawn only when its text changes), `Spline`
    (auto / bezier / linear knots, by distance) and `SplineAnimate`.
  - Examples (2026-10-18): `sine-circle-demo` and `raindrop-demo` (MiniGameEngine ports, CC0
    music credited in `assets/CREDITS.md`), `shapes`, `splines`, `clock` (twelve faces: analog,
    dials, meter, a 3D sundial, sand timer, lava lamp after a CodePen by Corevision, underwater,
    chandelier, rainbow, morphing, flip, alarm clock), `fishtank` (an aquarium-channel look,
    depth haze by colour steps since there's no fog yet). The VB zombie game was ported, then
    moved to its own repository (zombie-horders, branch `iron`); history purged from this repo.
  - Script overhead (2026-10-18): transforms batched into one `scene.locals` op per script
    turn (`__iron_flush` after every call into JS), lifecycle hooks run in one `__iron_run`
    call per hook, identity-parent fast paths, scalar `Spline` maths. 123 fish went from ~40 ms
    to ~16 ms a frame (release).
  - Fixes (2026-10-18/19): fixed-size windows (`iron.display.resizable`, scripts told the real
    size); plane-locked bodies keep their friction (`M4-PHY-07`, rapier PR above);
    `examples/physics` keeps its bodies in the plane; an inline `svg`'s own opacity and
    transform apply once, not also in its markup (`M3-SVG-04`).
  - iron_ts (another session, branch `claude/recursing-wright-856ffa`, ~106 commits ahead):
    an AOT compiler from TypeScript to native code (oxc → Iron IR → Cranelift) with a precise
    GC and closures; the zombie sim and the whole SineDemo compile and run natively (M9.b2).
    Not merged yet.
- **Next (in this order, see TODO.md):**
  1. Merge iron_ts back (from `claude/recursing-wright-856ffa`; history has been rewritten, so
     check its base first; bring it in as linear commits, no merge commits, as with other
     agents' work).
  2. Fog (TODO.md M7): linear, exponential and exponential-squared, then height fog and
     volumetric shafts; the fish tank's depth haze can then use it.
  3. The editor (M5.e): the manual check of the Scene view controls, an Inspector bound to
     the scene (`@property` fields), undo/redo, accessibility; then M6 (Script IO).
  - Follow-ups left inside M5.c / M5.d are unticked items there (TAA, refraction, LOD
    cross-fade, morph targets, linked prefabs, cloth self-collision, mobile texture
    formats, ...): later.
  - "Optimisation fixes" in TODO.md (backdrop blur cost, the reload benchmark, QuickJS
    allocation in hot maths, batching hundreds of 2D shapes, the rest of the frame) wait until
    they're picked.
  - Also in TODO.md: a canvas UI element embedding the 3D engine in React UI (an Electron
    replacement); the zombie game's multiplayer (waits on M6 / M8).
  - Also open in TODO.md: the rest of M6 file IO, more classic test scenes (downloads
    need the user's OK first).
  - API names and units follow Unity/s&box (degrees for Euler angles). Decided: own wgpu
    renderer; `iron_scene` without an ECS. Later items in TODO.md: GPU-tessellated SVG with
    SMIL and path morphing; UI and SVG editors (M5.e).
  Milestones: M5.a–e objects, scene, assets, rendering, editor; M6 Script IO; M7 lighting; M8
  multiplayer; M9.a–c backends, AOT, release packaging; M10 optional (OpenXR, Steam Audio, ray
  tracing). Commit per step; refresh docs once per group (Sonnet subagent, changed code only).
- **Also planned:** "Script IO" (files, fetch, WebSocket) is M6, designed in
  `docs/research/networking-and-files.md`.
- **GPU crashes:** the user's PC has hard-reset with the window open (an existing NVIDIA
  instability, also seen before Iron). Warn before launching the interactive window; prefer
  `--screenshot` runs for checks.
- **Needs the user:** the CI run, deferred until close to the end (the user's call).
