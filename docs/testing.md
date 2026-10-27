# Testing

The plan (kinds, IDs, per-milestone checks) is [TESTS.md](../TESTS.md); the commands are in
[CLAUDE.md](../CLAUDE.md). This page explains how the tests are physically organised so you can find
or add one. Every test that implements a plan item carries its ID in a comment (for example
`// M3-EVT-05`) and tests are named after the behaviour they check.

## Running

```bash
cargo test --workspace          # Rust unit + integration + headless render + golden
bun test                        # js/engine and js/react
bun run typecheck               # js/*, editor, fixtures with tsc; then every example via platypus-build check --tsc
bun run build:fixtures          # rebuild tests/fixtures/*.js after changing js/
cargo bench -p platypus_ui --bench restyle
```

The workspace `Cargo.toml` sets `[profile.dev.package."*"] opt-level = 3`: dependencies (QuickJS,
Parley, swash, wgpu) are optimized even in debug builds, Platypus's own crates are not.

CI ([ci.yml](../.github/workflows/ci.yml)) runs fmt, clippy `-D warnings`, build and test on Windows
and Ubuntu (software Vulkan via Mesa, WARP on Windows), and a JS job that builds examples,
typechecks, runs `bun test` and checks that **committed fixtures match their TypeScript sources**
(`build:fixtures` then `git diff --exit-code tests/fixtures`). Forgetting `build:fixtures` fails CI.

## Where tests live

| Kind | Location | Examples |
|---|---|---|
| Rust unit | `#[cfg(test)]` inside the source file | [css.rs](../crates/platypus_ui/src/css.rs) (parser, `rebase_urls`, `var()`), [color_fn.rs](../crates/platypus_ui/src/color_fn.rs) (colour functions, `color-mix()`), [input.rs](../crates/platypus_platform/src/input.rs), [assets.rs](../crates/platypus_core/src/assets.rs), [attrs.rs](../crates/platypus_ui/src/attrs.rs), [paint.rs](../crates/platypus_ui/src/paint.rs) |
| Rust integration | `crates/*/tests/*.rs` | see below |
| JS unit | `js/*/test/*.test.ts(x)` (`bun:test`) | [reconciler.test.tsx](../js/react/test/reconciler.test.tsx), [events.test.tsx](../js/react/test/events.test.tsx), [engine.test.ts](../js/engine/test/engine.test.ts), [input.test.ts](../js/engine/test/input.test.ts), [physics.test.ts](../js/engine/test/physics.test.ts), [audio.test.ts](../js/engine/test/audio.test.ts), [scene.test.ts](../js/engine/test/scene.test.ts), [math.test.ts](../js/engine/test/math.test.ts), [time.test.ts](../js/engine/test/time.test.ts), [helpers.test.ts](../js/engine/test/helpers.test.ts), [camera.test.ts](../js/engine/test/camera.test.ts) |
| Render (headless) | [platypus_render/tests](../crates/platypus_render/tests) | `headless.rs`, `ui.rs`, `world.rs` (world pass, M5-RND-05 to 15, M5-CUL-02) |
| Golden images | `tests/golden/<id>.png` | compared inside `ui.rs` |
| Layout fixtures | `tests/layout/*.json` | run by `layout_fixtures.rs` |
| JS bundles for Rust | `tests/fixtures/*.js` (built from `tests/fixtures/src`) | `boxes.js`, `events.js` (also has an `Input`), `hello.js`, `input.js`, `physics.js`, `physics2d.js`, `rigidbodies.js`, `scene.js`, `throws.js`; `refresh-vendor.js` with `refresh-v1.js` / `refresh-v2.js`, a dev build pair for M4-HOT-06 (`build:refresh`: the vendor IIFE, and the game as CommonJS with `--react-fast-refresh` and a `LABEL` define) |
| Benchmarks | `crates/*/benches` (Criterion) | `restyle.rs`, `layout.rs`, `commits.rs` |

### `platypus_ui` integration tests ([crates/platypus_ui/tests](../crates/platypus_ui/tests))

