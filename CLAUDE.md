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
| `crates/iron_ui` | UI tree built from React commits (opcodes in `opcodes.rs`), styles (`style.rs`, `text_style.rs`, `paint.rs`), Taffy layout (`layout.rs`), Parley text (`text.rs`), CSS (`css.rs` parser, `cascade.rs` matching/cascade/invalidation), draw list (`draw.rs`), hit testing (`tree.rs`), pointer events + `:hover`/`:active` (`events.rs`), focus/keys/default actions (`interaction.rs`) |
| `crates/iron_render` | wgpu renderer for the draw list: quads, SDF rounded boxes, shadows, blur layers, gradients, images, glyph atlas (swash); `render_overlay` blends UI over the 3D world. `world/` is the 3D pass (`WorldRenderer`, `WorldCamera`, `WorldQuad`). `testing.rs` has golden helpers and a shared test GPU |
| `crates/iron_scene` | GameObjects: generational `ObjectId`s, names, enabled/active, parent/children, local `Transform` (glam) and cached world matrices, `take_moved()`; `ScriptScene` runs `scene.*` ops (script ids, engine ids from 2^30) and batches engine moves for `__iron_scene` |
| `crates/iron_physics` | rapier world driven by `physics.*` ops (script-chosen body/collider IDs); `step()` per fixed tick, `take_batch()` → `__iron_physics(moved, contacts, rays, characters)` |
| `crates/iron_audio` | kira mixer driven by `audio.*` ops: sounds (decoded or streamed), playbacks, buses, spatial emitters; `Mixer` trait over a device or `OfflineBackend`; finished sounds → `__iron_audio(ended)` |
| `crates/iron_app` (panels) | `panels.rs`: world panels (a UI tree + interaction + texture each), camera rays and panel hits; `world_view.rs`: cameras and planes from `render.*` ops; `main.rs` routes the pointer and keys between the screen and panels (`Surface`) |
| `crates/iron_app` | The `iron` binary: `iron <game folder> [--screenshot out.png] [--frames N]`. Loads `dist/game.js` + `dist/game.css` and wires scripts → UI → renderer |
| `js/engine` | `@iron/engine`: `Component`, `register`, `op`, `loadFont`, `addStyleSheet`/`removeStyleSheet`, `input` (keys, gamepads), `physics` (`RigidBody`, `Collider`, raycasts, character controller), `audio` (`Sound`, `Bus`, `Emitter`), `GameObject`/`Transform`/`Component` (scene.ts), Unity-style math (math.ts), `DateTime`/`TimeSpan` (time.ts), `Time`/`TimeSince`/`Task` (clock.ts), `Debug`/`Log`/`Console`, `Random` |
| `js/react` | `@iron/react`: reconciler (`host.ts`), UI event dispatch (`events.ts`), style/attr diffing, `createRoot`, `Text`/`Span` |
| `examples/*` | `hello`, `boxes`, `text`, `css`, `form`, `physics`, `world` (3D), `platformer` (2D), `classics` (test models) games (each has `bun run build`) |
| `docs/` | Implementation docs: how the code works today (pipeline, crates, events, rendering, testing). Start at `docs/README.md` |
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

- `bun run build` builds all the examples.
- `bun run build:fixtures` rebuilds `tests/fixtures`. Run it after changing js/.
- `bun run dev` in an example rebuilds on every edit (`bun build --watch`); a running `iron` window
  reloads the scripts when `dist/` changes (state isn't kept yet; see TODO.md M5).
- The `cargo run` line renders a screenshot and exits; check it by eye.
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

## Status (2026-10-17)
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
    camera system, `WorldPanel` (React UI on planes). M5.a2 step 1: `Rigidbody`/colliders.
- **Next (M5.a2, see TODO.md for the exact order):** meshes and lights are done (`MeshRenderer`, `Light`,
  `createPrimitive`), CharacterController, joints/rope, AudioSource/AudioListener, textures/sprites, particles,
  glows (halos, bloom, spot beams), 2D, Cursor, Application/SystemInfo, glTF models,
  the missing-texture checker, directional shadows and the M5.a2 docs too. Left in M5.a2:
  collider scale changes, the rope tube, the Cornell box; then M5.b (TODO.md). API names and units follow Unity/s&box
  (degrees for Euler angles). Decided: own wgpu renderer; `iron_scene` without an ECS. Later items in TODO.md: GPU-tessellated SVG with SMIL and path
  morphing; UI and SVG editors (M5.e).
  Milestones: M5.a–e objects, scene, assets, rendering, editor; M6 Script IO; M7 lighting; M8
  multiplayer; M9.a–c backends, AOT, release packaging; M10 optional (OpenXR, Steam Audio, ray
  tracing). Commit per step; refresh docs once per group (Sonnet subagent, changed code only).
- **Also planned:** "Script IO" (files, fetch, WebSocket) is M6, designed in
  `docs/research/networking-and-files.md`.
- **GPU crashes:** the user's PC has hard-reset with the window open (an existing NVIDIA
  instability, also seen before Iron). Warn before launching the interactive window; prefer
  `--screenshot` runs for checks.
- **Needs the user:** the CI run, deferred until close to the end (the user's call).
