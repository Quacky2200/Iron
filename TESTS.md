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
| M2-LAY-06 | Integration | A React render error unmounts the UI (with absolutely positioned boxes in it) and the tree still lays out: removing a node takes it off its containing block's out-of-flow list, so no cached layout visits it. |

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
ancestor relayout work, so a Yoga spike isn't justified now. Revisit at M5.e with the real editor UI
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
| M3-FNT-02 | Unit | Picking weight/style from a family with several faces chooses the closest match (CSS font matching); a family with only an italic face uses it for normal text instead of falling back to another family. |

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
| M3-EVT-06 | Integration + JS unit | Pointer capture needs a held button; while captured, moves and hover go to the capturer even over another element; the release goes to it and clicks it, then `lostpointercapture` and hover returns to what's under the pointer; capture ends on `releasePointerCapture`, removal and window blur; React sends the ops and `hasPointerCapture` follows the events. |
| M3-EVT-05 | Integration + Golden | `overflow: hidden` clips children (including rounded clips); `overflow: scroll` scrolls with the wheel and keeps hit testing consistent with what's drawn. |
| M3-EVT-07 | Integration | A span inside text is the target over its own characters (and in event paths, under its text); elsewhere, and over a `pointerEvents: none` span, the text element is hit. |
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
| M3-XF-01 | Unit | Every 2D and 3D transform function maps points like CSS (rotation direction, order, origin, `matrix`/`matrix3d` layouts, perspective scaling); homographies invert; back faces are detected; the individual properties and `transform-origin` parse; bad functions are rejected. |
| M3-XF-02 | Integration + Golden | Hit testing follows rotation, 3D turns and `backface-visibility`; `preserve-3d` combines turns before flattening; transform properties accept every form and keyword; a rotated bordered box with text, a skewed gradient, a card in perspective with a clipped child, and scaled text render. |
| M3-VIS-01 | Integration | `visibility: hidden` keeps layout space, draws nothing and takes no hits (the element behind is hit); a `visible` child still draws and is hit; a visible span in hidden text draws only its own glyphs. |
| M3-LST-01 | Integration + Golden | List items get markers numbered by position (renumbered after removals); every marker type, strings and `none`; outside markers hang in the padding while inside ones take space; `::marker` rules style markers; discs, numbers, inside roman numerals, string markers on text items and image markers render. |
| M3-SEL-01 | Integration | Static text: dragging selects and Ctrl+C copies; a click collapses; double-click selects a word and triple-click a line; `user-select: none` blocks it (and shows the arrow cursor), `all` takes the whole block, a child can opt back in with `text`; presses map through transforms. |
| M3-SEL-02 | Integration | Selected text draws a highlight (default blue); an ancestor's `::selection` sets its background and recolours the selected glyphs; `::placeholder` sets a field placeholder's colour, opacity and font size. |
| M3-CUR-01 | Unit + Integration | `cursor` image lists parse (keyword last, two-number hotspots) and inherit; bad lists are rejected; disabled fields show `not-allowed`; PNG cursors take the CSS hotspot and scale with the window, `.cur` files bring their own hotspot, oversized or broken images are skipped; every CSS keyword maps to an OS cursor. |
| M3-DLG-01 | Integration + JS unit | Closed dialogs are hidden and keep their authored `display`; open ones centre; modal ones paint and hit above fixed `zIndex` elements over a window-sized `::backdrop` (default tint, `::backdrop` rules); a modal takes focus, traps Tab and gives focus back; Escape and (with `closedBy="any"`) backdrop clicks send `cancel` then `close`, a prevented `cancel` stops `close`, `none` ignores both; `[open]`/`[modal]` selectors; React sends the attributes and runs non-bubbling `onCancel`/`onClose`. |
| M3-DLG-02 | Golden | A modal dialog over a darkened, blurred `::backdrop` on a striped page. |
| M3-ANI-01 | Unit | Timing functions (`ease` mid-point, `cubic-bezier` limits, every `steps()` position); values interpolate token by token (lengths, numbers, colours premultiplied, mixed units through `calc()`, transforms with matching functions, `none` as the neutral form); non-matching values don't; the `transition` shorthand and property matching. |
| M3-ANI-02 | Integration + JS unit | Hover rules and inline styles (without stylesheets) transition over time with delays and easing; a reversal starts from the current value; unlisted properties snap and removing `transition` ends it; `transitionrun`/`start`/`end`/`cancel` events reach scripts with the property and elapsed seconds, and `onTransitionEnd` bubbles in React. |
| M3-ANI-03 | Unit + Integration + JS unit | `@keyframes` parse (offsets sorted, selector lists, bad blocks dropped, `!important` ignored); the `animation` shorthand; sampling by phase, direction, fill mode and pause; keyframes with implicit ends and discrete flips; stylesheet and inline animations run, iterate, fill, pause and resume, end and report events; infinite alternating animations keep running; `onAnimationEnd` gets the name. |
| M3-ANI-04 | Integration | New elements transition from `@starting-style`; elements already styled ignore it when a class adds one; opening a dialog runs its starting style. |
| M3-ANI-05 | Integration | With `:outro` rules a removed element stays, fading out, unhittable and unfocusable, until its transition ends; without `:outro` rules (or with nothing to animate) removal is immediate; an exit animation holds removal but an ongoing infinite one doesn't. |
| M3-SEL-03 | Golden | A focused field's selected text (default highlight), static text dragged across under an ancestor's `::selection` (yellow, recoloured glyphs), and a field's styled `::placeholder`. |
| M3-ANI-06 | Golden | One keyframe animation (colour, rotation, radius) sampled at 0/25/50/75/100% through negative delays, and a width + colour transition caught halfway. |
| M3-SCR-01 | Integration | `scrollbar-gutter: stable` narrows a clipping box's content by the scrollbar and its margins (less for `thin`, nothing for `visible`); `overscroll-behavior: contain` stops a wheel at an inner scroller's end from scrolling the outer one. |
| M3-GAME-01 | Golden | Image tints: an untinted grayscale tile, red and half-transparent blue tints, a green-tinted border image and a yellow-tinted `image` element. |
| M3-GAME-02 | Unit + JS unit | UI scale modes (none, fixed, target height, fit a design) give the right factor, clamped and safe for a minimised window; the `uiScale` op's arguments parse; `setUiScale` sends each form. |
| M3-CSS-11 | Unit + Integration | Nested rules flatten against their parent (`&`, implicit descendant, `>`, compound `&.x`, ident-first `text:first-child`, selector lists via `:is()`), declarations after nested rules still belong to the parent, bad nested rules and nested at-rules warn; nested `:hover` rules restyle children. |
| M3-CSS-12 | Integration | `@import` loads files relative to the importing sheet (nested imports relative to theirs), applies them before the importer's rules, rebases their `url()`s, and warns about missing files, cycles and conditional imports. |
| M3-SVG-01 | Golden | An SVG file (gradient, dashed stroke, path, text) as a 32px background and a 96px `image` rasterized sharp at that size, and a tiled `data:image/svg+xml` background. |
| M3-SVG-02 | Unit + Integration | Attribute names (camelCase kept, kebab otherwise), natural sizes from width/height/viewBox, escaping, CSS transforms in SVG syntax; an `svg` sizes from its attributes, serializes its content with attributes, CSS presentation properties and `currentColor`, restyles on `:hover` and transitions `fill`; its content isn't laid out, it's hit as one box, its `text` isn't an Iron text block, and attribute/CSS size changes resize it. |
| M3-SVG-03 | Golden | Inline SVG at 2x: a gradient from `defs` clipped by a `clipPath`, a dashed `currentColor` ring, a CSS-rotated polygon and SVG text, all styled from a stylesheet. |
| M3-SVG-04 | Integration | An `svg` whose opacity transitions on a parent's `:hover` fades in and out through its box's layer only (not also through an `opacity` in its markup), and isn't drawn at opacity 0. |
| M3-FNT-03 | Unit + Integration | WOFF and WOFF2 subsets of Inter register under a family name and decode to plain TrueType data; truncated files are rejected; `@font-face` `src` lists pick the first usable format, skipping `local()`, EOT and SVG fonts. |
| M3-MQ-01 | Unit | Media queries: min/max and range syntax, em units, media types with `not`/`only`, comma lists, nested `and`/`or`/`not`, aspect ratios, resolution in dppx/dpi, colour scheme, pointer/hover, reduced motion; unknown features don't match and malformed queries are rejected. |
| M3-MQ-02 | Integration | `@media` rules (top level and nested in a rule) apply and stop applying as the window resizes and the colour scheme changes; `print` never applies; `@import ... (min-width)` imports apply only when the query matches, and `supports()` imports are skipped. |
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
| M4-HOT-02 | Integration | `@property` values and script components' plain fields survive a reload (`speed` set to 42 is still 42, a counter keeps counting), references between objects are remapped, `onStart` doesn't run again and `onReload` does, and the clock carries on. |
| M4-HOT-03 | Integration | Objects, hierarchy and transforms come back where they were after a script reload (rebuilt from the saved scenes). |
| M4-HOT-04 | Integration | A reload with a syntax error (or a throwing top level) keeps the old code running and reports the error. |
| M4-HOT-05 | Integration | A renamed or removed property is dropped with a warning; new properties get their defaults; a saved class that isn't registered any more makes the reload start over. |
| M4-HOT-06 | Integration | A dev build reloaded in place: a React counter keeps its count while rendering with the new code (Fast Refresh), a live component answers with its new class and keeps its fields, and the top level's style sheet is swapped, not added twice. |
| M4-HOT-07 | Benchmark | Reload of a ~1 MB bundle with 1,000 components (each with a child object), keeping their state: **< 100 ms** (`crates/iron_scene/benches/reload.rs`; ~146 ms on 2026-10-17, not met yet). |
| M4-HOT-08 | Manual | Edit a TS file while the game runs; the change appears without restarting. |
| M4-HOT-09 | Integration | Clearing the UI tree for a reload removes every script node (attached or not, skipping `:outro`), and the new bundle's reconciler can reuse IDs from 1. |
| M4-HOT-10 | Integration | The bundle watcher reports a change to `game.js` once, after it settles; other files in the folder are ignored. |
| M4-HOT-11 | Unit | A texture made with `Texture.create` (point filtered, clamped, no mipmaps) is saved by `__iron_save_state` and restored with its pixels and options, re-sent to the engine as before, and the restored material uses it. |

