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
- [x] **Decision: keep Taffy.** Costs fit the frame budget (~0.8 ms worst-case incremental update for 2,246 nodes); Yoga would do the same ancestor relayout. Revisit at M7 with the real editor UI

**M2 status:** steps 1–7 done. Remaining: the manual resize check (`M2-MAN-01`) and a CI run (needs a push).

**Suggested order:** 1 → 2 → 3 → 4 → 5 → 6 → 7. Steps 2 and 3 are independent once the opcode list (step 2's first item) is agreed, so they can run in parallel.

## M3 — Real UI: text, CSS, interaction

**Goal:** game UIs with real text and fonts, CSS stylesheets with selectors and pseudo-elements, and
interaction (pointer, keyboard, focus, text input with IME, scrolling). Planned in three parts that land
in order: **M3a** text and fonts → **M3b** CSS stylesheets → **M3c** interaction. Each part ends with
its tests passing and a commit per step.

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
   - [ ] Known gap: a family with only an italic face falls back to another family for normal text
     (CSS would use the italic face); revisit with fontique
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
   - [ ] Parse stylesheets with `cssparser`: style rules, `@font-face` (feeds `loadFont`), comments,
     `!important`; unsupported at-rules warn — `M3-CSS-01`
   - [ ] Sources: `dist/game.css` loaded with the bundle; `addStyleSheet(cssText)` / `removeStyleSheet`
   - [ ] Declarations: kebab-case → our camelCase props; values as CSS strings into `apply_style`
   - [ ] Custom properties (`--x`) and `var(--x, fallback)` — `M3-CSS-05`
2. **Selectors** (`selectors` crate, `Element` implemented for UI nodes)
   - [ ] Simple selectors: type (`box`, `text`, `span`, `input`, `textarea`, `checkbox`), universal
     `*`, `.class` (`className` prop), `#id` (`id` prop), attribute selectors on `data-*` props — `M3-CSS-08`
   - [ ] Combinators: descendant (space), child `>`, next sibling `+`, subsequent siblings `~` — `M3-CSS-02`
   - [ ] Structural: `:first-child`, `:last-child`, `:only-child`, `:nth-child(an+b [of S])`,
     `:nth-last-child()`, `:first-of-type`, `:last-of-type`, `:nth-of-type()`, `:root` — `M3-CSS-02`
   - [ ] Emptiness: `:empty` (no children and no text) and `:blank` (no children, or only whitespace
     text; Selectors Level 4, so implemented as our own pseudo-class) — `M3-CSS-09`
   - [ ] Logical: `:not()`, `:is()`, `:where()`, `:has()` (relative selectors) — `M3-CSS-03`
   - [ ] User action (driven by M3c, testable earlier by setting states directly): `:hover`, `:active`,
     `:focus`, `:focus-visible`, `:focus-within` — `M3-CSS-10`
   - [ ] Form state: `:enabled` / `:disabled` (the `disabled` prop on inputs, buttons and boxes with
     `tabIndex`; disabled elements also ignore pointer and focus), `:checked` (the `checked` prop on
     a new `<checkbox>` element and on `<input type="checkbox" | "radio">`) — `M3-CSS-10`
3. **Cascade**
   - [ ] Specificity + source order; inline `style` prop beats stylesheet rules (unless `!important`);
     inheritance from M3a — `M3-CSS-04`
4. **Pseudo-elements**
   - [ ] `::before` / `::after` generate anonymous first/last children (invisible to React), styled by
     their rules; `content: ""` boxes for decoration and `content: "text"` for text — `M3-CSS-06`
5. **Restyle**
   - [x] Rule index: selectors bucketed by the id / class / type of their rightmost compound
   - [x] Dependency-aware invalidation: at load, record which classes, ids, attributes and states appear
     in ancestor positions, before `+`/`~`, inside `:has()`; a change restyles the node plus only the
     subtree / later siblings / ancestors it can affect. Structural changes restyle siblings only when
     structural selectors exist — `M3-CSS-07`
   - [x] Benchmark: one class change on the 2,246-node tree 0.20 ms (target < 1 ms); whole tree 8.8 ms — `M3-BEN-01`
6. **Example + checks**
   - [ ] `examples/boxes` restyled with a `.css` file using classes, structural selectors, `:has`,
     `::before`/`::after`

### M3c — Interaction
1. **Hit testing and pointer events**
   - [ ] Hit test in reverse paint order: rounded corners, `display`/`visibility`, `pointerEvents:
     none`, overflow clipping — `M3-EVT-01`
   - [ ] Pointer events to JS with the target's ancestor path; capture + bubble phases,
     `stopPropagation`, `preventDefault`; `onClick`, `onPointerDown/Up/Move/Enter/Leave`, `onWheel`,
     double click — `M3-EVT-02`, `M3-EVT-03`
   - [ ] `:hover` / `:active` states; `cursor` CSS property → OS cursor
2. **Keyboard and focus**
   - [ ] `tabIndex`, Tab / Shift-Tab order, `focus`/`blur` events, `:focus`, `:focus-visible`,
     `:focus-within`; key events to the focused node — `M3-EVT-04`
   - [ ] Imperative `ref.current.focus()` / `blur()` via ops
3. **Text input**
   - [ ] `<input>` and `<textarea>` as controlled elements (`value`, `onChange`, `placeholder`,
     `disabled`) backed by Parley `PlainEditor`: caret, selection (mouse and keyboard), word/line
     navigation, clipboard (arboard)
   - [ ] IME: winit `Ime` events → preedit display and commit; candidate window placed with
     `set_ime_cursor_area` — `M3-IME-01`
4. **Overflow and scrolling**
   - [ ] `overflow: visible | hidden | scroll | auto`: `DrawCmd::PushClip`/`PopClip` (scissor for
     rectangles, layer mask for rounded clips), wheel scrolling, simple scrollbars — `M3-EVT-05`
5. **Gamepads**
   - [ ] `gilrs` events to scripts; optional spatial focus navigation with a d-pad — `M3-PAD-01`
6. **Example + checks**
   - [ ] `examples/form`: buttons with hover/active styles, a text field with IME, a scrolling list;
     manual check on Windows with a Japanese or Chinese IME — `M3-MAN-01`

**Later (not M3):** CSS transitions and animations, `textOverflow: ellipsis`, WOFF2, accessibility
(accesskit), pointer capture, drag and drop, and **`@media` queries**: `width`/`height` (with
`min-`/`max-` and range syntax), `orientation`, `aspect-ratio`, `resolution` (the window's scale
factor), `prefers-color-scheme`, and `pointer`/`hover` (useful to tell mouse, touch and gamepad apart),
re-evaluated when the window resizes or moves between monitors. M3b's stylesheet parser keeps
`@media` blocks (with a warning) so adding this later doesn't change the parser's structure.

## M4 — Hot reload

- [ ] Bun watch → rebuild → engine notified
- [ ] Wipe-and-rebuild with property serialization — `M4-HOT-01`..`M4-HOT-05`
- [ ] React Fast Refresh for panels — `M4-HOT-06`
- [ ] Reload benchmark — `M4-HOT-07`

## M5 — Scene, assets, rendering

- [ ] **Decision:** own renderer vs reuse Bevy crates (DESIGN §7.4)
- [ ] **Decision:** ECS vs GameObject + Component
- [ ] Scene serialization, GUID `.meta` files — `M5-SCN-*`
- [ ] glTF import + cook, platform texture formats, asset hot reload — `M5-AST-*`
- [ ] PBR, shadows, IBL, post-processing — `M5-RND-*`

## M6 — Physics and audio

- [ ] `rapier` integration + TS components — `M6-PHY-*`
- [ ] `kira` integration — `M6-AUD-*`

## M7 — Editor shell

- [ ] Docking, tree view, virtualized list, inspector, undo/redo — `M7-EDT-*`

## Later

- [ ] Steam multiplayer (`steamworks`), `@sync` / `@rpc`, QUIC transport
- [ ] Baked lighting, light/reflection probes
- [ ] Android, Quest VR, PC VR (OpenXR)
- [ ] Steam Audio spatial sound
- [ ] Ray tracing tier
- [ ] iOS (QuickJS backend, Xcode wrapper)
- [ ] Packaging and distribution (pack files, Steam Workshop?)
- [ ] `deno_core` and Bun script backends
