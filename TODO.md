# Iron — TODO

Work plan by milestone. Each task lists the tests from [TESTS.md](TESTS.md) that prove it's done. Architecture lives in [DESIGN.md](DESIGN.md).

Legend: `[ ]` to do · `[~]` in progress · `[x]` done

---

## Decisions needed before / during M1

- [x] Window/input: `winit` + `gilrs`
- [x] Audio: `kira`
- [x] Layout: Taffy first, Yoga fallback behind `LayoutEngine`
- [x] **First script backend:** QuickJS (`rquickjs` 0.14). `deno_core` follows as a second backend.
- [x] Rust edition 2024, latest stable (1.99 at time of writing)
- [ ] Licence: private for now (see README) until a working demo; open-source licence to be chosen then

---

## Proposed repository layout

```
Iron/
├─ Cargo.toml              # workspace
├─ package.json            # Bun workspace root
├─ DESIGN.md  TESTS.md  TODO.md
├─ crates/
│  ├─ iron_core/           # IDs, math re-exports, op buffer types, logging
│  ├─ iron_platform/       # Platform layer: winit + gilrs
│  ├─ iron_render/         # wgpu device, surfaces, offscreen targets
│  ├─ iron_script/         # ScriptRuntime trait + QuickJS backend + shared test suite
│  ├─ iron_ui/             # (M2) UI tree, LayoutEngine trait, Taffy backend, UI renderer
│  └─ iron_app/            # the `iron` binary: main loop wiring everything together
├─ js/
│  ├─ engine/              # @iron/engine: Component base class, engine API, .d.ts
│  └─ react/               # (M2) @iron/react: custom reconciler
├─ examples/
│  └─ hello/               # TS game: a Hello component
└─ tests/
   ├─ fixtures/            # tiny TS bundles, glTF files
   └─ golden/              # reference PNGs
```

---

## M1 — Hello Iron (scaffold)

**Goal:** `cargo run -p iron_app -- examples/hello` opens a window with a clear colour, loads a Bun-built TS bundle, and a `Hello` component logs `hello` once and `tick` every frame.

### 1. Repository and tooling
- [x] `git init`, `.gitignore` (target/, node_modules/, dist/), `.gitattributes` (LF line endings, PNGs as binary)
- [x] Cargo workspace with crates from the layout above — `M1-BLD-01`
- [x] `rustfmt.toml`; `cargo fmt --check` / `cargo clippy -D warnings` clean — `M1-BLD-02`
- [x] Bun workspace: root `package.json`, `js/engine`, `examples/hello`, `tests/fixtures`, shared `tsconfig.base.json` (strict) — `M1-BLD-03`, `M1-BLD-04` (Bun 1.4.3)
- [x] Logging via `tracing` + `tracing-subscriber` (`RUST_LOG` to override)

### 2. `iron_core`
- [x] Typed IDs (`EntityId`, `ComponentId`, `NodeId`) as `u32` newtypes
- [x] `Op` and `OpBuffer` (name + `Value` args; binary encoding comes later)
- [x] `Value` enum for values crossing the script boundary
- [x] Re-export `glam` for math

### 3. `iron_platform`
- [x] `Platform` wrapping the winit event loop (`ApplicationHandler`)
- [x] Headless mode for tests — `M1-PLT-01`
- [x] Convert winit events to Iron `InputEvent`s — `M1-PLT-02`
- [x] Clean exit on window close — `M1-PLT-03` (manual, passed 2026-10-16 on Windows)
- [x] `gilrs` polling stub (wired up properly in M3)

### 4. `iron_render`
- [x] wgpu instance/adapter/device; software fallback when no hardware adapter — `M1-RND-01`
- [x] Window surface (sRGB, handles outdated/lost/minimized) + offscreen target
- [x] Clear to colour, read back pixels (with row padding) — `M1-RND-02`
- [x] Resize and 0×0 (minimized) are safe — `M1-RND-03`
- [x] Golden image helper (`testing::assert_golden`, `IRON_UPDATE_GOLDEN=1`)

### 5. `iron_script`
- [x] `ScriptRuntime` trait: `load_bundle`, `eval`, `call_global`, `drain_ops`, `reset`, `set_log_sink`
- [x] `ScriptError` with message, file, line, stack
- [x] QuickJS backend (`rquickjs`) — `M1-SCR-01`..`M1-SCR-03`
- [x] `console.*` → log sink (default: `tracing`, target `script`) — `M1-SCR-04`
- [x] `reset()` creates a fresh context, keeps natives and log sink — `M1-SCR-05`
- [x] Native `__iron_op(name, ...args)` queues ops — `M1-SCR-06`
- [x] UTF-8 ↔ UTF-16 round-trip test — `M1-SCR-07`
- [x] Backend-generic suite (`runtime_conformance_tests!` in `iron_script::conformance`) — `M1-SCR-08`

### 6. `@iron/engine` (TS) and components
- [x] `Component` base class with `onStart()` / `onUpdate(dt)`
- [x] `register({ Hello })` registry (object form keeps names stable under minification) — `M1-CMP-01`
- [x] `ComponentHost` calls methods by `(component id, method name)` via `__iron_dispatch` — `M1-CMP-02`, `M1-CMP-03`
- [x] Exceptions disable only the failing component — `M1-CMP-04`
- [x] Hand-written globals `.d.ts` (`js/engine/src/globals.ts`; generate from Rust later)
- [x] `bun test` unit tests for the JS host (`js/engine/test/`, 4 tests)

