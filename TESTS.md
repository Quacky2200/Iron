# Iron — Test Plan

The tests we work towards, milestone by milestone. Each test has an ID (e.g. `M1-SCR-02`) that [TODO.md](TODO.md) refers to. A milestone is **done** when all of its tests pass in CI.

See [DESIGN.md](DESIGN.md) for the architecture these tests check.

---

## Test kinds

| Kind | Tool | Where | Runs in CI |
|---|---|---|---|
| **Unit** | `cargo test` | inside each crate (`#[cfg(test)]`) | Yes |
| **Integration** | `cargo test` | `crates/*/tests/` | Yes |
| **JS unit** | `bun test` | `js/*/test/` | Yes |
| **Headless render** | `cargo test` + wgpu offscreen | `crates/iron_render/tests/` | Yes (software adapter if no GPU) |
| **Golden image** | headless render compared to a reference PNG with tolerance | `tests/golden/` | Yes |
| **Benchmark** | `criterion` | `crates/*/benches/` | Tracked, warns on regression; doesn't fail |
| **Manual** | a person runs the app | listed in TODO.md | No |

### Conventions
- Headless tests must not open a window. Rendering tests draw to an offscreen texture and read it back.
- If no GPU adapter is available, headless tests fall back to a software adapter (WARP on Windows, llvmpipe on Linux). Tests that truly need a GPU are marked `#[ignore]` and run locally.
- Golden images live in `tests/golden/<test-id>.png`. Updating them is an explicit step (`IRON_UPDATE_GOLDEN=1 cargo test`), never automatic. Each run also writes its renders to `tests/output/` (git-ignored).
- Test fixtures (tiny TS bundles, glTF files) live in `tests/fixtures/`.

---

## M1 — Hello Iron (scaffold)

### Workspace and build
| ID | Kind | Checks |
|---|---|---|
| M1-BLD-01 | CI | `cargo build --workspace` succeeds on Windows and Linux. |
| M1-BLD-02 | CI | `cargo clippy --workspace -- -D warnings` and `cargo fmt --check` pass. |
| M1-BLD-03 | CI | `bun install && bun run build` produces `examples/hello/dist/game.js`. |
| M1-BLD-04 | CI | `bunx tsc --noEmit` passes for all JS packages and examples. |

### Platform
| ID | Kind | Checks |
|---|---|---|
| M1-PLT-01 | Integration | The `Platform` layer can be created and destroyed without opening a window (headless mode). |
| M1-PLT-02 | Unit | winit events convert to Iron input events (key down/up, mouse move, resize, close). |
| M1-PLT-03 | Manual | `cargo run -p iron_app -- examples/hello` opens a window; closing it exits cleanly with code 0. |

### Rendering
| ID | Kind | Checks |
|---|---|---|
| M1-RND-01 | Headless | A wgpu device can be created (hardware or software fallback). |
| M1-RND-02 | Golden | Clearing an offscreen 64×64 target to a set colour reads back that exact colour. |
| M1-RND-03 | Headless | Resizing the surface/target to 0×0 (minimized window) doesn't panic. |

### Scripting
| ID | Kind | Checks |
|---|---|---|
| M1-SCR-01 | Unit | The QuickJS backend loads a bundle and evaluates `1 + 1` to `2`. |
| M1-SCR-02 | Unit | A JS syntax error returns `Err` with file/line info. It doesn't panic. |
| M1-SCR-03 | Unit | A thrown JS exception returns `Err` with the message and stack. |
| M1-SCR-04 | Unit | `console.log` from JS reaches the Rust log with a `[script]` tag. |
| M1-SCR-05 | Unit | `reset()` wipes globals: a global set before reset is `undefined` afterwards. |
| M1-SCR-06 | Unit | JS can queue ops and Rust receives them in order via `drain_ops()`. |
| M1-SCR-07 | Unit | Strings with non-ASCII text (`"héllo 世界 🎮"`) cross the boundary in both directions unchanged. |
| M1-SCR-08 | Integration | The `ScriptRuntime` trait test suite is generic, so every future backend runs the same tests. |

