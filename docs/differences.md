# Differences from browsers

Where Iron's CSS and rendering knowingly behave differently from a browser (or from the spec),
and why. **Read this before changing one of these areas**: each entry is a deliberate trade-off or
a known gap, not an accident. When you fix one, delete its entry; when you add one, say why.

## SVG (M3d step 5.1)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Rendering | Rasterized on the CPU with resvg at the drawn size (cached; re-rasterized when content, style or size changes) | Native vector rendering | Full SVG support quickly; GPU paths are a TODO ("Later") |
| Animation | CSS transitions/animations on SVG content work (each frame re-rasterizes); SMIL (`<animate>` ...) is ignored; `d` only morphs between paths with the same commands | SMIL and path morphing | resvg has no SMIL; see TODO "Later" |
| Hit testing | The whole `svg` box; shapes inside aren't targets and `pointer-events` on shapes is ignored | Per shape | Content isn't laid out by Iron |
| `<style>` inside SVG, `class` in markup | Ignored; Iron stylesheets style SVG content instead | Supported | Styles are resolved by Iron's cascade |
| Text | The bundled Inter only | System and web fonts | resvg gets one font database |
| Images in SVG (`<image>`), filters | `<image>` isn't loaded from the game folder; SVG filters render via resvg | Supported | Not wired yet |