### 7. `iron_app` and the example
- [x] `iron` binary: game folder (`dist/game.js`) or `.js` path; opens window; runs loop
- [x] Fixed 60 Hz tick with catch-up limit; clear colour each frame
- [x] `examples/hello`: `Hello` logs `hello` on start and `tick` on update
- [x] `tests/fixtures/{hello,throws}.js` built by Bun from `tests/fixtures/src/*.ts` (`bun run build:fixtures`; CI checks they're up to date)
- [x] `iron examples/hello` runs the Bun-built game end to end

### 8. CI
- [x] GitHub Actions workflow (`.github/workflows/ci.yml`): Windows + Linux Rust jobs, JS job with Bun
- [x] Software rendering on CI runners (WARP on Windows, Mesa llvmpipe on Linux)
- [ ] Push to GitHub and see it go green (not yet run)

**M1 done when:** all `M1-*` tests pass in CI and `M1-PLT-03` passes manually.

**Status:** all 24 Rust tests and 4 JS tests pass locally (Windows, RTX 5080); build and typecheck are clean. `iron examples/hello` runs on Vulkan: `hello` once, ~60 ticks/s. The manual close test passed. Remaining: a CI run.

---

## M2 — Boxes (React → Taffy → screen)

**Goal:** `iron examples/boxes` shows a React-rendered layout of coloured, rounded, bordered boxes that relayouts when the window resizes and when React state changes (a timer-driven counter that adds/removes boxes).

**Versions checked 2026-10-16:** React 19.3.1, react-reconciler 0.34.0, Taffy 0.15.0, criterion 0.8.2.

### M2 decisions (proposed, confirm before starting)
- **D1 — Op transport:** the reconciler collects ops in a JS array during a commit and calls a native `__iron_commit(ops)` **once** in `resetAfterCommit`. Ops are flat arrays: `[opcode, ...args]`. Start with the existing `Value` conversion; switch to a binary `ArrayBuffer` encoding only if `M2-BEN-04` says so.
- **D2 — Style values:** numbers mean logical pixels; strings are CSS values parsed in Rust (Taffy's `parse` feature for layout properties, a small parser for colours). Styles travel as flat `[prop, value, prop, value, ...]` lists, so `Value` needs no object type. M3's CSS files reuse the same parsers.
- **D3 — Elements in M2:** `<box>` only. Text arrives in M3; `createTextInstance` throws a clear "text isn't supported until M3" error.
- **D4 — Scheduling:** React 19 + react-reconciler 0.34. The script runtime gains `setTimeout`/`clearTimeout`/`queueMicrotask` and runs the QuickJS job queue every frame. Root renders use `updateContainerSync` + `flushSyncWork` so a frame's state changes commit in the same frame.
- **D5 — Crate split:** a new `iron_ui` crate owns the node tree, op decoding, style parsing, the `LayoutEngine` trait with the Taffy backend, and produces a GPU-free **draw list** of quads. `iron_render` gains a `QuadRenderer` that draws any draw list. UI stays testable without a GPU, and the quad renderer is reusable for world panels later.
- **D6 — Units:** layout in logical pixels; the window's scale factor is applied only in the renderer.

### 1. Script runtime additions (`iron_script`)
- [x] `setTimeout` / `clearTimeout` in a shared JS prelude (`iron_script/src/prelude.js`; Rust holds no JS function refs); `queueMicrotask` is built into QuickJS-ng. Timers fire from `ScriptRuntime::tick(now)`, zero-delay chains in the same tick, capped at `MAX_TIMERS_PER_TICK` — `M2-SCR-01`
- [x] Pending QuickJS jobs (promises, microtasks) run after every call into JS, including between timers — `M2-SCR-02`
- [x] `__iron_commit(ops)` native → `Commit` (in `iron_core`), collected with `drain_commits()` — `M2-SCR-03`
- [x] Uncaught timer/job errors collected via `__iron_report_error` and returned by `tick()`
- [x] Bundle defines: `process.env.NODE_ENV` via `bun build --define` (development for `examples/hello`, production for test fixtures)
- [x] New tests are in the backend conformance suite (7 added; 15 total)
- [x] `iron_app` calls `tick` every frame (timers work in games now); commits drained until step 6 consumes them

### 2. `iron_ui` crate: tree, styles, layout (no GPU)
- [x] Opcodes (`iron_ui::opcodes`): `CREATE`, `SET_STYLE`, `APPEND_CHILD`, `INSERT_BEFORE`, `REMOVE_CHILD`. Simplified from the plan: the root container is node `0` (no separate root ops), and `REMOVE_CHILD` destroys the subtree (no `DESTROY`), because React never reinserts removed instances
- [x] `UiTree`: applies a commit, syncs with layout once per commit; bad ops (unknown IDs, duplicates, cycles, bad args) are reported and skipped — `M2-UI-01`, `M2-UI-02`
- [x] Style parsing: Taffy `parse` for layout props; colours as `#rgb(a)`, `#rrggbb(aa)`, `rgb()`/`rgba()` (comma, space, `/ alpha`), `transparent`, a few names. `null` resets a prop — `M2-STY-01`, `M2-STY-02`
- [x] M2 style subset: `display` (flex/grid/block/none), flex props + `flex` shorthand, `width/height/min/max`, `aspectRatio`, `padding`/`margin`/`borderWidth`/`inset` (1–4 value shorthands and per side), `gap`/`rowGap`/`columnGap`, `position` + insets, alignment, `gridTemplateColumns/Rows`, `gridAutoFlow`, `gridColumn/Row`, `backgroundColor`, `borderColor`, `borderRadius`, `opacity`
- [x] `LayoutEngine` trait + `TaffyLayout` backend; `layout_if_needed` only runs when something changed — `M2-LAY-01`..`M2-LAY-04`
- [x] 24 layout fixtures as data (`tests/layout/*.json`: wrapping, shrink weighting, min/max, auto margins, justify distribution, reverse, alignSelf, aspect ratio, nested percentages, grid repeat/spans/alignment, absolute insets), run through the full op pipeline by a backend-generic runner — `M2-LAY-05`
- [x] Root resize relayouts the tree — `M2-UI-03`
- [x] Draw list: one `Quad` per visible box (absolute rect, colours with inherited opacity, resolved border widths, radius) in paint order — `M2-UI-04`

### 3. `@iron/react` package (`js/react`)
- [x] Host config for react-reconciler 0.34 (mutation mode) in `js/react/src/host.ts`; React 19's required host functions (priorities, transitions, suspend-commit hooks, `bindToConsole`) stubbed in one place. Ops flushed with one `__iron_commit` in `resetAfterCommit`
- [x] Node IDs: numbers from a counter, never reused in a session — `M2-REC-05`
- [x] `commitUpdate` diffs `style` and emits only changed properties (`null` for removed ones); unchanged styles emit nothing — `M2-REC-02`
- [x] Suspense support via `hideInstance`/`unhideInstance` (toggles `display`)
- [x] `createRoot().render(<App/>)` (one root per game, sync commits via `updateContainerSync` + `flushSyncWork`); `<box>` typed through `JSX.IntrinsicElements` with a typed `IronStyle`
- [x] Clear errors for text children (until M3) and unknown elements, reported through React's error callbacks
- [x] `bun test` suite (9 tests) with a fake `__iron_commit` feeding a model of the Rust tree, so tests check the resulting tree rather than React's ID order — `M2-REC-01`..`M2-REC-05`
- [ ] Note: renders abandoned by concurrent React could leave created-but-never-attached nodes in the Rust tree. Not an issue with sync rendering; revisit if we enable transitions

### 4. Integration in QuickJS
- [x] React fixture (`tests/fixtures/src/boxes.tsx` → `boxes.js`, production + minified, ~148 KB) runs in `QuickJsRuntime` with no extra shims; its first render is one commit, and applied to `UiTree` it gives the expected tree, layout and draw commands — `M2-INT-01`
- [x] A `setTimeout`-driven state change commits once within `tick`, creating only the two new nodes and keeping existing IDs; the grid relayouts to two rows — `M2-INT-02`
- [x] Built fixtures marked as generated in `.gitattributes` (no noisy diffs; CI still checks they match their sources)

### 2b. Box visuals in `iron_ui` (styles + draw commands, no GPU) — see DESIGN §5.2
- [x] Colours: `hsl()/hsla()` (deg/turn/rad/grad hues) and all 148 CSS named colours — `M2-VIS-01`
- [x] `border` shorthand (`"2px solid #fff"`, any order, `thin/medium/thick`; only `solid`/`none`/`hidden` styles; like CSS, no style means no border) — `M2-VIS-02`
- [x] `borderRadius` with 1–4 values, per-corner props, percentages; resolved at draw time and scaled down like CSS when corners would overlap. Circular corners only (no `a / b` elliptical radii yet) — `M2-VIS-03`
- [x] `boxShadow` list parser: offsets, blur, spread, colour, `inset`, `none` — `M2-VIS-04`
- [x] `filter: blur()` and `backdropFilter: blur()` (`none` resets; other filter functions rejected for now) — `M2-VIS-05`
- [x] `backgroundImage` / `background`: `linear-gradient(...)` (angles, `to` sides and corners), `radial-gradient(...)` (circle/ellipse, centred), `url(...)`; `backgroundSize`, `backgroundPosition`, `backgroundRepeat` — `M2-VIS-06`
- [x] Draw list is now `Vec<DrawCmd>`: `BackdropBlur`, `PushLayer`, `BoxShadow`, `Quad` (with optional image fill and per-corner radii), `InsetShadow`, `PopLayer`, in CSS paint order — `M2-VIS-07`
- [x] Group opacity through layers; simple single-quad leaves just scale alpha — `M2-VIS-08`
- [ ] Later: per-side border colours, elliptical radii, `radial-gradient` position/size keywords, more filter functions (`brightness`, `grayscale`, `drop-shadow`, ...), multiple backgrounds

### 5. Quad renderer (`iron_render`)
- [x] `UiRenderer` (`iron_render::ui`): instanced quads with an SDF shader for per-corner radii and borders (background extends under the border, like CSS), premultiplied alpha, one physical pixel of anti-aliasing
- [x] Linear and radial gradient fills (up to 8 stops, CSS stop fix-up, magic-corner directions, premultiplied-sRGB interpolation)
- [x] Image fills: `url(...)` loaded from the game folder (relative paths only; `..`, absolute paths and URL schemes rejected), cached textures, `backgroundSize`/`Position`/`Repeat`, clipped to the rounded box, border drawn on top
- [x] Analytic box shadows (outer clipped outside the box, inset clipped to the padding box), one quad each
- [x] Offscreen layers for `filter: blur` and group opacity; backdrop copy + separable Gaussian blur, clipped to the rounded box, for `backdropFilter`. Textures pooled across frames
- [x] Frame drawn into an internal sRGB canvas, then blitted to any target format
- [x] Golden + pixel-assertion tests, all checked by eye — `M2-DRW-01`, `M2-DRW-02`, `M2-DRW-05` (+ images), `M2-DRW-06`..`M2-DRW-08`
- [x] 2,000 boxes in one draw call — `M2-DRW-03`
- [x] Scale factor applied at draw time — `M2-DRW-04`
- [ ] Later: blending happens in linear light (browsers blend in sRGB, so semi-transparent edges differ slightly); large blurs should downsample (dual Kawase) instead of up to 257 taps; per-op uniform buffers and bind groups could be pooled; gradients ignore `backgroundSize`/`Position` for now

### 6. App wiring and example
- [x] `iron_app`: each frame calls `tick(now)`, applies commits to `UiTree` (bad ops logged), relayouts if dirty, and draws with `UiRenderer` via `WindowSurface::present_with`
- [x] Window size and scale factor → root size in logical pixels → relayout
- [x] Bundles can register components, render UI, or both; `url()` images load from the game folder
- [x] `iron <game> --screenshot out.png [--frames N]` saves the UI after N frames and exits (used to verify the examples)
- [x] `examples/boxes`: showcase of every M2 feature (glass header over blobs, gradients, images, shadows, radii, blur, group opacity) plus a timer-driven counter that adds and removes boxes; checked by screenshot
- [x] Found and fixed while doing this: unitless numbers inside CSS strings (`padding: "0 20px"`) were rejected by Taffy's parser
- [x] Test GPU shared and serialised (`testing::shared_gpu`): creating devices on parallel test threads deadlocked on this machine's Vulkan setup
- [ ] Manual check: resize the window and watch it relayout — `M2-MAN-01` (**needs you**: `cargo run -p iron_app -- examples/boxes`)

### 7. Benchmarks and the Taffy/Yoga decision
- [x] `criterion` benches in `iron_ui` on a 2,246-node editor-like tree — `M2-BEN-01`..`M2-BEN-03`
- [x] JS→Rust commit transfer bench (QuickJS) — `M2-BEN-04` (0.40 ms per 1,000 ops; binary encoding not needed yet)
- [x] Profiled and fixed our overhead: boxed UI nodes and FxHash maps (commit apply 5.1 → 3.1 ms)
- [x] Results, analysis and revised targets recorded in TESTS.md
- [x] **Decision: keep Taffy.** Costs fit the frame budget (~0.8 ms worst-case incremental update for 2,246 nodes); Yoga would do the same ancestor relayout. Revisit at M5.e with the real editor UI

**M2 status:** steps 1–7 done. Remaining: the manual resize check (`M2-MAN-01`) and a CI run (needs a push).

**Suggested order:** 1 → 2 → 3 → 4 → 5 → 6 → 7. Steps 2 and 3 are independent once the opcode list (step 2's first item) is agreed, so they can run in parallel.

## M3 — Real UI: text, CSS, interaction

**Goal:** game UIs with real text and fonts, CSS stylesheets with selectors and pseudo-elements, and
interaction (pointer, keyboard, focus, text input with IME, scrolling). Planned in three parts that land
in order: **M3a** text and fonts → **M3b** CSS stylesheets → **M3c** interaction. Each part ends with
its tests passing and a commit per step. **M3d** (CSS parity with s&box) is a backlog of CSS breadth
to pick from after M3c; its items can also be pulled forward when an example needs them.

**Versions checked 2026-10-16:** parley 0.12 (+ fontique 0.12, harfrust shaping), swash 0.2.10,
selectors 0.41, cssparser 0.38, cosmic-text 0.19 (alternative), arboard for the clipboard.

### M3 decisions (proposed, confirm before starting)
- **D1 — Text engine: Parley.** Same Linebender family as Taffy (Blitz renders CSS with taffy + parley +
  selectors). `TreeBuilder` maps nested styled spans directly, `InlineBox` supports inline elements
  (icons, `::before` boxes), `PlainEditor` handles editing with IME composition, and `fontique` gives
  font registration plus system fallback. cosmic-text is the fallback if Parley's still-young API hurts.
- **D2 — Glyph rasterization: swash**, at physical pixel size with 4 horizontal subpixel bins, into a
  glyph atlas (R8 coverage pages + an RGBA page for colour emoji).
- **D3 — Glyphs batch with boxes.** The atlas is bound in the quad pipeline and glyphs are a new
  instance kind, so text interleaves with boxes in paint order without breaking batches.
- **D4 — Text shadows via layers:** each shadow draws the text's glyphs tinted and offset into a layer
  that is blurred with the existing blur pass (exact CSS Gaussian). Multiple shadows stack in CSS order.
- **D5 — CSS engine:** `cssparser` for stylesheets, the `selectors` crate (Servo's) for matching against
  our tree, and the existing `apply_style` for declarations (kebab-case → camelCase). Stylesheets come
  from Bun-bundled CSS imports (Bun writes `dist/game.css` next to `game.js`) and from an
  `addStyleSheet(cssText)` API for CSS-in-JS.
- **D6 — Events:** Rust hit-tests and sends the target's ancestor path to JS; `@iron/react` runs capture
  and bubble phases over React props (`onClick`, `onPointerDown`, ...) with `stopPropagation` and
  `preventDefault`, like React DOM.
- **D7 — Text inputs are Rust-owned editors** (Parley `PlainEditor`: caret, selection, IME, clipboard),
  exposed as controlled React elements (`value` + `onChange`).
- **D8 — Fonts:** a bundled default family (Inter, SIL OFL) so games look the same everywhere; system
  fonts through fontique for fallback (CJK, emoji); game fonts via `@font-face` and `loadFont()` from
  the game folder (same path rules as images). TTF/OTF first; WOFF2 later.

### M3a — Text and fonts
1. **Text in the tree**
   - [x] Elements `text` and `span` (exposed as `Text`/`Span` components, since React's JSX types already
     claim `<text>` and `<span>`); React strings become text nodes (`CREATE_TEXT`, `SET_TEXT`); strings
     outside text containers are anonymous text blocks — `M3-TXT-07`
   - [x] Opcode additions (`CREATE_TEXT`, `SET_TEXT`) mirrored in `js/react/src/opcodes.ts`
2. **Text styles and inheritance**
   - [x] `color`, `fontFamily` (fallback lists), `fontSize` (px/em/%), `fontWeight` (1–1000, `bold`,
     `normal`), `fontStyle`, `lineHeight` (factor or length), `letterSpacing`, `wordSpacing`, `textAlign`,
     `textDecoration`, `whiteSpace` (normal / nowrap / pre / pre-wrap), `textShadow` (parsed) — `M3-TXT-04`
   - [x] Computed-style inheritance (`UiTree::computed_text`); `currentColor` for backgrounds, borders and
     shadows, and as the CSS default when `border`/`boxShadow` omit a colour — `M3-TXT-06`
   - [ ] `currentColor` inside gradient stops (later)
3. **Fonts**
   - [x] Bundled Inter (variable, upright + italic, SIL OFL) for `sans-serif`/`system-ui`; fontique
     system fallback in the app (`UiTree::with_system_fonts`), bundled-only in tests — `M3-FNT-02`
   - [x] `loadFont({ family, src, weight, style })` → op → loaded from the game folder (shared
     `iron_core::resolve_asset` path rules) → registered; text relayouts — `M3-FNT-01`
   - [x] A family with only an italic face uses it for normal text (CSS font matching), instead of
     falling back to another family: works with Parley 0.12 / fontique; guarded by a test (2026-10-17)
     — `M3-FNT-02`
4. **Layout integration**
   - [x] Text blocks are Taffy measured leaves (Taffy 0.15's `compute_leaf_layout` + Parley line
     breaking for min-content / max-content / definite widths) — `M3-TXT-02`
   - [x] Shaping happens only for dirty blocks; re-measuring just re-breaks lines — `M3-TXT-03`
5. **Rendering**
   - [x] Glyph atlas (2048² R8 coverage page + 1024² RGBA colour page, cleared when full), swash
     rasterization at physical size with 4 horizontal subpixel bins, variable-font coordinates,
     synthetic bold/italic — `M3-TXT-01`
   - [x] `DrawCmd::Text` (glyph runs + decorations) → glyph instances in the quad pipeline (texel-exact
     `textureLoad`), so text batches with boxes
   - [x] Text contrast: coverage boost for dark text to offset linear-light blending
   - [x] Underline / line-through from font metrics, drawn as quads
   - [x] `textShadow` lists: tinted, offset copies under the text; blurred ones in layers — `M3-TXT-05`
   - [x] Golden text sampler at 1× and 2× (weights, italic, sizes, decorations, shadows, spans), checked by eye
   - [x] Colour emoji and CJK through system fonts, verified in `examples/text` (goldens use the bundled font only)
6. **Example + checks**
   - [x] `examples/text`: weights 300–800, italic face, decorations, wrapping and alignment, a font loaded
     at runtime (Charm, regular + bold, Latin and Thai), hard/soft/glow text shadows, CJK, Greek,
     Cyrillic and colour emoji via system fallback, and a live clock; verified by `--screenshot`.
     The Charm files are only present locally (git-excluded); without them the card uses fallback fonts

**M3a status:** done.

### M3b — CSS stylesheets and selectors
1. **Stylesheets**
   - [x] Parse stylesheets with `cssparser`: style rules, `@font-face` (feeds `loadFont`), comments,
     `!important`; unsupported at-rules warn — `M3-CSS-01`
   - [x] Sources: `dist/game.css` loaded with the bundle; `addStyleSheet(cssText)` / `removeStyleSheet`
   - [x] Declarations: kebab-case → our camelCase props; values as CSS strings into `apply_style`
   - [x] Custom properties (`--x`) and `var(--x, fallback)` — `M3-CSS-05`
2. **Selectors** (`selectors` crate, `Element` implemented for UI nodes)
   - [x] Simple selectors: type (`box`, `text`, `span`, `input`, `textarea`, `checkbox`), universal
     `*`, `.class` (`className` prop), `#id` (`id` prop), attribute selectors on `data-*` props — `M3-CSS-08`
   - [x] Combinators: descendant (space), child `>`, next sibling `+`, subsequent siblings `~` — `M3-CSS-02`
   - [x] Structural: `:first-child`, `:last-child`, `:only-child`, `:nth-child(an+b [of S])`,
     `:nth-last-child()`, `:first-of-type`, `:last-of-type`, `:nth-of-type()`, `:root` — `M3-CSS-02`
   - [x] Emptiness: `:empty` (no children and no text) and `:blank` (no children, or only whitespace
     text; Selectors Level 4, so implemented as our own pseudo-class) — `M3-CSS-09`
   - [x] Logical: `:not()`, `:is()`, `:where()`, `:has()` (relative selectors) — `M3-CSS-03`
   - [x] User action (driven by M3c, testable earlier by setting states directly): `:hover`, `:active`,
     `:focus`, `:focus-visible`, `:focus-within` — `M3-CSS-10`
   - [x] Form state: `:enabled` / `:disabled` (the `disabled` prop on inputs, buttons and boxes with
     `tabIndex`; disabled elements also ignore pointer and focus), `:checked` (the `checked` prop on
     a new `<checkbox>` element and on `<input type="checkbox" | "radio">`) — `M3-CSS-10`
3. **Cascade**
   - [x] Specificity + source order; inline `style` prop beats stylesheet rules (unless `!important`);
     inheritance from M3a — `M3-CSS-04`
4. **Pseudo-elements**
   - [x] `::before` / `::after` generate anonymous first/last children (invisible to React), styled by
     their rules; `content: ""` boxes for decoration and `content: "text"` for text — `M3-CSS-06`
5. **Restyle**
   - [x] Rule index: selectors bucketed by the id / class / type of their rightmost compound
   - [x] Dependency-aware invalidation: at load, record which classes, ids, attributes and states appear
     in ancestor positions, before `+`/`~`, inside `:has()`; a change restyles the node plus only the
     subtree / later siblings / ancestors it can affect. Structural changes restyle siblings only when
     structural selectors exist — `M3-CSS-07`
   - [x] Benchmark: one class change on the 2,246-node tree 0.20 ms (target < 1 ms); whole tree 8.8 ms — `M3-BEN-01`
6. **Example + checks**
   - [x] `examples/css`: a `.css` file imported through Bun (`dist/game.css`, `url()`s rebased, small
     assets inlined as `data:` URLs), classes, combinators, structural selectors, `:has`/`:not`/`:empty`,
     `:checked`/`:disabled`, `::before`/`::after`, `@font-face`, and a runtime `addStyleSheet` that
     overrides `--accent` every second

**M3b status:** done. The `<checkbox>` / `<input>` element types arrive with M3c; `:checked` and
`:disabled` already work on any element through the `checked` / `disabled` props.


### M3c — Interaction
1. **Hit testing and pointer events**
   - [x] Hit test in reverse paint order: rounded corners, `display: none`, `pointerEvents: none`
     (inherited, children can opt back in); text hits its text element, `::before`/`::after` their
     owner — `M3-EVT-01` (`crates/iron_ui/tests/hit.rs`)
   - [x] Clipping in hit tests: `overflow` and `clipPath` cut away what isn't drawn — `M3-EVT-05`
     (`crates/iron_ui/tests/clip.rs`)
   - [ ] Spans inside text as targets (needs node ids in the text brush); `visibility`
   - [x] Pointer events to JS with the target's ancestor path; capture + bubble phases,
     `stopPropagation`, `preventDefault`; `onClick`, `onPointerDown/Up/Move/Enter/Leave`, `onWheel`,
     double click, `onContextMenu`, `onMouse*` aliases — `M3-EVT-02`, `M3-EVT-03`
     (`iron_ui::Pointer` in `events.rs`, `js/react/src/events.ts`, `crates/iron_ui/tests/pointer.rs`,
     `events_quickjs.rs`)
   - [x] `:hover` / `:active` states (hover re-tested each frame when the tree moves under the
     pointer); `cursor` CSS property (inherited) → OS cursor
2. **Keyboard and focus**
   - [x] `tabIndex`, Tab / Shift-Tab order (wrapping), `onFocus`/`onBlur` (bubbling, with
     `relatedTarget`), `:focus`, `:focus-visible` (keyboard only), `:focus-within`; `onKeyDown` /
     `onKeyUp` to the focused element with DOM `key`/`code`/`repeat`; pressing focuses the nearest
     focusable ancestor or blurs (unless `pointerdown` is prevented) — `M3-EVT-04`
     (`iron_ui::Interaction` in `interaction.rs`, `crates/iron_ui/tests/focus.rs`, `events_quickjs.rs`)
   - [x] Imperative `ref.current.focus()` / `blur()` via ops
3. **Text input**
   - [x] `<input>` and `<textarea>` (`Input` / `TextArea` components) backed by Parley
     `PlainEditor`: typing, Backspace/Delete (Ctrl for words), arrows/Home/End with Shift and Ctrl,
     Ctrl+A, Enter in textareas; caret placement, word (double-click) and line (triple-click)
     selection and drag selection; blinking caret, selection highlight, dimmed `placeholder`, the
     caret kept in view; `value` (controlled, restored when `onChange` rejects an edit like React
     DOM), `defaultValue`, `readOnly`, `disabled`, `rows`; focusable by default; `onChange` /
     `onInput` with `e.target.value`; Ctrl+C/X/V produce clipboard requests — `M3-INP-01`
     (`crates/iron_ui/tests/editor.rs`, golden `M3-INP-01-fields`)
   - [x] Ctrl/Cmd shortcuts fall back to the physical key `code` on non-Latin layouts (Ctrl+ф is
     Ctrl+A on a Russian layout); AltGr still types — `M3-INP-02`. Hand-checked 2026-10-16:
     Ctrl+A/C/X/V work on English, French, Russian, Arabic, Hebrew, the Chinese Pinyin layout and
     with the Japanese IME on
   - [x] Pasting into an `<input>` flattens line breaks to spaces and drops a trailing one
     (`M3-INP-01`)
   - [x] Placeholders re-wrap when the field resizes (they kept the wrap width of their first
     layout) — `M3-INP-01`
   - [x] System clipboard (arboard, opened on first use) for the requests above, in `iron_app`
   - [ ] Later: undo/redo, `type="password"`, `maxLength`, wheel scrolling inside a textarea,
     `::placeholder` / `::selection` / `caret-color` styling
   - [x] Selecting static text and `user-select`: done in M3d step 5h
   - [x] IME: winit `Ime` events → pre-edit shown underlined at the caret (not part of the value,
     no `input` events), commit inserted with an `input` event, pre-edit dropped when focus
     leaves; IME is allowed only while an editable field has focus and the candidate window
     follows `set_ime_cursor_area` — `M3-IME-01`. Manual check (`M3-MAN-01`, 2026-10-16, Windows
     Microsoft IME, driven by computer use): Japanese pre-edit, conversion, commit, Backspace /
     Escape in the pre-edit, textarea lines, the candidate window under the field, IME only while
     a field has focus, Russian and French typing, and the clipboard all work. Re-checked after the
     shortcut, segmentation and RTL changes with no regressions. Chinese Pinyin composition not
     checked: Windows reports its dictionaries aren't ready, so it stays in English (英) mode
   - [x] Complex-script segmentation: Parley's `complex-scripts` feature compiles in ICU4X
     dictionaries, so CJK word moves/selection and Thai, Lao, Khmer and Burmese line breaks
     follow word boundaries (no more "No segmentation model" log) — `M3-TXT-08`. Costs about
     3.8 MB in the release binary
   - [x] Right-to-left text: inherited CSS `direction` and the `dir` attribute (CSS wins;
     `[dir=rtl]` matches) set Parley's paragraph direction; `text-align: start/end` follow it;
     rtl inputs/textareas align right (placeholder too) with visual arrow keys — `M3-TXT-09`.
     Shown in `examples/css` (Arabic line and input). Hand-checked 2026-10-16 with the Arabic (101)
     and Hebrew keyboards: typing, shaping, bidi order, Home/End, visual arrows, Backspace and
     Ctrl+A/C/X/V all work
   - [ ] RTL follow-ups: `dir="auto"`, flex/grid `row` order and `justify-content: start` in rtl
     (Taffy has no direction), logical properties (`margin-inline-start`, `inset-inline-*`)
4. **Overflow and scrolling**
   - [x] `overflow` / `overflowX` / `overflowY` (`visible | hidden | clip | scroll | auto`, CSS
     computed-value rules) clip children to the padding box with inner radii; `clipPath`
     (`inset() round`, `circle()`, `ellipse()`, box keywords) and legacy `clip: rect()` clip the
     element itself. `DrawCmd::PushClip`/`PopClip`; the renderer keeps a clip table (elliptical
     corners, chained) and tags each instance, so clips don't split batches; layers are clipped
     when composited; fully clipped commands are culled — `M3-EVT-05` (golden `M3-EVT-05-clip`)
   - [x] Scrolling: scroll offsets shift content (rects, hit testing and drawing agree; offsets
     clamp when content shrinks); the wheel scrolls the innermost container that can move and
     chains outward (Shift for horizontal); arrow keys, Page Up/Down, Space, Home/End; Tab and
     `focus()` scroll the element into view; overlay scrollbars (`scroll` always, `auto` when
     needed) with thumb dragging and track paging; `scrollbarWidth`, `scrollbarColor`; `onScroll`
     and `ref.scrollTo/scrollBy/scrollIntoView()` — `M3-EVT-05` (`crates/iron_ui/tests/scroll.rs`,
     golden `M3-EVT-05-scroll`)
   - [ ] Later: smooth scrolling, scroll snapping, culling off-screen children in the draw list
     (the renderer already culls their GPU work), touchpad momentum
5. **Gamepads**
   - [x] `gilrs` events with stable button/axis names (`South`, `DPadUp`, `LeftStickX`, ...) to
     scripts through `@iron/engine`'s `input` (`isKeyDown`, `gamepads()`, `isButtonDown`,
     `on("gamepadbuttondown" | "keydown" | ...)`; keys typed into a text field stay the
     field's); spatial focus navigation with the d-pad / left stick and South to click, only
     while UI has focus — `M3-PAD-01` (`crates/iron_platform/src/gamepad.rs`,
     `crates/iron_app/src/game_input.rs`, `crates/iron_ui/tests/navigation.rs`,
     `js/engine/test/input.test.ts`)
   - [ ] Later: rumble, button repeat while held, mouse state in `input`
6. **Example + checks**
   - [x] Manual IME check on Windows (`M3-MAN-01`, IME part): done in `examples/css` with the
     Microsoft Japanese IME (see Text input above)
   - [x] `examples/form`: a profile form with controlled fields (validation, a character
     counter), keyboard- and gamepad-operable toggles and buttons (`:hover`, `:active`,
     `:focus-visible`, `:disabled`), Enter to submit, and a scrolling list of saved profiles;
     focus starts in the first field so the d-pad works at once (checked by screenshot)
   - [x] Hand check: `examples/form` with a real gamepad (d-pad / A navigation), 2026-10-16
   - [x] Hand check: resizing `examples/css` relayouts smoothly (`M2-MAN-01`), 2026-10-16
   - [ ] Hand check still open: Chinese Pinyin composition once Windows has its dictionaries

**M3c status:** done (2026-10-16). Left for hand checks: Chinese Pinyin.

### M3d — CSS parity with s&box

Gaps found by comparing our CSS with s&box's (survey of 2026-10-16, from the official
`Facepunch/sbox-docs` style-properties page and the `Facepunch/sbox-public` engine source). Items
already planned for M3c (`overflow`, `cursor`, `pointer-events`, scrolling) aren't repeated here.
Where s&box has a custom extension, we prefer the standard CSS form if there is one.

**Order agreed for the rest of M3d (2026-10-17):** 5h, 5i, 5j, then groups 7, 8, 9, 10, then
5.1 (SVG); then stop and check in with the user before M4. All done (2026-10-17) except
`sound-in`/`sound-out`, skipped as s&box-specific (CSS stays about display). Added afterwards: groups 11 (pointer capture),
12 (WOFF/WOFF2) and 13 (`@media`). Known deviations from browsers go in
[docs/differences.md](docs/differences.md).

We're already **ahead** of s&box on: custom properties and `var()` (s&box has none), `!important`
(accepted but ignored there), attribute selectors, `:checked`/`:disabled`/`:enabled`/`:focus-within`/
`:focus-visible`/`:blank`, `:nth-child(An+B of S)` and the `*-of-type` family, `:is()`/`:where()`,
and `@font-face`.

1. **Units and values**
   - [x] `em`, `rem`, `vw`, `vh`, `vmin`, `vmax` (and the `d`/`s`/`l` viewport variants) in every
     property, inline or from stylesheets: resolved to px at restyle like `var()`
     (`crates/iron_ui/src/units.rs`); `em` is the element's own font size (the parent's for
     `fontSize`); nodes using them restyle when the window, their font size or the root font
     size changes (`crates/iron_ui/tests/units.rs`)
   - [x] `calc()`, `min()`, `max()`, `clamp()` with pixels, relative units and numbers, nested;
     invalid math is ignored (stylesheets warn, inline values are rejected and keep the old value)
   - [x] `calc()` / `min()` / `max()` / `clamp()` with percentages (`calc(100% - 20px)`) in layout
     lengths (sizes, insets, margins, padding, borders, gaps, `flex-basis`): parsed once and
     interned (`crates/iron_ui/src/calc.rs`), evaluated by Taffy through our own Taffy tree
     (`TaffyLayout` now implements Taffy's tree traits instead of using `TaffyTree`, whose
     `resolve_calc_value` returns 0). Paint lengths (`border-radius`, shadows) don't take them yet
   - [x] `boxSizing: border-box | content-box`; Iron keeps Taffy's (and s&box's) `border-box`
     default, where CSS defaults to `content-box`
   - [x] CSS-wide keywords `inherit`, `initial`, `unset`, `revert` (and `revert-layer`, both like
     `unset`: no user-agent stylesheet) on every property, inline, in stylesheets and on
     `::before`/`::after`; `inherit` copies the parent's value for non-inherited properties
     (`apply_wide_keyword`, `crates/iron_ui/tests/wide_keywords.rs`)
   - [x] Colours: `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color(srgb | srgb-linear |
     display-p3 ...)`, `none` channels, and `color-mix(in srgb | srgb-linear | lab | oklab | lch |
     oklch, ...)` (premultiplied, shorter hue), the standard form of s&box's `mix()` /
     `darken()` / `lighten()`, which are supported too (SCSS semantics: HSL lightness ± amount;
     `mix(a, b, weight of a)` in sRGB) (`crates/iron_ui/src/color_fn.rs`); out-of-gamut colours clip
   - [x] Angles in `deg`, `rad`, `grad`, `turn` (hues and gradients); times (`s`/`ms`) come with
     transitions (group 7), the first properties that take them
2. **Layout**
   - [x] `order` (layout children are stably sorted by it; Tab order stays in document order),
     `z-index` (paint order and hit testing among siblings, then `order`, then document order;
     each z-indexed element stacks on its own, so it never lifts above an ancestor's siblings),
     `isolation` (accepted, no effect in that model) (`crates/iron_ui/tests/layout_props.rs`)
   - [x] `display: contents` (children join the layout parent, flattened before Taffy sees them;
     the element paints nothing and isn't hit, but stays in event paths and passes on inherited
     styles); `position: fixed` (Taffy lays it out as an absolute child of the root; it ignores
     ancestor scrolling and clips and paints above the whole tree, ordered by `zIndex`; ancestor
     opacity and filters don't apply to it)
   - [x] `flex-flow` (Bun's minifier emits it)
   - [x] Shorthands `place-items`, `place-self`, `place-content`
   - [x] Grid: `grid-auto-rows`/`grid-auto-columns`, `grid-template` (`rows / columns` and the area
     strings form with row sizes), `grid-template-areas` (rectangles checked), `grid-area` (a name
     or four lines), `grid-row-start`/`-end`, `grid-column-start`/`-end`
   - [x] Logical properties: `margin-block`/`-inline`, `padding-block`/`-inline`,
     `inset-block`/`-inline` (with `-start`/`-end`), `block-size`/`inline-size` (+ `min-`/`max-`),
     mapped left-to-right (`crates/iron_ui/tests/layout_props.rs`); RTL-aware mapping later
   - [x] An `image` element (`Image` in `@iron/react`) with `src` (a path inside the game folder
     or a `data:` URL) and `object-fit` (`fill`, `contain`, `cover`, `none`, `scale-down`) and
     `object-position`: the tree asks the app for natural sizes (`take_image_requests` /
     `set_image_size`, loaded by the renderer's image cache) and measures images like replaced
     elements (one known axis keeps the ratio); drawn in the content box, clipped to the inner
     rounded corners — `M3-IMG-01`
3. **Backgrounds and borders**
   - [x] `conic-gradient()` (`from <angle>`, `at <position>`, stops in angles or percentages) and
     `repeating-linear-gradient()` / `repeating-radial-gradient()` / `repeating-conic-gradient()`;
     two-position colour stops (`red 0 25%`) in every gradient — `M3-BG-01`
   - [x] `background-clip` (`border-box`, `padding-box`, `content-box`, `text` for gradient text:
     the topmost gradient layer fills the element's own glyphs, under the text colour);
     `background-position-x`/`-y`; `background-repeat: round | space` (and per-axis pairs);
     multiple background layers (comma lists, cycled like CSS) — `M3-BG-02`. Images still
     position against the border box (no `background-origin` yet)
   - [x] `background-blend-mode` and `mix-blend-mode`: all 16 W3C modes, blended in sRGB. A
     blended element draws into a layer that the composite pass blends with a copy of its
     backdrop; blended background layers do the same inside an isolated group — `M3-BG-03`
   - [x] `border-style`: `dashed`, `dotted`, `double`, `groove`, `ridge`, `inset`, `outset`; per-side
     colours and styles (`borderColor`/`borderStyle` with 1-4 values, `borderTopColor`,
     `borderTopStyle`, ... and the `borderTop`/... shorthands). Uniform solid borders stay on the
     quad; others are a `DrawCmd::Border` styled per side in the shader (dashes and dots fit a
     whole number per side). Iron's default style stays `solid` — `M3-BRD-01`
   - [x] Elliptical corner radii (`border-radius: 10px / 20px`, `borderTopLeftRadius: "10px 20px"`);
     percentages now resolve like CSS (horizontal of the width, vertical of the height, so `50%`
     on a wide box is an ellipse). `Radii { x, y }` runs through the draw list, hit testing and
     clips; instances carry `radii_y`. Blurred shadows approximate them with circles — `M3-BRD-02`
   - [x] `outline`, `outline-width`, `outline-color`, `outline-style` (`auto` is solid),
     `outline-offset`: a `DrawCmd::Border` around the border box, following the corner radii,
     drawn after the element's children; no layout or hit-testing effect — `M3-BRD-03`
   - [x] `border-image` and its longhands (source, slice with `fill`, width, outset, repeat:
     `stretch`/`repeat`/`round`/`space`), from images and gradients. `BorderImage::pieces`
     computes the nine pieces; image pieces draw a part of the texture, gradient pieces stretch
     (no tiling for gradients yet) — `M3-BRD-04`
   - [x] Shaped boxes: s&box's custom `border-shape: polygon() | circle()` (SDF, hit testing follows
     the shape), done the standard way with `clip-path` — `inset()`, `circle()`, `ellipse()` (M3c),
     now `polygon()` and `path()` (SVG path data, curves and arcs flattened; `nonzero`/`evenodd`).
     Polygon points ride in the clip table; both shaders compute a signed distance, and hit
     testing uses the same fill rule — `M3-CLIP-01`. `url()` needs SVG `<clipPath>` elements,
     which Iron doesn't have
4. **Text**
   - [x] `text-transform` (inherited; applied to run text before shaping, `capitalize` across
     inline runs), `text-overflow: ellipsis` (not inherited; lines wider than the content box of
     a block whose x overflow clips drop whole glyphs and end in a separately shaped `…`, on the
     left for rtl) — `M3-TXT-10`
   - [x] `word-break` (`break-word` as `overflow-wrap: anywhere`), `overflow-wrap` / `word-wrap`;
     `white-space: pre-line | break-spaces` — all Parley options — `M3-TXT-11`
   - [x] `font` shorthand; `font-size` keywords (`xx-small` ... `xxx-large`, `smaller`, `larger`);
     `font-weight` keywords (`thin` ... `black`, `bolder`/`lighter` from the parent);
     `font-variant-numeric` and `font-feature-settings` as Parley font features — `M3-TXT-12`
   - [x] Decoration: `overline`, `text-decoration-style` (`double`, `dotted`, `dashed`, `wavy`),
     `-color`, `-thickness`, `text-underline-offset`, and the full `text-decoration` shorthand.
     Runs carry their decoration in Parley's brush; Iron draws the lines from font metrics (dots
     and dashes through the styled-border shader, waves through their own kind) — `M3-TXT-13`.
     `text-decoration-skip-ink` is parsed but underlines don't skip descenders yet
   - [x] Outlined text: `-webkit-text-stroke` / `text-stroke` and their width/colour longhands
     (inherited). Glyph runs carry the stroke; the atlas rasterizes stroked outlines with swash
     (keyed by stroke width) and draws them over the fill. Colour glyphs aren't stroked — `M3-TXT-14`
   - [x] Gradient text: `background-clip: text` (M3d step 3b). s&box's gradient `color` isn't
     added: the standard form covers it
   - [x] `caret-color` (inherited; `auto` is the text colour); `font-smooth` (s&box's `auto` /
     `never` / `always`) and `-webkit-font-smoothing`: `never` / `none` thresholds glyph coverage
     in the shader for crisp pixel fonts — `M3-TXT-15`
5. **Effects**
   - [x] `filter` / `backdrop-filter` functions beyond `blur()`: `brightness()`, `contrast()`,
     `saturate()`, `grayscale()`, `sepia()`, `hue-rotate()`, `invert()`, `opacity()`, and
     `drop-shadow()` (follows the shape); chains. Colour functions fold into one 4×5 matrix
     applied in sRGB when the layer (or backdrop copy) is composited; drop shadows are blurred,
     tinted, shifted copies of the layer drawn beneath it. Iron applies blur, then colour, then
     shadows, whatever the written order — `M3-FX-02`
   - [x] `image-rendering` (`pixelated`, `crisp-edges`, `auto`/`smooth`/`high-quality`), inherited,
     for background images, `Image` elements and border images: each image has a second bind
     group with a nearest-neighbour sampler — `M3-FX-03`
   - [x] Masks: `mask-image` (images and gradients; `mask` takes just the images), `mask-mode`
     (`alpha`/`match-source`, `luminance`; the top layer's mode applies to all), `mask-size`,
     `mask-position`, `mask-repeat`, with `-webkit-` aliases. A masked element draws in a layer;
     `BeginMask` switches drawing to a mask texture (ordinary background quads), which the
     composite multiplies by — `M3-FX-04`. Along the way, gradients now follow
     `background-size`/`-position`/`-repeat` (one tile, repeated in the shader) — `M3-BG-04`
   - [x] `opacity` as a percentage (and as a numeric string) — `M3-FX-01`
   - [x] **5e** Transforms (moved up from group 6): `transform` with every function — `matrix`,
     `matrix3d`, `perspective`, `rotate`/`rotateX`/`rotateY`/`rotateZ`/`rotate3d`, `translate`/
     `translateX`/`Y`/`Z`/`translate3d`, `scale`/`scaleX`/`Y`/`Z`/`scale3d`, `skew`/`skewX`/`skewY`, `none`,
     several functions, and the CSS-wide keywords; the `translate` / `rotate` / `scale` properties;
     `transform-origin` (with z); `perspective` / `perspective-origin`; `transform-style: flat |
     preserve-3d`; `backface-visibility`. Drawing and hit testing through them. Absolute length units
     (`in`, `cm`, `mm`, `Q`, `pt`, `pc`) everywhere. Each element's 4×4 transform flattens to a 3×3
     homography (`transform.rs`); `PushTransform`/`PopTransform` in the draw list, transforms stored
     in the renderer's clip table, instances carry a transform index (the vertex shader maps corners,
     fragments map back to the plane), clips remember their plane; hit testing inverts — `M3-XF-01`,
     `M3-XF-02`
   - [x] **5f** `visibility: visible | hidden | collapse` (keeps its layout space, isn't drawn or hit;
     children can opt back in with `visible`; inherited; text runs carry it, so visible spans in
     hidden text still draw; hidden elements can't take focus) — `M3-VIS-01`
   - [x] **5g** Lists: a `list` element (React `<List>`; UA style: column, 24px left padding) whose
     element children are list items (also `display: list-item`); `list-style-type` (`disc`,
     `circle`, `square`, `decimal`, `decimal-leading-zero`, `lower/upper-alpha`, `lower/upper-roman`,
     `lower-greek`, `none`, a string), `list-style-position`, `list-style-image`, `list-style`
     (all inherited); markers are generated `::marker` children (outside: absolutely placed left of
     the item; inside: first child), styled by `::marker` rules, renumbered when items change.
     A list without markers (`list-style: none`, no image) drops its default indent unless its start
     padding is set. Paragraphs and headings stay `text` with classes (no `p`/`h1`). Counters aren't done —
     `M3-LST-01`
   - [x] **5h** `user-select: none | auto | text | all` (and `-webkit-`; `auto` takes the parent's
     value, so children can opt back in) and selecting static text: drag, double-click word,
     triple-click line, Shift+click extends, Ctrl+C copies through the fields' clipboard request.
     The tree keeps a Parley `Selection` on one text block (presses map through transforms, which
     fields now do too); `::selection` sets the highlight and recolours the selected glyphs (also in
     fields); `::placeholder` styles placeholders (text properties, colour, opacity); the I-beam
     shows only over selectable text — `M3-SEL-01`, `M3-SEL-02`. Selections stay in one block (see
     docs/differences.md)
   - [x] **5i** Custom cursor images: `cursor: url(a.cur), url(b.png) 4 4, pointer` (any number of
     images, then the required keyword; hotspot in image pixels, else the `.cur` file's own, else the
     top-left). The app decodes the first image that loads (PNG, JPEG, `.cur`; over 128px is skipped,
     as in browsers), scales it for the window's DPI and asks the platform, which creates and caches
     winit `CustomCursor`s (`PlatformApp::take_custom_cursor`). Fields show `text`, or `not-allowed`
     when disabled; every CSS keyword is checked to map to a winit cursor (winit picks the nearest
     per platform) — `M3-CUR-01`. SVG's extra `pointer-events` values (`visiblePainted`, `fill`,
     `stroke`, `bounding-box`, ...) come with 5.1
   - [x] **5j** A `dialog` element (React `<Dialog open modal closedBy onCancel onClose>`): closed
     dialogs are `display: none` (the authored value comes back on open); open ones are centred
     (`left/top: 50%`, `translate: -50% -50%`). Modal ones are fixed and go in a top layer painted
     and hit above everything, `zIndex` and fixed elements included, each over a generated
     `::backdrop` box covering the window (default `rgba(0,0,0,0.1)`; `dialog::backdrop` rules,
     `backdrop-filter` too). Escape (and backdrop clicks with `closedBy="any"`) send `cancel`
     (cancelable) then `close`; scripts close by clearing `open`. A modal takes focus (its first
     focusable element, else itself), keeps Tab and the d-pad inside, and gives focus back when it
     closes; `[open]` / `[modal]` match — `M3-DLG-01`, `M3-DLG-02`. No `popover` yet
   - [x] **5.1** SVG: an `svg` element (React `<Svg>` and children) with the core shapes (`rect`,
     `circle`, `ellipse`, `line`, `polyline`, `polygon`, `path`, `g`, `text` basics), `viewBox` /
     `preserveAspectRatio`, `use`/`defs`, gradients (`linearGradient`, `radialGradient`), `clipPath`
     and `mask` (which also unlocks `clip-path: url(#id)`), and loading `.svg` files as images
     (`url(icon.svg)`, `Image src`). CSS styling: presentation attributes and properties (`fill`,
     `stroke`, `stroke-width`, `stroke-linecap`/`-linejoin`/`-dasharray`/`-dashoffset`,
     `fill-rule`, `opacity`/`fill-opacity`/`stroke-opacity`, `transform` on shapes,
     `currentColor`), selectors matching SVG elements, inheritance, transitions later. Rendering:
     tessellate fills and strokes (e.g. lyon) into triangles, or rasterize with a vector rasterizer
     into a texture (e.g. resvg/tiny-skia for files) — decided: resvg, rasterized at the drawn size
     - [x] **5.1a** `.svg` files and `data:image/svg+xml` URLs as images (`url()`, `Image src`): usvg
       parses (with the bundled font for `<text>`), resvg rasterizes at the tile's physical size
       (rounded up to 8px, cached per size), layout uses the SVG's natural size — `M3-SVG-01`
     - [x] **5.1b** Inline `<svg>`: `svg` is a measured box (width/height/viewBox, like an image);
       the SVG elements inside are tree nodes React creates (`g`, shapes, `text`/`tspan`, `defs`,
       `use`, gradients/`stop`, `clipPath`, `mask`, `pattern`, `symbol`) with any attributes. CSS
       styles them: SVG presentation properties (`fill`, `stroke*`, `stop-color`, geometry like `r`)
       are stored as written; `opacity`, `transform` (converted to SVG syntax), `clip-path:
       url(#id)`, `mask: url(#id)` and font properties are kept too. At draw time the tree
       serializes the subtree with those values (`currentColor` from the `svg`'s colour) into a
       `data:` SVG, which the renderer rasterizes at the drawn size (stale rasters are evicted). So
       selectors, `:hover`, transitions and animations work on SVG content — `M3-SVG-02`, `M3-SVG-03`.
       Hit testing is per `svg` box; GPU paths, SMIL and path morphing are under "Later"
6. **Transforms**
   - [x] Done as step 5e (2D and 3D); see group 5
7. **Transitions and animations**
   - [x] **7a** `transition` (+ `-property`, `-duration`, `-delay`, `-timing-function`, `-behavior`)
     for numbers, lengths, colours, shadows, filters and transforms; discrete properties snap.
     `motion.rs` interpolates computed values as CSS text, token by token (mixed units through
     `calc()`, colours premultiplied, `none` as the neutral list); restyle compares each property's
     winning value, starts transitions from the current value (so reversals don't jump) and applies
     running ones on top; `set_time` restyles moving elements each frame; inline-only games switch to
     restyle once a `transition` is set. `transitionrun`/`start`/`end`/`cancel` events reach React
     (`onTransitionEnd`, ...) — `M3-ANI-01`, `M3-ANI-02`
   - [x] **7b** `@keyframes` (`from`/`to`/percentages, selector lists, per-frame
     `animation-timing-function`, `!important` ignored) and `animation` (+ name, duration, delay,
     iteration count, direction, fill mode, play state, timing function). Keyframe values resolve
     `var()` and units per element; missing 0%/100% frames use the element's own value; discrete
     values flip halfway; animations apply under transitions. `animationstart`/`iteration`/`end`/
     `cancel` events reach React (`onAnimationEnd`, ...) — `M3-ANI-03`
   - [x] Timing functions: `linear`, `ease*`, `cubic-bezier()`, `steps()` (all jump positions),
     `step-start`/`-end` (done with 7a; `linear()` with stops isn't)
   - [x] **7c** Entry styling (s&box's `:intro`): `@starting-style` rules (their own rule index) are
     the before-change style of new elements and of elements whose `display` goes from `none` to
     shown (opening a dialog restyles it), so transitions run from them; elements already styled
     ignore them — `M3-ANI-04`
   - [x] **7d** Exit styling, s&box's `:outro`: when any rule uses `:outro`, an element React removes
     stays in the tree with `:outro` set (no hits, focus or Tab stops) until the transitions and
     animations its exit started finish, then goes; ongoing animations (a spinner) don't hold it.
     Done in the tree, so the reconciler needs no changes — `M3-ANI-05`
8. **Scrolling** (extends M3c overflow)
   - [x] `scrollbar-width`, `scrollbar-color` and `overflow: clip` (already done in M3c);
     `scrollbar-gutter: stable` (Taffy keeps the bar's width and margins at the end of a box that
     clips or scrolls vertically; Iron's bars otherwise overlay), `overscroll-behavior` (+ `-x`/`-y`/
     `-inline`/`-block`; `contain` and `none` stop wheel and key scrolling from chaining outward;
     there's no bounce to turn off) — `M3-SCR-01`
9. **Game-specific extensions** (s&box custom properties worth copying)
   - [~] **Skipped:** s&box's `sound-in` / `sound-out` (play a named sound when a rule starts or
     stops applying). CSS is for UI display, not sound, so this stays an s&box-specific feature;
     Iron games play UI sounds from scripts (`onPointerEnter`, `onClick`) with M4 audio
   - [x] **9a** Tint for images: s&box's `background-image-tint` (background images and an `image`
     element's picture) and `border-image-tint`, a colour (alpha too) multiplied into the image's
     pixels in the image shader; gradients aren't tinted — `M3-GAME-01`
   - [x] **9b** UI scale: `setUiScale({ height: 1080 })` (s&box's ScreenPanel), `{ width, height }`
     (fit a design), a fixed number, or `null`, from `@iron/engine` (`uiScale` op). The app lays
     the tree out in UI pixels and multiplies the factor (clamped to 0.25–8) into rendering, pointer
     and wheel input and the IME area, so text stays sharp — `M3-GAME-02`
10. **Stylesheet authoring**
    - [x] **10a** Native CSS nesting (`&`, nested rules, implicit descendant, `> + ~` relative
      rules, selector lists as `:is()`): nested rules are flattened at parse time after their
      parent; `text:hover { ... }` is told apart from a declaration by its block; nested at-rules
      are skipped with a warning — `M3-CSS-11`
    - [x] **10b** `@import url(x.css)` / `@import "x.css"` for runtime stylesheets (Bun already inlines
      them when bundling): `UiTree::add_stylesheet_with_imports` loads them through the app (game
      folder only), relative to the importing sheet, inlined before its rules (fonts, keyframes and
      `@starting-style` too) with their URLs rebased; cycles, deep chains, missing files and
      conditional imports (media, `supports()`, `layer`) are skipped with warnings — `M3-CSS-12`
    - [ ] SCSS (`$variables`, `@mixin`/`@include`) is **not planned** in the engine: custom properties
      and nesting cover most uses, and a Bun plugin can compile `.scss` for those who want it

11. **Pointer capture**
    - [x] `ref.setPointerCapture(pointerId)` / `releasePointerCapture` / `hasPointerCapture`: while
      captured (only while a button is held, as in browsers), the capturing element is the hit
      target, so moves, `:hover` and the release (and its click) go to it wherever the pointer is;
      `gotpointercapture` / `lostpointercapture` events (`onGotPointerCapture`, ...); capture ends
      when the buttons are released, on `releasePointerCapture`, when the element is removed or
      hidden, or when the window loses focus; `hasPointerCapture` follows the events. One pointer,
      so `pointerId` is ignored — `M3-EVT-06`
    - [ ] `touch-action` isn't needed until touch input exists; drag and drop is in M5.e
12. **Web fonts: WOFF and WOFF2**
    - [x] `.woff` and `.woff2` fonts from `@font-face` and `loadFont`: `register_font` recognizes
      their signatures and decompresses them to TTF/OTF with the pure-Rust `wuff` crate before
      Parley sees them; `@font-face` `src` lists take the first entry Iron can load (by
      `format()`, else extension), skipping `local()`, EOT and SVG fonts — `M3-FNT-03`
13. **`@media` queries**
    - [x] `media.rs`: `width`/`height` (with `min-`/`max-` and range syntax like
      `400px <= width < 800px`; px, em/rem as 16px), `orientation`, `aspect-ratio`, `resolution`
      (dppx/dpi/x: the window's DPI times the UI scale), `prefers-color-scheme` (the window's
      theme), `prefers-reduced-motion`, `pointer`/`any-pointer`, `hover`/`any-hover`; `and`, `or`,
      `not`, `only`, media types (`screen`/`all` match, `print` doesn't), comma lists; unknown
      features never match. `@media` blocks at the top level, nested in rules and on `@import`
      give rules conditions the cascade checks; the tree keeps a `MediaEnv` (size from
      `set_size`, the rest from `set_media_env`) and restyles when any condition's result flips
      — `M3-MQ-01`, `M3-MQ-02`

Accessibility and drag and drop moved to M5.e, as UI requirements before the editor starts.

## M4 — Hot reload, physics and audio

Physics and audio come ahead of the scene work (M5), so its object model is designed with them in
mind. Hot reload is split (2026-10-17): the plumbing is here; keeping state across a reload waits
for GameObjects and scene serialization (M5.a/b), which it reuses — see "Hot reload, part 2" in M5.

- [x] Bun watch → rebuild → engine notified: `bun run dev` in an example (`bun build --watch`);
  the app watches `dist/game.js` and `game.css` with `notify` and reloads once they settle —
  `M4-HOT-10`
- [x] Reload plumbing: the new bundle loads into a fresh runtime and is swapped in only if it
  loads and its components spawn; otherwise the old code keeps running with the error logged.
  The UI tree, script stylesheets and `game.css`, physics and audio are reset, and components
  start over (`onStart` runs again) — `M4-HOT-01`, `M4-HOT-04`, `M4-HOT-09`
- [ ] Edit a TS file while the game runs; the change appears without restarting (manual, needs
  the window) — `M4-HOT-08`
- [ ] `rapier` integration + TS components — `M4-PHY-*`
  - [x] `iron_physics` crate: rapier world driven by `physics.*` ops with script-chosen IDs;
    bodies (dynamic, fixed, kinematic), colliders (box, ball, capsule, cylinder, convex, mesh;
    sensors), forces, character controller, raycasts; one flat batch back per tick —
    `M4-PHY-01`..`M4-PHY-03`, `M4-PHY-05`, `M4-PHY-06`
  - [x] TS API in `@iron/engine` (`physics`: `RigidBody`, `Collider`, collision handlers,
    `raycast` promises, `makeCharacter`/`move`), stepped in the app's fixed tick with results
    sent back through `__iron_physics`; `examples/physics` — `M4-PHY-02`
  - [ ] Bodies survive script hot reload (moved to hot reload part 2, M5) — `M4-PHY-04`
  - [x] `Rigidbody` / `Collider` components on GameObjects (M5.a2)
- [ ] `kira` integration (sounds, music, buses) + TS API — `M4-AUD-*`
  - [x] `iron_audio` crate: kira mixer driven by `audio.*` ops with script-chosen IDs; decoded
    and streamed sounds, instances (volume, rate, pan, loop, fades, seek, pause/stop), buses,
    spatial emitters around one listener, ended instances polled; an offline backend for tests
    and deviceless runs — `M4-AUD-01`..`M4-AUD-04`
  - [x] TS API in `@iron/engine` (`audio`: `Sound`, `Playback` with `onEnded`, `Bus`, `Emitter`,
    `setListener`, `audio.play(src)`); the app opens the mixer on the first `audio.*` op (offline
    for `--screenshot` runs or without a device), loads files from the game folder and reports
    finished sounds through `__iron_audio`; `examples/physics` knocks on impacts
    (`CollisionEvent.speed`)

## M5 — Scene, assets, rendering, editor

- [ ] **M5.a** GameObject + Component core (the s&box way), from TS components, for 2D and 3D
  objects. The first plane use is UI in 3D space (a menu drawn on a plane in the world).
  **Decided (2026-10-17):** our own wgpu renderer (DESIGN §7.4); an `iron_scene` crate with
  GameObjects in a generational arena, no ECS underneath (§7.5).
  API style (2026-10-17): Unity / s&box naming and units: `Vector2` (2D), `Vector3`
  (`forward`, `up`, `right`, ...), `Quaternion` with Euler angles in degrees,
  `transform.forward` / `eulerAngles` / `lookAt`, components on GameObjects.
  - [x] `iron_scene`: GameObjects (generational ids, names, enabled, parent/children), local and
    world transforms (glam `Vec3`/`Quat`/scale), moved-object tracking — `M5-OBJ-01`..`M5-OBJ-03`
  - [x] TS API: `Vector2`, `Vector3`, `Quaternion`, `Transform`, `GameObject`; script components
    attached to GameObjects (`addComponent`, `getComponent`, `this.gameObject`,
    `this.transform`); engine-side transform changes synced back in one batch per tick
    (`scene.*` ops, `ScriptScene`, `__iron_scene`); the starting components each get a
    GameObject; inactive objects' components don't update — `M5-OBJ-04`..`M5-OBJ-07`
  - [x] Unity / C# value types: `Vector2Int`, `Vector3Int`, `Vector4`, `Rect`, `RectInt`,
    `Bounds`, `Ray`, `Plane`, `Color`, `Mathf` — `M5-MTH-01`..`M5-MTH-03`
  - [x] C# `DateTime` / `TimeSpan` on JS `Date` (millisecond resolution, C# formats) —
    `M5-TIM-01`, `M5-TIM-02`
  - [x] Everyday helpers: `Debug` / `Log` / `Console` logging, `Time` (fed by `__iron_tick`,
    with `timeScale`), `TimeSince` / `TimeUntil`, `Task.delay` / `frame`, `Random` and a seedable
    `Rng`, Unity-style input (`getKey`, `getKeyDown`, `getAxis`, `getButton`) —
    `M5-HLP-01`..`M5-HLP-04`
  - [x] 3D view basics: `Camera` component, a perspective pass with depth in `iron_render`
    (`WorldRenderer`: instanced quads, back-to-front, textured or coloured), `PlaneRenderer`
    (a plane in the object's XZ plane), screen UI blended over the world
    (`UiRenderer::render_overlay`); `examples/world` — `M5-RND-05`
  - [x] Camera system: draw order by `depth`, perspective and orthographic (2D) projections,
    viewport rects (split screen, minimaps), `Camera.main` / `allCameras`, `Matrix4x4`,
    `Screen`, screen/viewport/world conversions (`screenPointToRay`, `worldToScreenPoint`,
    `viewportToWorldPoint`, ...), `CameraFollow` and `CameraOrbit` — `M5-CAM-01`, `M5-RND-06`
  - [ ] Cameras rendering to a texture (`targetTexture`)
  - [x] Lifecycle hooks the Unity / s&box way: `onStart`, `onUpdate` and `onLateUpdate` per
    frame, `onFixedUpdate` on the 60 Hz tick, `onPreRender`, `onDestroy`; the engine only
    calls the hooks a component has — `M5-OBJ-08`
  - [x] Planes: a `WorldPanel` component (s&box's name) draws React UI on a plane in the world:
    `createRoot(panel)` commits to the panel's own UI tree (`__iron_commit(ops, target)`), drawn
    into a texture each frame; the pointer is raycast from the camera under it into the
    nearest panel (screen UI wins where it has elements), drags stay with the panel, keys go
    to the surface clicked last; stylesheets and fonts reach every tree — `M5-PNL-01`..`M5-PNL-03`
- [ ] **M5.a2** Unity / s&box-style components (its own step, before the rest of M5):
  - [x] `Rigidbody` component (mass, drag, gravity, kinematic, velocity, angular velocity,
    `addForce` with `ForceMode` Force / Acceleration / Impulse / VelocityChange, at a point,
    explosions, constraints, continuous detection) and collider components (`BoxCollider`,
    `SphereCollider`, `CapsuleCollider`, `MeshCollider`, triggers, `PhysicsMaterial`) driving
    GameObject transforms both ways (`physics.attach`); `onCollisionEnter/Exit` and
    `onTriggerEnter/Exit` on the object's components; `onAwake` for components; the low-level
    handles are now `PhysicsBody` / `PhysicsCollider` — `M5-RB-01`, `M5-RB-02`
  - [ ] Collider sizes follow later scale changes (they take the scale when added)
  - [x] `CharacterController` component (Unity's: a `Collider` with `move` returning
    `CollisionFlags`, `simpleMove` with gravity, `isGrounded`, `velocity`, slope limit, step
    offset) on rapier's character controller; character moves wait a step after colliders
    are added (refreshing queries mid-step tripped rapier's islands) — `M5-RB-03`
  - [x] Joints as components: `FixedJoint`, `HingeJoint` (axis, limits, motor),
    `SpringJoint`, `CharacterJoint` (ball), `RopeJoint` (max distance), with `connectedBody`
    (or the world), anchors and auto-configured connected anchors; `iron_physics` `joint` /
    `removeJoint` ops; a `Rope` component builds a visible, colliding chain of capsule
    segments on ball joints, holding an optional end body — `M5-JNT-01`, `M5-JNT-02`
  - [ ] Rope rendering as one smooth tube (a line renderer) instead of capsule segments
  - [ ] Meshes and lights. Order of work (2026-10-17), done so far:
    - [x] Renderer: `iron_render::world::mesh` builds Unity's primitives (cube, sphere, capsule,
      cylinder, plane, quad; Unity sizes) and fills in missing normals; `WorldRenderer` has a
      lit mesh pass (instanced by mesh, `set_mesh` / `remove_mesh` for script meshes,
      primitives at `primitive_id`), `MeshInstance` (colour, emission, smoothness, unlit,
      texture), `Light` / `LightKind` (directional, point with range, spot with soft cone),
      per-view `ambient`; opaque meshes first, then see-through meshes and quads back to
      front — `M5-MSH-01`, `M5-RND-07`
    - [x] Script side: `render.mesh` / `render.meshRenderer` / `render.light` /
      `render.ambient` ops (`crates/iron_app/src/world_view.rs`; script meshes uploaded before
      drawing); TS `MeshRenderer` (primitive name or `Mesh`), `Material` (color, emission,
      smoothness, unlit), `Mesh.create`, `Light` (directional / point / spot, color,
      intensity, range, spot angles), `RenderSettings.ambientLight`,
      `GameObject.createPrimitive` / `PrimitiveType` (mesh + matching collider);
      `examples/world` drops lit primitives with Rigidbodies — `M5-MSH-02`
    - [ ] Docs for meshes and lights (and the Rigidbody components) at the end of M5.a2
    - [x] Textures on meshes: `Material.mainTexture` / `mainTextureScale` /
      `mainTextureOffset` (`withTexture`), texture alpha on see-through meshes; `Mesh.create`
      takes Unity's bottom-up UVs — `M5-TEX-01`, `M5-RND-08`
    - [x] glTF models (`.glb` / `.gltf`, `iron_app/src/models.rs`): meshes (one engine mesh per
      primitive, ids from 2^30), base colour textures with their samplers, materials (colour,
      emission and strength, roughness → smoothness, unlit, blend), node trees; buffers and
      images only from inside the game folder; TS `Model.load` / `whenLoaded` /
      `instantiate({ colliders: "mesh" | "convex" })`, `MeshCollider.sharedMesh` (resolved
      engine-side from the render mesh's data); `examples/world` places a house — `M5-GLB-01`,
      `M5-GLB-02`
    - [x] Missing textures (never set, failed to load, a model image that can't be read) draw
      a magenta / black checker (`MISSING_TEXTURE`), sprites too — `M5-RND-08`
    - [ ] glTF follow-ups: normal / metallic-roughness / emissive / occlusion maps (with PBR,
      M5.d), skins and animations (M5.d), cameras and lights from the file, KTX2 / Draco /
      meshopt compression, texture transforms
    - [x] Classic test models: `examples/classics` shows the Stanford Bunny and the Newell
      teaset (teapot, teacup, teaspoon) as .glb (converted by `tools/make_models.py` from the
      downloaded originals, which aren't in the repo) on turntables, plus the material sweep
      (spheres from matte to glossy)
    - [ ] More classic test scenes (asked for 2026-10-17): the Cornell box (procedural; after
      shadows), and, with downloads the user approves first, Suzanne, Sponza, the Damaged
      Helmet (PBR), and the Stanford Dragon / Armadillo / Lucy
    - [ ] Shadows (directional shadow map)
  - [x] `AudioSource` (clip as a `Sound` or path, `playOnAwake`, volume, pitch, pan, mute,
    loop, `spatialBlend` 0 = flat / above 0 = from an emitter following its GameObject,
    min / max distance, `outputBus`, `play` / `stop` / `pause` / `unPause` / `isPlaying`,
    `playOneShot`, `playClipAtPoint`) and `AudioListener` (its object's pose, static
    `volume`) components, in TS over the existing `audio.*` ops; `examples/world`'s
    primitives thud in 3D — `M5-AUD-01`, `M5-AUD-02`
  - [ ] Audio follow-ups: partial `spatialBlend` (kira's spatialization strength), Doppler,
    a source's playback `time`, `onDisable` pausing sources
  - [x] Particles: a `ParticleSystem` component simulated in Rust (`iron_app/src/particles.rs`):
    duration / loop / delay, ranges for lifetime, speed, size, rotation and spin, two-colour
    start, gravity, drag, local or world space, max particles, rate and bursts, point /
    sphere / hemisphere / cone / box shapes (along the object's up), size and colour over
    lifetime, texture sheets, alpha or additive blending, `play` / `stop` / `pause` /
    `clear` / `emit`, `particleCount` / `isPlaying` / `onStopped` / `stopAction` through
    `__iron_particles`; drawn as camera-facing quads (a built-in soft spot texture,
    additive quads without depth writes, straight-alpha textures premultiplied) —
    `M5-PRT-01`, `M5-PRT-02`
  - [ ] Particle follow-ups: GPU simulation for big systems, collisions, velocity / force
    over lifetime, stretched billboards, sub-emitters, trails, soft particles (fade near
    surfaces), `AnimationCurve` / `Gradient` types for the over-lifetime keys
  - [ ] Glows and spotlights (2026-10-17, asked for with particles):
    - [x] Light halos: `Light.haloSize` draws an additive glow in the light's hue, pulled
      towards the camera so nearby surfaces don't slice it
    - [x] Bloom: with `RenderSettings.bloom` on, the world draws into an `Rgba16Float`
      texture, a soft-knee threshold and dual-filter down / tent up chain (scatter mixes the
      levels) blurs the bright parts, and a composite adds them back — `M5-RND-09`
    - [x] Visible spotlight beams: `Light.beamIntensity` draws an open cone (`BEAM_MESH`)
      as an additive `MeshGlow::Beam` mesh, soft at its edges and fading along the range;
      additive meshes (`MeshGlow::Additive`) render without depth writes from both sides
    - [ ] Glow follow-ups: tonemapping (ACES / neutral) and exposure, per-camera
      post-processing settings, `Material` blend modes for additive meshes in scripts,
      lens flares
  - [x] Textures and sprites: `iron_render` keeps textures (`set_texture`: sRGB, point /
    bilinear / trilinear, repeat / clamp / mirror, CPU mipmaps averaged in linear light);
    `WorldTexture::Frame` (panels) or `Asset`; `render.texture` / `textureRemove` ops
    (PNG/JPEG from the game folder or pixels; sizes back through `__iron_textures`); TS
    `Texture.load` / `Texture.create` / `whenLoaded`, `Sprite.create` / `Sprite.grid`
    (Unity's bottom-left rects and pivots, pixels per unit) and `SpriteRenderer` (colour,
    flips; unlit quads); `examples/world` has animated coin sprites — `M5-TEX-01`,
    `M5-TEX-02`, `M5-RND-08`
  - [ ] Texture follow-ups: SVG textures, async decoding off the frame, `sortingOrder` /
    sorting layers for sprites, sprite atlases, `Texture.getPixel` / `setPixels` after
    creation, render textures (a camera into a texture)
  - [x] 2D: `Rigidbody2D` (body type, mass, drag, gravity scale, freeze rotation, Vector2
    velocity, degrees, `addForce` / `addTorque` with `ForceMode2D`, `movePosition`) and
    `BoxCollider2D` / `CircleCollider2D` / `CapsuleCollider2D` / `PolygonCollider2D` /
    `EdgeCollider2D` (offset), `onCollisionEnter2D` / `onTriggerEnter2D` and friends,
    `Physics2D.raycast` / `gravity`, in the same rapier world as 3D (planar bodies, colliders
    extruded `Physics2D.depth` deep so z doesn't matter); the physics batch now carries
    angular velocity (`Rigidbody.angularVelocity` reads it too); sprites get
    `sortingOrder`, see-through things sort by order then depth (along the view for
    orthographic cameras), quads depth-test `LessEqual`; `examples/platformer` — `M5-RB2D-01`,
    `M5-RB2D-02`
  - [ ] 2D follow-ups: a real 2D physics world (rapier2d) if the slabs fall short,
    concave polygons (decomposition), 2D joints, `Physics2D.overlapCircle` and friends,
    tilemaps, sorting layers, pixel-perfect camera
  - [x] `Cursor` API: `visible`, `lockState` (`CursorLockMode`: locked falls back to
    confined + hidden where the OS can't pin it, as on Windows; re-applied on focus),
    `setCursor(texture or path, hotspot)` through the CSS cursor code (UI cursors still win
    where set); game mouse input: `input.mousePosition` (UI pixels, top-left),
    `getMouseButton` / `Down` / `Up`, raw `mouseDelta` from device motion (keeps coming
    while locked), `mouseScrollDelta`, `getAxis("Mouse X" / "Mouse Y" / "Mouse
    ScrollWheel")`; mouse buttons are `MouseLeft` / `MouseRight` / `MouseMiddle` keys —
    `M5-CUR-01`
  - [x] `SystemInfo`: OS name, family and version, device type and name, CPU model and
    count, memory, architecture, language, touch; GPU name, vendor, API, kind and driver and
    the monitor's size, refresh rate and DPI once the window opens (`sysinfo`, `sys-locale`,
    wgpu's adapter info, winit's monitor); gamepads are `input.gamepads()` — `M5-APP-02`
  - [x] `Application`: name, `productName`, version, company, identifier and engine
    version from the game's `package.json` (`iron` section; the window title and icon use
    them), `platform`, `systemLanguage`, `isMobilePlatform`, `quit()`; installed as
    `globalThis.__iron_info` before the bundle runs (and on reload) — `M5-APP-01`, `M5-APP-02`
  - [ ] App follow-ups: `Application.targetFrameRate`, `runInBackground`, focus events,
    `persistentDataPath` (with M6 file IO), live monitor changes, battery / power state
- [ ] **M5.b** Scene representation and serialization (objects, components, hierarchy,
  transforms) with skybox support
  - [ ] Scene API (`SceneManager`): create, load (single or additive), unload and switch
    scenes; objects that persist across loads (`dontDestroyOnLoad`)
- [ ] **Hot reload, part 2** (moved from M4, after M5.a/b): keep state across a reload by reusing
  scene serialization
  - [ ] `@property` (standard decorators, which Bun supports) feeds serialization; wipe and
    rebuild restores properties, including handles to engine objects; `onReload` lets components
    re-attach callbacks — `M4-HOT-02`, `M4-HOT-05`
  - [ ] Rust-owned state survives (GameObjects, transforms, physics bodies, sounds); script-side
    registries and ID counters are rebuilt from it — `M4-HOT-03`, `M4-PHY-04`
  - [ ] React Fast Refresh for panels (React and the reconciler in a vendor bundle kept across
    reloads) — `M4-HOT-06`
  - [ ] Reload benchmark — `M4-HOT-07`
- [ ] **M5.c** Rendering depth: a basic cube scene end to end, better shadows, materials
  - [ ] Materials (asked for 2026-10-17), Unity Standard / URP Lit style:
    - [ ] PBR shading (metallic / roughness, GGX specular, energy conserving) replacing the
      Blinn-Phong `smoothness` (kept as 1 - roughness, Unity's name)
    - [ ] Shininess / specular colour (the specular workflow) and `metallic`
    - [ ] Reflective and mirror surfaces: a skybox / environment cube map for reflections
      (IBL), reflection probes (baked and realtime), planar mirrors (a camera rendering
      the reflected view), screen-space reflections later
    - [ ] Maps: normal, metallic-roughness, occlusion, emission (glTF's), detail and tiling
      per map
    - [ ] Transparency modes (fade, transparent with specular kept), alpha cut-off,
      double-sided; glass with refraction later
    - [ ] A material test scene (spheres sweeping roughness × metallic, the classic grid)
- [ ] **M5.d** The rest of scene, assets and rendering:
  - [ ] Animation: clips (transforms and component properties, curves), an `Animator`
    component with blending; skeletons and skinned meshes from glTF
  - [ ] Cloth and soft bodies (rapier's soft bodies), with cloth rendering
  - [ ] Scene serialization, GUID `.meta` files — `M5-SCN-*`
  - [ ] glTF import + cook, platform texture formats, asset hot reload — `M5-AST-*`
  - [ ] PBR, shadows, IBL, post-processing — `M5-RND-*`
- [ ] **M5.e** Editor shell. UI requirements first (before the editor work starts):
  - [ ] Accessibility: an AccessKit tree from the UI tree (roles, names, states, focus, actions),
    so screen readers and platform accessibility tools work
  - [ ] Drag and drop in the UI: `draggable`, `dragstart` / `drag` / `dragenter` / `dragover` /
    `dragleave` / `drop` / `dragend` with a data payload, drop targets, a drag image, and files
    dropped onto the window (winit's `DroppedFile`); builds on pointer capture (M3d group 11)
  - [ ] Then the shell: docking, tree view, virtualized list, inspector, undo/redo — `M5-EDT-*`
  - [ ] Later editor tasks:
    - [ ] UI editor: pick elements in a running game's UI, inspect their computed styles and box
      model, edit styles live (like browser devtools), and edit/arrange React UI visually
    - [ ] SVG editor: draw and edit shapes and paths (nodes, curves), gradients, strokes and
      groups; preview CSS styling and animations on them; save `.svg` files

## M6 — Networking and files for scripts (Script IO)

Research and design: [docs/research/networking-and-files.md](docs/research/networking-and-files.md).

- [ ] Bridge: `Value::Bytes`, JS-side promise table (`asyncOp`), one `__iron_io(events)` batch per
  frame with a budget, new `iron_io` crate (`IoHost`, generations for hot reload) — `IO-BRG-*`
- [ ] Files: game manifest; read-only `game` root and per-game writable `data` root (cap-std,
  stricter names, quotas, atomic writes); `Bun.file`-shaped async API — `IO-FS-*`
- [ ] `fetch`: lazy tokio runtime, reqwest over rustls; policy (manifest hosts, HTTPS-only when
  shipped, resolved-address guard with pinning, checked redirects, forbidden headers, limits);
  `fetch`/`Headers`/`Request`/`Response`/`AbortController`/`URL` — `IO-NET-*`
- [ ] WebSocket: tokio-tungstenite on the same policy; text/binary, close codes, backpressure,
  limits — `IO-WS-*`
- No raw TCP/UDP for scripts: multiplayer stays engine-side (M8)

## M7 — Lighting

- [ ] Baked lighting (lightmaps), light and reflection probes

## M8 — Multiplayer

- [ ] QUIC transport, Steam multiplayer (`steamworks`), `@sync` / `@rpc`

## M9 — Script backends, AOT and release packaging

- [ ] **M9.a** `deno_core` and Bun script backends; packaging and distribution (pack files, Steam
  Workshop?)
- [ ] **M9.b** Ahead-of-time scripts for shipped games: compile the Bun-bundled TypeScript down when the game
  is built, instead of interpreting it in QuickJS. Native code compiled into the executable is
  the preferred output; bytecode is the fallback. Candidates to evaluate:
  - QuickJS bytecode (the cheapest step: skips parsing at startup)
  - TypeScript → Rust native via Perry or Vercel's scriptc
  - AssemblyScript → wasm → wasm IR → Rust via `wasm2rs`
  - Questions: keeping the `__iron_op` / `__iron_commit` boundary unchanged, React's dynamic
    code under each option, and keeping dev builds on QuickJS for hot reload (M4)
- [ ] **M9.c** Release packaging for macOS, Linux, Android and iOS (iOS: QuickJS backend, Xcode
  wrapper)

## M10 — Optional extras

- [ ] OpenXR: Quest VR, PC VR
- [ ] Steam Audio spatial sound
- [ ] Ray tracing tier
- [ ] Video: a `VideoClip` asset and `VideoPlayer` component (to a texture, UI or a plane)

## Later

- [ ] SVG on the GPU ("native" SVG): tessellate fills and strokes (e.g. lyon) into triangles drawn
  by the UI renderer, instead of rasterizing every change on the CPU with resvg (M3d 5.1), so
  animated and large SVGs stay cheap; per-shape hit testing (`pointer-events` on shapes). Then:
  - [ ] SMIL animation (`<animate>`, `<animateTransform>`, `<animateMotion>`, `<set>`), which resvg
    ignores
  - [ ] `d` path morphing between paths with different commands (normalize both to cubic curves
    with matching segment counts before interpolating)