---

## M5 — Scene, assets and rendering

| ID | Kind | Checks |
|---|---|---|
| M5-OBJ-01 | Unit | GameObjects nest; walking is depth-first in order; parenting under itself is refused; destroying takes the subtree (parent first) and stale ids miss after their slot is reused. |
| M5-OBJ-02 | Unit | World transforms compose through the hierarchy (rotation and scale), cached or not; re-parenting can keep the world position; setting a world position under a rotated parent lands there. |
| M5-OBJ-03 | Unit | Moved objects (with their children) are reported once; unchanged sets aren't moves; a disabled parent deactivates its subtree. |
| M5-OBJ-04 | JS unit | GameObjects: creating and parenting send ops and mirror the hierarchy; re-parenting keeps the world position; world transforms compose through parents (`position`, `forward`, `lookAt`, `eulerAngles`, `inverseTransformPoint`); engine moves update the mirror without echoing ops. |
| M5-OBJ-05 | JS unit | Components: `addComponent` with props, `getComponent`, `findObjectOfType`, removal; engine-spawned components get the engine's object. |
| M5-OBJ-06 | Integration | In QuickJS, a component creates a turret with a child and a `Spinner`; the spinner turns it 45° in 30 ticks (the child swings with it in Rust's scene), stops when the turret is disabled, and gets `onDestroy` when it's destroyed with its children. |
| M5-OBJ-07 | Integration | `scene.*` ops: bad ops fail; script changes aren't echoed back but engine moves are, as local transforms; destroying reports the subtree; ids past 2^24 stay exact. |
| M5-MTH-01 | JS unit | `Vector3` / `Vector2` / `Vector2Int` / `Mathf` behave like Unity's (magnitude, normalized, angle, reflect, projectOnPlane, moveTowards, signedAngle, deltaAngle, ...) and are immutable. |
| M5-MTH-02 | JS unit | `Quaternion`: Euler degrees in Unity's order round-trip; `lookRotation`, `fromToRotation` (also opposite vectors), `slerp`, `rotateTowards`; Unity's statics `Quaternion.slerp` / `slerpUnclamped` / `lerp` / `lerpUnclamped` / `rotateTowards` / `dot` (clamped, the short way round). |
| M5-MTH-03 | JS unit | `Plane` / `Ray` / `Bounds` / `Rect` queries; `Pose` and `Vector4`; `Color.hex` (sRGB to linear and back) / `lerp`; `Mathf` extras (angles, `smoothDamp`, Perlin noise, powers of two, gamma). |
| M5-TIM-01 | JS unit | `TimeSpan`: parts, totals, C# constant and custom formats, parsing, millisecond rounding. |
| M5-TIM-02 | JS unit | `DateTime`: months 1-12, calendar arithmetic with month-end clamping, C# standard and custom formats, JS `Date` conversion, ISO parsing, UTC/local views. |
| M5-HLP-01 | JS unit | `Debug` / `Log` / `Console` reach the right console level and format vectors, objects, maps and cycles. |
| M5-HLP-02 | JS unit | `Time` advances per tick and `timeScale` scales the `dt` components get; `TimeSince`, `TimeUntil`, `Task.frame`. |
| M5-HLP-03 | JS unit | Seeded `Rng` / `Random` repeat and stay in range (ints, floats, unit circle/sphere, rotations, shuffle). |
| M5-HLP-04 | JS unit | Unity-style input: `getKeyDown`/`getKeyUp` last one tick (repeats aren't presses); `getAxis` combines keys and sticks; `getButton`. |
| M5-RND-05 | Golden | The world pass: a floor in perspective, a green plane seen through a half-transparent blue one, a textured plane, and a translucent screen-UI bar blended over the 3D view. |
| M5-RND-06 | Golden | Cameras draw into their viewports in depth order: a perspective view on the left, an orthographic top-down view on the right (a floor marker to the east appears right of centre), and a picture-in-picture inset over the left view. |
| M5-CAM-01 | JS unit | `Matrix4x4` (TRS, inverse, determinant); camera conversions for perspective and orthographic cameras with a `rect` (screen and viewport points round-trip, parallel orthographic rays); `Camera.main` / `allCameras` order; `CameraFollow` and `CameraOrbit`. |
| M5-OBJ-08 | Integration | Lifecycle hooks run in Unity's order: start, fixed updates, update, late update, pre-render, destroy. |
| M5-PNL-01 | JS unit | Each React root (screen or world panel) commits to its own engine tree with its target id; updates to a panel's nodes only go to the panel's tree; one root per target. |
| M5-PNL-02 | Unit | Rays meet panel quads at the right UV and distance (centre, corner); rays behind or parallel miss. |
| M5-PNL-03 | Manual | In `examples/world`, hovering the world panel's menu items highlights them, clicking counts, and the screen HUD still takes clicks over the panel. |
| M5-RB-01 | Integration | In QuickJS with the app's physics sync: a crate with a `Rigidbody` and `BoxCollider` falls through a trigger and lands on a static floor (its GameObject at rest 0.5 m up; `onTriggerEnter` then `onCollisionEnter`); a kinematic body follows its script-moved GameObject. |
| M5-RB-02 | JS unit | Colliders make a static body at the object (scaled sizes, materials); a `Rigidbody` makes it dynamic with mass and constraints; colliders share the body; `ForceMode` impulses; Unity's capsule height; contacts reach `onCollisionEnter/Exit` and `onTriggerEnter` on both objects. |
| M5-MSH-01 | Unit | Built-in primitives are valid, have unit normals, wind outwards (positive volume) and have Unity's sizes; missing normals are made from triangles. |
| M5-ANI-01 | JS unit | `AnimationCurve`: linear and eased segments, steps (infinite tangents), loop / ping-pong wrapping, smooth keys through their points, sorted keys, saving with steps kept. |
| M5-ANI-02 | JS unit | `AnimationClip`: curves by path pose descendants' transforms (one op each) and component properties, including a field of a value (`color.g`); missing paths are skipped; JSON round trip; events call methods on the object's components once per pass, looping too. |
| M5-ANI-03 | JS unit | `Animation`: play, crossFade weights (halfway is an even blend), clips played once hold their end and stop, inactive objects don't animate. |
| M5-ANI-04 | JS unit | `Animator`: a 1D blend tree mixes by its parameter; triggers start transitions and are used up; exit-time and Any State transitions (no re-entry), cross-fades; rotations blend the short way; `play` / `crossFade`; controllers save as JSON and in scenes. |
| M5-ANI-05 | Integration | `__iron_animate(dt)` runs each frame after every `onUpdate`. |
| M5-SKN-01 | Unit | A skinned glTF (`worm.glb`, made by `make_worm.py`) gives joints and normalized weights per vertex, its skin's joints and inverse bind matrices, and its animation's channels (rotation keys, a step channel). |
| M5-SKN-02 | JS unit | Skinned model parts get a `SkinnedMeshRenderer` with their bones (ids and bind poses in the op); the file's animations become clips with paths from the copy's root and play on their own; script meshes send bone weights (normalized) and bind poses. |
| M5-RND-21 | Golden | A tube skinned to two bones bends where the upper bone turns, ignoring its object's matrix, and casts a bent shadow. |
| M5-RND-22 | Golden | The editor's ground grid (`WorldView.grid`): 1 m and 10 m lines on y = 0 fading with distance, the X axis red and the Z axis blue, hidden behind a cube standing on the origin. |
| M5-RND-23 | Golden + JS unit | Gizmo lines (`WorldView.lines`, `render.gizmos`): a constant width on screen, faint behind a cube, cut at the near plane. `Gizmos` draws through `matrix` in `color` (lines, rays, a cube's 12 edges, a sphere's three circles, a frustum along -z) and `present` hands them to one camera, replacing what it had. |
| M5-RND-24 | Golden + unit + JS unit | Strokes (`WorldView.strokes`, `render.stroke`, `LineRenderer2D`): capsules a width in world units with round joins and caps, soft edges, a 2-pixel line still shows, hidden behind a cube; segments follow their object, a closed loop joins the last point to the first, a camera's layers hide them, `remove` drops them; the component sends its points flat once per frame and `render.remove` on destroy. |
| M5-RND-25 | Golden + unit + JS unit | Fills (`tessellate_fill`, `WorldView.fills`, `LineRenderer2D.fillColor`): a square fills with two triangles covering its area, a self-crossing star has a hollow middle with even-odd and a solid one with non-zero, broken points are skipped; rendered fills sit under their outline and a see-through fill blends; fills follow their object and a width of 0 draws no outline; `Shapes` (rectangles, rounded rectangles, circles, ellipses, arcs, pies, regular polygons, Béziers, cardinal curves through every point, tension 0 straight, closed curves). Shapes draw by `sortingOrder`, then the order they were made, each fill with its outline (`WorldView.shapes`). |
| M5-TXT-01 | Unit + JS unit | `TextMesh` (`render.text`): its panel fits the text (wider for longer text, two lines for a line break, wrapped and taller at `maxWidth`), takes the colour as its tint and `pixelsPerUnit` as its scale, and isn't interactive; the component sends its text and look once per frame and `render.remove` on destroy. `examples/shapes` shows it in the window. |
| M5-SPL-01 | JS unit | `Spline`: passes through its knots, auto tangents match GDI's cardinal curve, linear knots are straight and bezier knots use their handles, `evaluate` goes by distance across uneven segments, closed splines join back, `nearest` finds the closest point, knots can be inserted, moved, re-moded (an auto knot turned bezier keeps its shape) and removed. `SplineAnimate` moves its object along the path (loop, ping-pong, once), facing along it, and in a container's space. |
| M5-PRE-01 | JS unit | `Prefab`: captured from a live tree, copies are independent and placed where asked, references inside point at the copy's objects and outside ones are dropped; later changes to the original don't leak in; JSON round trip; copies from a file save its path; other data is refused. |
| M5-AST-01 | Unit | The asset watcher reports changed files by game path once settled (not builds); a texture whose file changes loads again under its id with its new size, a model again with the same mesh ids, other files change nothing. |
| M5-AST-02 | JS unit | `Assets.onChanged` listeners hear changed paths until stopped; a throwing listener doesn't stop the others. |
| M5-AST-03 | Bun unit | `scripts/assets.ts`: assets (not code, builds or config) get a `.meta` with a GUID and their kind's importer; reruns change nothing; a moved file's meta follows it with its GUID; orphans are reported and pruned only on request. |
| M5-AST-04 | Unit | The engine reads `.meta` files into paths, GUIDs and importers (sorted; packages and broken metas skipped). |
| M5-AST-05 | JS unit | GUID lookups both ways; `"guid:..."` texture and sound sources; importer settings as `Texture.load` defaults (options win) and a streamed sound; scenes save GUIDs and load a renamed texture by its GUID. |
| M5-CLO-01 | Unit | The cloth solver: a grid pinned along an edge hangs about its length below the pins, keeps its edges' lengths, and its pins follow the object; a falling sheet drapes over a sphere without entering it; seams weld into one particle; capsules push points out sideways. |
| M5-CLO-02 | Unit | The app's cloth system: a plane pinned along an edge hangs (the far edge swings down), its mesh gets normals each step with one upload queued; cloths go with their objects; unknown ops fail. |
| M5-CLO-03 | JS unit | `Cloth` sends its settings, pin box and colliders, draws a simulated copy of its renderer's mesh, and gives the original back (freeing the copy) when removed. |
| M5-AST-06 | Unit | Cooking: BC1 / BC3 blocks round-trip closely (one-way gradients tightly), cut-outs keep their holes and smooth alpha its steps; cooked textures and models save and load back (models under fresh ids, descriptions renumbered); a linear request doesn't take an sRGB cook; a newer source makes the cook stale. |
| M5-MAT-01 | Unit | Tangents made from UVs point along increasing u with the bitangent (up the image) on the side `w` says; mirrored UVs flip it; every primitive's tangents are unit length and perpendicular to its normals. |
| M5-MAT-02 | JS unit | `Material` sends its maps, specular colour, rendering mode, cut-off and double-sidedness with `render.meshRenderer`; `with()` changes one setting and keeps the rest; scenes save the settings that differ from the defaults (and textures' `linear` / `anisoLevel`) and load them back. |
| M5-RB-03 | Integration | A `CharacterController` walking with `simpleMove` along a floor covers the expected distance, stays grounded and keeps its capsule on the floor. |
| M5-JNT-01 | Integration | Joints in `iron_physics`: a rope keeps a ball within its length of a world anchor, a ball joint keeps a pendulum's length, a motorised hinge turns, a spring settles near its rest length; bad joints are refused; removing one works. |
| M5-JNT-02 | Integration | Through the components: a `Rope` from a world point holds a weight about its length below; a `HingeJoint` with a motor turns its wheel. |
| M5-AUD-01 | JS unit | `AudioSource`: a 2D source plays its clip on awake (path clips load once) and applies volume, pitch and mute to what's playing; pause / unPause / stop; a 3D source plays from an emitter that follows its object (only real moves are sent) and is removed with it; one-shots don't interrupt `play()`; `playClipAtPoint` removes its emitter when the sound ends. |
| M5-AUD-02 | JS unit | `AudioListener` sends its pose on awake and when it moves or turns; `AudioListener.volume` sets the master volume. |
| M5-TEX-01 | Unit + JS unit | Textures: `Texture.load` sends the file and settings and gets its size from `__iron_textures` (`whenLoaded`, failures reject); `Texture.create` sends sRGB bytes top row first from linear colours bottom row first; materials carry a texture and tiling to `MeshRenderer`; the app decodes pixels into uploads, refusing mismatched sizes and paths outside the game; mip levels average in linear light, weighted by alpha. |
| M5-TEX-02 | Unit + JS unit | Sprites: `SpriteRenderer` sends its sprite (rect, pivot, pixels per unit, colour, flips) and clearing it removes the quad; `Sprite.grid` cuts sheets left to right, top to bottom; the app sizes the quad by pixels per unit, puts the pivot on the object's origin and flips by UV. |
| M5-RND-08 | Golden | Textured meshes and sprites: a tiled, mipmapped checker floor, a textured cube and an offset sphere, a see-through cube using a texture's alpha, a missing texture drawing the magenta checker, and point-filtered sprites upright and flipped. |
| M5-PRT-01 | Unit | Particle simulation (`crates/iron_app/src/particles.rs`): rate and bursts emit on time, particles die after their lifetime, `stop` lets them finish; one-shot systems stop after their duration and keep to `maxParticles`; gravity pulls local particles down in the world whatever the object's rotation; cones spray within their angle from their disc; size and colour over lifetime and texture-sheet frames; additive quads; unknown settings are refused. |
| M5-PRT-02 | JS unit | `ParticleSystem` sends its settings on awake and again only when they change (nested edits too); `play` / `stop` / `pause` / `emit` become ops; `__iron_particles` reports update `particleCount` / `isPlaying` and fire `onStopped` and `stopAction: "destroy"`. |
| M5-RND-09 | Golden | Glows: bloom (HDR world pass, threshold, intensity, scatter) on a hot emissive bar, an additive sphere, additive soft spots, and a spot light's beam (soft edges, fading along its length). |
| M5-RB2D-01 | Integration | 2D physics through the app's sync (`tests/fixtures/src/physics2d.ts`): a box at z = 2 falls through a 2D trigger onto an edge at z = 0 (`onTriggerEnter2D`, `onCollisionEnter2D`), rests on it and keeps its z and its x/y rotation; a circle rolls down an edge slope; a capsule with `freezeRotation` stands upright; a polygon lands; `Physics2D.raycast` hits the trigger above the box. |
| M5-RB2D-02 | JS unit | 2D colliders: boxes and circles become slabs `Physics2D.depth` deep (circles turned to face z), polygons convex slabs, edges static meshes; `Rigidbody2D` keeps to the plane (constraints), speaks Vector2 and degrees, reads spin from the engine's angular velocity; `Physics2D.gravity`. |
| M5-CUR-01 | JS unit | Game mouse input: `mousePosition`, mouse buttons (`getMouseButton*`, Fire1), raw `mouseDelta` adding up within a frame (y up), `getAxis("Mouse X" / "Mouse Y" / "Mouse ScrollWheel")`; `Cursor.visible` / `lockState` / `setCursor` become `cursor.*` ops. Showing, locking and images need the window: a manual check. |
| M5-APP-01 | Unit | The game's manifest comes from its `package.json` (`name`, `version`, `author`, and `iron.title` / `company` / `identifier` / `icon`), with the folder name and defaults when missing; `__iron_info` carries the app and system sections and later GPU / screen additions. |
| M5-APP-02 | JS unit | `Application` and `SystemInfo` read `__iron_info` (defaults before it exists); `Application.quit()` is an op. |
| M5-APP-04 | Unit | The log goes to `Player.log` in the game's settings folder (or `IRON_LOG_FILE`); the last run's is kept as `Player-prev.log`. |
| M5-APP-03 | Unit | `iron` without a game argument runs the current folder; the game folder may come before or after the options, and a second folder or an unknown option is an error. |
| M5-APP-05 | Unit | winit's `Focused` and `Occluded` window events become `InputEvent::Focused` / `Occluded`. |
| M5-APP-06 | Integration | Muting the mixer for the background silences everything and fades back to the scripts' master volume, including a master set while muted. |
| M5-APP-07 | Unit | Losing focus or minimising (0 × 0 or occluded) pauses unless `runInBackground` (or a screenshot run); pause and focus changes are reported once each, and again to reloaded scripts; the fixed clock caps catch-up at five ticks and restarts after a pause with a single tick. |
| M5-APP-08 | Unit | The `targetFrameRate` cap is a frame interval only with vsync off, a positive rate and no screenshot run. |
| M5-APP-09 | Unit | `iron.runInBackground` comes from the manifest (off by default); `runInBackground` and `targetFrameRate` set by scripts go into `__iron_info` for reloads. |
| M5-APP-10 | JS unit | `Application.runInBackground` reads the engine's value and sends `app.runInBackground`; `__iron_focus` updates `isFocused` / `isPaused`, calls `onFocusChanged` listeners (until unsubscribed, a throwing one doesn't stop the rest) and `onApplicationPause` then `onApplicationFocus` on components of active objects only. |
| M5-GLB-01 | Unit | glTF loading (`tests/fixtures/assets/house.glb`, made by `make_house.py`): each primitive becomes an engine mesh (normals made up when missing), embedded images become textures with their sampler's filter, the description has nodes (names, TRS, children, primitives), roots and materials (colour, roughness to smoothness, unlit, texture); buffers and images outside the game folder are refused; mesh colliders resolve a render mesh (engine ids past f32 precision too), scaled. |
| M5-GLB-02 | JS unit | `Model.load` loads once per path; `instantiate` builds the node tree as GameObjects (one child per part when a node has several), with materials and textures from the model, mesh colliders on request (`renderMesh` shapes), even when placed before loading finished; failures reject `whenLoaded` and place nothing. |
| M5-RND-10 | Golden | Directional shadows: a soft-shadowing sun over a cube, sphere, capsule and a floating plank whose shadow lands on the sphere and the floor (shadow map fitted to the camera, normal offset and slope bias, 3 × 3 PCF). |
| M5-RB-04 | JS unit | Colliders resize when their object or a parent is scaled, and when reparented (the old shape is removed); setting the same scale rebuilds nothing. |
| M5-LIN-01 | JS unit | `LineRenderer`: a tube has rings at its radius around a Catmull-Rom smoothed path through every point; it redraws only when its points change. |
| M5-RND-11 | Golden | PBR under a procedural sky: spheres sweeping smoothness (columns) and metallic (rows) reflect the sky's cubemap at roughness-matched mips, a smooth dark floor reflects it, and the camera shows the sky behind everything (skybox clear). |
| M5-RND-12 | Golden | A planar mirror: a tinted metallic quad reflects the scene from the reflected camera (projection flipped to keep winding, a clip plane dropping what's behind the mirror: the yellow cube there never shows), sampled in screen space. |
| M5-RND-13 | Golden | A box-projected reflection probe in a room: a chrome ball reflects the red, green and blue walls, the ceiling, the open side and the gold ball (whose rougher reflection is blurrier), from the probe's six-face cubemap. |
| M5-RND-14 | Golden | Quality low and high side by side: half render scale, no MSAA and a small hard shadow map against 4x MSAA, full scale and a big soft (5 x 5) shadow map, on thin leaning bars and their shadows. |
| M5-RND-15 | Golden | Culling masks: a sun that lights layers 0 and 1 (casting their shadows), a red lamp that lights only layer 2, and a mirror that leaves layer 1 out of its reflection. |
| M5-RND-17 | Golden | Material maps and modes under a sky: a normal-mapped cube (round bumps lit from the side), an occlusion-striped sphere, a specular-workflow sphere (green reflectance), an emission-mapped cube with a detail checker, a double-sided cut-out leaf whose shadow has its holes, and a faded sphere next to a transparent one in front of a red bar. |
| M5-RND-18 | Golden + JS unit | Post-processing: the same bright scene clipped, with neutral tonemapping, with ACES a stop darker, and with ACES plus FXAA (which changes only pixels near edges, the thin bar's among them); `RenderSettings.tonemapping` / `postExposure` send `render.post`, `Camera.antialiasing` goes out with the camera op. |
| M5-RND-19 | Golden | A second camera draws the scene into a render texture (yellow background, from behind) before the screen, and a monitor quad in the main view shows it. |
| M5-RTX-01 | JS unit | `RenderTexture` sends its size (`render.renderTexture`) and `setSize` resends; `Camera.targetTexture` goes out with the camera op (null clears it); a texture camera isn't `Camera.main`. |
| M5-RND-20 | Golden + Integration | Shadows: posts receding into the distance under four sun cascades, a cube under a spot light, a ring of pillars round a point light (six faces); 4 + 1 + 6 maps drawn once a frame, and a reflection probe refreshed every frame adds none. |
| M5-SHD-01 | Unit | Shadow planning: cascades grow with distance and each covers its slice; spot (1 layer) and point (6) lights nearest the camera share a 24-layer budget; a point light's +x face sees what's to its +x. |
| M5-SHD-02 | JS unit | `QualitySettings.shadowCascades` rounds to 1, 2 or 4 and goes out with `render.shadowSettings`; levels set it (Low 1, Medium 2, High and Ultra 4). |
| M5-RND-16 | Golden | Screen UI with `backdrop-filter: blur()` over the 3D world blurs the world behind it (sharp stripes outside the frosted panel, smooth ones inside), drawn the app's way: the world into the backdrop texture, the UI over a copy of it. |
| M5-CUL-01 | Unit | Frustums from perspective and orthographic view-projections keep boxes in view (and big ones reaching in) and drop ones behind, beside or past the far plane; lights' spheres count while their range reaches in; rotated boxes grow to hold their corners. |
| M5-CUL-02 | Integration | A camera looking down skips a mesh far to the side, a quad behind it and a point light out of reach (`stats()` counts them), and doesn't draw a plank above it whose shadow still lands on the floor. |
| M5-LAY-01 | Integration | `scene.layer`: objects start on layer 0, keep the layer set, refuse ones outside 0 to 31; unknown objects read as 0. |
| M5-LAY-02 | Unit | Cameras draw only the layers in their culling mask, and a layer's things only out to its cull distance, measured along the view or (spherical) straight from the camera. |
| M5-LAY-03 | JS unit | `LayerMask` names (built-ins and the manifest's, bad entries skipped), `getMask` / `contains`; `GameObject.layer` sends and range-checks; `Camera.cullingMask` (signed masks made unsigned), `layerCullDistances` (copies) and `layerCullSpherical` go out with `render.camera`; `fieldOfView` is clamped to 1 to 179 and converts to and from `horizontalFieldOfView`. |
| M5-LAY-04 | JS unit | `Light.cullingMask`, `ReflectionProbe.cullingMask` and `Mirror.cullingMask` go out with their ops, unsigned (all layers by default). |
| M5-LOD-01 | Unit | A LOD group shows its detailed renderer up close, the simple one farther off and nothing when tiny; `lodBias` 2 keeps detail twice as far; a forced level wins at any distance. |
| M5-LOD-02 | JS unit | `LODGroup` sends transition heights and renderer ids per level, hands out copies, forces levels and refuses heights that go up. |
| M5-DSP-01 | Unit | Window modes this platform lacks fall back to the nearest it has (no exclusive fullscreen on Wayland, only fullscreen on phones); mode names round-trip. |
| M5-DSP-02 | Unit | Saved display settings override the manifest's `iron.display`; bad values are ignored; saving and reading back is exact. |
| M5-DSP-03 | JS unit | `Screen` reads the engine's display state (mode, window size, supported modes, monitor resolutions) and sends `screen.set`; `Application.targetFrameRate` and `QualitySettings.vSyncCount` go to the engine; quality choices are saved to prefs and come back on the next launch without being saved again. |
| M5-DSP-04 | Unit + JS unit | Exclusive fullscreen without a size takes the monitor's current resolution and rate (not the smallest mode), an explicit size is kept, and leaving fullscreen goes back to the remembered window size; `Screen.fullScreenMode` / `fullScreen` ask for a mode without a size. |
| M5-PRF-01 | Unit | The prefs file saves and loads back (making its folders), skips unchanged writes, and a broken file starts afresh. |
| M5-PRF-02 | JS unit | `PlayerPrefs` reads what the engine loaded (fallbacks for missing keys and wrong types, ints truncated) and sends set / delete / deleteAll / save. |
| M5-QUA-02 | JS unit | Texture quality: `QualitySettings.anisotropicFiltering` and `globalTextureMipmapLimit` (clamped 0 to 3) go out with the quality op and come with each level. |
| M5-QUA-01 | JS unit | `QualitySettings`: levels (Low / Medium / High / Ultra) send the quality and shadow ops together; single settings change on their own (anti-aliasing rounded to 1 / 2 / 4 / 8, render scale clamped); unknown levels throw; `lodBias` goes out with the quality op; the manifest picks the default level and overrides levels. |
| M5-MSH-02 | JS unit | `MeshRenderer` sends its mesh (primitive or script `Mesh`) and material and resends on change; `Mesh.create` sends flat arrays and `destroy` removes it; `Light` and `RenderSettings.ambientLight` ops; `shadowCastingMode` reaches the op; `Mirror` (quads along +z) and `ReflectionProbe` (size, resolution, box projection, `renderProbe`) ops; `createPrimitive` adds the mesh and the matching collider. |
| M5-RND-07 | Golden | Lit primitives: a plane, cube, sphere and capsule under a directional and a point light, an unlit emissive cube and a see-through cylinder. |
| M5-SCN-01 | JS unit | Scene save → load round-trips objects, hierarchy, layers, enabled flags, engine and script components, `@property` values, references, shared meshes and textures and render settings exactly (saving again gives the same data); properties are set before `onAwake`; unknown components and values are skipped with warnings. |
| M5-SCN-02 | Integration | In QuickJS, a component saves a scene (with `@property` fields and an engine `Light`), loads it back as the single scene and keeps itself with `dontDestroyOnLoad`; Rust's scene ends up with the loaded objects where they were saved. |
| M5-SCN-03 | JS unit | `SceneManager`: the `Main` scene, additive loads, the active scene taking new objects, moving roots between scenes, unloading (not the last), events in order; single loads replacing all but `DontDestroyOnLoad`; `loadSceneAsync` from a file, failures rejecting; bad data refused. |
| M5-SCN-04 | JS unit | `GameObject.instantiate` copies objects with children and components; references inside the copy point at the copies, outside ones stay; world placement without a parent, local under one. |
| M5-SCN-05 | — | Asset references survive moving/renaming the file (GUID in `.meta`): covered by `M5-AST-05`. |
| M5-SCN-06 | JS unit | `HideFlags`: objects flagged `DontSave` (and their children) stay out of saved scenes; a camera on a `HideInHierarchy` object is never `Camera.main`. |
| M5-AST-07 | Integration (planned) | Cooking is deterministic: the same input gives byte-identical output. |
| M5-AST-08 | Integration (planned) | Cooking per platform: BC7 / BC5 for desktop, ASTC for mobile. |
| M5-RND-01 | Golden | A PBR sphere lit by a directional light. |
| M5-RND-02 | Golden | A cube casting a shadow onto a plane. |
| M5-RND-03 | Golden | IBL from a fixture HDRI on metallic and rough spheres. |
| M5-RND-04 | Benchmark | Sponza (or similar) scene frame time tracked over time. |

---

## M4 — Physics and audio

| ID | Kind | Checks |
|---|---|---|
| M4-PHY-01 | Integration | A dropped box lands on a plane and comes to rest at the right height. |
| M4-PHY-02 | Integration | Collision enter/exit events reach TS components. |
| M4-PHY-03 | Integration | Character controller walks up a step and stops at a wall. |
| M4-PHY-04 | Integration | Physics bodies survive a script hot reload. |
| M4-PHY-05 | Integration | Raycasts queued in a tick are answered after the step with the collider, distance and normal; misses report `-1` in the flat batch. |
| M4-PHY-06 | Integration | Bad physics ops (unknown kinds and shapes, missing bodies, zero directions, NaN) are rejected without changing the world; a triangle-mesh collider catches a ball. |
| M4-PHY-07 | Integration | A box kept in a plane (`constraints`: no z movement, turning only about z) stops under friction like a free one when shoved, stays in its plane when pushed along z, and is held where `position` puts it. |
| M4-PHY-07 | Integration | A shoved box stops from friction when its axes are locked to a plane (z movement and x/y turns locked); ignored until the rapier friction fix in TODO.md lands. |
| M4-AUD-01 | Integration | Playing a WAV (decoded and streamed) through `kira` with an offline backend produces non-silent output, and both are reported ended after they play out; undecodable files fail to load. |
| M4-AUD-02 | Integration | An emitter 1 m to the right pans right, fades at 10 m, is silent past its max distance, and pans left when moved to the left. |
| M4-AUD-03 | Integration | Volume and hard panning set the channel levels; a bus at ¼ volume scales what plays on it; muting the master silences everything; a stopped sound is reported ended once it has faded. |
| M4-AUD-04 | Unit | Gains convert to decibels (0 → silence); bad audio ops (unknown sounds, buses and emitters, NaN, unknown names) are rejected. |

---

## M5.e — Editor shell

| ID | Kind | Checks |
|---|---|---|
| M5-EDT-01 | Integration | Hierarchy panel shows the scene tree and updates when entities are added or removed. |
| M5-EDT-02 | Integration | Inspector shows `@property` fields; editing one changes the live component. |
| M5-EDT-03 | Integration | Undo/redo reverses and reapplies property edits and entity creation. |
| M5-EDT-04 | Benchmark | A 10,000-item virtualized list scrolls with layout + draw under **2 ms** per frame. |
| M5-EDT-05 | Manual | Dock, undock and resize panels; layout persists across restarts. |
| M5-EDT-06 | JS unit | Virtual list: the mounted range is the rows in view plus overscan (a million rows mount a handful); scroll-to-index moves only as far as needed or aligns start / center / end, clamped. (`js/editor-ui/test/models.test.ts`) |
| M5-EDT-07 | JS unit | Virtual list keys: arrows, Home / End and Page Up / Down move the selection within bounds; other keys are ignored. |
| M5-EDT-08 | JS unit | Tree view: flattening walks only expanded nodes (20,000 levels deep without overflowing); click selects, Ctrl toggles, Shift ranges from the anchor, Ctrl+Shift adds a range. |
| M5-EDT-09 | JS unit | Tree view keys: Up / Down / Home / End / Page keys move, Right expands then enters, Left collapses then goes to the parent, Shift extends, Ctrl moves focus only, Ctrl+Space toggles; revealing a node expands its ancestors. |
| M5-EDT-10 | JS unit | Docking: a tab dropped on a group's middle joins it; on an edge it splits the group (merging into a parent split the same way); emptied groups and single-child splits collapse; tabs reorder, activate and close; splitting a group by its only tab does nothing. |
| M5-EDT-11 | JS unit | Docking geometry: splitters resize neighbours within the minimum; computed rects match the flex layout; drops find the group and zone (tab strip and middle: center; outer quarter: that side). |
| M5-EDT-12 | JS unit | Dock layouts save to JSON and load back equal; unknown tabs (and their empty groups) are dropped; malformed JSON gives null. |
| M5-EDT-13 | JS unit | Menubar accelerators parse (`Ctrl+Shift+S`, `F5`, `Ctrl++`, `Del`), show normalized and match key presses by code and modifiers, searching submenus. |
| M5-EDT-14 | JS unit + component | Menubar keys skip separators and disabled items, Right / Left open and close submenus or move between menus, Enter chooses, Escape closes a level at a time; the React `MenuBar` opens on press, switches on hover, runs items, closes on Escape and on a press outside (`menubar.test.tsx`). |
| M5-EDT-15 | Unit | The log capture layer keeps records (level, target, message plus fields) in order in a ring buffer read by sequence number (`logging.rs`). |
| M5-EDT-16 | Unit | `editor.openFile`: VS Code's `code -g file:line:col` by default when on PATH, else nothing to run; an `IRON_EDITOR_OPEN` template is split with quotes and fills `{file}` / `{line}` / `{col}`; relative paths resolve against the first root that has them (`editor.rs`). |
| M5-EDT-17 | Unit | `editor.*` ops are refused outside `iron_editor`. |
| M5-EDT-18 | JS unit | Console code links: `path/file.ext:line[:col]`, Windows drives and backslashes, absolute Unix paths; URLs with ports and times aren't links. |
| M5-EDT-19 | JS unit | Console: identical entries collapse with counts at the first one's place; level counts (debug as info); filters by level and by search over message and target. |
| M5-EDT-20 | JS unit | The log feed asks for history once (`editor.logHistory`), parses `__iron_log` records and skips ones already seen (after a reload); clearing empties it; `openInEditor` is an op. |
| M5-DND-01 | Integration | A press on a `draggable` element moved past 4 px drags it: `dragstart`, `drag`, `dragenter` / `dragleave` as the element under the pointer changes, `dragover`; no pointer events meanwhile; a prevented `dragover` accepts, so the release sends `drop` then `dragend` (detail 1), and no click. (`crates/iron_ui/tests/dnd.rs`) |
| M5-DND-02 | Integration | A drop nobody accepted ends with `dragleave` and `dragend` (detail 0); Escape cancels the same way; preventing `dragstart` leaves a normal move; a short press is still a click; elements without `draggable` don't drag. |
| M5-DND-03 | Integration | The drag image is the dragged element moved by the pointer's travel (drawn last, translucent, in a transform), or another element grabbed at a point (`SetDragImage`); it goes when the drag ends. |
| M5-DND-04 | Integration | Files from the system: `FileDragEnter` sends `dragenter` / `dragover` with the paths, moves change the target, `FileDrop` sends `drop` with the paths as the event's strings, `FileDragCancel` a `dragleave`. |
| M5-DND-05 | Unit | The app gathers winit's per-file hover / drop / cancel events into one `FileDragEnter`, moves only when the cursor moved, one `FileDrop` or `FileDragCancel` (`file_drop.rs`). |
| M5-DND-06 | JS unit | `IronDragEvent`: one `dataTransfer` from `dragstart` to `drop` and `dragend` (`setData` / `getData` / `types`, `dropEffect`, `dropped`); `setDragImage` is a `dragImage` op; `draggable` is an attribute; system files arrive as `dataTransfer.files` with names and `"Files"` in `types`. |
| M5-DND-07 | Unit | Project file ops: paths can't leave the project (`..`, absolute, drives); listings put folders first and skip dot files; copies come in renamed instead of overwriting, folders whole; moves refuse overwriting, moving into itself or the root (`project.rs`). |
| M5-DND-08 | JS unit | Project paths (join, parent, breadcrumbs, `canMoveInto`, sizes); `listDir` / `copyIn` / `move` requests resolve or reject through `__iron_editor_fs`. |
| M5-DND-09 | Integration | End to end through QuickJS and React (`tests/fixtures/dnd.js`): dragging the item onto the folder drops its data and `dragend` says dropped; system files dropped on the folder reach its `onDrop`. |
| M5-DND-10 | JS component | The project browser lists the folder from `editor.listDir`; files dropped on a folder row call `editor.copyIn` into it; a file dragged onto a folder calls `editor.move`, and onto itself isn't accepted. |
| M5-DND-11 | Manual | In the window (`iron_editor <project>`): drag a file from Explorer / Finder onto the Project panel and onto a folder in it (copied there); drag an item onto a folder or breadcrumb (moved); the drag image follows the pointer and the cursor shows whether it can drop. |
| M5-EDT-21 | Manual | `iron_editor --screenshot out.png` (or `bun run screenshots editor`): menubar, Hierarchy tree with icons and chevrons, Scene / Inspector placeholders, Console with coloured levels, counts and underlined code links, status bar. Checked by eye, also with a menu and submenu open and with tabs docked elsewhere. |
| M5-EDT-22 | JS unit | Scene view camera (`scene-camera.ts`): yaw / pitch agree with the rotation; look keeps the eye, orbit the pivot; pan moves a view height per view height dragged; zoom scales and clamps the distance; fly speeds up with distance and Shift; F frames a sphere; the axis gizmo snaps to an axis and lists the axes farthest first. |
| M5-EDT-23 | Manual | `iron_editor --screenshot`: the Scene panel shows the demo scene through the see-through panel with the ground grid, SceneIcons (sun, bulb, spot, camera, speaker, probe) at their objects and the orientation gizmo in the corner; the Hierarchy lists the live objects. In the window: right-drag looks (WASD / QE fly, Shift faster), middle-drag pans, the wheel zooms, Alt+left-drag orbits, F frames the selection, clicking an icon selects its object (and the Hierarchy row) and shows its wire gizmos (frustum, light range / cone / rays, probe box, audio distances), clicking a gizmo axis looks along it and the centre toggles Persp / Iso; Q / W / E / R (or the toolbar) pick view / move / rotate / scale handles on the last selected object, which light up on hover and drag it; the Game tab shows the main camera. |
| M5-EDT-24 | JS unit | Transform handles (`handles.ts`): axes a constant length on screen; the pointer picks the centre or the nearest axis or ring (front arcs only); moving follows the pointer along local axes (Ctrl snaps to 1 m) or in the view plane; rings turn about their axis (Ctrl snaps to 15°); scale by the ratio along an axis, or uniformly from the centre, never below 0.01. |

---

## M7 — Lighting

| ID | Kind | What |
|---|---|---|
| M7-FOG-01 | Golden | Distance fog (`WorldView.fog`): red cubes from 3 m to 60 m in four views, clear air, linear (none before the start, all fog past the end), exponential and exponential squared (clearer nearby than exponential, both thick far away); the background stays clear. |
| M7-FOG-02 | Integration | Orthographic cameras fog by depth along the view (two quads at one depth match wherever they are on screen, a deeper one is foggier), and fog covers quads, filled shapes and strokes. |
| M7-FOG-03 | Unit + JS unit | `render.fog` reaches the views of cameras that show it and `null` clears it; an unknown mode is an error; `Camera.fog` (on by default) opts out. `RenderSettings.fog` / `fogMode` / `fogColor` / `fogDensity` / `fogStartDistance` / `fogEndDistance` send the fog only while it's on; scenes save it and bring it back. |
| M7-FOG-04 | Golden + unit + JS unit | Height fog (`Fog.height`, exact exponential integral): a camera 4 m up over a floor sees pillars hazy at the foot and clear at the top, the floor fading to fog far off, and the background thick with fog just above the horizon but nearly clear 30° up. `render.fog`'s seventh argument and `RenderSettings.heightFog` carry it; scenes save it. |
| M7-FOG-05 | Golden + unit + JS unit | Light shafts (`Fog.volumetric`): a low sun through a slatted roof in exponential fog; without shafts the air in front of the back wall is evenly dim, with them it's brighter and banded by the slats' shadows. `render.fog`'s eighth argument (clamped, with an optional albedo) and `RenderSettings.volumetricFog` (defaults for missing fields) carry it; `Light.volumetricIntensity` (default 1, not below 0) goes out with `render.light`. |
| M7-FOG-06 | Golden + unit + JS unit | Fog volumes (`WorldView.fog_volumes`): a box of mist over a pond hides the lower part of a post behind it, the part above shows, a post beside it is clear, and its top edge is soft. `render.fogVolume` follows its object's position and scale, cameras with `fog` off hide volumes, `render.remove` drops them; the `FogVolume` component sends its size, density, colour, blend distance (not below 0) and shape (box or ellipsoid). |
| M7-FOG-07 | Integration + unit + JS unit | Fog culling (`Fog::cull`, `cull_distance`): a view with linear fog ending at 40 m draws the cubes at 10 and 30 m and the near quad and skips (and counts in `WorldStats::fog_culled`) the cubes at 50 and 200 m and the far quad; exponential squared at 0.02 is opaque at about 117.7 m; with culling off, no fog or no density nothing extra is culled. `FogReach` measures perspective views by distance and orthographic ones by depth. `RenderSettings.fogCulling` rides on `render.fog`; `__iron_render_stats` fills `RenderStats`. |
| M7-FOG-08 | Golden | Spot and point lights in fog (`local_shafts`, worked out per pixel): in dark fog before a black wall, a spot light's cone is lit inside and dark just outside with a sharp edge right up to its tip, nothing above it; a point light glows all round, falling off from its middle. |
| M7-FOG-09 | Conformance + JS unit | Typed arrays cross the op boundary as number lists, read straight from their bytes (`Float32Array`, `Uint32Array`, views from `subarray`, `Uint8Array`); `Mesh.createFlat` / `setFlat` send flat lists (typed arrays as they are, UVs flipped into a `Float32Array`) and make plain arrays only when the mesh is saved. |
| M7-FOG-10 | Golden + JS unit | God rays (`VolumetricFog.sharp_distance`, `sharp_sun`): under a grate of thin bars, with the sun's shafts worked out per pixel near the camera the air ripples with thin beams the volume alone smooths away (the row's total variation at least 10% more); `volumetricFog.sharpDistance` / `sharpSamples` ride on `render.fog`. |
| M7-FOG-11 | Integration + JS unit | Temporal light volume (`cs_volume_light`, `volume::SIZES`): each cell takes one jittered sample a frame blended with its reprojected history, so under a grate of thin bars a row through the averaged volume ripples less than a single frame's (no stair steps); `volumetricFog.resolution` (low, medium, high, ultra) rides on `render.fog` (index 9), `denoising` (HDRP's mode, index 10: none / reprojection / gaussian / both; each cell's samples shifted by a hash of the cell, 64 a cycle, and the volume read through a four-tap filter, so blinds leave no moire or grain). |
| M7-FOG-12 | Integration | The light volume follows a turning sun (0.25 degrees a frame, a fast day) without trailing its old beams: after 60 frames of turning it differs from the same sun held still by less than half as much as a sun 2.5 degrees (10 frames) back would. |
| M7-FOG-13 | Integration + JS unit | The light volume's noise settings (`VolumetricFog.history_weight`, Unreal's history weight, and `samples` per cell per frame; `volumetricFog.historyWeight` / `samples`, op indices 11 and 12): with a still camera under a grate's beams, frame-to-frame change is nil without denoising (it steps instead), small with the defaults (1 sample, 0.9) and less than a third of that with 4 samples and 0.96. |
| M7-FOG-14 | Integration | A turning camera's beams stay close to a still camera's at the same view: each light-volume cell trusts its history less the further it moved (in cells), so a long history (0.96, 4 samples) smears no more than the default while turning (it smeared 40% more before); the history is read through a five-tap Catmull-Rom filter (sharp, so reprojecting doesn't blur); cells whose view moved take `motionSamples` (2) and smear less than with 1. |
| M7-FOG-15 | Integration | A spot light shining at a solid wall's back puts no haze on its front: the volume is read a slice and a half short of a surface (Unreal's depth bias), since the cells at its depth reach behind it (the shed's scanner showed its round cross-section through the planks). Shadowed spot and point lights shaft through the volume; a moving one's history fades by how far its beam swept. |
| M7-SPOT-01 | Integration | A pyramid spot light (HDRP's Pyramid shape: `outer_degrees` high, `aspect` times as wide in tangent) lights a wide, thin rectangle on a wall, dark above, below and past its sides; turned a quarter about its direction (`right` along y) it lights a tall, thin one. |
| M7-SPOT-02 | Unit | `Light.spotShape` (`cone` / `pyramid`) and `aspectRatio` (kept above 0) go out with the light's op. |
| M7-EXP-01 | Integration + JS unit | Auto exposure (`AutoExposure`, `cs_meter` in bloom.wgsl): a dim view and one twenty times brighter settle to the same grey; switching from dim to bright dazzles on the first frame, then eases off over the next; `RenderSettings.autoExposure` rides on `render.post` (index 3). |
| M7-EXP-02 | Integration | Each camera adapts on its own: on one split screen a dim view and one twenty times brighter both settle to the same grey (state keyed by `WorldView.exposure_key`, each viewport metered and graded with its camera's). |
| M7-EXP-03 | Integration + JS unit | Histogram metering (`AutoExposure.low_percent` / `high_percent`, `metering`): averaging everything, a small blazing patch darkens a dim view; between 10% and 80% it's left out and the view shows as it would alone. |
| M7-EXP-04 | Integration + JS unit | Auto exposure blends with the manual exposure (`Grading.fixed_ev`, `transition`): turned off, it's halfway between adapted and manual half a transition later and manual after it; `fixedExposure` / `exposureTransition` / `freezeExposure()` ride on `render.post`. |
| M7-EXP-05 | Integration + JS unit | Scripts read each camera's exposure back (`WorldRenderer::exposure`, `__iron_exposure` → `Camera.exposure` / `sceneBrightness`): a flat 0.05 view meters and is shown at EV100 log2(0.4); a reset (`reset_exposure`, `Camera.resetExposure()`) makes a cut to a bright view show settled at once where without it it dazzles. |
| M7-EXP-06 | Integration + JS unit | A soft floor (`dark_adaptation` / `dark_below`) and a compensation curve keep a very dark view darker than full adaptation; `QualitySettings.autoExposure` (the player's switch) turns auto exposure off and on. |
| M7-EXP-07 | Integration | A change of the manual exposure (`Grading.fixed_ev`) eases over `transition` (per camera, smoothstep): two stops darker over a second is partway at half a second and there after the second. |
| M7-RP-01 | Unit + JS unit | One render pipeline, as Unity 7 (URP; HDRP and Built-in go away): the manifest's `iron.lightUnits` (`physical` or `legacy`, the default) reaches the renderer and scripts (`GraphicsSettings.lightUnits`); the old `iron.renderPipeline` choice no longer picks anything, it only warns. |
| M7-RP-02 | Integration | A quality level's pipeline asset (`PipelineAsset`, Unity's URP asset per quality level): without light shafts a fogged view draws as if none were asked for, and a cap of 32 additional lights draws the nearest 32 of 40 point lights where the default draws all 40. |
| M7-RP-03 | Integration + JS unit | The asset's other settings: with HDR off a bright view clips at white instead of being tonemapped; without additional light shadows a shadowed spot light gets no shadow map. Each quality level carries them (`QualitySettings.volumetricFog`, `hdr`, `additionalLights`, `additionalLightShadows`; Low, the phones' tier, has no light shafts and 8 additional lights without shadows), sent with `render.quality`; the manifest's default level can go by platform (`iron.quality.default: { Android: "Low", default: "High" }`). |
| M7-HD-01 | Integration + Unit | Real-world light units (`iron.lightUnits: "physical"`, `WorldRenderer::physical_units`): directional lights' intensity is read as lux (renderer units lux / pi), spot and point lights' as lumens (/ 4 pi²); everything drawn is pre-exposed by the camera's last exposure and the post pass divides it out, so a view at 200000 nits (past fp16's 65504) settles to the same grey as one at 2 nits (the histogram reaching +32 stops). |
| M7-HD-02 | JS unit | A physical camera (`Camera.aperture`, `shutterSpeed`, `iso`, HDRP's defaults f/16, 1/200 s, ISO 200) gives `physicalExposure` = log2(N²/t) - log2(ISO/100); `Light.illuminanceAt(point, normal)` sums lux from every light (lumens to candela over 4 pi, inverse square, cosine); `Light.measureIlluminance` also counts shadows by physics rays (checked by eye in `examples/light-units`: 8 lux in the hall, 81438 in the sun). |

---

## Later milestones (outline)

- **Networking:** `@sync` fields replicate; `@rpc.server` runs only on the server; prediction and reconciliation under simulated latency and packet loss.
- **Android / Quest:** app launches, renders, survives pause/resume and rotation.
- **VR:** stereo render at the headset's refresh rate; world panel interaction with controllers.
- **iOS:** QuickJS backend passes the full M1-SCR suite; app passes TestFlight.
- **Packaging:** packed build runs from a clean machine with no dev tools installed.