| File | Covers |
|---|---|
| `tree.rs` | Commit semantics: ops, moves, removal, errors, text nodes |
| `layout_fixtures.rs` | JSON layout cases through the full commit pipeline, generic over `LayoutEngine` |
| `text.rs` | Fonts (including WOFF/WOFF2 from `tests/fixtures/fonts/`, M3-FNT-03), wrapping, inheritance, shaping counts |
| `text_props.rs` | M3d group 4 text properties through the tree: transform, ellipsis, white-space/word-break, font shorthand and features, decorations, stroke, smoothing |
| `selection.rs` | Static text selection (M3-SEL-01/02): drag, click, double and triple click, Ctrl+C, `user-select`, transforms, highlight and `::selection` draw commands, `::placeholder` |
| `dialog.rs` | Dialogs (M3-DLG-01): hidden, open and modal layout, top-layer paint and hit order, `::backdrop`, focus trap and restore, Escape and backdrop `cancel`/`close`, `closedBy`, `[open]`/`[modal]` selectors |
| `motion.rs` | Transitions, `@keyframes` animations, `@starting-style` and `:outro` removal (M3-ANI-02..05); timing/interpolation/parsing units (M3-ANI-01) are in `motion.rs` and `css.rs` source |
| `css.rs` | Selectors, cascade, variables, `::before/::after`, invalidation |
| `hit.rs` | Hit testing rules |
| `pointer.rs` | `Pointer`: hover/active/click/dblclick/wheel |
| `focus.rs` | Tab order, focus events, `:focus-visible` |
| `clip.rs` | `overflow` and `clip-path` (including `polygon()`/`path()`, M3-CLIP-01) in draw list and hit testing |
| `scroll.rs` | Scrolling (M3c step 4b): offsets and hit testing, which axes scroll, wheel chaining, Shift+wheel, key scrolling, Tab into view, script `scrollTo/By/IntoView`, scrollbar geometry, thumb drag and track paging, inherited `scrollbarColor` |
| `editor.rs` | Text fields (M3c step 5): sizing from the font, typing and editing keys with `input` events, textarea Enter and line moves, prevented `keydown`, controlled `value` / `defaultValue`, read-only and disabled, tab order, pointer caret / word / drag selection, cut / copy / paste requests, draw list (selection, blinking caret, dimmed placeholder, caret kept in view), text cursor, IME pre-edit and commit, focus loss dropping pre-edit |
| `navigation.rs` | Gamepad UI navigation (M3-PAD-01): `Input::Navigate` picks the nearest focusable element in a direction (in-line beats closer), nothing without focus, `Input::Activate` clicks with detail 0 |
| `units.rs` | Relative units and math (M3d step 1): `em` from own font size, `rem`, viewport units, `calc`/`min`/`max`/`clamp` (including `%` math in sizes, margins, padding, gaps and shorthands), `boxSizing`, restyle when font size or window size changes, invalid values rejected |
| `layout_props.rs` | Layout properties (M3d steps 2a-2c; `order` re-sorting layout, `zIndex` paint and hit order, Tab order unchanged, `display: contents` flattening and `position: fixed` placement, paint and hit): `gridTemplateAreas` and `gridArea` by name or lines, `gridTemplate` forms, line longhands, `gridAutoRows`, `placeItems`/`placeSelf` setting both axes, bad grid/place/logical values rejected, logical properties mapped left-to-right |
| `layout_props.rs` (more) | Transforms (M3-XF-02): hit testing follows rotation, 3D turns and `backface-visibility`, every property form and keyword is accepted and resets, `preserve-3d` combines turns before flattening; `visibility` (M3-VIS-01): hidden boxes keep space but draw and hit nothing, visible children and spans still do |
| `wide_keywords.rs` | CSS-wide keywords (M3d step 1c): `inherit` on non-inherited properties, `initial` stopping inheritance, `unset`/`revert`, stylesheets without warnings, `inherit` at the root, `::before` inheriting |
| `typing_profile.rs` | `#[ignore]`d profile of typing into `examples/form` (needs a Bun-built `examples/form/dist/game.js`, no longer made by an example script): `cargo test --release -p platypus_ui --test typing_profile -- --ignored --nocapture` |
| `react_quickjs.rs`, `events_quickjs.rs` | Real React in QuickJS driving a real `UiTree` |
| `common/mod.rs` | `Dom` builder helper: builds ops with a CSS string and lays out a 400x400 tree |

