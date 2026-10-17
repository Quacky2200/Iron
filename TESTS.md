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
| M4-HOT-01 | Integration | Changing a component's `onUpdate` and reloading runs the new code on the next tick (components start over until part 2 keeps their state). |
| M4-HOT-02 | Integration | `@property` values survive a reload (e.g. `speed` set to 42 is still 42). |
| M4-HOT-03 | Integration | Entities, transforms and physics bodies are untouched by a script reload (Rust owns the world). |
| M4-HOT-04 | Integration | A reload with a syntax error (or a throwing top level) keeps the old code running and reports the error. |
| M4-HOT-05 | Integration | A renamed or removed property is dropped with a warning; new properties get their defaults. |
| M4-HOT-06 | Integration | React panel state (e.g. a counter) survives a Fast Refresh edit. |
| M4-HOT-07 | Benchmark | Reload of a ~1 MB bundle with 1,000 components: **< 100 ms**. |
| M4-HOT-08 | Manual | Edit a TS file while the game runs; the change appears without restarting. |
| M4-HOT-09 | Integration | Clearing the UI tree for a reload removes every script node (attached or not, skipping `:outro`), and the new bundle's reconciler can reuse IDs from 1. |
| M4-HOT-10 | Integration | The bundle watcher reports a change to `game.js` once, after it settles; other files in the folder are ignored. |

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
| M5-MTH-02 | JS unit | `Quaternion`: Euler degrees in Unity's order round-trip; `lookRotation`, `fromToRotation` (also opposite vectors), `slerp`, `rotateTowards`. |
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
| M5-GLB-01 | Unit | glTF loading (`tests/fixtures/assets/house.glb`, made by `make_house.py`): each primitive becomes an engine mesh (normals made up when missing), embedded images become textures with their sampler's filter, the description has nodes (names, TRS, children, primitives), roots and materials (colour, roughness to smoothness, unlit, texture); buffers and images outside the game folder are refused; mesh colliders resolve a render mesh (engine ids past f32 precision too), scaled. |
| M5-GLB-02 | JS unit | `Model.load` loads once per path; `instantiate` builds the node tree as GameObjects (one child per part when a node has several), with materials and textures from the model, mesh colliders on request (`renderMesh` shapes), even when placed before loading finished; failures reject `whenLoaded` and place nothing. |
| M5-RND-10 | Golden | Directional shadows: a soft-shadowing sun over a cube, sphere, capsule and a floating plank whose shadow lands on the sphere and the floor (shadow map fitted to the camera, normal offset and slope bias, 3 × 3 PCF). |
| M5-RB-04 | JS unit | Colliders resize when their object or a parent is scaled, and when reparented (the old shape is removed); setting the same scale rebuilds nothing. |
| M5-LIN-01 | JS unit | `LineRenderer`: a tube has rings at its radius around a Catmull-Rom smoothed path through every point; it redraws only when its points change. |
| M5-RND-11 | Golden | PBR under a procedural sky: spheres sweeping smoothness (columns) and metallic (rows) reflect the sky's cubemap at roughness-matched mips, a smooth dark floor reflects it, and the camera shows the sky behind everything (skybox clear). |
| M5-RND-12 | Golden | A planar mirror: a tinted metallic quad reflects the scene from the reflected camera (projection flipped to keep winding, a clip plane dropping what's behind the mirror: the yellow cube there never shows), sampled in screen space. |
| M5-RND-13 | Golden | A box-projected reflection probe in a room: a chrome ball reflects the red, green and blue walls, the ceiling, the open side and the gold ball (whose rougher reflection is blurrier), from the probe's six-face cubemap. |
| M5-RND-14 | Golden | Quality low and high side by side: half render scale, no MSAA and a small hard shadow map against 4x MSAA, full scale and a big soft (5 x 5) shadow map, on thin leaning bars and their shadows. |
| M5-LAY-01 | Integration | `scene.layer`: objects start on layer 0, keep the layer set, refuse ones outside 0 to 31; unknown objects read as 0. |
| M5-LAY-02 | Unit | Cameras draw only the layers in their culling mask, and a layer's things only out to its cull distance, measured along the view or (spherical) straight from the camera. |
| M5-LAY-03 | JS unit | `LayerMask` names (built-ins and the manifest's, bad entries skipped), `getMask` / `contains`; `GameObject.layer` sends and range-checks; `Camera.cullingMask` (signed masks made unsigned), `layerCullDistances` (copies) and `layerCullSpherical` go out with `render.camera`; `fieldOfView` is clamped to 1 to 179 and converts to and from `horizontalFieldOfView`. |
| M5-QUA-01 | JS unit | `QualitySettings`: levels (Low / Medium / High / Ultra) send the quality and shadow ops together; single settings change on their own (anti-aliasing rounded to 1 / 2 / 4 / 8, render scale clamped); unknown levels throw; the manifest picks the default level and overrides levels. |
| M5-MSH-02 | JS unit | `MeshRenderer` sends its mesh (primitive or script `Mesh`) and material and resends on change; `Mesh.create` sends flat arrays and `destroy` removes it; `Light` and `RenderSettings.ambientLight` ops; `shadowCastingMode` reaches the op; `Mirror` (quads along +z) and `ReflectionProbe` (size, resolution, box projection, `renderProbe`) ops; `createPrimitive` adds the mesh and the matching collider. |
| M5-RND-07 | Golden | Lit primitives: a plane, cube, sphere and capsule under a directional and a point light, an unlit emissive cube and a see-through cylinder. |
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

## M4 — Physics and audio

| ID | Kind | Checks |
|---|---|---|
| M4-PHY-01 | Integration | A dropped box lands on a plane and comes to rest at the right height. |
| M4-PHY-02 | Integration | Collision enter/exit events reach TS components. |
| M4-PHY-03 | Integration | Character controller walks up a step and stops at a wall. |
| M4-PHY-04 | Integration | Physics bodies survive a script hot reload. |
| M4-PHY-05 | Integration | Raycasts queued in a tick are answered after the step with the collider, distance and normal; misses report `-1` in the flat batch. |
| M4-PHY-06 | Integration | Bad physics ops (unknown kinds and shapes, missing bodies, zero directions, NaN) are rejected without changing the world; a triangle-mesh collider catches a ball. |
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

---

## Later milestones (outline)

- **Networking:** `@sync` fields replicate; `@rpc.server` runs only on the server; prediction and reconciliation under simulated latency and packet loss.
- **Android / Quest:** app launches, renders, survives pause/resume and rotation.
- **VR:** stereo render at the headset's refresh rate; world panel interaction with controllers.
- **iOS:** QuickJS backend passes the full M1-SCR suite; app passes TestFlight.
- **Packaging:** packed build runs from a clean machine with no dev tools installed.