### Components (minimal)
| ID | Kind | Checks |
|---|---|---|
| M1-CMP-01 | Integration | Loading `tests/fixtures/hello.js` registers a `Hello` component class. |
| M1-CMP-02 | Integration | The engine calls `onStart()` once and `onUpdate(dt)` each tick, looked up by `(component id, method name)`. |
| M1-CMP-03 | Integration | After 3 ticks the component has logged `hello` once and `tick` three times. |
| M1-CMP-04 | Integration | A component that throws in `onUpdate` is reported and disabled; other components keep running. |

---

## M2 — Boxes (React → Taffy → screen)

### Script runtime additions
| ID | Kind | Checks |
|---|---|---|
| M2-SCR-01 | Unit (conformance) | `setTimeout` callbacks fire on the first `tick(now)` at or after their due time, in due-time order; `clearTimeout` cancels. |
| M2-SCR-02 | Unit (conformance) | Promise reactions and `queueMicrotask` callbacks run before control returns to Rust. |
| M2-SCR-03 | Unit (conformance) | `__iron_commit(ops)` arrives as one commit per call, in order, via `drain_commits()`; `reset()` drops pending commits. |

### Reconciler (JS)
| ID | Kind | Checks |
|---|---|---|
| M2-REC-01 | JS unit | Rendering `<box/>` emits `create` + `appendChild` ops. |
| M2-REC-02 | JS unit | Changing a prop emits only a `setStyle`/`setProp` op for that node, not a recreate. |
| M2-REC-03 | JS unit | Removing a child emits `remove`; reordering emits `insertBefore`. |
| M2-REC-04 | JS unit | One React commit produces exactly one op buffer flush. |
| M2-REC-05 | JS unit | Node IDs are numbers and never reused within a session. |

### UI tree and layout (Rust)
| ID | Kind | Checks |
|---|---|---|
| M2-UI-01 | Unit | Applying an op buffer builds the expected tree (parents, children, order). |
| M2-UI-02 | Unit | Invalid ops (unknown node ID) are reported and skipped; the tree stays valid. |
| M2-UI-03 | Unit | Changing the root size relayouts; percentage widths follow the new size. |
| M2-UI-04 | Unit | The draw list has one quad per visible box in paint order (parents before children, siblings in order); `display: none` and `opacity: 0` subtrees are skipped. |
| M2-STY-01 | Unit | Layout values parse: `10` → 10px, `"50%"`, `"auto"`, `"space-between"`, `"1fr 2fr"`; unknown properties warn and are ignored. |
| M2-STY-02 | Unit | Colours parse: `#f80`, `#ff8800`, `#ff880080`, `rgb(255 136 0)`, `rgba(255,136,0,0.5)`, `transparent`; bad colours warn and keep the previous value. |
| M2-LAY-01 | Unit | Row flexbox: three 100px children in a 300px row land at x = 0, 100, 200. |
| M2-LAY-02 | Unit | `flexGrow`, `padding`, `margin`, `gap`, `justifyContent`, `alignItems` match the CSS spec on fixed cases. |
| M2-LAY-03 | Unit | Grid: a 3×3 grid places items in the right cells. |
| M2-LAY-04 | Unit | Layout only recomputes when something is dirty: a second `compute` with no changes does no work. |
| M2-LAY-05 | Unit | The same fixtures pass through the `LayoutEngine` trait, so a Yoga backend can run them later. |

### Drawing
| ID | Kind | Checks |
|---|---|---|
| M2-DRW-01 | Golden | Three coloured boxes in a row render as expected. |
| M2-DRW-02 | Golden | Rounded corners and borders render as expected. |
| M2-DRW-03 | Headless | 2,000 boxes are drawn in a small, fixed number of draw calls (batched). |
| M2-DRW-04 | Golden | The same draw list at scale factor 2 renders at twice the pixel size. |
| M2-DRW-05 | Golden | Per-corner radii and borders, gradient fills, an image fill with `cover` and `contain`; repeated tiles; no-repeat edges don't bleed; missing or unsafe `url()`s fall back to the background colour. |
| M2-DRW-06 | Golden | Outer and inset box shadows with blur and spread, including on rounded boxes. |
| M2-DRW-07 | Golden | `filter: blur` blurs a box and its children; nothing outside the layer bounds changes. |
| M2-DRW-08 | Golden | `backdropFilter: blur` blurs only what's behind the box, clipped to its rounded shape. |