Most tests build a tree by emitting ops directly (helpers such as `Dom` or the `Scene` struct in
`platypus_render/tests/ui.rs`), which keeps them independent of React. Tests that need the real
reconciler use a fixture bundle instead.

## Fixture bundles

`tests/fixtures/src/*.ts(x)` are tiny games bundled by Bun into IIFEs (`bun build ... --format iife
--define process.env.NODE_ENV='"production"'`; `boxes` and `events` are minified; the
`refresh-*` dev build pair is the exception, see above), checked into git,
and embedded with `include_str!("../../../tests/fixtures/events.js")`. Typical test shape:

1. `QuickJsRuntime::new()`, `capture_logs`, `load_bundle("events.js", EVENTS)`.
2. `rt.drain_commits()` and `UiTree::apply_commit` for each (assert no `UiError`s).
3. Drive input through `Interaction::handle` with a dispatch closure that calls `__platypus_ui_event`.
4. Run `rt.tick(..)` so React's scheduled work runs, drain commits again, assert on the tree.

`ref.focus()`/`blur()` round-trip as ops: assert on `rt.drain_ops()`.

The JS reconciler test file contains a tiny model of the Rust `UiTree` (it replays ops) to check
that React's mutations are valid sequences without needing Rust.

## Script runtime conformance

[conformance.rs](../crates/platypus_script/src/conformance.rs) is a public module of reusable checks and
the `runtime_conformance_tests!` macro, which generates one `#[test]` per check for a backend
constructor ([tests/quickjs.rs](../crates/platypus_script/tests/quickjs.rs) is two lines). A future
`deno_core` backend only needs the same two lines. `capture_logs(rt)` installs a log sink and
returns the captured messages.

## Golden images

[testing.rs](../crates/platypus_render/src/testing.rs):

- `assert_golden(id, w, h, &pixels, tolerance)` compares RGBA8 pixels with `tests/golden/<id>.png`,
  each channel within `tolerance` (render tests use 3, to allow driver differences). It panics with
  the count of differing pixels and the first mismatch.
- The `Scene` harness in `platypus_render/tests/ui.rs` builds a tree from ops and CSS. Its optional
  `prepare` hook (`Scene::prepare`) runs on the laid-out tree just before drawing, for state the
  ops can't set up (hover, focus, text selection, an animation clock), as in the `M3-ANI-06` and
  `M3-SEL-03` goldens.
- `PLATYPUS_UPDATE_GOLDEN=1 cargo test -p platypus_render` rewrites references instead of comparing.
- Every `assert_golden` call also writes its render to `tests/output/<id>.png`, passing or
  not, for inspection. The folder is git-ignored.
- `bun run screenshots [names...]` builds `platypus_app` and renders each example to
  `tests/output/examples-<name>.png` (build the examples with `platypus-build build` first). `tests/output/history/`
  keeps older screenshots for comparison.
  Updating goldens is always explicit; review the PNG by eye before committing.
- `shared_gpu()` is **required** in GPU tests: one device per test binary behind a mutex
  (creating devices from parallel threads can deadlock some Vulkan layers).
- Render tests mostly assert pixels analytically ("this pixel is the border colour") and use a
  golden only as a regression net. They render a scene built through the real `UiTree`, so draw-list
  and renderer bugs both show up. `RenderStats` is asserted for batching and layer counts.

## Benchmarks