## Stylesheet authoring (M3d group 10)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Nested at-rules | `@media` inside a rule works; other at-rules there (`@starting-style`, `@supports`, ...) are skipped with a warning | Allowed | Top-level `@starting-style` works |
| `@import` conditions | Media queries work; imports with `supports()` or `layer()` are skipped; `@import` anywhere in a sheet still loads (browsers ignore late ones) | Conditional and layered imports | No `@supports` or cascade layers |
| `@media` | Size, aspect ratio, orientation, resolution, colour scheme, reduced motion, pointer/hover; `em` in queries is 16px; `prefers-reduced-motion` stays `no-preference` and `pointer` stays `fine` until the app reports otherwise; unknown features never match | Many more features (`color`, `display-mode`, ...) | The ones games use |
| Nested rule order | Flattened after the parent rule; declarations written after nested rules still belong to the parent | Same (the spec's "nested declarations" rule) | — |

## Game extensions (M3d group 9)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `sound-in` / `sound-out` | Not supported | s&box only (not CSS) | CSS stays about display; play UI sounds from event handlers |
| UI scale | `setUiScale` zooms all screen UI (layout in UI pixels) | CSS `zoom` per element | One switch for the whole HUD, like s&box's ScreenPanel |
| `background-image-tint`, `border-image-tint` | s&box properties, no CSS equivalent; multiply image pixels by a colour; gradients and masks aren't tinted | — | Common in game UI (one white icon, many colours) |

## Scrolling extras (M3d group 8)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `scrollbar-gutter: stable both-edges` | Reserves the end edge only | Both edges | Taffy reserves one side |
| `scrollbar-gutter` on the x axis | Only vertical scrollbars get a gutter | Same (gutters follow the block axis' bar) | — |
| `overscroll-behavior: none` | Same as `contain` | Also disables bounce / pull-to-refresh | Iron has no overscroll effects |

## Transitions (M3d step 7a)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| What interpolates | Computed values as text, token by token: same-shaped values only (same functions, same number of shadows/layers); mixed units via `calc()` | Typed interpolation per property (e.g. transforms with different functions go through matrices) | One generic interpolator covers most real uses |
| `auto`, `currentColor`, keywords | Change at once | Some interpolate (`currentColor` in colours, `interpolate-size`) | Not representable as plain tokens |
| Colour space | Premultiplied sRGB | Same for legacy colours; Color 4 spaces otherwise | Matches what games usually expect |
| Shorthands | `transition: background 1s` also covers `backgroundColor` and the reverse (name prefixes) | Exact shorthand expansion | No longhand tables |
| `transition-behavior: allow-discrete` | Accepted, no effect | Discrete properties flip at 50% | Rarely needed |
| Inherited values | A transition on a parent's `color` animates; children follow through inheritance | Same | — |
| Animations | `@keyframes` by name from any stylesheet (the last wins); no `animation-composition`, `animation-timeline`, `animation-range` or the Web Animations API | Full | Enough for UI motion |
| `@starting-style` | Top-level blocks only; applies to new elements and to an element whose own `display` leaves `none` (not to its descendants) | Also nested in rules, and descendants of a newly shown ancestor | Nesting comes with group 10 |
| `:outro` | s&box's exit state (no standard equivalent): removed elements linger while it animates; structural selectors still count them | — | Lets exit transitions finish without reconciler work |
| Cost | Each moving element is restyled (selector matching included) every frame | Browsers update animated values without a full restyle | Simple; fine for UI-sized numbers of elements |

## Dialogs (M3d step 5j)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Opening | `open` and `modal` attributes (React state) | `show()` / `showModal()` methods; `modal` isn't an attribute | Declarative fits React; scripts own the state |
| Closing | Escape / backdrop clicks send `cancel` then `close`; the script clears `open` | The browser closes the dialog, then fires `close` | The engine never changes React-owned attributes |
| UA style | Centred with `left/top: 50%` + `translate: -50% -50%`; no border, padding or background | `margin: auto`, `fit-content`, a border, padding and a canvas background | Iron's elements are unstyled; authors setting `translate` or insets take over the centring |
| `:modal`, `popover`, `returnValue`, `inert` | No `:modal` (use `dialog[modal]`), no popovers, no return value; outside the modal only focus and pointer input are blocked | All supported | Not needed yet |

## Cursors (M3d step 5i)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Disabled fields | `not-allowed` | The arrow (`default`) | Clearer in game UI; override with `cursor` |
| Image formats | PNG, JPEG, `.cur` | Also SVG, `.ani`, `image-set()` | SVG comes with step 5.1; animated cursors aren't planned |
| Keyword shapes | winit's nearest cursor per platform | Each browser's own set | Some keywords (e.g. `vertical-text`, `alias`) fall back to similar shapes on Windows |

## Selection (M3d step 5h)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Extent | A selection stays inside one text block (one `text` element and its spans) | Selections run across elements | Each block has its own Parley layout; cross-block ranges need a document-wide position model |
| `::selection` | Only `color` and `background(-color)`; each comes from the nearest element (the block or an ancestor) with a `::selection` rule that sets it; spans inside the block can't restyle their part | Highlight inheritance per element, including spans | Enough for theming; per-span styles need per-run highlight drawing |
| `user-select: contain` | Same as `text` | Keeps the selection inside the element | Selections can't leave a block anyway |
| Ctrl+A outside fields | Does nothing | Selects the whole page | No page-wide selection (see Extent) |
| Default | Text is selectable, as in browsers | Same | Set `userSelect: "none"` on menus and HUDs where double-clicks shouldn't highlight |

## Lists (M3d step 5g)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Outside markers on `text` items | Drawn inline, like `inside` | Hang outside the line box | A text element's children are inline content; the marker can't be positioned outside it. Wrap the text in a box item for hanging markers |
| Elements | `list` + any element children; no `ul`/`ol`/`li`/`p`/`h1` | HTML elements | Iron's element set stays small; use `list` and `text` with classes |
| `list-style: none` | The list's default 24px indent goes too (flush left), unless the author sets its start padding | The UA's `padding-left: 40px` stays; sites reset it by hand | What most pages want; set `paddingLeft` to keep an indent |
| Counters | Markers number items by position; no `counter-reset` / `counter-increment` / `counter()`, `start` or `reversed` | Full CSS counters | Not implemented yet |
| `::marker` | All properties apply | Only a few (colour, font, content, ...) | More permissive |

## Transforms (M3d step 5e)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `transform-style: preserve-3d` | Children's 3D transforms combine before flattening (each plane lands exactly), but they still paint in tree order | Planes in a 3D context are depth-sorted and intersect | No depth buffer or plane splitting; order elements by hand (or with `z-index`) |
| Text under rotation, skew or perspective | Glyph masks are sampled nearest-neighbour in the element's plane | Re-rasterized glyphs | Turned text looks slightly jagged; fine for motion, softer than a browser at rest |
| Anti-aliasing under scale/perspective | One window pixel of feathering is computed in the element's plane | Exact | Strongly scaled-down or steep planes get softer or harsher edges |
| `visibility: collapse` | Same as `hidden` | Removes table rows/columns | Iron has no tables |
| `position: fixed` inside a transformed element | Still fixed to the window | The transformed ancestor contains it | Fixed nodes paint at the top level |
| Points behind the viewer | A plane crossing behind the eye (w ≤ 0) can draw wrongly | Clipped at the eye plane | Rare with sensible `perspective` values |

## Effects (M3d group 5)

| Area | Iron | Browsers / spec | Why, and what fixing it would take |
|---|---|---|---|
| `filter` order | Always blur, then all colour functions (folded into one matrix), then `drop-shadow()`s, whatever order is written | Each function applies in written order | One layer pass, one matrix. A strict chain needs a pass (or an intermediate layer) per function. `brightness(0.5) invert(1)` and `invert(1) brightness(0.5)` *are* ordered correctly: it's only blur and shadows that move |
| Several `drop-shadow()`s | All drawn beneath the content, the first listed on top | Each shadows the result so far (a later shadow also shadows earlier shadows) | Rarely visible; would need one layer per shadow |
| Mask modes | The top mask layer's `mask-mode` applies to every layer | Per layer | Masks render into one texture; per-layer modes need one texture per mode, or converting luminance to alpha while drawing each layer |
| `mask-composite` | Layers combine with normal alpha blending (≈ `add`) | `add`, `subtract`, `intersect`, `exclude` | Not implemented |
| `mask-clip` / `mask-origin` | Always the border box | Configurable | Not implemented |
| Soft fades and translucency | Compositing (layers, masks, opacity, blending of translucent colours) happens in **linear light** | Browsers blend in sRGB | The whole renderer works in linear light; gradients and blend modes are interpolated in sRGB as CSS says. Visible effect: the faint end of a fade (alpha 0.025 over black) is about 42/255 grey instead of a browser's ~6 |
| Filter colour space | Colour filters work on sRGB values (like browsers) | Spec default is linearRGB (`color-interpolation-filters`) | Matches what browsers actually do |

## Backgrounds and borders (M3d group 3)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `background-origin` | Images and gradients position against the **border box** | Default `padding-box` | Not implemented; `background-clip` is |
| `background-repeat: space` for gradients | The gaps aren't kept (tiles repeat edge to edge) | Gaps as for images | Gradient tiles repeat in the shader without the gap logic images have |
| `border-style` default | `solid`: a width alone draws a border (as in s&box) | `none`: no border without a style | Matches s&box authoring; the `border` shorthand still follows CSS (no style, no border) |
| Gradients in `border-image` | Pieces stretch; `repeat`/`round`/`space` don't tile them | Tile like images | Image pieces tile; gradient pieces don't yet |
| `clip-path: url()` | Unsupported | References an SVG `<clipPath>` | Iron has no SVG documents |
| Blurred shadows on elliptical corners | Drawn with circular corners (the mean radius) | Elliptical | The analytic blurred-box shader is circular-only |
| Background colour with `background-clip: text` | Dropped | Clipped to the text | Only the topmost gradient layer fills text |

## Layout (M3d group 2)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `box-sizing` default | `border-box` (as in Taffy and s&box) | `content-box` | Matches s&box |
| `z-index` | Orders siblings only; every z-indexed element is its own stacking context | Stacking contexts nest; z-index can lift an element above its parent's siblings | Simpler model; `isolation` is accepted and has no effect |
| `position: fixed` | Ignores ancestor opacity, filters and clips (but not `display: none` / opacity 0) | Ancestors with transforms/filters contain it | Fixed nodes paint at the top level |

## Text (M3d group 4)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| `text-decoration-skip-ink` | Parsed, no effect | Underlines skip descenders | Needs glyph outlines in the decoration pass |
| `text-overflow: ellipsis` | Cuts whole glyphs; uses the block's style for the `…` | Same, but uses the style at the cut | Close enough for UI text |
| `background-clip: text` | Fills the element's own text only, with the topmost gradient layer | Descendants' text too, all layers | |
| `text-decoration` | Inherited (it propagates visually anyway) | Not inherited, but propagates | Same result in practice |
