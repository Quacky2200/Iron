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
| M2-UI-OP-01 | Integration | `UiTree::apply_ops` with typed `Op`s builds the same tree as the script arrays; `Op::decode` rejects bad arity, types and opcodes (`crates/iron_ui/tests/tree.rs`). |
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

## M9.b — AOT scripts (`tools/iron_ts` spike)

Run from `tools/`: `cargo test --workspace` (the build script compiles `sim.ts` with `iron_ts` and links the object into the test crate).

| ID | Kind | Description |
|---|---|---|
| M9-TS-01 | Integration | Port of `sim.test.ts` "a baked mask keeps the runs of walls and open floor" against the native code (`tools/iron_ts_tests/tests/sim_port.rs`). |
| M9-TS-02 | Integration | Port: zombies spawn in single player. |
| M9-TS-03 | Integration | Port: a level's zombie budget grows by 3 to 5 each level (8..=10 at level 1). |
| M9-TS-04 | Integration | Port: a player moves 400 px a second with one key held. |
| M9-TS-05 | Integration | Port: a wall stops the player. |
| M9-TS-06 | Integration | Port: a press fires one pistol round and turns the aim toward the mouse. |
| M9-TS-07 | Integration | Port: holding fire with the pistol does not repeat. |
| M9-TS-08 | Integration | Port: an empty clip starts a reload that blocks shooting (and emits the `reload` event). |
| M9-TS-09 | Integration | Port: a hit takes health and scores 10; a kill scores 80 once the fade ends. |
| M9-TS-10 | Integration | Port: four zombie hits kill a player with 100 health, the score is floored at 0, game over clears the events. |
| M9-TS-11 | Integration | Port: the next level starts when the last zombie of this one dies. |
| M9-TS-12 | Integration | Port: touching an ammo powerup adds 320 rounds. |
| M9-TS-13 | Integration | Port: zombies speed up a little each level until level 3, then hold. |
| M9-TS-14 | Unit | `iron_ts` lowers a discriminated union of object types to one struct holding every variant's fields, with string-literal tags interned. |
| M9-TS-15 | Unit | Unsupported code is a compile error naming the line (`typeof`, `any`). |
| M9-TS-16 | Unit | Array-method callbacks are inlined (no closure call) and exported functions get `ts_` symbols in the object. |
| M9-TS-17 | Unit | `iron_ts_rt`: arrays grow and keep values (17a), strings concat and compare (17b), `Uint8Array.fill` clamps (17c). |
| M9-TS-18 | Manual | `cargo run --release -p iron_ts_tests --bin bench` (from `tools/`): QuickJS, bun and the native code run 10,000 steps of a busy state and print the same checksum; timings in `docs/research/iron-ts-spike.md`. |
| M9-TS-19 | Manual | `cargo run --release -p iron_ts_tests --bin sine_bench` (from `tools/`): the sine-circle curve (`bench/sine.ts`) in QuickJS with and without the `__iron_op` crossing, bun and native code; one checksum for all.
| M9-TS-20 | Unit | `iron_ts_rt` GC: unreachable objects are freed and their blocks handed out again. |
| M9-TS-21 | Unit | `iron_ts_rt` GC: host roots and what they reach survive, also in stress mode; unrooted ones go. |
| M9-TS-22 | Unit | `iron_ts_rt` GC: pointer arrays are traced and shadow-stack frame slots are roots; popping the frame frees what only it held. |
| M9-TS-23 | Integration | Collecting at every allocation (stress mode, freed memory poisoned) gives the same sim result as normal collection (`tools/iron_ts_tests/tests/gc.rs`). |
| M9-TS-24 | Integration | A long run (8,000 steps) collects on its own, hands out far more than is alive, and live bytes and objects stay small afterwards. |
| M9-TS-25 | Integration | After 300 steps and a stress collection, the host-rooted state and what it reaches are still readable (no poison). |
| M9-TS-26 | Manual | `IRON_TS_GC=stress cargo test -p iron_ts_tests --test all sim_port`, and `IRON_TS_GC=stress cargo run --release -p iron_ts_tests --bin bench 1500` / `--bin sine_bench 100`: the 13 ported tests pass and the checksums agree with QuickJS and bun with a collection at every allocation. |
| M9-TS-27 | Integration | A pointer read from a field and held as an argument while a later argument overwrites that field and allocates is not freed (`sum(h.item, replaceItem(h, 100))` = 105 as in JS; `bench/gap.ts`). Failed before the root analysis was tightened. |
| M9-TS-28 | Integration | A local reassigned while its old value waits as an argument is not freed (`sum(a, (a = fresh(100), fresh(1).v))` = 6 as in JS). Failed before the fix. |
| M9-TS-29 | Integration | A pointer waiting to be compared with `===` while the right side frees and reuses its object is not equal to the new object (`h.item === swapAndAlloc(h)` is false as in JS). Failed before comparisons rooted their operands. |
| M9-TS-30 | Integration | The zombie sim and `gap.ts` run interleaved in one heap with a collection at every allocation: their struct ids do not collide (`struct_base`). Crashed before. |
| M9-TS-31 | Unit | A collector does not mark objects of another thread's heap found through a shared global slot; the owner's next collection still traces them. Failed before headers carried a heap id. |
| M9-TS-32 | Unit | Array data counts toward the live size, so a heap with a large live array collects less often. |
| M9-TS-33 | Unit | Registering a different pointer map under a struct id already taken is refused; the same one again is fine. |
| M9-TS-34 | Integration | A closure and the function that made it share one variable (`counterDemo`, 33). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-35 | Integration | Closures from one function keep separate captured values (101011). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-36 | Integration | Two closures over one variable (`inc`, `get` fields of a counter object) see each other's writes; another counter is independent (7101). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-37 | Integration | A `for...of` variable is a new variable every iteration for closures (10409). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-38 | Integration | A `const` declared in a `while` body is new every iteration (211101). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-39 | Integration | Top-level functions work as values: passed, and as `xs.map(double)` next to an inlined arrow callback (42060608). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-40 | Integration | A closure can call itself through an annotated variable (720). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-41 | Integration | A closure over an object changes the object everyone holds (34). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-42 | Integration | A closure made in an inlined array callback captures the callback's parameter (332211). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-43 | Integration | An immediately called arrow works (42). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-44 | Integration | Closures held by the host keep their captured state, including an object, through collections and garbage churn, with a collection at every allocation. Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-45 | Integration | 20,000 short-lived arrays, cells and closures are collected. Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-46 | Integration | Closures are passed, returned and called through a call result while two fresh ones wait for each other (`compose(makeAdder(1), makeAdder(2))(10)` = 13). Runs normally, in stress GC mode and with collection off (`tests/closures.rs`). |
| M9-TS-47 | Integration | A module-level `let counter = 0` is changed by functions, an arrow function and a `forEach` callback, continues across calls and starts over after `ts_init` (19, 38, 0). Runs normally, in stress GC mode and with collection off (`tests/globals.rs`). |
| M9-TS-48 | Integration | A `let` global replaced by a call while its old value waits as an argument (`sum(current, swapCurrent(100))` = 101) is not freed. Fails if `let` globals are trusted like constants. Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-49 | Integration | A `let` global assigned inside a later argument, with an object literal and no call after it (`sum(current, (current = {v: 50}, {v: 7}.v))` = 8), keeps the old value alive. Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-50 | Integration | A global `Item | null` holds an object, then nothing (`remember`, `lastOr`, `forget`). Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-51 | Integration | A global `number | null` keeps the largest value seen (9). Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-52 | Integration | `let pending: Item | undefined;` and `let n: number | undefined;` without a value start as null and can be assigned. Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-53 | Integration | A closure stored in a global keeps what it captured (`handler = (n) => n * k + base`; 17), and the global's first value (`(n) => n`) is callable. Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-54 | Integration | A linked list reachable only from a module variable survives collections at every allocation (4950, then 4995). Runs in all three GC modes (`tests/globals.rs`). |
| M9-TS-55 | Unit | A `let` with a literal initialiser is a data slot written by the function, not folded away; a `const` literal is folded (`iron_ts/src/lib.rs`). |
| M9-TS-56 | Unit | Assigning a `const` (number or object) is an error naming the constant (`iron_ts/src/lib.rs`). |
| M9-TS-57 | Integration | `for (let i = 0; i < n; i++)` sums 0..n-1, and `continue` still runs the update clause (sum of odd numbers below 10 = 25). Runs normally, in stress GC mode and with collection off (`tests/loops.rs`). |
| M9-TS-58 | Integration | A `for` head with two declarators and a comma update, with no test, with no initialiser, or with no update (30, 8, 398). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-59 | Integration | A `do...while` body runs once when the test starts false (111), and `continue` reaches its test (1206). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-60 | Integration | `continue outer` and `break outer` reach the outer loop (15, 26). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-61 | Integration | Labels on a `while`, a `for...of` and a plain block (600100, 100100, 2001, 2, 3). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-62 | Integration | A captured `for` variable is a new binding every iteration (closures see 0, 1, 2) and a closure's change to it carries into the next iteration (12, 3). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-63 | Integration | Objects allocated in the head, body and update of a `for` survive a collection at every allocation (5050). Runs in all three GC modes (`tests/loops.rs`). |
| M9-TS-64 | Integration | A switch on an enum value picks the case, the shared `case Left: case Right:` body and the default. Runs normally, in stress GC mode and with collection off (`tests/switches.rs`). |
| M9-TS-65 | Integration | Enum members are numbered (0, 1, 5, 6), refer to earlier members (`B = A << 1`) and fold to constants; functions take and return enum values. Runs in all three GC modes (`tests/switches.rs`). |
| M9-TS-66 | Integration | A switch falls through into the next clause and a `default` in the middle runs only when nothing matches, then falls into the following case. Runs in all three GC modes (`tests/switches.rs`). |
| M9-TS-67 | Integration | A switch on a string-literal union and on a string enum compares tags. Runs in all three GC modes (`tests/switches.rs`). |
| M9-TS-68 | Integration | `continue` in a switch reaches the loop, `break` leaves only the switch, no match and no default does nothing, `break label` leaves a labelled switch from a nested loop. Runs in all three GC modes (`tests/switches.rs`). |
| M9-TS-69 | Integration | A discriminant and case expressions that allocate are evaluated once and in order and survive a collection at every allocation; case blocks keep their own constants. Runs in all three GC modes (`tests/switches.rs`). |
| M9-TS-70 | Unit | Non-constant enum members, unknown members, a string member followed by a member without a value, `continue` outside a loop and `continue` to a non-loop label are errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-71 | Integration | Object patterns rename (`{ x: ax }`), nest (`min: { x, y }`) and work as function parameters. Runs normally, in stress GC mode and with collection off (`tests/destructure.rs`). |
| M9-TS-72 | Integration | A pattern default replaces a missing (null) property and the value wins when present; the default may allocate (a call). Runs in all three GC modes (`tests/destructure.rs`). |
| M9-TS-73 | Integration | Array patterns skip holes and give a missing element its default (`[a, b, c = 5] = [1, 2]`). Runs in all three GC modes (`tests/destructure.rs`). |
| M9-TS-74 | Integration | `for (const { x, y } of items)`, `for (const [a, b] of rows)` and patterns as inlined callback parameters. Runs in all three GC modes (`tests/destructure.rs`). |
| M9-TS-75 | Integration | A closure takes a pattern parameter, and a closure captures a destructured variable (a cell: it sees later writes). Runs in all three GC modes (`tests/destructure.rs`). |
| M9-TS-76 | Integration | Patterns over freshly allocated objects and arrays inside a loop survive a collection at every allocation. Runs in all three GC modes (`tests/destructure.rs`). |
| M9-TS-77 | Integration | A missing argument takes the parameter's default, which may use earlier parameters (`hi = lo + 10`); several defaults can be left out together. Runs normally, in stress GC mode and with collection off (`tests/params.rs`). |
| M9-TS-78 | Integration | An optional parameter `y?: number` and `it?: Item` is null when left out. Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-79 | Integration | Defaults that are object literals or calls that allocate (`n = make(3).v, m = n * 2`) are evaluated per call. Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-80 | Integration | A rest parameter collects the remaining arguments, alone or after fixed and default parameters. Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-81 | Integration | Spread in a call (into a rest parameter) and in array literals (`[...a, 3, ...a]`, arrays of objects). Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-82 | Integration | `f(...a)` and `[...a]` give new arrays; writing to them leaves `a` alone. Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-83 | Integration | Defaults, rest arrays and spreads that allocate, in a loop, survive a collection at every allocation. Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-84 | Unit | Too few or too many arguments, and a spread into a fixed parameter, are errors (`iron_ts/src/lib.rs`). |
| M9-TS-85 | Integration | `Math.ceil` and `Math.trunc` agree with JS bit for bit, including `-0` (`ceil(-0.5)`, `trunc(-0.2)`). Runs normally, in stress GC mode and with collection off (`tests/mathlib.rs`). |
| M9-TS-86 | Integration | `Math.round` sends halves toward +Infinity, keeps `-0`, and `round(0.49999999999999994)` is 0. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-87 | Integration | `Math.pow` and `**` follow JS for NaN, `1 ** Infinity`, `0 ** 0`, negative bases, right associativity. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-88 | Integration | `Math.clz32`, `Math.fround` and unary `~` agree with JS. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-89 | Integration | `tan asin acos atan exp log log2 log10 cbrt` and the `LN2`.. constants agree with JS to nine decimals. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-90 | Integration | `??=`, `\|\|=`, `&&=`, `**=` assign only when needed, and an indexed target with a side effect is evaluated once. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-91 | Integration | `Math.random()` stays in [0, 1) over 10,000 draws and repeats after the same seed. Runs in all three GC modes (`tests/mathlib.rs`). |
| M9-TS-92 | Integration | `pop` and `shift` return the element, or null on an empty array. Runs normally, in stress GC mode and with collection off (`tests/arraylib.rs`). |
| M9-TS-93 | Integration | `unshift` puts values in front and returns the new length. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-94 | Integration | `slice` with negative and out-of-range indexes, and `slice()` as a copy. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-95 | Integration | `splice` removes, inserts and replaces, with negative start and no count. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-96 | Integration | `concat` joins arrays and single values into a new array. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-97 | Integration | `reverse` is in place and returns the same array. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-98 | Integration | `sort` with a comparator orders numbers ascending and descending. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-99 | Integration | `sort` is stable for objects, and the comparator may allocate (found a merge buffer that was traced before it was initialised). Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-100 | Integration | 200 pseudo-random items sort into a stable order. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-101 | Integration | Sorting with a collection at every allocation, including a comparator that captures variables, loses nothing (`tests/arraylib.rs`). |
| M9-TS-102 | Integration | `fill` over a range and `new Array(5).fill(2)`. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-103 | Integration | `indexOf`, `lastIndexOf`, `findIndex` and `includes`, found and not found. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-104 | Integration | `at` counts negative indexes from the end. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-105 | Integration | `reduce` over numbers and objects, with index and initial value. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-106 | Integration | `xs.length = n` truncates. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-107 | Integration | `Array.from({ length }, fn)`, `Array.from(xs)` (a copy), `Array.of`, `Array.isArray`. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-108 | Integration | Array operations on arrays of objects in a loop (sort, splice, slice, concat, reverse, unshift, pop, reduce) survive a collection at every allocation. Runs in all three GC modes (`tests/arraylib.rs`). |
| M9-TS-109 | Unit | `sort` keeps its merge buffer alive while the comparator collects: with the roots removed the elements come back poisoned (`iron_ts_rt/src/lib.rs`). |
| M9-TS-110 | Unit | `sort()` without a comparator, `reduce` without an initial value and `pop` over booleans are errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-111 | Integration | A class with a constructor, methods, a getter and statics (`new Vec2(3, 4).add(Vec2.of(1, 2)).scale(2)`, `length`, a static counter, `static readonly zero`) computes like JS. Runs normally, in stress GC mode and with collection off (`tests/classes.rs`). |
| M9-TS-112 | Integration | Constructor parameters take their defaults (`new Vec2()`, `new Vec2(7)`). Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-113 | Integration | Private fields and methods (`#count`, `#bump`), a getter/setter pair, chaining by returning `this`, `this` captured by an arrow function and used in an inlined callback, an optional field starting undefined. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-114 | Integration | Objects linked through class fields (a stack of cells) survive collections. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-115 | Integration | Constructors run parameter properties, then field initialisers in order (`b = this.a + 1`), then the body; a bare `return` returns the object. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-116 | Integration | A method receiver waiting for its arguments is not freed: `h.item.plus(swap(h))` gets the old `this` (5 + 1). Fails when field reads are trusted as stable. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-117 | Integration | Two names for one object, and `===` as identity. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-118 | Integration | Methods, constructors and a stack in a loop survive a collection at every allocation. Runs in all three GC modes (`tests/classes.rs`). |
| M9-TS-119 | Unit | `extends`, decorators, a method used as a value, a field without a type and `this` outside a method are errors that say what is missing (`iron_ts/src/lib.rs`). |
| M9-TS-120 | Integration | A call on a base-class variable runs the subclass's method (areas of a Rect, Square, Circle and Tri held as `Shape`). Runs normally, in stress GC mode and with collection off (`tests/inherit.rs`). |
| M9-TS-121 | Integration | `this` dispatches on the real class inside a base-class method (`describe()` calls `this.area()`). Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-122 | Integration | Getters can be overridden and are dispatched too. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-123 | Integration | `super.area()` calls the base implementation; a base-typed variable still gets the override. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-124 | Integration | `instanceof` is true for the class and its subclasses and false for null. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-125 | Integration | A member only a subclass has is usable after `instanceof` narrowing or `as`. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-126 | Integration | Subclass objects flow through variables, arrays and conditionals as their base class. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-127 | Integration | A subclass constructor assigns after `super(...)` returns. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-128 | Integration | A field a subclass declares again is the same slot and its initialiser wins. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-129 | Integration | A base constructor calling an overridden method reaches the subclass's version. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-130 | Integration | A derived class without a constructor forwards its parameters, then runs its initialisers. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-131 | Integration | An override calls `super` through a recursive structure. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-132 | Integration | A static counter in the base class counts every subclass construction. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-133 | Integration | Virtual calls, `instanceof` and subclass arrays under allocation pressure survive a collection at every allocation (`tests/inherit.rs`). |
| M9-TS-134 | Unit | Instantiating an abstract class, a subclass declared before its base, a field that changes type in a subclass, and `instanceof` on a non-class are errors that say so (`iron_ts/src/lib.rs`). |
| M9-TS-135 | Integration | A virtual call's receiver waiting for its arguments is not freed (`h.s.scaled(swapShape(h))` = 3). Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-136 | Integration | A call that returns nothing and needs a default filled in still runs (it was skipped when its value was void). Runs in all three GC modes (`tests/params.rs`). |
| M9-TS-137 | Integration | Methods as values (`g.twice`, `xs.map(this.twice)`) and an arrow-function field that captures `this`. Runs normally, in stress GC mode and with collection off (`tests/classes.rs`). |
| M9-TS-138 | Integration | A method taken as a value from a base-typed variable still dispatches on the real class. Runs in all three GC modes (`tests/inherit.rs`). |
| M9-TS-139 | Integration | The `@property` table lists only the marked fields, in declaration order with the base class's first, with byte offsets and type names (`PROPS_Player`, `PROPS_Hero`). Not GC-dependent (`tests/props.rs`). |
| M9-TS-140 | Integration | A host writes properties through the table (a number, a boolean, a subclass's own through its table and a base property through the base table) and the compiled code sees the new values; run with a collection at every allocation (`tests/props.rs`). |
| M9-TS-142 | Integration | A script calls engine functions through `#[iron_api]` shims with the engine's number types (f64, i32 clamp, u32 and bool); fractional numbers truncate toward zero at an i32 parameter. Runs normally, in stress GC mode and with collection off (`tests/engine.rs`). |
| M9-TS-143 | Integration | f32 arguments narrow and f32 results widen at the call (`Demo.half(0.1)` equals `(0.1f32 * 0.5f32) as f64`). Runs in all three GC modes (`tests/engine.rs`). |
| M9-TS-144 | Integration | Handles are plain ids: `===` compares them, a destroyed entity's id misses even when its slot is reused, and an engine function can answer for a stale id. Runs in all three GC modes (`tests/engine.rs`). |
| M9-TS-145 | Integration | A handle lives in objects, arrays, nullable variables and closure cells without being traced, between allocations that collect at every step (`handle !== 0` is the null test). Runs in all three GC modes (`tests/engine.rs`). |
| M9-TS-146 | Unit | Calls of the engine API are checked against the manifest: an unknown function, a wrong argument count and a malformed manifest are errors; each symbol is declared once (`iron_ts/src/lib.rs`). |
| M9-TS-147 | Integration | A value class is an object in the script, passed to the engine as scalars and coming back as a new object; its fields are f32-rounded. Runs normally, in stress GC mode and with collection off (`tests/engine.rs`). |
| M9-TS-148 | Integration | A value stored in the engine next to a handle reads back (`tests/engine.rs`). |
| M9-TS-149 | Integration | Dozens of short-lived vectors survive a collection at every allocation (`tests/engine.rs`). |
| M9-TS-150 | Unit | Value classes of the manifest are checked: flattened parameters, an out pointer for a result, wrong constructor arity, a number where a value is expected (`iron_ts/src/lib.rs`). |
| M9-TS-151 | Integration | Compiled code builds, parents, moves and destroys a hierarchy in the real `iron_scene::Scene`. Runs normally, in stress GC mode and with collection off (`tests/scene.rs`). |
| M9-TS-152 | Integration | A per-frame update loop moves an object round a circle through the `Scene` API (`tests/scene.rs`). |
| M9-TS-153 | Integration | Stale object handles miss even when the slot is reused (`tests/scene.rs`). |
| M9-TS-154 | Integration | A `Quaternion` value goes in and comes back normalised (`tests/scene.rs`). |
| M9-TS-155 | Integration | Thirty objects and their handles in an array survive a collection at every allocation (`tests/scene.rs`). |
| M9-TS-156 | Integration | A hierarchy built and moved with handle classes (`child.transform.setParent(..)`, `transform.localPosition = new Vector3(..)`, `go.destroy()`) in the real scene (`tests/scene.rs`). |
| M9-TS-157 | Integration | Boolean, number and handle properties and a method of `GameObject`, and the `transform.gameObject` round trip (`tests/scene.rs`). |
| M9-TS-158 | Integration | An update loop moves an object through `transform.position` (`tests/scene.rs`). |
| M9-TS-159 | Integration | `Vector3` methods and getters (`add`, `scale`, `dot`, `cross`, `lerp`, `distance`, `magnitude`, `normalized`) compile to direct calls, chained on call results (`tests/scene.rs`). |
| M9-TS-160 | Integration | `Quaternion` `rotate`, `mul` and `inverse` (`tests/scene.rs`). |
| M9-TS-161 | Integration | A vector kept across the allocations of a loop of method calls survives collection at every allocation (`tests/scene.rs`). |
| M9-TS-162 | Unit | A string's `length` counts UTF-16 units (accents, CJK, a pair is 2, a lone surrogate 1) (`iron_ts_rt/src/string.rs`). |
| M9-TS-163 | Unit | Both units of a surrogate pair are reachable by index; `codePointAt` on the low half is the lone surrogate (`string.rs`). |
| M9-TS-164 | Unit | Slicing through a pair leaves lone surrogates (WTF-8) that rejoin into the original on concatenation (`string.rs`). |
| M9-TS-165 | Unit | Ordering is by UTF-16 code unit, not code point (an emoji sorts before U+FF21) (`string.rs`). |
| M9-TS-166 | Unit | Search converts between byte offsets and unit indices; a lone low surrogate is found inside a pair (`string.rs`). |
| M9-TS-167 | Unit | A forward and a backward scan of a long non-ASCII string through the cursor are correct (`string.rs`). |
| M9-TS-168 | Differential | `length`, `charCodeAt`, `codePointAt`, `charAt` at every index of a 24-string corpus (ASCII, accents, CJK, astral, lone surrogates) equal what bun answers (`iron_ts_rt/tests/strings.rs`, expected data from `tests/gen_strings.ts`). |
| M9-TS-169 | Differential | `slice` and `substring` for every index pair equal bun's (`tests/strings.rs`). |
| M9-TS-170 | Differential | Equality, `<` ordering and concatenation (rejoining split pairs) over every pair of strings equal bun's (`tests/strings.rs`). |
| M9-TS-171 | Differential | `indexOf`, `lastIndexOf`, `includes`, `startsWith`, `endsWith` with every position over every pair equal bun's (`tests/strings.rs`). |
| M9-TS-172 | Unit | A string is one collectable object: an unrooted one is freed, a rooted one survives (`string.rs`). |
| M9-TS-173 | Differential | `trim`, `trimStart`, `trimEnd`, `toUpperCase`, `toLowerCase` (special casing, final sigma) over a corpus with Unicode white space, accents and lone surrogates equal bun's (`iron_ts_rt/tests/strings_more.rs`). |
| M9-TS-174 | Differential | `at`, `repeat`, `padStart`, `padEnd` (fillers cut inside a pair) equal bun's (`tests/strings_more.rs`). |
| M9-TS-175 | Differential | `split` (empty, one-unit, multi-unit and lone-surrogate separators) and `join` (rejoining split pairs) equal bun's (`tests/strings_more.rs`). |
| M9-TS-176 | Differential | `String(x)`, `toFixed(0..6)` (ties round up on the exact value) and `toString(radix)` equal bun's (`tests/strings_more.rs`). |
| M9-TS-177 | Differential | `Number(s)`, `parseFloat(s)` and `parseInt(s, radix)` equal bun's over hex/binary/octal, signs, exponents, white space, trailing text and bad radixes (`tests/strings_more.rs`). |
| M9-TS-178 | Integration | Compiled string code (`length`, comparison by code unit, search, `switch`, truthiness, parsing, a loop over `charCodeAt`) returns what bun returns, in the normal, stress and collection-off modes (`iron_ts_tests/tests/strings.rs`, `bench/strings.ts`, expected data from `bench/gen_strings_expected.ts`). |
| M9-TS-179 | Integration | `+`, `+=` with numbers/booleans/null, template literals, method chains, padding, `fromCharCode`, `toFixed`, `toString(16)`, `split`/`join` and nested temporaries equal bun's (`tests/strings.rs`). |
| M9-TS-180 | Integration | Strings in arrays (`includes`, `indexOf`, `sort` with a comparator), object fields, `string \| null` and string-literal unions mixed with `string` equal bun's (`tests/strings.rs`). |
| M9-TS-181 | Integration | Three hundred strings made in a loop survive a collection at every allocation (`tests/strings.rs`). |
| M9-TS-182 | Unit | Unsupported string methods (`localeCompare`, `normalize`, `replace`) and a literal with a lone surrogate are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-183 | Integration | `GameObject.name` set and read, and `GameObject.find(name)`, through the real scene: strings in as script strings and out as new ones (`iron_ts_tests/tests/scene.rs`). |
| M9-TS-184 | Integration | A name with accents and an emoji round-trips; a lone surrogate reaches the engine as U+FFFD (`tests/scene.rs`). |
| M9-TS-185 | Integration | Thirty names made, read back and found survive a collection at every allocation (`tests/scene.rs`). |
| M9-TS-186 | Integration | `iron_api::ScriptStr` has the layout of the runtime's `Str` (`tests/scene.rs`). |
| M9-TS-187 | Integration | `throw` in a callee and `catch` in the caller: message, name, `toString`, a subclass with its own field, `instanceof` to tell the classes apart, equal to bun (`iron_ts_tests/tests/errors.rs`, `bench/errors.ts`, expected data from `bench/gen_errors_expected.ts`). |
| M9-TS-188 | Integration | An exception unwinds through six frames holding strings, is rethrown wrapped, and leaves an inlined array callback and a closure (`tests/errors.rs`). |
| M9-TS-189 | Integration | `continue` and `break` inside `catch`, and two hundred throws and catches with a collection at every allocation (`tests/errors.rs`). |
| M9-TS-190 | Integration | An uncaught exception leaves the exported function with the exception pending; the host takes the error object (`tests/errors.rs`). |
| M9-TS-191 | Unit | Only functions that can throw are checked after a call; callers of non-throwing code pay nothing (`iron_ts/src/lib.rs`). |
| M9-TS-192 | Unit | `throw` of a non-Error is a compile error with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-193 | Integration | `finally` runs once on a normal exit, after `catch`, and on `return`, in nested order, and while an exception propagates (`tests/errors.rs`). |
| M9-TS-194 | Integration | `break`, `continue`, `continue label` and `return` pass through `finally`; a `return` in `finally` replaces the one in `try` (`tests/errors.rs`). |
| M9-TS-195 | Integration | Runtime errors (`repeat(-1)`, `toFixed(101)`, `toString(37)`, `fromCodePoint(-1)`) are catchable `RangeError`s with bun's messages (`tests/errors.rs`). |
| M9-TS-196 | Integration | An uncaught runtime error reaches the host as a pending `RangeError` (`tests/errors.rs`). |
| M9-TS-197 | Unit | A throwing function nothing reaches does not make closure calls checked (`iron_ts/src/lib.rs`). |
| M9-TS-198 | Integration | A `break` inside an array callback inlined into a `try ... finally` stays inside the callback (`iron_ts_tests/tests/errors.rs`). |
| M9-TS-199 | Integration | `typeof` of numbers, strings, booleans, functions, objects, arrays, a missing value and an undeclared name, and `typeof x === "..."` comparisons equal bun's (`iron_ts_tests/tests/stdlib.rs`, `bench/stdlib.ts`, expected data from `bench/gen_stdlib_expected.ts`). |
| M9-TS-200 | Integration | `Number.isInteger/isNaN/isFinite/isSafeInteger`, global `isNaN`/`isFinite`, the number constants, `Number.parseFloat/parseInt` equal bun's (`tests/stdlib.rs`). |
| M9-TS-201 | Integration | `Math.sinh cosh tanh asinh acosh atanh expm1 log1p` equal bun's to ten decimals (`tests/stdlib.rs`). |
| M9-TS-202 | Integration | `toSorted`, `toReversed`, `findLast`, `findLastIndex`, `reduceRight`, `flat`, `flatMap`, the default `sort()` of numbers and strings, and `entries()`/`keys()`/`values()` in `for...of` equal bun's (`tests/stdlib.rs`). |
| M9-TS-203 | Integration | `console.log/warn/error` join their arguments with spaces and reach the host's console hook with the level (`tests/stdlib.rs`). |
| M9-TS-204 | Integration | `Object.keys/values/entries/assign/freeze/hasOwn`, `in`, object spread (new fields, source order) and rest, and array rest equal bun's (`tests/stdlib.rs`). |
| M9-TS-205 | Unit | `Object.entries` outside `for...of`, `Object.values` over mixed types, `in` with a computed key and `console.log` of an object are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-206 | Unit | `Map` numbers follow SameValueZero (`NaN` is a key, `-0` is `0`) and keep insertion order (`iron_ts_rt/src/map.rs`). |
| M9-TS-207 | Unit | A map iterator (sequence-number cursor) survives deletes, adds and table rebuilds (`map.rs`). |
| M9-TS-208 | Unit | A map is one collectable object that keeps its keys and values alive; string keys compare by content (`map.rs`). |
| M9-TS-209 | Integration | `Map<string, number>`: `set` chained, `get`, `has`, `delete`, `size`, `clear`, `keys()`/`values()`, `for...of`, `forEach`, equal to bun (`iron_ts_tests/tests/collections.rs`, `bench/collections.ts`, expected data from `bench/gen_collections_expected.ts`). |
| M9-TS-210 | Integration | Number keys (SameValueZero) and object keys (identity) equal bun's (`tests/collections.rs`). |
| M9-TS-211 | Integration | `Set`: from an array, `add`, `has`, `delete`, spread, `Array.from`, `for...of`, `forEach`, element type inferred, equal to bun's (`tests/collections.rs`). |
| M9-TS-212 | Integration | Iteration is live: deleted-before-reached entries are skipped, added entries are visited, the table rebuilds under the loop (`tests/collections.rs`). |
| M9-TS-213 | Integration | Two thousand string keys with deletes and adds under a collection at every allocation, and `WeakMap`/`WeakSet` (`tests/collections.rs`). |
| M9-TS-214 | Unit | Collections without a type, `entries()` outside `for...of`, `Map.get` over booleans and a `Map` as a key are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-215 | Integration | `JSON.stringify` of objects, arrays, escapes, lone surrogates, `NaN`/`-0`/exponents, booleans, empty collections, with and without indentation equals bun's (`iron_ts_tests/tests/json.rs`, `bench/json.ts`, expected data from `bench/gen_json_expected.ts`). |
| M9-TS-216 | Integration | Optional fields are left out when absent, `T \| null` prints `null`, a subclass instance prints its own fields (`tests/json.rs`). |
| M9-TS-217 | Integration | `JSON.parse` into typed objects (unknown fields skipped, surrogate pairs from escapes, optional fields), arrays, nullable numbers and literal unions, and back through `stringify`, equals bun's (`tests/json.rs`). |
| M9-TS-218 | Integration | Sixty-seven malformed texts raise a `SyntaxError` with bun's text (`tests/json.rs`). |
| M9-TS-219 | Integration | Three hundred objects stringified, parsed and stringified again under a collection at every allocation (`tests/json.rs`). |
| M9-TS-220 | Integration | A shape mismatch is a `TypeError`, and a syntax error anywhere wins over it (`tests/json.rs`). |
| M9-TS-221 | Unit | `JSON.parse` without a type, with a reviver, `stringify` with a replacer and of a function are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-222 | Integration | `exec` and `test`: the match object (`m[0]`, groups, `index`, `length`, `input`), a miss, `source`, `flags`, `global` equal bun's (`iron_ts_tests/tests/regexp.rs`, `bench/regexp.ts`, expected data from `bench/gen_regexp_expected.ts`). |
| M9-TS-223 | Integration | A `g` regex steps through a string with `lastIndex`; a miss resets it (`tests/regexp.rs`). |
| M9-TS-224 | Integration | Named groups (`m.groups.year`), a missing name, no `groups` without named groups (`tests/regexp.rs`). |
| M9-TS-225 | Integration | The flags `m s i y` change matching as in JavaScript (`tests/regexp.rs`). |
| M9-TS-226 | Integration | Unicode: `u` matches a surrogate pair as one character, indices count UTF-16 units, property escapes and code point escapes (`tests/regexp.rs`). |
| M9-TS-227 | Integration | Lookbehind, backreferences, alternation, lazy quantifiers, negated classes, word boundaries (`tests/regexp.rs`). |
| M9-TS-228 | Integration | `new RegExp(str, flags)`, `RegExp(..)`, copying with new flags, canonical flag order, `source` escaping `/`, `String(re)` (`tests/regexp.rs`). |
| M9-TS-229 | Integration | Bad patterns and flags are `SyntaxError`s with JavaScriptCore's text (`tests/regexp.rs`). |
| M9-TS-230 | Integration | A match object works as an array: destructuring, spread, `for...of`, `map` (`tests/regexp.rs`). |
| M9-TS-231 | Unit | An invalid regex literal is a compile error with the engine's message (`iron_ts/src/lib.rs`). |
| M9-TS-232 | Integration | `search` with a regex or a string (UTF-16 index) equals bun's (`iron_ts_tests/tests/regexp2.rs`, `bench/regexp2.ts`, expected data from `bench/gen_regexp2_expected.ts`). |
| M9-TS-233 | Integration | `match`: a plain regex gives the match object, a `g` regex the texts of all matches (`lastIndex` reset), a string pattern is a regex (`tests/regexp2.rs`). |
| M9-TS-234 | Integration | `matchAll` steps over every match, past empty matches; a non-global regex is bun's `TypeError` (`tests/regexp2.rs`). |
| M9-TS-235 | Integration | `replace` / `replaceAll` with string patterns (matched literally) and `$` patterns; `replaceAll` with a non-global regex is bun's `TypeError` (`tests/regexp2.rs`). |
| M9-TS-236 | Integration | `replace` with a regex: `$1 $01 $10 $& $` $' $<name> $$`, global and first-only, empty matches, non-ASCII text (`tests/regexp2.rs`). |
| M9-TS-237 | Integration | `replace` with a function: match, groups, offset, string and the named-groups object; a `const` regex keeps its `lastIndex` (`tests/regexp2.rs`). |
| M9-TS-238 | Integration | `split` with a regex (captures included, limit, empty input, empty pattern, `u` flag) and a string with a limit (`tests/regexp2.rs`). |
| M9-TS-239 | Unit | A replacement function with more than the match as a parameter needs a regex whose groups are known; too many parameters is a compile error (`iron_ts/src/lib.rs`). |
| M9-TS-240 | Integration | Local and UTC fields of nine instants (epoch, DST, year -1, the extremes) equal bun's in five time zones (`iron_ts_tests/tests/date.rs`, `bench/date.ts`, expected data from `bench/gen_date_expected.ts`). |
| M9-TS-241 | Integration | `Date.UTC` with overflowing and fractional parts, two-digit years and the range edges (`tests/date.rs`). |
| M9-TS-242 | Integration | `new Date(y, m, ...)` in local time, including a clock set forward or back and years far from now (`tests/date.rs`). |
| M9-TS-243 | Integration | `new Date(ms)`, `(string)`, `(date)`, NaN and out-of-range values (`tests/date.rs`). |
| M9-TS-244 | Integration | Every `set*` method in local time and UTC: overflow, optional extra arguments, an invalid date, `setTime` (`tests/date.rs`). |
| M9-TS-245 | Integration | `toISOString` (expanded years) and `toJSON`, and the `RangeError` text for an invalid date (`tests/date.rs`). |
| M9-TS-246 | Integration | `toUTCString`, `toGMTString` and `toDateString` including negative and five-digit years and an invalid date (`tests/date.rs`). |
| M9-TS-247 | Integration | `Date.parse` of 34 ISO-form strings: date only is UTC, date-time is local, offsets, `24:00`, expanded years, bad input (`tests/date.rs`). |
| M9-TS-248 | Integration | `Date.parse` of 42 other forms: month names, slashes, AM/PM, zone names and offsets, two-digit years, bad input (`tests/date.rs`). |
| M9-TS-249 | Integration | A date in arithmetic and comparison: `b - a`, `+a`, `Number(a)`, `<`, `===` is identity (`tests/date.rs`). |
| M9-TS-250 | Integration | `getTimezoneOffset` over DST changes, old local mean time and the range edges (`tests/date.rs`). |
| M9-TS-251 | Integration | `Date.now()` and `new Date()` give the current time (`tests/date.rs`). |
| M9-TS-252 | Integration | Time clipping at +-8.64e15 and truncation of fractions (`tests/date.rs`). |
| M9-TS-253 | Unit | `toLocaleString`, `toLocaleDateString` and `toLocaleTimeString` are compile errors that point to Intl (`iron_ts/src/lib.rs`). |
| M9-TS-254 | Integration | `localeCompare` with default and explicit locales (de, sv, tr, a locale list) equals bun's (`iron_ts_tests/tests/intl.rs`, `bench/intl.ts`, expected data from `bench/gen_intl_expected.ts`). |
| M9-TS-255 | Integration | `localeCompare` options: `sensitivity`, `numeric`, `caseFirst`, `ignorePunctuation`, `usage` (`tests/intl.rs`). |
| M9-TS-256 | Integration | An invalid language tag or option value is a `RangeError` with JavaScriptCore's text (`tests/intl.rs`). |
| M9-TS-257 | Integration | `normalize` NFC, NFD, NFKC, NFKD, Hangul, and a lone surrogate left alone (`tests/intl.rs`). |
| M9-TS-258 | Integration | A bad normalization form is bun's `RangeError` (`tests/intl.rs`). |
| M9-TS-259 | Integration | `toLocaleUpperCase` / `toLocaleLowerCase` with tr, az, lt, el, de, special casing (`tests/intl.rs`). |
| M9-TS-260 | Integration | `array.sort(collator.compare)` for en, de numeric, sv, ja, zh and case/sensitivity options gives bun's order (`tests/intl.rs`). |
| M9-TS-261 | Integration | `new Intl.Collator().compare(a, b)` results (`tests/intl.rs`). |
| M9-TS-262 | Integration | `new Intl.Collator` with a bad tag or option is bun's `RangeError` (`tests/intl.rs`). |
| M9-TS-263 | Unit | Intl options that are not literals, and `resolvedOptions`, are compile errors (`iron_ts/src/lib.rs`). |
| M9-TS-264 | Integration | Decimal locales: `toLocaleString` for 15 locales (grouping, decimal marks, Indian grouping, Arabic digits, minus signs, NaN and infinity) equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-265 | Integration | Fraction digits: `minimumFractionDigits`, `maximumFractionDigits`, `minimumIntegerDigits` and their defaults equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-266 | Integration | Significant digits: `minimumSignificantDigits` / `maximumSignificantDigits` equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-267 | Integration | Percent and sign display: `style: percent` in three locales, `signDisplay` (auto, never, always, exceptZero, negative), NaN and infinity equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-268 | Integration | Currency: `style: currency` in ten locale/currency pairs (symbols, placement, currency digits, rounding) equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-269 | Integration | Currency display: `currencyDisplay` code/name/narrowSymbol, `currencySign: accounting`, lower-case codes, fraction overrides equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-270 | Integration | Compact: `notation: compact` short and long in en, de, ja, fr, en-IN, with carry into the next unit equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-271 | Integration | Scientific: `notation: scientific` and `engineering` equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-272 | Integration | Grouping: `useGrouping` false/true/min2 and locales that group from five digits equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-273 | Integration | Rounding: all nine `roundingMode`s and `trailingZeroDisplay` equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-274 | Integration | Format object: `new Intl.NumberFormat(..).format`, also as a detached function and as a `map` callback equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-275 | Integration | Errors: bad `style`, `currency`, `currencyDisplay`, `notation`, `signDisplay`, digit ranges and tags are bun's `RangeError`/`TypeError` text equals bun's (`iron_ts_tests/tests/numfmt.rs`, `bench/numfmt.ts`). |
| M9-TS-276 | Integration | `toString`, `toTimeString`, `String(date)`, template strings and `date + x` give bun's text, including the zone name ("Eastern Daylight Time"), old dates, the range edges and an invalid date, in five zones (`tests/date.rs`). |
| M9-TS-277 | Integration | Date and time with a locale and no options: `toLocaleString`, `toLocaleDateString`, `toLocaleTimeString` for 15 locales equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-278 | Integration | `dateStyle` full, long, medium, short: in 15 locales (all but the columns listed in `docs/differences.md`) equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-279 | Integration | `timeStyle`: full, long, medium, short in 15 locales with zone names equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-280 | Integration | `dateStyle` with `timeStyle`: five locales equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-281 | Integration | Component options: year, month, day, weekday, hour, minute, second, `hour12`, `hourCycle`, `fractionalSecondDigits` in en-US, en-GB, de, fr, ja equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-282 | Integration | `timeZoneName`: short, long, shortOffset, longOffset, shortGeneric, longGeneric equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-283 | Integration | `timeZone` option: UTC, Tokyo, Los Angeles, Lord Howe, Kiritimati equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-284 | Integration | `Intl.DateTimeFormat.format`: also detached and as a `map` callback equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-285 | Integration | An invalid date: `toLocale*String` gives `Invalid Date`, `format` throws bun's `RangeError` equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-286 | Integration | Errors: bad option values, `dateStyle` with other options, a style the entry point does not allow, bad zone, bad tag: bun's text equal bun's in five time zones (`iron_ts_tests/tests/datefmt.rs`, `bench/datefmt.ts`, made by `bench/gen_datefmt_program.py`). |
| M9-TS-287 | Integration | `encodeURIComponent` / `encodeURI` of ASCII, reserved characters, accents, euro, emoji and control characters equal bun's (`iron_ts_tests/tests/uri.rs`, `bench/uri.ts`). |
| M9-TS-288 | Integration | `decodeURIComponent` / `decodeURI` (which keeps escapes of reserved characters, upper and lower case hex) equal bun's (`tests/uri.rs`). |
| M9-TS-289 | Integration | Encode then decode returns the text, for several scripts and emoji (`tests/uri.rs`). |
| M9-TS-290 | Integration | A lone surrogate, a bad or truncated escape, an overlong, surrogate or too-large UTF-8 sequence are `URIError`s with JavaScriptCore's text and class (`tests/uri.rs`). |
| M9-TS-291 | Integration | `escape` / `unescape` equal bun's (`tests/uri.rs`). |
| M9-TS-292 | Integration | `new Error(message, { cause })` chains, `cause` absent is null (`iron_ts_tests/tests/errors2.rs`, `bench/errors2.ts`). |
| M9-TS-293 | Integration | `cause` set in a `catch`, `instanceof` on it (`tests/errors2.rs`). |
| M9-TS-294 | Integration | `AggregateError`: `errors`, `name`, message, `cause`, `instanceof` equal bun's (`tests/errors2.rs`). |
| M9-TS-295 | Integration | A thrown `AggregateError` is caught and read (`tests/errors2.rs`). |
| M9-TS-296 | Integration | `Intl.PluralRules` cardinal categories for 28 numbers in 20 locales (zero, one, two, few, many, other) equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-297 | Integration | `Intl.PluralRules` ordinal categories in 8 locales equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-298 | Integration | `PluralRules` with `minimumFractionDigits` / `maximumFractionDigits`, NaN and infinity equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-299 | Integration | `PluralRules` bad type and bad tag give bun's `RangeError` text equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-300 | Integration | `Intl.ListFormat` conjunction, long/short/narrow, 17 locales, lists of 0 to 4 equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-301 | Integration | `Intl.ListFormat` disjunction equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-302 | Integration | `Intl.ListFormat` unit equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-303 | Integration | `ListFormat` default options equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-304 | Integration | `ListFormat` bad style, type and tag equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-305 | Integration | `Intl.RelativeTimeFormat` numeric always, long: 17 locales, 8 units, 9 values equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-306 | Integration | `RelativeTimeFormat` numeric always, narrow equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-307 | Integration | `RelativeTimeFormat` numeric always, short equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-308 | Integration | `RelativeTimeFormat` numeric auto (yesterday, tomorrow, this year), long equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-309 | Integration | `RelativeTimeFormat` numeric auto, narrow equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-310 | Integration | `RelativeTimeFormat` numeric auto, short equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-311 | Integration | `RelativeTimeFormat` accepts plural unit names and `-0` equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-312 | Integration | `RelativeTimeFormat` bad unit, NaN, infinity, bad options and tag equal bun's (`iron_ts_tests/tests/intl4.rs`, `bench/intl4.ts`, made by `bench/gen_intl4_program.py`). |
| M9-TS-313..317 | Integration | `Int8Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-318..322 | Integration | `Uint8Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-323..327 | Integration | `Uint8ClampedArray`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-328..332 | Integration | `Int16Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-333..337 | Integration | `Uint16Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-338..342 | Integration | `Int32Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-343..347 | Integration | `Uint32Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-348..352 | Integration | `Float32Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-353..357 | Integration | `Float64Array`: stores of 28 awkward numbers (wrapping, clamping, rounding, NaN, infinities), constructors (length, `number[]`, other typed arrays, `from`), `forEach/map/filter/slice/reduce/indexOf/includes/find/some/every/at/join`, `fill/set/sort/reverse/toSorted/toReversed`, `for...of`, spread and out-of-range writes equal bun's (`iron_ts_tests/tests/typed.rs`, `bench/typed.ts`, made by `bench/gen_typed_program.py`). |
| M9-TS-358 | Integration | `set` out of range or with a negative offset is bun's `RangeError` text (`tests/typed.rs`). |
| M9-TS-359 | Integration | Sorting floats with NaN and infinities, and `Float32Array` rounding (`tests/typed.rs`). |
| M9-TS-360 | Integration | Literals in decimal, hex, binary, octal and with separators, 30 digits and more equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-361 | Integration | Arithmetic: `+ - * / % **` with truncating division, negative operands and large powers equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-362 | Integration | Bitwise `& | ^ ~ << >>` over positive, negative and huge operands (all pairs) equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-363 | Integration | Comparison with bigints and numbers (`< <= > >= == != === !==`, NaN, infinities, fractions, 2^64) equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-364 | Integration | Conversions: `BigInt(number | string | boolean | bigint)`, `Number(big)`, `toString(radix)`, `String`, template, `typeof` equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-365 | Integration | Truthiness of `0n` equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-366 | Integration | `BigInt.asIntN` / `asUintN` at several widths equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-367 | Integration | `++`, `--`, `*=`, `for` and `while` loops, factorial of 30, Fibonacci of 150, arrays of bigints equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-368 | Integration | `RangeError` / `SyntaxError` texts: non-integer, bad string, division by zero, negative exponent, radix, too big, negative bits equal bun's (`iron_ts_tests/tests/bigint.rs`, `bench/bigint.ts`). |
| M9-TS-369 | Unit | Mixing a bigint with a number in arithmetic, unary `+`, and `>>>` are compile errors; comparing with a number is allowed (`iron_ts/src/lib.rs`). |
| M9-TS-370 | Integration | `a || b` and `a && b` on numbers give the operand, not a boolean equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-371 | Integration | The same on strings equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-372 | Integration | The same with a nullable left side (`x || 5`) and with objects equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-373 | Integration | Chains (`a || b || c`, `a && b && c`, mixed) equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-374 | Integration | In conditions (`if`, `while`, `?:`, `!`) they stay tests, including `obj && obj.v > 3` and `!(n || false)` equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-375 | Integration | Defaults (`name || "anonymous"`, an empty string is falsy; a nullable string tests for empty) equal bun's (`iron_ts_tests/tests/logic.rs`, `bench/logic.ts`). |
| M9-TS-376 | Integration | `for...of` over classes with `[Symbol.iterator]()`: a separate iterator class, an iterator that returns itself, string values, nested loops equal bun's (`iron_ts_tests/tests/iterators.rs`, `bench/iterators.ts`). |
| M9-TS-377 | Integration | `break` and `continue`, nested iterables equal bun's (`iron_ts_tests/tests/iterators.rs`, `bench/iterators.ts`). |
| M9-TS-378 | Integration | Spread `[...iterable]` and `Array.from(iterable)` equal bun's (`iron_ts_tests/tests/iterators.rs`, `bench/iterators.ts`). |
| M9-TS-379 | Integration | Calling `obj[Symbol.iterator]()` and `next()` by hand equal bun's (`iron_ts_tests/tests/iterators.rs`, `bench/iterators.ts`). |
| M9-TS-380 | Integration | An exhausted iterator that returns itself yields nothing the second time equal bun's (`iron_ts_tests/tests/iterators.rs`, `bench/iterators.ts`). |
| M9-TS-381 | Unit | A coroutine yields, is resumed with a value, keeps its shadow-stack frames alive while suspended through collections, stays done, and can be finished early under stress GC (`iron_ts_rt/src/coro.rs`). |
| M9-TS-382 | Integration | `function*` with `for`, `next()` results, `done`, exhaustion equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-383 | Integration | An infinite generator taken with `break` equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-384 | Integration | A generator of strings, spread and `Array.from` equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-385 | Integration | A generator of objects equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-386 | Integration | `yield*` of a generator, an array and a counter, in order equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-387 | Integration | A `*[Symbol.iterator]()` method (an in-order tree walk with recursive `yield*`) and a `*method()` using `this` equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-388 | Integration | Two generators advanced alternately equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-389 | Integration | `return()` finishes a generator early equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-390 | Integration | `try` / `finally` around a `yield`, a throw inside a generator reaching the caller, and a generator that has thrown stays finished equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-391 | Integration | `next(value)` sending values in (`Generator<number, void, number | undefined>`) equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-392 | Integration | 300 short generators and 20 kept suspended under collection equal bun's (`iron_ts_tests/tests/generators.rs`, `bench/generators.ts`). |
| M9-TS-393 | Unit | A generator without a written return type, and `yield*` used as a value, are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-394 | Integration | `Symbol("x")` values compare by identity (`===`, `!==`) equal bun's (`iron_ts_tests/tests/symbols.rs`, `bench/symbols.ts`). |
| M9-TS-395 | Integration | `symbol.description` and `toString()` equal bun's (`iron_ts_tests/tests/symbols.rs`). |
| M9-TS-396 | Integration | `Symbol.for` returns one symbol per key, `Symbol.keyFor` finds registered ones (`iron_ts_tests/tests/symbols.rs`). |
| M9-TS-397 | Integration | `typeof symbol` is `"symbol"` (`iron_ts_tests/tests/symbols.rs`). |
| M9-TS-398 | Integration | The `symbol` type in annotations and arrays equals bun's (`iron_ts_tests/tests/symbols.rs`). |
| M9-TS-399 | Integration | `generator.throw()` caught inside the generator equals bun's (`iron_ts_tests/tests/generators2.rs`, `bench/generators2.ts`). |
| M9-TS-400 | Integration | `throw()` escaping the generator finishes it, with `finally` run (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-401 | Integration | `throw()` before the first `next()` throws and finishes (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-402 | Integration | `return(v)` runs `finally` blocks and reports `v` (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-403 | Integration | `return(v)` before the first `next()` (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-404 | Integration | A `catch` inside a generator does not swallow `return()` (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-405 | Integration | `return()` while a `finally` yields, then `next()` finishing with the value (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-406 | Integration | `return value` in a generator (`Generator<number, number>`) is the final `value` (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-407 | Integration | An early `return` inside a loop (`iron_ts_tests/tests/generators2.rs`). |
| M9-TS-408 | Integration | An async function returns a value and `await` waits for it equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-409 | Integration | `await` of a plain value takes one tick equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-410 | Integration | The order of async functions, promise reactions, `queueMicrotask` and a timer equals bun's equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-411 | Integration | An async function that throws rejects, and `try/catch` around `await` catches it equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-412 | Integration | `.catch(handler)` returning a replacement value equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-413 | Integration | `.then` chains that change the value's type (number to string) equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-414 | Integration | `.then` returning a promise flattens it, with bun's tick count equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-415 | Integration | `.finally` runs on both outcomes and passes the result through, with bun's ordering equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-416 | Integration | `Promise.all` of promises of different speeds, in order equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-417 | Integration | `Promise.all` rejects with the first rejection equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-418 | Integration | `Promise.race` settles with the fastest equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-419 | Integration | Timers run by due time and then insertion order (`setTimeout` 0, 5, 5, 10) equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-420 | Integration | `setInterval` repeats until `clearInterval` equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-421 | Integration | `clearTimeout` stops a timer equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-422 | Integration | Async methods, async arrows with a block and with an expression body equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-423 | Integration | The executor of `new Promise` settles once; a throwing executor rejects equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-424 | Integration | `return promise` in an async function takes more ticks than `return await promise`, as in bun equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-425 | Integration | 200 async calls under collection (coroutines kept suspended while the collector runs) equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-426 | Integration | `await` in a loop runs the steps in sequence equal bun's (`iron_ts_tests/tests/promises.rs`, `bench/promises_src.ts`). |
| M9-TS-427 | Integration | `then(f, g)` with both callbacks, fulfilled and rejected, equal bun's (`iron_ts_tests/tests/promises.rs`). |
| M9-TS-428 | Integration | `Promise.any` resolves with the first fulfilment and rejects with an `AggregateError` holding every reason (`iron_ts_tests/tests/promises.rs`). |
| M9-TS-429 | Integration | `Promise.allSettled` results (`status`, `value`, `reason`) equal bun's (`iron_ts_tests/tests/promises.rs`). |
| M9-TS-430 | Unit | Async limits (async generators, `await` outside async, `new Promise` without a type, untyped async return, `then(f, g)`) are compile errors with a reason (`iron_ts/src/lib.rs`). |
| M9-TS-431 | Integration | Typed arrays over one `ArrayBuffer` (`Uint8Array`, `Uint32Array`, `Float32Array` with an offset) share memory equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-432 | Integration | Views with offsets and lengths report `length`, `byteOffset` and `byteLength` equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-433 | Integration | Bad view offsets and lengths throw `RangeError` equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-434 | Integration | `subarray` shares memory with its source, including a subarray of a subarray and negative indexes equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-435 | Integration | `array.buffer` of an owning typed array shares its memory equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-436 | Integration | `subarray(..).buffer` is the whole buffer and `byteOffset` is the window's start equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-437 | Integration | `ArrayBuffer.slice` copies (positive and negative indexes) equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-438 | Integration | `DataView` integer getters and setters, both byte orders equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-439 | Integration | `DataView` float getters and setters, both byte orders equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-440 | Integration | `DataView` over a window, its `buffer` and its bounds errors equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-441 | Integration | Views stay alive while their buffer is only reachable through them, under collection equal bun's (`iron_ts_tests/tests/buffers.rs`, `bench/buffers.ts`). |
| M9-TS-442 | Unit | The host copies bytes into script as a typed array and reads them back; a trailing partial element is dropped (`iron_ts_rt/src/typed.rs`). |
| M9-TS-443 | Integration | `structuredClone` of nested objects and arrays equals bun's (`iron_ts_tests/tests/clone.rs`, `bench/clone.ts`). |
| M9-TS-444 | Integration | The clone is independent of the original (objects, arrays, nested fields) (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-445 | Integration | `structuredClone` keeps null fields null (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-446 | Integration | `structuredClone` of a recursive tree type (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-447 | Integration | `structuredClone` of arrays of arrays and of strings (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-448 | Integration | `structuredClone` of a typed array copies its elements (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-449 | Integration | `structuredClone` of numbers, strings, booleans and bigints (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-450 | Integration | 300 clones under collection (`iron_ts_tests/tests/clone.rs`). |
| M9-TS-451 | Integration | `WeakRef.deref()` finds its target (`iron_ts_tests/tests/weak.rs`, `bench/weak.ts`). |
| M9-TS-452 | Integration | `WeakRef<Derived>` typed and used (`iron_ts_tests/tests/weak.rs`). |
| M9-TS-453 | Integration | An array of `WeakRef`s (`iron_ts_tests/tests/weak.rs`). |
| M9-TS-454 | Integration | `WeakMap` get/set/has/delete by object identity (`iron_ts_tests/tests/weak.rs`). |
| M9-TS-455 | Integration | `WeakSet` add/has/delete by object identity (`iron_ts_tests/tests/weak.rs`). |
| M9-TS-456 | Integration | Graphemes of ASCII text equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-457 | Integration | Graphemes with combining marks equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-458 | Integration | Graphemes of emoji sequences, flags and skin tones equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-459 | Integration | Words of English text with punctuation, numbers and apostrophes equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-460 | Integration | `isWordLike` of word segments equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-461 | Integration | Sentences, including an abbreviation equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-462 | Integration | `segments.containing(index)` equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-463 | Integration | The default granularity is grapheme equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-464 | Integration | A bad granularity is a `RangeError` equal bun's (`iron_ts_tests/tests/segmenter.rs`, `bench/segmenter.ts`). |
| M9-TS-465 | Integration | `Intl.Locale` language, script, region, base name equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-466 | Integration | Canonical form of mixed-case tags equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-467 | Integration | Unicode extension keywords (calendar, hour cycle, numbering system, collation, case first, numeric) equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-468 | Integration | Options (region, calendar, hour cycle) applied to a tag equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-469 | Integration | `maximize()` equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-470 | Integration | `minimize()` equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-471 | Integration | `maximize`/`minimize` keep extensions equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-472 | Integration | Invalid tags are `RangeError`s equal bun's (`iron_ts_tests/tests/locale.rs`, `bench/locale.ts`). |
| M9-TS-473 | Integration | `Intl.DisplayNames` of regions equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-474 | Integration | Regions in French, German and Japanese equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-475 | Integration | Scripts equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-476 | Integration | Languages with dialects (`en-US` is American English) equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-477 | Integration | `languageDisplay: "standard"` equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-478 | Integration | Languages in French equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-479 | Integration | A missing `type` is a `TypeError` equal bun's (`iron_ts_tests/tests/dispnames.rs`, `bench/dispnames.ts`). |
| M9-TS-480 | Integration | Temporal: date fields equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-481 | Integration | Temporal: date week years equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-482 | Integration | Temporal: date constrain equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-483 | Integration | Temporal: date add equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-484 | Integration | Temporal: date with equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-485 | Integration | Temporal: date until equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-486 | Integration | Temporal: date compare equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-487 | Integration | Temporal: date errors equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-488 | Integration | Temporal: date formats equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-489 | Integration | Temporal: time basics equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-490 | Integration | Temporal: time math equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-491 | Integration | Temporal: time compare equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-492 | Integration | Temporal: time errors equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-493 | Integration | Temporal: date time basics equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-494 | Integration | Temporal: date time math equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-495 | Integration | Temporal: date time diff equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-496 | Integration | Temporal: date to date time equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-497 | Integration | Temporal: duration basics equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-498 | Integration | Temporal: duration strings equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-499 | Integration | Temporal: duration math equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-500 | Integration | Temporal: duration total equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-501 | Integration | Temporal: duration errors equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-502 | Integration | Temporal: instant basics equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-503 | Integration | Temporal: instant offsets equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-504 | Integration | Temporal: instant math equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-505 | Integration | Temporal: instant errors equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-506 | Integration | Temporal: now is sane equal bun's (`iron_ts_tests/tests/temporal.rs`, `bench/temporal.ts`). |
| M9-TS-507 | Integration | Temporal.ZonedDateTime: parse and print equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-508 | Integration | Temporal.ZonedDateTime: fields equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-509 | Integration | Temporal.ZonedDateTime: spring forward equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-510 | Integration | Temporal.ZonedDateTime: fall back equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-511 | Integration | Temporal.ZonedDateTime: convert equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-512 | Integration | Temporal.ZonedDateTime: arithmetic equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-513 | Integration | Temporal.ZonedDateTime: from fields equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-514 | Integration | Temporal.ZonedDateTime: errors equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-515 | Integration | Temporal.ZonedDateTime: now zoned equal bun's (`iron_ts_tests/tests/zoned.rs`, `bench/zoned.ts`). |
| M9-TS-516 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month basics equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-517 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month math equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-518 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month diff equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-519 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month compare equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-520 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month conversions equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-521 | Integration | Temporal.PlainYearMonth / PlainMonthDay: year month errors equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-522 | Integration | Temporal.PlainYearMonth / PlainMonthDay: month day basics equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-523 | Integration | Temporal.PlainYearMonth / PlainMonthDay: month day math equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-524 | Integration | Temporal.PlainYearMonth / PlainMonthDay: month day errors equal bun's (`iron_ts_tests/tests/monthday.rs`, `bench/monthday.ts`). |
| M9-TS-525 | Integration | Temporal rounding: time units equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-526 | Integration | Temporal rounding: time modes equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-527 | Integration | Temporal rounding: time increments equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-528 | Integration | Temporal rounding: date time equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-529 | Integration | Temporal rounding: instant equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-530 | Integration | Temporal rounding: zoned equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-531 | Integration | Temporal rounding: duration equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-532 | Integration | Temporal rounding: errors equal bun's (`iron_ts_tests/tests/rounding.rs`, `bench/rounding.ts`). |
| M9-TS-533 | Integration | Intl.getCanonicalLocales: single equal bun's (`iron_ts_tests/tests/canon.rs`, `bench/canon.ts`). |
| M9-TS-534 | Integration | Intl.getCanonicalLocales: list equal bun's (`iron_ts_tests/tests/canon.rs`, `bench/canon.ts`). |
| M9-TS-535 | Integration | Intl.getCanonicalLocales: empty equal bun's (`iron_ts_tests/tests/canon.rs`, `bench/canon.ts`). |
| M9-TS-536 | Integration | Intl.getCanonicalLocales: invalid equal bun's (`iron_ts_tests/tests/canon.rs`, `bench/canon.ts`). |
| M9-TS-537 | Integration | formatToParts: decimal equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-538 | Integration | formatToParts: currency equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-539 | Integration | formatToParts: percent equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-540 | Integration | formatToParts: other locales equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-541 | Integration | formatToParts: signs equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-542 | Integration | formatToParts: compact equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-543 | Integration | formatToParts: scientific equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-544 | Integration | formatToParts: special equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-545 | Integration | formatToParts: date english equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-546 | Integration | formatToParts: date long equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-547 | Integration | formatToParts: date fields equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-548 | Integration | formatToParts: date other locales equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-549 | Integration | formatToParts: date hour12 equal bun's (`iron_ts_tests/tests/fparts.rs`, `bench/fparts.ts`). |
| M9-TS-550 | Integration | resolvedOptions: date time format equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-TS-551 | Integration | resolvedOptions: date time zones equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-TS-552 | Integration | resolvedOptions: default locale equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-TS-553 | Integration | resolvedOptions: number format equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-TS-554 | Integration | resolvedOptions: number currency equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-TS-555 | Integration | resolvedOptions: number options equal bun's (`iron_ts_tests/tests/resolved.rs`, `bench/resolved.ts`). |
| M9-API-01 | Unit | The `#[iron_api]` manifest lists every `pub` function with its script name, Rust name, symbol, docs, parameters and result; private functions are left out (`crates/iron_api/tests/api.rs`). |
| M9-API-02 | Unit | The TypeScript declaration is generated from the manifest: `declare namespace`, parameter and result types, handle types, doc comments as JSDoc (`crates/iron_api/tests/api.rs`). |
| M9-API-03 | Unit | The JSON manifest names each symbol and type (`handle:GameObject`, `f64`) for the compiler (`crates/iron_api/tests/api.rs`). |
| M9-API-04 | Unit | The generated `extern "C"` shims call the functions with the C ABI, including `f32`, `i32`, `bool` and handles as raw ids (`crates/iron_api/tests/api.rs`). |
| M9-API-05 | Unit | The generated QuickJS bindings install the namespace as a global with typed arguments and results (`crates/iron_api/tests/api.rs`). |
| M9-API-06 | Benchmark (ignored) | 1,000,000 typed QuickJS calls against the same through an `__iron_op`-style queue; prints the time per call (`crates/iron_api/tests/api.rs`). |
| M9-API-07 | Unit | Shims reach the world the host made current; with none they answer their defaults (`crates/iron_api/tests/context.rs`). |
| M9-API-08 | Unit | A nested call inside `context::with` finds no world instead of a second `&mut` (`crates/iron_api/tests/context.rs`). |
| M9-API-09 | Unit | Context scopes nest and restore the previous world (`crates/iron_api/tests/context.rs`). |
| M9-API-10 | Unit | The manifest, `.d.ts` class and JSON describe a value class (`crates/iron_api/tests/values.rs`). |
| M9-API-11 | Unit | The shims pass a value as scalars and return one through a pointer (`crates/iron_api/tests/values.rs`). |
| M9-API-12 | Unit | An `ObjectId` handle is never 0 (even index 0, generation 0) and converts back exactly (`crates/iron_scene/tests/api.rs`). |
| M9-API-13 | Unit | The `Scene` shims create, parent, move and destroy objects in the current scene, stale ids miss, and with no scene they answer defaults (`crates/iron_scene/tests/api.rs`). |
| M9-API-14 | Unit | A rotation is a `Quaternion` value; enabled/active flags and layers work through the shims (`crates/iron_scene/tests/api.rs`). |
| M9-API-15 | Unit | The `Scene` manifest declares the `GameObject` handle and the `Vector3` and `Quaternion` values, and the generated `.d.ts` has the expected signatures (`crates/iron_scene/tests/api.rs`). |
| M9-API-16 | Unit | Handle class functions are listed per class with the receiver first (`this`); a `set_x` next to `#[property] x` is the property's setter (`crates/iron_api/tests/classes.rs`). |
| M9-API-17 | Unit | The `.d.ts` has `declare class` with static, method and `get`/`set` members; a getter alone is read-only; a class replaces the plain handle alias (`crates/iron_api/tests/classes.rs`). |
| M9-API-18 | Unit | The class shims take the receiver first and call the methods, properties and statics (`crates/iron_api/tests/classes.rs`). |
| M9-API-19 | Unit | `GameObject` and `Transform` of `iron_scene` work through their shims, a transform shares its object's handle, and the declarations have accessors (`crates/iron_scene/tests/api.rs`). |
| M9-API-20 | Unit | Value class methods and getters (`#[method(Vector3)]`, `#[getter(Vector3)]` on free functions whose first parameter is the value) are declared inside the value class, with the receiver as `this` and passed as its fields (`crates/iron_api/tests/values.rs`). |
| M9-API-21 | Unit | Strings cross as script string objects (`&str` parameters, borrowed) and a returned buffer (`String` results); the manifest and `.d.ts` say `string` (`crates/iron_api/tests/strings.rs`). |
| M9-API-22 | Unit | A lone surrogate in a script string becomes one U+FFFD for the engine; a well formed string is borrowed, not copied; null is empty (`crates/iron_api/tests/strings.rs`). |

---

## Later milestones (outline)

- **Networking:** `@sync` fields replicate; `@rpc.server` runs only on the server; prediction and reconciliation under simulated latency and packet loss.
- **Android / Quest:** app launches, renders, survives pause/resume and rotation.
- **VR:** stereo render at the headset's refresh rate; world panel interaction with controllers.
- **iOS:** QuickJS backend passes the full M1-SCR suite; app passes TestFlight.
- **Packaging:** packed build runs from a clean machine with no dev tools installed.
| M9-TS-556 | Integration | The generated `classes()` table lists the registered classes and only the hooks each has (`iron_ts_tests/tests/components.rs`, `bench/components.ts`). |
| M9-TS-557 | Integration | A compiled component moves its GameObject every frame through the real scene; fields persist; all GC modes. |
| M9-TS-558 | Integration | Host order: `onStart` once, every `onUpdate` before any `onLateUpdate`. |
| M9-TS-559 | Integration | Subclasses inherit hooks and override one; inactive objects are skipped; `destroy` runs `onDestroy`. |
| M9-TS-560 | Integration | A throwing hook disables its component and reports it; the others run; an unknown class is an error. |
| M9-TS-561 | Integration | `fixed_update` reaches only classes with `onFixedUpdate`. |
| M9-TS-562 | Integration | Generics `identityTypes` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-563 | Integration | Generics `firstOf` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-564 | Integration | Generics `mapping` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-565 | Integration | Generics `pairs` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-566 | Integration | Generics `fills` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-567 | Integration | Generics `reducing` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-568 | Integration | Generics `boxes` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-569 | Integration | Generics `stacks` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-570 | Integration | Generics `lists` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-571 | Integration | Generics `subclass` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-572 | Integration | Generics `aliases` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-573 | Integration | Generics `nested` equals bun (`iron_ts_tests/tests/generics.rs`, `bench/generics.ts`). |
| M9-TS-574 | Unit | Generic mistakes are errors with a reason: cannot infer, exported generic, conflicting inference, generic static or accessor. |
| M2-NAT-01 | Integration | Native reconciler: a description becomes nodes in a real `UiTree` (`crates/iron_ui_native/tests/reconcile.rs`). |
| M2-NAT-02 | Integration | State setter re-renders only its component, text updated in place; same value renders nothing. |
| M2-NAT-03 | Integration | Keyed children keep their nodes when reordered, inserted and removed. |
| M2-NAT-04 | Integration | Style diff sends what changed; a removed property resets. |
| M2-NAT-05 | Integration | Components and fragments place their nodes in the container; a component switching output replaces in place. |
| M2-NAT-06 | Integration | Effects and cleanups run in real React's order (log from `js/react/test/native_oracle.test.tsx`). |
| M2-NAT-07 | Integration | `useRef`/`useMemo`/effect without deps keep values and run as React does. |
| M2-NAT-08 | Integration | Portals mount under another node; events bubble through the component tree. |
| M2-NAT-09 | Integration | Capture then bubble, `stopPropagation`, `preventDefault`, state set in a handler is committed before dispatch returns. |
| M2-NAT-10 | Integration | Refs set and cleared, context follows the provider, unmount cleans up and frees. |
| M2-NAT-11 | Integration | An effect that sets state every render is reported after 100 chained updates. |
| M2-NAT-12 | JS unit | Real React's effect/cleanup order for mount, update, removal, replacement and unmount (`js/react/test/native_oracle.test.tsx`). |
| M9-TS-575 | Integration | A TSX counter (useState, style, onClick, text and number children) renders into a real UiTree and handles clicks (`iron_ts_tests/tests/ui.rs`, `bench/ui.ts`). |
| M9-TS-576 | Integration | A keyed list from `items.map(..)` keeps its nodes when reversed. |
| M9-TS-577 | Integration | `useEffect` with dependencies and cleanup: +1, no rerun, -1 +2, cleanup on unmount. |
| M9-TS-578 | Integration | `useMemo` recomputes only when its dependencies change. |
| M9-TS-579 | Integration | Boolean and string state and `useRef` persist across renders; all GC modes. |
| M9-TS-580 | Integration | Context: a consumer reads the nearest provider, the default without one, and follows the provider (`iron_ts_tests/tests/ui.rs`). |
| M9-TS-581 | Integration | `ref={r}` writes the node into the ref on attach (an effect sees it). |
| M9-TS-582 | Integration | `useImperativeHandle` fills the parent's ref before the parent's effect runs. |
| M9-TS-583 | Integration | `useSyncExternalStore` follows a store, subscribes once, unsubscribes on unmount. |
| M9-TS-584 | Integration | `memo` skips a child with equal props and renders it when a prop changes. |
| M9-TS-585 | Integration | `forwardRef` passes the ref as the second parameter. |
| M9-TS-586 | Integration | `useReducer` applies actions to the latest state (two dispatches in one handler both count). |
| M2-NAT-13 | Integration | A controlled text field: an accepted (even transformed) edit shows the new value prop, a rejected one sends the old value back (`crates/iron_ui_native/tests/reconcile.rs`). |
| M9-TS-587 | Integration | The compiler exports each component's stable id and ordered hook list (`__uiSignatures`). |
| M9-TS-588 | Integration | Hot reload with `Runtime::refresh`: state is kept, except for components in the remount list. |
| M9-TS-589 | Integration | The updater form `setN(n => n + 1)`: two updaters in one handler apply to the latest state (`iron_ts_tests/tests/ui.rs`). |
| M9-TS-590 | Integration | `style={obj}` with a typed object sets its fields; an optional field left out is not set. |
| M9-TS-591 | Integration | `{...obj}` on a host element sets attributes; on a component it spreads into the props. |
| M9-API-23 | Integration | QuickJS bindings: a value class is a plain object (arguments read from it, results new objects) (`crates/iron_api/tests/quickjs.rs`). |
| M9-API-24 | Integration | QuickJS bindings: `&str`/`String` are JS strings (non-BMP and accents). |
| M9-API-25 | Integration | A value object missing a field is a catchable JS exception. |
| M9-API-26 | Integration | The `Scene` namespace of `iron_scene` works from QuickJS on the current scene (`crates/iron_scene/tests/api.rs`). |
| M9-TS-592 | Unit | A class field's type is inferred from constants, arithmetic, `C.x` of a constant object and object literals; a call is still an error asking for an annotation. |
| M9-API-27 | Integration | Namespace properties: a `#[property]` function without a receiver is a getter, `set_x(v)` its setter; manifest kinds, `.d.ts` (`let` / `const`) and JSON (`crates/iron_api/tests/ns_props.rs`). |
| M9-API-28 | Integration | The shims of namespace properties take no receiver. |
| M9-API-29 | Integration | QuickJS gets accessors for namespace properties (`Cfg.level = 9`). |
| M9-API-30 | Integration | `Color` (`crates/iron_engine_api/tests/engine.rs`): constants, `hex` (sRGB to linear), `lerp`, `toHex`, `withAlpha`. |
| M9-API-31 | Integration | `Time` follows the host's frames and the time scale (a bad scale reads as 1). |
| M9-API-32 | Integration | `input` reports this frame's presses and releases; key repeats are not presses. |
| M9-API-33 | Integration | `Screen` and `QualitySettings` are read and set. |
| M9-API-34 | Integration | `Gizmos` collect lines and present them to a camera; the host takes the frame once. |
| M9-API-35 | Integration | `Camera` defaults, field of view clamp, orthographic settings, removal with its object. |
| M9-API-36 | Integration | Value classes are JS classes in QuickJS (constructor, methods, getters, instances as results) (`crates/iron_scene/tests/api.rs`). |
| M9-TS-593 | Integration | `Color` statics, `new Color`, `lerp`/`withAlpha`/`toHex` work in compiled code (`iron_ts_tests/tests/engine2.rs`, `bench/engine2.ts`). |
| M9-TS-594 | Integration | Namespace properties (`Time.timeScale = ..`, `Time.deltaTime`, `Screen.fullScreen`, `QualitySettings.vSyncCount`) and `input` work in compiled code. |
| M9-TS-595 | Integration | `Camera.attach(go)` with property assignments, and `Gizmos` (static colour, `drawLine`, `present`) work in compiled code. |
| M9-TS-596 | Integration | The whole `SineDemo` component (copied from `9dc2f51`) compiles and wakes up: camera made with `new GameObject(..)` / `addComponent(Camera, {..})`, music loaded with options, the curve drawn as 1,083 gizmo lines (`iron_ts_tests/tests/sine_demo_full.rs`). |
| M9-TS-597 | Integration | Keys drive the compiled demo: Space plays the music looped and the tweens move the curve, a digit picks a formula, F5 adds the 24-line centre ring. |
| M9-TS-598 | Integration | `boolean \| undefined` fields: `??`, truthiness, assignment (`bench/nullbool.ts`, bun-compared). |
| M9-TS-599 | Integration | Optional boolean parameters and `=== undefined` (bun-compared). |
| M9-TS-600 | Integration | Optional boolean members of a class (bun-compared). |
| M9-TS-601 | Integration | `NativeHost::properties`: each class of the generated table carries its `@property` list (name, type); a class without any has none (`tests/all/components2.rs`). |
| M9-TS-602 | Integration | `get_property` / `set_property` through the table: numbers, nullable numbers, booleans, strings (collected between write and read); a wrong kind is an error naming the property and changes nothing. |
| M9-TS-603 | Integration | `save_properties` / `load_properties`: values saved from one component load into another, component references travel as ids; bad entries are reported and the rest apply. |
| M9-TS-604 | Integration | `this.getComponent(X)`, `go.addComponent(X, { .. })` between compiled components: the sibling is found, the added component has its options and runs `onAwake`, a missing class is `undefined`, destroying the object destroys it. |
| M9-TS-605..610 | Integration | `Rng` (`new Rng(seed?)`, `nextDouble`, `next`, `float`, `int`, `bool`) and the shared `Random`: mulberry32 sequences equal the original `random.ts` run in bun (`bench/random.ts`, `tests/all/random.rs`). |
| M9-TS-611..614 | Integration | `LayerMask`: built-in and game layer names (renaming, out-of-range and empty names ignored), `getMask`, `contains`, `everything` / `nothing`, compared with `layers.ts` in bun. |
| M9-TS-615..616 | Integration | `PlayerPrefs`: typed reads with and without fallbacks, truncating `setInt`, `hasKey`, `deleteKey`, `deleteAll`, compared with `prefs.ts` in bun. |
| M9-TS-617..620 | Integration | `Assets`: GUID and path lookups (backslashes and `./` cleaned), `guid:` sources, importer JSON, enumeration, compared with `assets.ts` in bun through a prelude that adapts the shape. |
| M9-TS-621..623 | Integration | `Application` and `SystemInfo` read in compiled code against the manifest and machine the host installs, settable `targetFrameRate` / `runInBackground`, compared with `app.ts` in bun. |
| M9-TS-624..626 | Integration | `TimeSince`, `TimeUntil` and `Time` after three engine ticks, compared with `clock.ts` in bun. |
| M9-API-37 | Integration | `Random` in QuickJS gives the TypeScript generator numbers for a seed; an omitted `bool` chance defaults to 0.5. |
| M9-API-38 | Integration | `LayerMask` in QuickJS: names, masks, `contains`. |
| M9-API-39 | Integration | Log lines reach the host sink with their level; `Debug.assert` logs only when false. |
| M9-API-40 | Integration | `PlayerPrefs` in QuickJS, and the ordered changes the host writes to the file. |
| M9-API-41 | Integration | `Assets` in QuickJS: lookups, `guid:` sources, importer JSON, polled changes. |
| M9-API-42 | Integration | `Application` / `SystemInfo` in QuickJS, script writes returned as requests, focus and pause changes. |
| M9-API-43 | Integration | `TimeSince` / `TimeUntil` value classes in QuickJS follow the game clock. |
| M9-TS-627..636 | Integration | `input` after the engine delivered events and ticked: keys held / pressed / released, mouse position, delta and wheel, mouse buttons as keys, `getAxis` (keys, stick, mouse), `getButton` / `getButtonDown`, gamepads by id; a second program checks the frame after (a key released, a pad gone, a blur), compared with `input.ts` in bun (`inputs`, `inputs2`). |
| M9-TS-637..642 | Integration | `TimeSpan`: `new TimeSpan(ms)` rounding, `of`, `fromX`, parts and totals (negative too), arithmetic, comparison, `toString` constant and custom formats, `parse`, compared with `time.ts` in bun. |
| M9-TS-643..650 | Integration | `DateTime` in New York time: fields, epoch and kind, every standard and custom format, month / year / day arithmetic with clamping, daylight-saving gaps and overlaps, field carrying (`new DateTime(2024, 13, 1)`), ISO parsing, statics, compared with `time.ts` in bun. |
| M9-API-44 | Integration | `input` in QuickJS follows the tick (`time::advance`): events become this frame, the stick beats the keys when pushed further, pads are read by id, a blur clears held keys. |
| M9-API-45 | Integration | `TimeSpan` and `DateTime` as JavaScript classes in QuickJS: `new X(..)` goes through `create`, members and statics work. |
| M9-TS-651 | Integration | `Task.delay`, `Task.delaySeconds` and `Task.frame` in an async function wait on the runtime timers advanced with engine time (`tests/all/task.rs`). |
| M9-TS-652..660 | Integration | `Mathf` (constants, interpolation, stepping, angles, comparisons, powers of two, Perlin noise, colour space, `smoothDamp`): every number printed in full equals the original `math.ts` in bun (`bench/mathf.ts`). |
| M9-TS-661..664 | Integration | `Vector2`, `Vector3`, `Vector4` (all members and statics, four values each paired with every other) and `Vector2Int` / `Vector3Int`, bit-for-bit equal to `math.ts` (`bench/vectors.ts`; `Math.hypot` of three or more numbers uses the native algorithm in both, see `docs/iron-api.md`). |
| M9-TS-665..670 | Integration | `Quaternion` (construction, `euler` both forms, products, `slerp`, `rotateTowards`, `eulerAngles` over a grid, `lookRotation`, `fromToRotation`) and `Matrix4x4` (`trs`, `mul`, `inverse`, `determinant`, projections, point and vector transforms, `fromColumns`), bit-for-bit equal to `math.ts`. |
| M9-TS-671..674 | Integration | `Rect` / `RectInt`, `Ray`, `Plane` (NaN where the original answers null), `Bounds` and `Pose`, bit-for-bit equal to `math.ts`. |
| M9-API-46 | Integration | The math types as JavaScript classes in QuickJS: `new X(..)` through `create`, instances from methods, statics and getters (`crates/iron_math/tests/quickjs.rs`). |
| M9-API-47 | Integration | Overloads by argument count in QuickJS (`Quaternion.euler` two forms, `lookRotation`, `Matrix4x4.trs`), and the shapes (`Ray`, `Plane`, `Bounds`, `Pose`). |
| M9-TS-675 | Integration | The vector-valued members of `Rng` and `Random` (`insideUnitCircle`, `onUnitSphere`, `insideUnitSphere`, `rotation`) equal the original in bun, bit for bit. |
| M9-TS-676..683 | Integration | Animation curves, clips and sampling (`Keyframe`, `AnimationCurve`, `AnimationClip.sampleAnimation`): Hermite evaluation over a custom curve and steps, wrap modes before and after, key editing and indexes, clip length / curve replacement / removal / events, sampling a three-level rig (position, scale, quaternion and Euler channels, `GameObject.active`, a missing path), clip wrap modes, JSON round trip; bit-for-bit equal to `animation.ts` run in bun over `scene.ts` (`bench/anim_clips.ts`). |
| M9-TS-684..691 | Integration | The `Animation` component: named states and `play` / `stop` / `isPlaying`, state properties (`time`, `speed`, `weight`, `normalizedTime`), `crossFade` weights over time, wrap modes of a state (once holds the last frame then stops), clip events (loop passes, once), the default clip and `playAutomatically`, animating `Camera` properties (`fieldOfView`, `backgroundColor.r`, `orthographic`) through the property registry, blending two clips with sign-aligned quaternions, objects inactive in the hierarchy; bit-for-bit equal to `animation.ts` in bun (`bench/anim_player.ts`). |
| M9-TS-692..697 | Integration | The `Animator` and its controller made with the chained builder and with the explicit classes (identical output): parameters, states, transitions on conditions and exit time, triggers used up, `Any` transitions, blend trees with time scales, `play` / `crossFade`, default state, offsets, JSON round trip; bit-for-bit equal to `animation.ts` in bun (`bench/anim_animator.ts`). |
| M9-TS-698..702 | Integration | GameObject and Transform members against `scene.ts` in bun (`bench/scene_objects.ts`): hierarchy (children, `findChild`, `root`, `destroyed`, roots, `hideFlags`, `byId`), world position / rotation / scale / Euler angles composed down the parent chain, `forward/right/up`, `translate` / `rotate` / `lookAt`, `transformPoint` and its inverse, `setParent` keeping the world pose; numbers to three decimals (the scene stores floats). |
| M9-TS-703..706 | Integration | Component lookups against `scene.ts` in bun (`bench/comp_query.ts`): `getComponent` finds subclasses, `getComponents`, `getComponentInChildren`, `GameObject.findObjectOfType`, `removeComponent` / `destroy()`, `transform` / `gameObject` on components; three GC modes. |
| M9-TS-707..710 | Integration | Component lifecycle (`bench/comp_life.ts`): `onDestroy` after removal runs when the host settles, add+remove in one hook never wakes, `onApplicationFocus/Pause` flags (inactive skipped), destroyed objects take their components (and children's) with them. |
| M9-TS-711 | Integration | `go.getComponent(Camera)` / `Animation` / `Animator` finds the engine component on that object only (static `findOn`). |
| M9-TS-712..718 | Integration | Scenes for compiled programs (`bench/scenes.ts`, `iron_scenes`): a scene file loads (objects, transforms, layer, enabled, `Camera`, the program's components with number, `Vector3` and reference properties; unknown kinds warn; components wake at `settle`); save and load round trip; `GameObject.instantiate` with child remapping and `{ position, rotation, parent }`; references outside the copy stay; additive / single loads, unload, `dontDestroyOnLoad`; prefabs; the `examples/scenes` level file. |
| M9-TS-719 | Integration | An engine object (handle) stringifies as `{}`; script fields beside it are kept. |
| M9-TS-720..722 | Integration | Stack values (`iron_ts::sroa`): a vector built, combined and read in one function is not allocated; one that is returned or stored stays an object; aliases that change stay one object. |
| M9-TS-723..725 | Integration | Value classes (`Vector3`, `Color`, `Quaternion`, ...) stringify compactly as their numbers (`[1,2,3]`) and parse back from that or from the object form (`{"x":1,..}`, left-out fields 0, alpha 1) (a difference to the TypeScript classes, by decision). |
| M9-TS-726 | Integration | A `Vector2[]` is passed to the engine in place (`AnimationCurve.fromPoints(points, mode?)`; modes linear / step; default linear). |
| M9-API-48 | Unit | `script_f64s` reads a script `number[]` in place; null and empty give an empty slice. |
| M9-TS-727..730 | Integration | Native physics from compiled code (`bench/physics.ts`): a ball falls and rests on a floor; a ray answers at once (distance, point, normal; a miss); an attached body moves its GameObject; gravity, impulse with mass, body kind. |
| M9-TS-1000..1003 | Integration | Contact hooks on compiled components (`bench/contacts.ts`, `tests/all/contacts.rs`): a collision tells both objects (other object, impact speed); Stay while touching and Exit when it ends, inherited hooks included; sensors call the trigger hooks; hand-made events skip inactive objects, a hook that destroys its component or throws is safe and reported. |
| M9-TS-1010..1013 | Integration | The component / GameObject surface of scene.ts in compiled code (`bench/comp_api.ts`): `children`, `root`, `parent`, null tests of handles; `components`; `GameObject.roots`, `find`, `byId`; `go.scene`. |
| M9-API-200..209 | Unit | Physics components (`crates/iron_physics_api/src/tests.rs`): Rigidbody + BoxCollider rest on a floor; a kinematic object moved by script carries its body; colliders remade at the object's scale; centers, triggers and `collider_info` for contacts; fixed and rope joints; a Rope hangs; a CharacterController lands and is grounded; 2D bodies keep z and a 2D ray names its object; kinds save/load/copy/destroy; destroyed objects lose their physics. |
| M9-TS-1100..1105 | Integration | Physics components from compiled code (`bench/physics_comp.ts`): the component world matches the same ops through `iron_physics::Physics`; options round-trip through `getComponent`; `ForceMode.Impulse`; FixedJoint and Rope; CharacterController; 2D components, `Physics2D.raycast` and gravity. |
| M9-TS-1200..1207 | Integration | Closures given to the engine (`bench/callbacks.ts`): `input.onKeyDown/onKeyUp/onGamepad*` get codes, flags, ids and values and are removed by `input.off(token)`; a listener keeps captured variables; a Rust listener may unsubscribe inside an event; `Assets.onChanged`, `Application.onFocusChanged` (only on a change) and `SceneManager.onSceneLoaded/Unloaded/ActiveSceneChanged` (scene handles, mode text); all in the normal, stress and no-GC modes. |
| M9-API-300..302 | Unit | `Callback<(..)>` parameters: manifest `fn(f64,string):void` and the declaration `(a: number, b: string) => void`; listener tokens unique and ordered; a script callback without a runtime bridge is ignored. |
| M9-TS-1210..1219 | Integration | Native audio from compiled code (`bench/audio_native.ts`): `audio.load/play/createBus/createEmitter/setListener/setMasterVolume`, `Sound.play({..})` options by name, playback changes, `onEnded`, emitters; `AudioSource` (flat and 3D with an emitter that follows the object, one shots, pause/stop, `playClipAtPoint`) and `AudioListener`; destroying an object stops its sources; scene files load and save both components and play on awake. The commands the host runs on the mixer are checked as `audio.*` ops. |
| M9-TS-1220..1227 | Integration | Rendering data API from compiled code (`bench/render_native.ts`, `iron_render_api`): `Light`, `MeshRenderer` (primitive meshes by name, script `Mesh`, `Material`), `Texture` (file, pixels, render texture) with camera extras, `Sprite` / `SpriteRenderer`, `LODGroup`, `Mirror`, `ReflectionProbe`, `PlaneRenderer`, `WorldPanel`, `LineRenderer2D`; the example `level1.scene.json` loads its Light, MeshRenderer, inline Material and camera settings and saves them back; the `assets` table (shared textures, meshes, render textures, inline sprites) loads once per entry and round-trips. |
| M9-TS-1228..1230 | Integration | `RenderSettings` (ambient light, procedural sky, bloom, tonemapping; unknown names ignored) and `Cursor` / `CursorLockMode` from compiled code; `CameraFollow` and `CameraOrbit` move their objects when the host runs `late_update_camera_follows` / `late_update_camera_orbits`; a scene file applies its `renderSettings` when it becomes active and saves the current ones. |
| M9-TS-1231..1235 | Integration | `ParticleSystem` (ranges, shape, bursts, curves, flipbook; play/stop/emit commands for the host; `report_particles` runs `onStopped` and `stopAction: "destroy"`), `LineRenderer` (a tube through a mesh renderer of its own, rebuilt on change), `Cloth` (swaps the renderer mesh for a simulated copy and back) and `Primitives.create` (collider hook); all three load from scene files and save the same data again. |
| M9-TS-1236..1238 | Integration | `Gizmos` shapes (ray, wire cube / disc / sphere, frustum, `matrix`, `lineCount`) from compiled code; `Screen` (modes, `setResolution` requests for the host, resolutions, `onChange` listeners run when the host updates the size) and `QualitySettings` (levels from the manifest, validation of names and numbers, level switches tell listeners, the choice is saved as `iron.quality` and comes back). |
| M9-TS-1239 | Integration | `Model.load` / `instantiate` (options `position`, `rotation`, `scale`, `parent`, `colliders`; an object literal with those keys is the options, not a value): the file is read once, copies asked for before the host reports the description get their nodes when it does (renderers per primitive, glTF materials, colliders through a hook); a scene saves a copy as `"model"` without its nodes and builds them again on load. |
| M9-TS-1500..1510 | Integration | JSX and hooks need no manifest from the caller (`tools/iron_ts/tests/jsx_builtin.rs`): a component with no manifests or with another one, a hook-only program (single line), fragments, `{cond && <x/>}`, `.map` children, spread props, text children and conditionals, a JSX syntax error reports its line and the TSX parser's message, TypeScript that is not TSX is not blamed on the TSX parse, an explicit `Ui` manifest is still accepted. |
| M9-TS-1511 | Integration | The `Ui` manifest embedded in `iron_ts` equals `iron_ts_ui::manifest_json()` (`tools/iron_ts_ui/tests/embedded_manifest.rs`). |
| M9-API-100 | Integration | A handle in QuickJS is an opaque object: the id is in an internal slot, `JSON.stringify` gives `{}`, no own or enumerable properties, `[object Thing]`, `instanceof`, script-added properties are own (`crates/iron_api/tests/handles_quickjs.rs`). |
| M9-API-101 | Integration | Handle class statics, methods, properties, setters and `new Class(args)` (through `create`) in QuickJS; a getter alone is read only. |
| M9-API-102 | Integration | The id 0 is `null` going out and coming in; optional handle parameters default to `null`. |
| M9-API-103 | Integration | A handle of the wrong class, a number or a plain object, a detached method and `new` on a class without `create` are `TypeError`s. |
| M9-API-104 | Integration | Modules share one class per handle name; a class named like its namespace is the namespace object. |
| M9-API-105 | Integration | Methods with more than four parameters, value results, `&[f64]` and `&[V]` parameters from JS arrays (objects, arrays or instances as elements). |
| M9-API-106 | Integration | The same calls from compiled shims (raw ids) and QuickJS agree. |
| M9-API-107 | Integration | QuickJS value classes: `toJSON` is the compact array, `X.fromJSON` reads the array (exact length) or the object form (left-out fields 0, alpha 1), and functions accept arrays. |
| M9-API-108 | Integration | `GameObject`, `Transform` and `Camera` as handle classes over the real scene (`crates/iron_engine_api/tests/handles_quickjs.rs`); classes with one id are not interchangeable. |
| M9-API-109 | Integration | Handles made by a script and read by the shims (and the reverse), and a handle made by the host with `handle_new`. |
| M9-API-110 | Integration | `AnimationCurve` factories, `fromPoints` from an array of vectors, methods on the returned handle. |
| M9-API-111 | Integration | `SceneManager`, `Scene` and `Prefab` as handle classes (`crates/iron_scenes/tests/handles_quickjs.rs`). |
| M9-API-112 | Integration | A namespace and a class with the same name (`Scene`) merge, whichever is registered first. |
| M9-API-113 | Integration | `QuickJsRuntime::add_native` installs a native API and reinstalls it after `reset()` (`crates/iron_script/tests/native.rs`). |
| M9-API-120 | Integration | Every function of every manifest (namespaces, class statics, methods, properties, value classes) exists in QuickJS after `iron_native::register_quickjs` (`crates/iron_native/tests/surface.rs`). |
| M9-API-121 | Integration | Methods of one class with the same script name dispatch by argument count (`this` is passed on); no version for the count is a `TypeError`. |
| M9-API-122 | Integration | A JS function passed as a `Callback` is kept (survives scripts ending and a collection), called with the event's arguments, and `off(token)` removes it (`crates/iron_api/tests/callbacks_quickjs.rs`). |
| M9-API-123 | Integration | Handle and boolean callback arguments reach JS as handle objects / `null` and booleans; a callback with no arguments. |
| M9-API-124 | Integration | An exception in a JS callback is recorded (`take_errors`), the other callbacks still run, and a callback may subscribe from inside a call. |
| M9-API-125 | Integration | A callback survives repeated collections while only the engine holds it; a non-function is a `TypeError`. |
| M9-API-126 | Integration | Physics from QuickJS: a Rigidbody with a BoxCollider falls and rests on a floor at y = 1 (as the compiled test), `ForceMode` / `RigidbodyConstraints`, a wrong handle is a `TypeError` (`crates/iron_native/tests/physics.rs`). |
| M9-API-127 | Integration | Raycast from QuickJS: distance, point, normal of the hit, a miss and a too-short ray are `null`, `maxDistance` optional. |
| M9-API-128 | Integration | Bodies, colliders and a rope joint made from script; gravity getter/setter; bad arguments are `TypeError`s. |
| M9-API-129 | Integration | `CharacterController` grounded and moving, `Rope` points, as the compiled tests. |
| M9-API-130 | Integration | 2D: `Rigidbody2D` rests on a 2D floor with z kept, `Physics2D.raycast` names the object. |
| M9-API-131 | Integration | `FixedJoint` holds a body, `HingeJoint` properties, `MeshCollider.setMesh` from arrays. |
| M9-API-132 | Integration | Audio from QuickJS: the host receives Load / Bus / Emitter / Play / Set / Stop / Listener commands with the script's values; `onEnded` runs when the host reports (`crates/iron_native/tests/audio.rs`). |
| M9-API-133 | Integration | `AudioSource` / `AudioListener` components, one-shots and static `playClipAtPoint`. |
| M9-API-134 | Integration | `input.on...` listeners get the events' arguments, tokens and `off` (`crates/iron_native/tests/services.rs`). |
| M9-API-135 | Integration | `Assets.onChanged`, `Application.onFocusChanged`, `Screen.onChange` callbacks and `Time`. |
| M9-API-136 | Integration | `Rng` handles and `Random` agree with the compiled shims for one seed. |
| M9-API-137 | Integration | Scenes from script: serialize, load additively, instantiate a prefab, `onSceneLoaded` / `onSceneUnloaded` / `onActiveSceneChanged`. |
| M9-API-138 | Integration | `Cursor`, `Screen` and `QualitySettings` property pairs. |
| M9-API-139 | Integration | `Light`, `MeshRenderer`, `Material` (enums, colors, numbers, handles), and the host's view of the data (`crates/iron_native/tests/render.rs`). |
| M9-API-140 | Integration | `Texture`, `Mesh.create` from arrays, `Sprite`, `SpriteRenderer`. |
| M9-API-141 | Integration | `RenderSettings`, `Primitives`, `LineRenderer.setPositions`, `Cloth`, `ParticleSystem`. |
| M9-API-142 | Integration | `ParticleSystem.onStopped` runs when the host reports the system stopped. |
| M9-API-143 | Integration | `Model.load` / `instantiate` with optional values left out. |
| M9-API-144 | Integration | Every component class with `attach` (and assets with `create()`): each get/set property round-trips its own value (265 properties, `surface.rs`). |
| M9-API-145 | Integration | GC stress: tiny collection threshold, thousands of handle objects and 200 engine-held callbacks stay correct. |
| M9-TS-1600 | Integration | An effect whose cleanup is an arrow around a void call (`return () => clearTimeout(id)`) and a setter inside the timer compile (`tools/iron_ts/tests/react_front.rs`; was a Cranelift verifier error). |
| M9-TS-1601 | Integration | Parameter types come from their default values (`phase = 0`, `name = "x"`, `on = false`). |
| M9-TS-1602 | Integration | A program class named like an engine handle (`class Scene`) shadows it (was a panic). |
| M9-TS-1603 | Integration | `Text`, `Span`, `Input`, `TextArea`, `Image` are the intrinsic elements. |
| M9-TS-1604 | Integration | A program component named `Text` keeps its meaning. |
| M9-TS-1605 | Integration | Text, numbers, lists and `cond && <x/>` are children of a user component (`children?: ReactNode`). |
| M9-TS-1606 | Integration | `key={4}` and boolean children. |
| M9-TS-1607 | Integration | `onClick={f}` with a 0-parameter function, `undefined` handlers, an optional `onPress` prop, `<Btn />` for all-optional props. |
| M9-TS-1608 | Integration | `style` values that are `number \| string`, or `undefined` (`cursor: on ? "pointer" : undefined`). |
| M9-TS-1609 | Integration | `useState(() => 5)`, `useState(read)`, `useState<number[]>([])` and setters inside an effect. |
| M9-TS-1610 | Integration | `IronStyle` constants and spreads, `React.ReactNode`, a function returning `IronStyle`. |
| M9-TS-1611 | Integration | Event fields `shiftKey`, `ctrlKey`, `code`, `deltaY`, `scrollTop`, `target.value`. |
| M9-TS-1612 | Integration | `useRef<IronElement>(null)` with `.focus()`, `.blur()`, `.scrollTo(x, y)`. |
| M9-TS-1613 | Integration | `static get twice()` reads like a field. |
| M9-TS-1614 | Integration | A component with `return null` and `return <box/>`. |