Criterion benches track, not gate: [restyle.rs](../crates/platypus_ui/benches/restyle.rs) (incremental
restyle on a ~2,200 node tree, M3-BEN-01), [layout.rs](../crates/platypus_ui/benches/layout.rs) (M2-BEN-*),
[commits.rs](../crates/platypus_script/benches/commits.rs) (commit crossing cost),
[reload.rs](../crates/platypus_scene/benches/reload.rs) (a state-keeping hot reload of 1,000 components
with a 1 MB bundle, M4-HOT-07). It measured about 146 ms on 2026-10-17 against the 100 ms target, so
the target is not met yet: ~54 ms parsing the bundle, ~27 ms saving, ~58 ms restoring.
Run it with `cargo bench -p platypus_scene --bench reload`.

## The `tools/` workspace

The experimental TypeScript compiler (`tools/platypus_ts`, `platypus_ts_rt`, `platypus_ts_tests`) is a separate
cargo workspace, so `cargo test --workspace` at the repo root does not run it. Run
`cargo test --workspace` from `tools/` instead; `PLATYPUS_TS_GC=stress cargo test -p platypus_ts_tests` repeats
the tests with a garbage collection at every allocation. Its tests are `M9-TS-*`. See
[platypus-ts.md](platypus-ts.md) for how they are organised and how to add one.

## Mapping to TESTS.md

