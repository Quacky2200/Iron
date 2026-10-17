# Testing

The plan (kinds, IDs, per-milestone checks) is [TESTS.md](../TESTS.md); the commands are in
[CLAUDE.md](../CLAUDE.md). This page explains how the tests are physically organised so you can find
or add one. Every test that implements a plan item carries its ID in a comment (for example
`// M3-EVT-05`) and tests are named after the behaviour they check.

## Running

```bash
cargo test --workspace          # Rust unit + integration + headless render + golden
bun test                        # js/engine and js/react
bun run build:fixtures          # rebuild tests/fixtures/*.js after changing js/
cargo bench -p iron_ui --bench restyle
```

The workspace `Cargo.toml` sets `[profile.dev.package."*"] opt-level = 3`: dependencies (QuickJS,
Parley, swash, wgpu) are optimized even in debug builds, Iron's own crates are not.

CI ([ci.yml](../.github/workflows/ci.yml)) runs fmt, clippy `-D warnings`, build and test on Windows
and Ubuntu (software Vulkan via Mesa, WARP on Windows), and a JS job that builds examples,
typechecks, runs `bun test` and checks that **committed fixtures match their TypeScript sources**
(`build:fixtures` then `git diff --exit-code tests/fixtures`). Forgetting `build:fixtures` fails CI.

## Where tests live

| Kind | Location | Examples |
|---|---|---|
| Rust unit | `#[cfg(test)]` inside the source file | [css.rs](../crates/iron_ui/src/css.rs) (parser, `rebase_urls`, `var()`), [color_fn.rs](../crates/iron_ui/src/color_fn.rs) (colour functions, `color-mix()`), [input.rs](../crates/iron_platform/src/input.rs), [assets.rs](../crates/iron_core/src/assets.rs), [attrs.rs](../crates/iron_ui/src/attrs.rs), [paint.rs](../crates/iron_ui/src/paint.rs) |
| Rust integration | `crates/*/tests/*.rs` | see below |
| JS unit | `js/*/test/*.test.ts(x)` (`bun:test`) | [reconciler.test.tsx](../js/react/test/reconciler.test.tsx), [events.test.tsx](../js/react/test/events.test.tsx), [engine.test.ts](../js/engine/test/engine.test.ts), [input.test.ts](../js/engine/test/input.test.ts), [physics.test.ts](../js/engine/test/physics.test.ts), [audio.test.ts](../js/engine/test/audio.test.ts) |
| Render (headless) | [iron_render/tests](../crates/iron_render/tests) | `headless.rs`, `ui.rs` |
| Golden images | `tests/golden/<id>.png` | compared inside `ui.rs` |
| Layout fixtures | `tests/layout/*.json` | run by `layout_fixtures.rs` |
| JS bundles for Rust | `tests/fixtures/*.js` (built from `tests/fixtures/src`) | `boxes.js`, `events.js` (also has an `Input`), `hello.js`, `input.js`, `physics.js`, `throws.js` |
| Benchmarks | `crates/*/benches` (Criterion) | `restyle.rs`, `layout.rs`, `commits.rs` |

### `iron_ui` integration tests ([crates/iron_ui/tests](../crates/iron_ui/tests))

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
| `typing_profile.rs` | `#[ignore]`d profile of typing into `examples/form` (needs `bun run build`): `cargo test --release -p iron_ui --test typing_profile -- --ignored --nocapture` |
| `react_quickjs.rs`, `events_quickjs.rs` | Real React in QuickJS driving a real `UiTree` |
| `common/mod.rs` | `Dom` builder helper: builds ops with a CSS string and lays out a 400x400 tree |

Most tests build a tree by emitting ops directly (helpers such as `Dom` or the `Scene` struct in
`iron_render/tests/ui.rs`), which keeps them independent of React. Tests that need the real
reconciler use a fixture bundle instead.

## Fixture bundles

`tests/fixtures/src/*.ts(x)` are tiny games bundled by Bun into IIFEs (`bun build ... --format iife
--define process.env.NODE_ENV='"production"'`; `boxes` and `events` are minified), checked into git,
and embedded with `include_str!("../../../tests/fixtures/events.js")`. Typical test shape:

1. `QuickJsRuntime::new()`, `capture_logs`, `load_bundle("events.js", EVENTS)`.
2. `rt.drain_commits()` and `UiTree::apply_commit` for each (assert no `UiError`s).
3. Drive input through `Interaction::handle` with a dispatch closure that calls `__iron_ui_event`.
4. Run `rt.tick(..)` so React's scheduled work runs, drain commits again, assert on the tree.

`ref.focus()`/`blur()` round-trip as ops: assert on `rt.drain_ops()`.

The JS reconciler test file contains a tiny model of the Rust `UiTree` (it replays ops) to check
that React's mutations are valid sequences without needing Rust.

## Script runtime conformance

[conformance.rs](../crates/iron_script/src/conformance.rs) is a public module of reusable checks and
the `runtime_conformance_tests!` macro, which generates one `#[test]` per check for a backend
constructor ([tests/quickjs.rs](../crates/iron_script/tests/quickjs.rs) is two lines). A future
`deno_core` backend only needs the same two lines. `capture_logs(rt)` installs a log sink and
returns the captured messages.