### Box visuals (styles and draw commands, no GPU)
| ID | Kind | Checks |
|---|---|---|
| M2-VIS-01 | Unit | `hsl()/hsla()` and named colours parse (`rebeccapurple`, `hsl(120 100% 25%)`). |
| M2-VIS-02 | Unit | `border: "2px solid red"` sets all widths and the colour; `border: "none"` clears; unsupported styles (`dashed`) warn. |
| M2-VIS-03 | Unit | `borderRadius` 1–4 values map to the right corners; per-corner props; `"50%"` gives a circle on a square box. |
| M2-VIS-04 | Unit | `boxShadow` lists parse: `"0 4px 12px rgba(0,0,0,.4), inset 0 0 0 1px #fff"` → two shadows with the right fields; bad lists keep the old value. |
| M2-VIS-05 | Unit | `filter`/`backdropFilter` accept `blur(Npx)` and `none`; other functions warn. |
| M2-VIS-06 | Unit | Gradient and `url()` backgrounds parse; size/position/repeat keywords and lengths parse. |
| M2-VIS-07 | Unit | Draw order per node (CSS painting order; `filter` and opacity apply to the box's own shadows too): backdrop blur → push layer (if filter or group opacity) → outer shadows (last in the list first) → fill/border → inset shadows → children → pop layer. Layer bounds cover everything drawn inside plus 3× the blur. |
| M2-VIS-08 | Unit | With group opacity, a half-opaque parent emits one layer with opacity 0.5 instead of multiplying children's alpha. |

### Integration (React in QuickJS)
| ID | Kind | Checks |
|---|---|---|
| M2-INT-01 | Integration | The `boxes` fixture bundle runs in `QuickJsRuntime`; applying its commits to `UiTree` gives the expected tree and layout rects. |
| M2-INT-02 | Integration | A `setTimeout`-driven state change produces only incremental ops (no recreation of unchanged nodes). |
| M2-MAN-01 | Manual | `iron examples/boxes`: resizing the window relayouts smoothly; the counter adds and removes boxes. |

### Performance
Benches: `cargo bench -p iron_ui` (layout) and `cargo bench -p iron_script` (commits). The tree is an
editor-like UI of **2,246 nodes** (toolbar, a 600-row file tree, a grid of 150 cards with shadows, an
inspector of 40 fields), applied as one commit of 6,738 ops.

| ID | Kind | Initial target | Measured (2026-10-16, release, RTX 5080 desktop CPU) | Revised target |
|---|---|---|---|---|
| M2-BEN-01 | Benchmark | Full layout: < 2 ms | **3.46 ms** | < 5 ms (one-off at load) |
| M2-BEN-02 | Benchmark | Relayout after one leaf change: < 0.2 ms | **0.58 ms** | < 1 ms |
| M2-BEN-03 | Benchmark | Apply the whole tree's commit: < 1 ms | **3.10 ms** (was 5.1 ms; boxing nodes halved our share) | < 5 ms (one-off at load) |
| M2-BEN-04 | Benchmark | JS→Rust transfer of 1,000 ops: < 0.5 ms | **0.40 ms** (0.59 ms including building them in JS) | < 0.5 ms ✓ |
| — | Reference | Relayout after a window resize | 0.43 ms | — |
| — | Reference | Draw list for the whole tree | 0.19 ms | — |

**Where the time goes** (profiled with a timing layout-engine wrapper): about 85% of a relayout is Taffy
recomputing flex/grid for the changed node's ancestors (its pixel rounding is only ~0.1 ms); our own
bookkeeping is ~0.1 ms. Applying a commit is ~1.8 ms of Taffy node creation plus ~1.7 ms of ours (about
0.3 ms of that is style parsing).

**Decision: keep Taffy.** The initial targets were guesses made before measuring. The measured costs
fit comfortably in a frame: a worst-case incremental update of this 2,246-node UI (relayout + draw list)
is ~0.8 ms, about 5% of a 60 Hz frame, and full builds happen once at load. Yoga would do the same
ancestor relayout work, so a Yoga spike isn't justified now. Revisit at M7 with the real editor UI
(virtualized lists will keep node counts down); the `M2-LAY-05` fixtures are ready if we do.

---

## M3 — Real UI: text, CSS, interaction

### M3a — Text and fonts
| ID | Kind | Checks |
|---|---|---|
| M3-TXT-01 | Golden | A text label renders crisply at 1× and 2×, including non-Latin text (CJK) and a colour emoji. |
| M3-TXT-02 | Unit | Text wraps to the container width; the measured height feeds back into layout (a `<text>` in a column pushes its siblings down). |
| M3-TXT-03 | Benchmark | Re-measuring unchanged text hits the cache (no shaping); changing one text node re-shapes only that node. |
| M3-TXT-04 | Golden | `color`, `fontSize`, `fontWeight: bold`, `fontStyle: italic`, `lineHeight`, `letterSpacing`, `textAlign`, `textDecoration`, `whiteSpace` render as expected. |
| M3-TXT-05 | Golden | `textShadow` with offset, blur and colour; multiple shadows stack in CSS order (first on top). |
| M3-TXT-06 | Unit | `color` and font properties inherit to descendants unless overridden; `currentColor` follows `color` (borders, shadows, gradients). |
| M3-TXT-07 | Unit / JS unit | React text children and nested `<span>`s produce text runs with the right styles; text updates send `SET_TEXT`; bare strings in `<box>` get an anonymous text wrapper. |
| M3-TXT-08 | Unit | Complex scripts segment with dictionaries: Ctrl+Shift+Right in `日本語の文章` selects `日本語`, and unspaced Thai wraps at word boundaries in a narrow box. |
| M3-TXT-09 | Unit / Integration / JS unit | Right-to-left text: `direction` parses and inherits; the `dir` attribute sets it unless CSS does (and `[dir]` selectors match); rtl blocks start at the right and `text-align: start/end` follow direction; mixed Latin/Hebrew runs order by the paragraph direction; rtl inputs and textareas align right (placeholder too) and arrow keys move visually; React sends `dir` as an attribute. |
| M3-TXT-10 | Unit + Integration + Golden | `text-transform` (capitalize across runs, `ß` → `SS`) and `text-overflow` (not inherited) parse; an ellipsis cuts lines only where overflow clips, keeps glyphs inside the box, ends with the `…` glyph, and goes on the left for rtl. |
| M3-TXT-11 | Integration | Long words break with `overflow-wrap: anywhere`, `word-wrap: break-word`, `word-break: break-word` and `break-all`; `pre-line` keeps newlines but collapses spaces, unlike `break-spaces`. |
| M3-TXT-12 | Unit + Integration | The `font` shorthand (resets left-out parts; bare numbers are weights), size keywords, named weights and `bolder`/`lighter`; `font-variant-numeric` and `font-feature-settings` become features, and `tabular-nums` gives digits one width. |
| M3-TXT-13 | Unit + Golden | The `text-decoration` shorthand (lines, style, colour, thickness; resets what it omits) and longhands parse; solid, double, dotted, dashed, wavy, overline and thick line-through decorations render. |
| M3-TXT-14 | Unit + Golden | `text-stroke` / `-webkit-text-stroke` and longhands parse and inherit; hollow outlined text and filled text with a stroke render. |
| M3-TXT-15 | Unit + Integration + Golden | `caret-color` and `font-smooth` / `-webkit-font-smoothing` parse and inherit; a field draws its caret in `caretColor` (or the text colour for `auto`); unsmoothed text has no partly covered pixels. |
| M3-FNT-01 | Integration | A font loaded with `loadFont` (and with `@font-face`) is used by `fontFamily`; text relayouts when it arrives; missing fonts fall back down the list, then to the bundled default; unsafe paths are rejected. |
| M3-FNT-02 | Unit | Picking weight/style from a family with several faces chooses the closest match (CSS font matching). |

### M3b — CSS stylesheets and selectors
| ID | Kind | Checks |
|---|---|---|
| M3-CSS-01 | Unit | Stylesheets parse: rules, comments, `!important`, `@font-face`; unknown properties and at-rules warn without dropping the rest of the sheet. |
| M3-CSS-02 | Unit | Structural selectors match: `:first-child`, `:last-child`, `:only-child`, `:nth-child(2n+1)`, `:nth-child(odd of .x)`, `:nth-last-child`, `:first-of-type`, `:empty`, `:root`; combinators `>`, `+`, `~`, descendant. |
| M3-CSS-03 | Unit | Logical selectors: `:not(.a)`, `:is(.a, .b)`, `:where()` (zero specificity), `:has(> .child)`, `:has(+ .sibling)`. |
| M3-CSS-04 | Unit | Cascade: specificity, then source order; inline `style` beats rules; `!important` beats inline; inherited properties flow down. |
| M3-CSS-05 | Unit | Custom properties and `var(--x, fallback)`, including inheritance and fallbacks for undefined variables. |
| M3-CSS-06 | Unit + Golden | `::before`/`::after` with `content: ""` and `content: "text"` create first/last generated children with their own styles; removing the rule removes them; they're invisible to React. |
| M3-CSS-07 | Unit | Restyle after changes: adding a class, inserting a sibling (`:nth-child`, `+`), and a descendant change that affects a `:has()` ancestor all update the right nodes. |
| M3-CSS-08 | Unit | Simple selectors: type, `*`, `.class` (multiple classes), `#id`, `[data-x]`, `[data-x="y"]`, and compound forms like `box.card#main`, with correct specificity. |
| M3-CSS-09 | Unit | `:empty` matches no children and no text; `:blank` also matches whitespace-only text; neither matches a node with a child box. |
| M3-CSS-10 | Unit + Integration | `:hover`, `:active`, `:focus`, `:focus-visible`, `:focus-within`, `:enabled`, `:disabled`, `:checked` follow element state; toggling `disabled`/`checked` props restyles; disabled elements ignore pointer and focus. |
| M3-BEN-01 | Benchmark | Incremental restyle on the 2,246-node tree after one class change: **< 1 ms**; full style of the tree tracked. |

M3b restyle results (`cargo bench -p iron_ui --bench restyle`; 2,246 nodes, ~30-rule editor stylesheet
with combinators, `:nth-child`, `:has`, `:focus-within` and variables; times include the following
layout pass):

| Measurement | Naive (match every rule, invalidate generously) | Rule index + dependency-aware invalidation |
|---|---|---|
| Style the whole tree | 50 ms | **8.8 ms** |
| One class change mid-list (M3-BEN-01) | 2.2 ms | **0.20 ms** ✓ |
| Hover a card | 0.61 ms | **0.21 ms** |
| Focus an input (`:focus-within`) | 0.21 ms | **0.20 ms** |

### M3c — Interaction
| ID | Kind | Checks |
|---|---|---|
| M3-EVT-01 | Integration | A click hits the topmost node under the point, respecting rounded corners, `pointerEvents: none`, `display: none` and overflow clipping. |
| M3-EVT-02 | Integration | Events capture down and bubble up the path; `stopPropagation()` stops them; `preventDefault()` is reported back. |
| M3-EVT-03 | Integration | `onClick` in a React component runs when its box is clicked; `:hover` / `:active` styles apply while the pointer is over / pressed. |
| M3-EVT-04 | Integration | Tab / Shift-Tab follow `tabIndex` and tree order; the focused node gets key events; `:focus`, `:focus-visible`, `:focus-within` apply. |
| M3-EVT-05 | Integration + Golden | `overflow: hidden` clips children (including rounded clips); `overflow: scroll` scrolls with the wheel and keeps hit testing consistent with what's drawn. |
| M3-IME-01 | Integration | IME composition (pre-edit) text shows in a text field and the committed text is inserted; the candidate window is positioned at the caret. |
| M3-BG-01 | Unit + Golden | `conic-gradient` (start angle, centre, angle/percentage stops, two-position stops) and the `repeating-` gradients parse, and render as a pie, repeating stripes and a rotated off-centre conic. |
| M3-BG-02 | Unit + Golden | Background layer lists (images, clips, repeats, positions, `position-x/-y`) parse; `round`/`space` fit whole tiles; a content-box-clipped colour, two clipped layers, spaced tiles and gradient text (`background-clip: text`) render. |
| M3-BG-03 | Unit + Golden | Blend modes parse; `mixBlendMode` multiply, screen, difference and color give the W3C results over a backdrop; `backgroundBlendMode` multiplies a layer into the colour without touching what's beneath the element. |
| M3-BG-04 | Golden | Gradients follow `background-size`, `-position` and `-repeat`: a repeated 20px tile makes stripes; a single no-repeat tile sits at the right. |
| M3-BRD-01 | Unit + Golden | Border styles and per-side colours parse (shorthands, longhands, 1-4 values; `none` sides take no space); side colours join on the diagonals, and dashed, dotted, double, inset and a single dashed side render. |
| M3-BRD-02 | Unit + Integration + Golden | Elliptical radii parse (`a / b`, per-corner pairs) and resolve like CSS (`50%` of a wide box is an ellipse); hit testing follows the ellipse; ellipses and shallow corners with a border render. |
| M3-BRD-03 | Unit + Golden | Outline properties parse (shorthand defaults, `auto`, negative offsets; `hidden` rejected); a solid outline sits outside the box past its offset, and a dashed one follows rounded corners. |
| M3-BRD-04 | Unit + Golden | `border-image` shorthand and longhands parse; the nine pieces map slices to the border (fill, round tiles, scaled-down widths, outset); tiling along an axis; an image 9-slice, a gradient border and a filled one render. |
| M3-CLIP-01 | Unit + Integration + Golden | SVG path data flattens (relative commands, curves, arcs, flags without separators; malformed data rejected); `polygon()` and `path()` clips resolve against the reference box and hit testing follows them (evenodd holes); a triangle, a framed hole and a clipped opacity group render. |
| M3-FX-01 | Unit | `opacity` accepts numbers, numeric strings and percentages. |
| M3-FX-02 | Unit + Golden | Filter chains parse (amount defaults and limits, blurs combine, colour functions compose in order, bad functions rejected; no `drop-shadow` in `backdrop-filter`); grayscale, brightness, invert, hue-rotate, sepia, a shape-following drop shadow and a greying backdrop filter render. |
| M3-FX-03 | Unit + Golden | `image-rendering` parses and inherits; a scaled-up image blends between texels by default and keeps them pure when `pixelated`. |
| M3-FX-04 | Unit + Golden | Mask lists and modes parse; alpha and luminance gradient masks fade a box; a repeated dot mask cuts out the box and its child. |
| M3-IMG-01 | Integration / JS unit | `image` elements: `Image` sends `src` as an attribute; the tree requests each `src` once, takes the natural size the app reports (one known axis keeps the ratio; failed loads stay empty), and draws the image in the content box per `objectFit` / `objectPosition`, clipped. |
| M3-INP-01 | Integration | A controlled `<input>`: typing calls `onChange`, the value round-trips, selection with mouse and Shift+arrows works, copy/paste uses the clipboard. |
| M3-INP-02 | Integration | Text-field shortcuts (Ctrl/Cmd+A/C/X/V) follow the physical key `code` when the layout's `key` isn't a Latin letter (Russian layout: `key` "ф", `code` "KeyA" selects all); AltGr (Ctrl+Alt) still types. |
| M3-PAD-01 | Integration | Gamepad button events from `gilrs` reach scripts (using a simulated device where possible). |
| M3-MAN-01 | Manual | `iron examples/form` on Windows: hover/active styles, typing with a Japanese or Chinese IME, scrolling a long list. |

---

## M4 — Hot reload

| ID | Kind | Checks |
|---|---|---|
| M4-HOT-01 | Integration | Changing a component's `onUpdate` and reloading runs the new code on the next tick. |
| M4-HOT-02 | Integration | `@property` values survive a reload (e.g. `speed` set to 42 is still 42). |
| M4-HOT-03 | Integration | Entities, transforms and physics bodies are untouched by a script reload (Rust owns the world). |
| M4-HOT-04 | Integration | A reload with a syntax error keeps the old code running and reports the error. |
| M4-HOT-05 | Integration | A renamed or removed property is dropped with a warning; new properties get their defaults. |
| M4-HOT-06 | Integration | React panel state (e.g. a counter) survives a Fast Refresh edit. |
| M4-HOT-07 | Benchmark | Reload of a ~1 MB bundle with 1,000 components: **< 100 ms**. |
| M4-HOT-08 | Manual | Edit a TS file while the game runs; the change appears without restarting. |

---

## M5 — Scene, assets and rendering

| ID | Kind | Checks |
|---|---|---|
| M5-SCN-01 | Unit | Scene save → load round-trips entities, components and properties exactly. |
| M5-SCN-02 | Unit | Asset references survive moving/renaming the file (GUID in `.meta`). |
| M5-AST-01 | Integration | A fixture glTF cooks into the engine format; vertex/index counts match the source. |
| M5-AST-02 | Integration | Cooking is deterministic: the same input gives byte-identical output. |
| M5-AST-03 | Integration | Cooking for `desktop` gives BC7 textures; for `mobile`, ASTC. |
| M5-AST-04 | Integration | Editing a source asset triggers re-cook and the GPU resource is swapped. |
| M5-RND-01 | Golden | A PBR sphere lit by a directional light. |
| M5-RND-02 | Golden | A cube casting a shadow onto a plane. |
| M5-RND-03 | Golden | IBL from a fixture HDRI on metallic and rough spheres. |
| M5-RND-04 | Benchmark | Sponza (or similar) scene frame time tracked over time. |

---

## M6 — Physics and audio

| ID | Kind | Checks |
|---|---|---|
| M6-PHY-01 | Integration | A dropped box lands on a plane and comes to rest at the right height. |
| M6-PHY-02 | Integration | Collision enter/exit events reach TS components. |
| M6-PHY-03 | Integration | Character controller walks up a step and stops at a wall. |
| M6-PHY-04 | Integration | Physics bodies survive a script hot reload. |
| M6-AUD-01 | Integration | Playing a sound through `kira` with a null/offline backend produces non-silent output. |
| M6-AUD-02 | Unit | Distance falloff and panning give the expected gains. |

---

## M7 — Editor shell

| ID | Kind | Checks |
|---|---|---|
| M7-EDT-01 | Integration | Hierarchy panel shows the scene tree and updates when entities are added or removed. |
| M7-EDT-02 | Integration | Inspector shows `@property` fields; editing one changes the live component. |
| M7-EDT-03 | Integration | Undo/redo reverses and reapplies property edits and entity creation. |
| M7-EDT-04 | Benchmark | A 10,000-item virtualized list scrolls with layout + draw under **2 ms** per frame. |
| M7-EDT-05 | Manual | Dock, undock and resize panels; layout persists across restarts. |

---

## Later milestones (outline)

- **Networking:** `@sync` fields replicate; `@rpc.server` runs only on the server; prediction and reconciliation under simulated latency and packet loss.
- **Android / Quest:** app launches, renders, survives pause/resume and rotation.
- **VR:** stereo render at the headset's refresh rate; world panel interaction with controllers.
- **iOS:** QuickJS backend passes the full M1-SCR suite; app passes TestFlight.
- **Packaging:** packed build runs from a clean machine with no dev tools installed.