| ID prefix | Area | Main files |
|---|---|---|
| `M1-PLT` | Platform input conversion | `platypus_platform/src/{input,platform}.rs` |
| `M1-RND` | Headless GPU, clear, 0x0 resize | `platypus_render/tests/headless.rs`, `tests/golden/M1-RND-02.png` |
| `M1-SCR`, `M1-CMP` | Script runtime, components | `platypus_script/src/conformance.rs`, `platypus_script/tests`, `js/engine/test` |
| `M2-REC` | React reconciler ops | `js/react/test/reconciler.test.tsx` |
| `M2-UI`, `M2-STY`, `M2-VIS`, `M2-LAY` | Tree, styles, visuals, layout | `platypus_ui/tests/{tree,layout_fixtures,layout_cache}.rs` (`M2-LAY-07`: size-query cache, text measure counts), `scroll.rs` (`M2-DRW-09`: draw culling), unit tests in `style.rs`/`paint.rs`/`calc.rs` |
| `M2-DRW` | Renderer | `platypus_render/tests/ui.rs` + `tests/golden/M2-DRW-*` |
| `M2-INT`, `M2-SCR` | React in QuickJS driving the tree | `platypus_ui/tests/react_quickjs.rs` |
| `M3-TXT`, `M3-FNT` | Text and fonts; right-to-left `direction`/`dir` (`M3-TXT-09`) | `platypus_ui/tests/text.rs`, `platypus_ui/tests/direction.rs` (plus rtl cases in `editor.rs`), `text_props.rs` (`M3-TXT-10..15`), `ui.rs` goldens `M3-TXT-*` |
| `M3-CSS` | Stylesheets and cascade; native nesting (`M3-CSS-11`); `@import` expansion, relative paths, cycles (`M3-CSS-12`); spaces between minified function calls (`M3-CSS-13`, `css.rs` unit); `@font-face` source choice (`M3-FNT-03`, `css.rs` unit) | `platypus_ui/src/css.rs` tests, `platypus_ui/tests/css.rs` |
| `M3-MQ` | `@media`: query parsing and matching (`M3-MQ-01`, unit in `media.rs`); rules applying and un-applying (`M3-MQ-02`) | `platypus_ui/src/media.rs`, `platypus_ui/tests/css.rs` |
| `M3-GAME` | Game extensions: `background-image-tint` / `border-image-tint` (`M3-GAME-01`, golden in `platypus_render/tests/ui.rs` using `tests/fixtures/images/gray.png`), UI scale (`M3-GAME-02`) | `platypus_render/tests/ui.rs`, `platypus_app/src/ui_scale.rs` units, `js/engine/test/engine.test.ts` |
| `M3-EVT` | Hit testing, events, focus, clipping, scrolling | `platypus_ui/tests/{hit,pointer,focus,clip,scroll,events_quickjs}.rs`, `js/react/test/events.test.tsx`, goldens `M3-EVT-05-clip` and `M3-EVT-05-scroll` (scrolled content clipped to a rounded scroller with overlay scrollbars); `scrollbar-gutter` and `overscroll-behavior` (`M3-SCR-01`, `scroll.rs`); inline spans as hit targets (`M3-EVT-07`, `hit.rs`); smooth scrolling, `scroll-behavior` (`M3-EVT-09`); `scroll-padding` in `scrollIntoView` (`M3-EVT-10`, `scroll.rs`) |
| `M3-INP` | Text fields: editing, selection, controlled values, clipboard, shortcuts on non-Latin layouts (`M3-INP-02`) | `platypus_ui/tests/editor.rs`, `platypus_ui/tests/events_quickjs.rs` (real React field), `js/react/test/events.test.tsx` (`onChange`, controlled restore), golden `M3-INP-01-fields` in `platypus_render/tests/ui.rs` (filled input, placeholder, focused input with selection) |
| `M3-IME` | IME composition | `platypus_platform/src/input.rs` (`converts_ime_events`), `platypus_ui/tests/editor.rs` |
| `M3-PAD` | Gamepads and game input | `platypus_platform/src/gamepad.rs` (names, connect/disconnect conversion, `announce_once` dropping repeat connects: `M3-PAD-02`), `platypus_app/src/game_input.rs` (nav mapping, stick hysteresis, `script_args`, and a QuickJS test running fixture `tests/fixtures/src/input.ts`), `platypus_ui/tests/navigation.rs`, `js/engine/test/input.test.ts` (keys, blur, pad state and events, throwing listeners) |
| `M3-IMG` | `image` element: natural-size measuring, object-fit/position, rounded clip | `platypus_ui/tests/image.rs`, reconciler test in `js/react/test` |
| `M3-BG`, `M3-BRD`, `M3-CLIP` | Backgrounds (conic and repeating gradients, layers, `background-clip`, round/space, blend modes: `M3-BG-01..03`; gradient size/position/repeat: `M3-BG-04`), borders (styles, per-side colours, elliptical radii, outline, `border-image`: `M3-BRD-01..04`), polygon and path clips (`M3-CLIP-01`) | `platypus_render/tests/ui.rs` (pixel asserts and goldens), `platypus_ui/tests/{clip,hit}.rs`, unit tests in `paint.rs`, `style.rs`, `svg_path.rs` and `platypus_render/src/ui/images.rs` |
| `M3-FX` | Effects: opacity percentages (`M3-FX-01`, unit), filter chains, drop shadows and backdrop colour filters (`M3-FX-02`, golden plus `paint.rs`/`style.rs` units), `image-rendering` (`M3-FX-03`, golden plus `text_style.rs` unit), masks (`M3-FX-04`, golden plus `style.rs` unit) | `platypus_render/tests/ui.rs`, unit tests in `paint.rs`, `style.rs`, `text_style.rs` |
| `M3-XF`, `M3-VIS` | Transforms (`M3-XF-01` unit tests in `platypus_ui/src/transform.rs`: function maths, inversion, backfaces, parsing; `M3-XF-03` `as_scale_translate` recognises scale-and-move maps and rejects rotation, uneven scale, perspective, mirroring; `M3-XF-02` integration in `layout_props.rs` plus a golden in `platypus_render/tests/ui.rs`: rotated bordered box with text, skewed gradient, perspective card with a clipped child, scaled text re-rasterized at its size); `visibility` (`M3-VIS-01`) | `platypus_ui/src/transform.rs`, `platypus_ui/tests/layout_props.rs`, `platypus_render/tests/ui.rs`, golden `tests/golden/M3-XF-02.png` |
| `M3-LST` | Lists: markers, numbering, `::marker`, inside/outside | `platypus_ui/tests/text_props.rs`, golden `M3-LST-01` in `platypus_render/tests/ui.rs` |
| `M3-SEL` | Static text selection, `::selection`, `::placeholder` | `platypus_ui/tests/selection.rs` |
| `M3-CUR` | `cursor` image lists, field cursors, cursor decoding | `platypus_ui/tests/{pointer,editor}.rs`, `platypus_app/src/cursor.rs` units |
| `M3-DLG` | `dialog`, top layer, `::backdrop`, focus and close events | `platypus_ui/tests/dialog.rs`, `js/react/test/events.test.tsx`, golden `M3-DLG-02` in `platypus_render/tests/ui.rs` |
| `M3-SVG` | SVG files as images, inline `<svg>` (see [svg.md](svg.md#tests)) | `platypus_ui/tests/svg.rs`, unit tests in `platypus_ui/src/svg.rs`, goldens `M3-SVG-01` and `M3-SVG-03` in `platypus_render/tests/ui.rs` |
| `M3-BEN`, `M2-BEN`, `M4-HOT-07` | Benchmarks | `benches/` |
| `M4-PHY` | Physics (`M4-PHY-04`, ID kept across hot reload, not done): box falls and rests (`01`), collision start/stop with script IDs, also through TS components (`02`), character controller climbs a step and stops at a wall (`03`), raycasts answered after the step (`05`), bad ops rejected and meshes collide (`06`) | `platypus_physics/tests/{physics,scripts}.rs` (scripts.rs runs fixture `tests/fixtures/src/physics.ts` in QuickJS), `js/engine/test/physics.test.ts` |
| `M4-AUD` | Audio, mixed offline: output then ended event (`01`), emitters fall off with distance and pan (`02`), volume, pan, buses, stop (`03`), dB conversion and bad ops (`04`) | `platypus_audio/tests/audio.rs` (`OfflineBackend::render`), `js/engine/test/audio.test.ts` |
| `M4-HOT` | Hot reload: new code runs after a reload (`01`); script components' `@property` values and plain fields survive, references are remapped, `onStart` doesn't run again, `onReload` does, the clock carries on (`02`); objects, hierarchy and transforms come back (`03`); a syntax error or throwing top level keeps the old code (`04`); a gone property is dropped with a warning and an unregistered saved class makes the reload start over (`05`); clearing the UI tree removes attached and detached script nodes and frees IDs (`09`); the bundle watcher reports a settled change once and ignores other files (`10`). A texture made with `Texture.create` is saved and restored with its pixels and options and re-sent the same, and its material uses it (`11`). `07` is the benchmark above. in place (dev builds): `refresh-vendor.js` and `refresh-v1.js` load, state changes, then `refresh-v2.js` loads through `load_game` in the same runtime. React keeps its `useState` value but renders the new code, a live `Ticker` component answers with its new class and keeps its field, and the style sheet is swapped, not added twice (`06`). `08` is not done | `platypus_script/tests/components.rs` (`01`, `04`, `06`), `platypus_scene/tests/scripts.rs` (`02`, `03`, `05`, fixture `tests/fixtures/src/scene.ts`), `platypus_ui/tests/tree.rs`, unit test in `platypus_app/src/reload.rs`, `js/engine/test/scenes.test.ts` (`11`) |
| `M5-OBJ` | GameObjects and components. Rust: hierarchy, nesting and stale ids (`01`), world transforms and re-parenting (`02`), moved sets and inactive subtrees (`03`) in `platypus_scene/tests/scene.rs`. JS mirror and `addComponent` (`04`, `05`) in `scene.test.ts`. In QuickJS with fixture `tests/fixtures/src/scene.ts`: a spinning turret with a child (`06`), `scene.*` ops, the move batch and ids past 2^24 (`07`), lifecycle order (`08`) in `platypus_scene/tests/scripts.rs` | `platypus_scene/tests/{scene,scripts}.rs`, `js/engine/test/scene.test.ts` |
| `M5-SCN` | Scenes: save and load round trips, load order and warnings (`01`), a save/load trip in QuickJS with fixture `tests/fixtures/src/scene.ts` (`02`, `platypus_scene/tests/scripts.rs`), `SceneManager` (`03`), `GameObject.instantiate` (`04`); `examples/scenes` is the manual check | `js/engine/test/scenes.test.ts`, `platypus_scene/tests/scripts.rs` |
| `M5-MTH` | Math types: `Vector*`, `Mathf` (`01`), `Quaternion` (`02`), `Plane`/`Ray`/`Bounds`/`Rect`/`Pose`/`Color` (`03`) | `js/engine/test/math.test.ts` |
| `M5-TIM` | `TimeSpan` (`01`) and `DateTime` (`02`), millisecond resolution | `js/engine/test/time.test.ts` |
| `M5-HLP` | Helpers: logging (`01`), `Time`, `TimeSince`, `TimeUntil`, `Task` (`02`), seeded `Rng`/`Random` (`03`), Unity-style input (`04`) | `js/engine/test/helpers.test.ts` |
| `M5-CAM` | `Matrix4x4` and camera conversions with viewport rects, `Camera.main`, `CameraFollow`, `CameraOrbit` (`01`) | `js/engine/test/camera.test.ts` |
| `M5-RND` | World pass goldens, `05` to `21` (`21`: a tube skinned to two bones, bent, with a bent shadow): perspective quads with UI over them (`05`), cameras by depth and viewport (`06`), lit primitives (`07`), textures and sprites (`08`), glows and beams (`09`), shadows (`10`), PBR under a sky (`11`), a planar mirror (`12`), a reflection probe (`13`), quality low and high (`14`), culling masks (`15`), the UI over the world with backdrop effects (`16`), material maps and modes (`17`), tonemapping, exposure and FXAA (`18`), a render texture (`19`), cascades and spot and point shadows (`20`); `01`-`04` are still to come | `platypus_render/tests/world.rs`, goldens `tests/golden/M5-RND-*.png` |
| `M5-MAT` / `M5-RTX` / `M5-SHD` / `M5-QUA` | Materials: tangents from UVs (`MAT-01`, unit in `mesh.rs`) and `Material` ops with scene saves (`MAT-02`, `meshes.test.ts`); render textures and `Camera.targetTexture` (`RTX-01`, `js/engine/test/camera.test.ts`); shadow planning (`SHD-01`, unit in `shadows.rs`) and `shadowCascades` (`SHD-02`, `meshes.test.ts`); texture quality settings (`QUA-02`, `quality.test.ts`). The goldens are `M5-RND-17` to `-20` | as listed |
| `M5-APP` | The app binary: `platypus-game` without a game argument runs the current folder (`APP-03`, unit in `platypus_app/src/main.rs`), the log file (`APP-04`, unit in `platypus_app/src/logging.rs`), focus and background pausing: window events (`APP-05`, `platypus_platform/src/input.rs`), muting the mixer (`APP-06`, `platypus_audio/tests/audio.rs`), `Lifecycle` and `FixedClock` (`APP-07`) and the frame cap (`APP-08`) in `platypus_app/src/lifecycle.rs`, the manifest's `runInBackground` and `__platypus_info` (`APP-09`, `platypus_app`), and `Application` and the focus hooks in JS (`APP-10`, `js/engine/test/app.test.ts`) | as listed |
| `M5-ANI` / `M5-SKN` | Animation of objects: curves, clips, `Animation`, `Animator` (`ANI-01..04`, `js/engine/test/animation.test.ts`), `__platypus_animate` after `onUpdate` (`ANI-05`, `platypus_script/tests/components.rs`); glTF skins and animations (`SKN-01`, unit in `platypus_app/src/models.rs` with `tests/fixtures/assets/worm.glb`), skinned renderers and clips from models in JS (`SKN-02`, `models.test.ts`); the skinning golden is `M5-RND-21`. `examples/animation` is the manual check. See [animation-3d.md](animation-3d.md) | as listed |
| `M5-PRE` | Prefabs: capture, independent copies, references, JSON and file round trips (`PRE-01`) | `js/engine/test/scenes.test.ts` |
| `M5-AST` (assets) / `M5-CLO` | Asset watcher and reload under the same ids (`AST-01`, units in `platypus_app/src/reload.rs` and `world_view.rs`), `Assets.onChanged` (`AST-02`), `.meta` files (`AST-03`, `scripts/assets.test.ts`, a Bun test), the app's asset database (`AST-04`, `platypus_app/src/asset_db.rs`), GUIDs and importers in JS (`AST-05`, `js/engine/test/assets.test.ts`), cooking (`AST-06`, `platypus_app/src/cook.rs`); cloth: solver (`CLO-01`, `platypus_physics/src/cloth.rs`), the app's system (`CLO-02`, `platypus_app/src/cloth_sync.rs`) and `Cloth` in JS (`CLO-03`, `cloth.test.ts`). `examples/cloth` is the manual check. TESTS.md also has older planned `M5-AST` rows for a later packaging step | as listed |
| `M5-LAY` / `M5-LOD` / `M5-CUL` | Layers and culling: `scene.layer` (`LAY-01`, `platypus_scene/tests/scripts.rs`), camera masks and cull distances (`LAY-02`, unit test in `world_view.rs`), `LayerMask`, `Camera.cullingMask` and field-of-view conversions (`LAY-03`) and light, probe and mirror masks (`LAY-04`) in `js/engine/test/layers.test.ts`, LOD groups (`LOD-01`, `world_view.rs`; JS in `js/engine/test/lod.test.ts`), frustums and culling stats (`CUL-01`, `CUL-02`, `platypus_render`); the culling-mask golden is `M5-RND-15`. `examples/layers` is the manual check | as listed |
| `M5-EDT` | Editor: `@platypus/editor-ui` models and the React menubar (`06`-`14`, `18`-`20`), log capture (`15`), `editor.openFile` and editor-only ops (`16`, `17`), the editor screenshot by eye (`21`); `01`-`05` are still to come | `js/editor-ui/test/*.test.ts(x)`, units in `platypus_app/src/editor.rs` and `logging.rs`, `bun run screenshots editor` |
| `M5-DND` | Drag and drop: the event order, accepting, cancelling, the drag image and system files in `Interaction` (`01`-`04`, `platypus_ui/tests/dnd.rs`), the app's file-drag state (`05`, `file_drop.rs`), `PlatypusDragEvent` / `dataTransfer` (`06`, `js/react/test/events.test.tsx`), project file ops (`07`, `project.rs`; `08`, `project.test.ts`), end to end through QuickJS and React (`09`, `platypus_ui/tests/dnd_quickjs.rs` with `tests/fixtures/dnd.js`), the project browser (`10`, `project-browser.test.tsx`) | as listed |
| `M5-PNL` | World panels: one commit target per React root (`01`, `js/react/test/reconciler.test.tsx`), ray to panel UV (`02`, unit test in `platypus_app/src/panels.rs`), hover and clicks on the `examples/world` panel (`03`, manual) | as listed |
| `M5-DSP` / `M5-PRF` | Display and prefs: mode fallbacks per platform (`DSP-01`, `platypus_platform/src/display.rs`), saved display settings over the manifest's (`DSP-02`, `platypus_app/src/display.rs`), `Screen`, `targetFrameRate`, `vSyncCount` and saved quality in JS (`DSP-03`, `js/engine/test/display.test.ts`), exclusive fullscreen taking the monitor's current resolution and leaving it restoring the window size (`DSP-04`, `platypus_platform/src/display.rs` and `display.test.ts`), the prefs file (`PRF-01`, `platypus_app/src/prefs.rs`), `PlayerPrefs` in JS (`PRF-02`). `examples/arcade` (Esc) is the manual check | as listed |
| `*-MAN` | Manual checks (resize, IME, gamepad) | no automated test; listed in TODO.md. Text input and gamepad checks (`M3-MAN-01`) use `examples/form` |

When you add a test for a plan item, put the ID in a comment, tick the checkbox in TODO.md and add
the ID to TESTS.md if it is new (working agreement in CLAUDE.md).