## Golden images

[testing.rs](../crates/iron_render/src/testing.rs):

- `assert_golden(id, w, h, &pixels, tolerance)` compares RGBA8 pixels with `tests/golden/<id>.png`,
  each channel within `tolerance` (render tests use 3, to allow driver differences). It panics with
  the count of differing pixels and the first mismatch.
- The `Scene` harness in `iron_render/tests/ui.rs` builds a tree from ops and CSS. Its optional
  `prepare` hook (`Scene::prepare`) runs on the laid-out tree just before drawing, for state the
  ops can't set up (hover, focus, text selection, an animation clock), as in the `M3-ANI-06` and
  `M3-SEL-03` goldens.
- `IRON_UPDATE_GOLDEN=1 cargo test -p iron_render` rewrites references instead of comparing.
- Every `assert_golden` call also writes its render to `tests/output/<id>.png`, passing or
  not, for inspection. The folder is git-ignored.
- `bun run screenshots [names...]` builds `iron_app` and renders each example to
  `tests/output/examples-<name>.png` (run `bun run build` first). `tests/output/history/`
  keeps older screenshots for comparison.
  Updating goldens is always explicit; review the PNG by eye before committing.
- `shared_gpu()` is **required** in GPU tests: one device per test binary behind a mutex
  (creating devices from parallel threads can deadlock some Vulkan layers).
- Render tests mostly assert pixels analytically ("this pixel is the border colour") and use a
  golden only as a regression net. They render a scene built through the real `UiTree`, so draw-list
  and renderer bugs both show up. `RenderStats` is asserted for batching and layer counts.

## Benchmarks

Criterion benches track, not gate: [restyle.rs](../crates/iron_ui/benches/restyle.rs) (incremental
restyle on a ~2,200 node tree, M3-BEN-01), [layout.rs](../crates/iron_ui/benches/layout.rs) (M2-BEN-*),
[commits.rs](../crates/iron_script/benches/commits.rs) (commit crossing cost).

## Mapping to TESTS.md

| ID prefix | Area | Main files |
|---|---|---|
| `M1-PLT` | Platform input conversion | `iron_platform/src/{input,platform}.rs` |
| `M1-RND` | Headless GPU, clear, 0x0 resize | `iron_render/tests/headless.rs`, `tests/golden/M1-RND-02.png` |
| `M1-SCR`, `M1-CMP` | Script runtime, components | `iron_script/src/conformance.rs`, `iron_script/tests`, `js/engine/test` |
| `M2-REC` | React reconciler ops | `js/react/test/reconciler.test.tsx` |
| `M2-UI`, `M2-STY`, `M2-VIS`, `M2-LAY` | Tree, styles, visuals, layout | `iron_ui/tests/{tree,layout_fixtures}.rs`, unit tests in `style.rs`/`paint.rs`/`calc.rs` |
| `M2-DRW` | Renderer | `iron_render/tests/ui.rs` + `tests/golden/M2-DRW-*` |
| `M2-INT`, `M2-SCR` | React in QuickJS driving the tree | `iron_ui/tests/react_quickjs.rs` |
| `M3-TXT`, `M3-FNT` | Text and fonts; right-to-left `direction`/`dir` (`M3-TXT-09`) | `iron_ui/tests/text.rs`, `iron_ui/tests/direction.rs` (plus rtl cases in `editor.rs`), `text_props.rs` (`M3-TXT-10..15`), `ui.rs` goldens `M3-TXT-*` |
| `M3-CSS` | Stylesheets and cascade; native nesting (`M3-CSS-11`); `@import` expansion, relative paths, cycles (`M3-CSS-12`); `@font-face` source choice (`M3-FNT-03`, `css.rs` unit) | `iron_ui/src/css.rs` tests, `iron_ui/tests/css.rs` |
| `M3-MQ` | `@media`: query parsing and matching (`M3-MQ-01`, unit in `media.rs`); rules applying and un-applying (`M3-MQ-02`) | `iron_ui/src/media.rs`, `iron_ui/tests/css.rs` |
| `M3-GAME` | Game extensions: `background-image-tint` / `border-image-tint` (`M3-GAME-01`, golden in `iron_render/tests/ui.rs` using `tests/fixtures/images/gray.png`), UI scale (`M3-GAME-02`) | `iron_render/tests/ui.rs`, `iron_app/src/ui_scale.rs` units, `js/engine/test/engine.test.ts` |
| `M3-EVT` | Hit testing, events, focus, clipping, scrolling | `iron_ui/tests/{hit,pointer,focus,clip,scroll,events_quickjs}.rs`, `js/react/test/events.test.tsx`, goldens `M3-EVT-05-clip` and `M3-EVT-05-scroll` (scrolled content clipped to a rounded scroller with overlay scrollbars); `scrollbar-gutter` and `overscroll-behavior` (`M3-SCR-01`, `scroll.rs`) |
| `M3-INP` | Text fields: editing, selection, controlled values, clipboard, shortcuts on non-Latin layouts (`M3-INP-02`) | `iron_ui/tests/editor.rs`, `iron_ui/tests/events_quickjs.rs` (real React field), `js/react/test/events.test.tsx` (`onChange`, controlled restore), golden `M3-INP-01-fields` in `iron_render/tests/ui.rs` (filled input, placeholder, focused input with selection) |
| `M3-IME` | IME composition | `iron_platform/src/input.rs` (`converts_ime_events`), `iron_ui/tests/editor.rs` |
| `M3-PAD` | Gamepads and game input | `iron_platform/src/gamepad.rs` (names, connect/disconnect conversion), `iron_app/src/game_input.rs` (nav mapping, stick hysteresis, `script_args`, and a QuickJS test running fixture `tests/fixtures/src/input.ts`), `iron_ui/tests/navigation.rs`, `js/engine/test/input.test.ts` (keys, blur, pad state and events, throwing listeners) |
| `M3-IMG` | `image` element: natural-size measuring, object-fit/position, rounded clip | `iron_ui/tests/image.rs`, reconciler test in `js/react/test` |
| `M3-BG`, `M3-BRD`, `M3-CLIP` | Backgrounds (conic and repeating gradients, layers, `background-clip`, round/space, blend modes: `M3-BG-01..03`; gradient size/position/repeat: `M3-BG-04`), borders (styles, per-side colours, elliptical radii, outline, `border-image`: `M3-BRD-01..04`), polygon and path clips (`M3-CLIP-01`) | `iron_render/tests/ui.rs` (pixel asserts and goldens), `iron_ui/tests/{clip,hit}.rs`, unit tests in `paint.rs`, `style.rs`, `svg_path.rs` and `iron_render/src/ui/images.rs` |
| `M3-FX` | Effects: opacity percentages (`M3-FX-01`, unit), filter chains, drop shadows and backdrop colour filters (`M3-FX-02`, golden plus `paint.rs`/`style.rs` units), `image-rendering` (`M3-FX-03`, golden plus `text_style.rs` unit), masks (`M3-FX-04`, golden plus `style.rs` unit) | `iron_render/tests/ui.rs`, unit tests in `paint.rs`, `style.rs`, `text_style.rs` |
| `M3-XF`, `M3-VIS` | Transforms (`M3-XF-01` unit tests in `iron_ui/src/transform.rs`: function maths, inversion, backfaces, parsing; `M3-XF-02` integration in `layout_props.rs` plus a golden in `iron_render/tests/ui.rs`: rotated bordered box with text, skewed gradient, perspective card with a clipped child, scaled text); `visibility` (`M3-VIS-01`) | `iron_ui/src/transform.rs`, `iron_ui/tests/layout_props.rs`, `iron_render/tests/ui.rs`, golden `tests/golden/M3-XF-02.png` |
| `M3-LST` | Lists: markers, numbering, `::marker`, inside/outside | `iron_ui/tests/text_props.rs`, golden `M3-LST-01` in `iron_render/tests/ui.rs` |
| `M3-SEL` | Static text selection, `::selection`, `::placeholder` | `iron_ui/tests/selection.rs` |
| `M3-CUR` | `cursor` image lists, field cursors, cursor decoding | `iron_ui/tests/{pointer,editor}.rs`, `iron_app/src/cursor.rs` units |
| `M3-DLG` | `dialog`, top layer, `::backdrop`, focus and close events | `iron_ui/tests/dialog.rs`, `js/react/test/events.test.tsx`, golden `M3-DLG-02` in `iron_render/tests/ui.rs` |
| `M3-SVG` | SVG files as images, inline `<svg>` (see [svg.md](svg.md#tests)) | `iron_ui/tests/svg.rs`, unit tests in `iron_ui/src/svg.rs`, goldens `M3-SVG-01` and `M3-SVG-03` in `iron_render/tests/ui.rs` |
| `M3-BEN`, `M2-BEN` | Benchmarks | `benches/` |
| `M4-PHY` | Physics (`M4-PHY-04`, ID kept across hot reload, is pending): box falls and rests (`01`), collision start/stop with script IDs, also through TS components (`02`), character controller climbs a step and stops at a wall (`03`), raycasts answered after the step (`05`), bad ops rejected and meshes collide (`06`) | `iron_physics/tests/{physics,scripts}.rs` (scripts.rs runs fixture `tests/fixtures/src/physics.ts` in QuickJS), `js/engine/test/physics.test.ts` |
| `M4-AUD` | Audio, mixed offline: output then ended event (`01`), emitters fall off with distance and pan (`02`), volume, pan, buses, stop (`03`), dB conversion and bad ops (`04`) | `iron_audio/tests/audio.rs` (`OfflineBackend::render`), `js/engine/test/audio.test.ts` |
| `*-MAN` | Manual checks (resize, IME, gamepad) | no automated test; listed in TODO.md. Text input and gamepad checks (`M3-MAN-01`) use `examples/form` |

When you add a test for a plan item, put the ID in a comment, tick the checkbox in TODO.md and add
the ID to TESTS.md if it is new (working agreement in CLAUDE.md).
