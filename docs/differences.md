# Differences from browsers

Where Iron's CSS and rendering knowingly behave differently from a browser (or from the spec),
and why. **Read this before changing one of these areas**: each entry is a deliberate trade-off or
a known gap, not an accident. When you fix one, delete its entry; when you add one, say why.

## Script API vs Unity, s&box and C# (M5.a)

The TS API follows Unity / s&box names and units (`Vector3`, `Quaternion`, Euler degrees,
`Debug.log`, `DateTime`, ...). Where it can't match:

| Area | Iron | Unity / s&box / C# | Why |
|---|---|---|---|
| Handedness | Right-handed, y up, forward is -z; positive pitch (about +x) tips forward **up** | Unity: left-handed, forward +z, positive pitch looks down. s&box: z up, forward +x | glam, rapier and wgpu are right-handed; Euler order matches Unity's (z, then x, then y) |
| Value types | Immutable classes: `v.add(w)`, `v.scale(s)`; no operator overloading | `v + w`, `v * s`; mutable structs | JS has no operators or structs; immutability stops `transform.position.x = 1` silently doing nothing |
| Time resolution | Milliseconds everywhere (`DateTime`, `TimeSpan`, `Time`); `f` format digits beyond three print as zeros; `TimeSpan.parse` drops sub-millisecond digits | 100 ns ticks | JS `Date` and engine time are millisecond clocks; no µs/ns timing, so don't profile with them |
| Months | `DateTime.month` is 1 to 12 | Same in C# (JS `Date` uses 0 to 11) | Follows C# |
| Formatting | `DateTime.toString` uses the invariant culture (`MM/dd/yyyy`, English names) | Current culture | No culture data in the runtime |
| `Time.timeScale` | Scales `Time.deltaTime`, `Time.time` and the `dt` components get; physics isn't scaled yet | Scales physics too | Physics time scaling comes with the Rigidbody component (M5.a2) |
| Hook names | `onUpdate` (per frame), `onFixedUpdate` (60 Hz), `onLateUpdate`, `onPreRender`, `onStart`, `onDestroy` | Unity: `Update`, `FixedUpdate`, ...; s&box: `OnUpdate`, `OnFixedUpdate`, ... | JS naming; same order and meaning |
| Screen coordinates | Screen points are UI pixels from the **top-left** (like pointer events); viewport points and `Camera.rect` are 0 to 1 from the top-left too | Unity: pixels and viewport from the bottom-left | Matches the UI and pointer events, so `screenPointToRay(event.clientX, ...)` just works |
| World panel input | Screen UI takes the pointer wherever it has an element; full-screen layout boxes need `pointerEvents: none` to let the pointer through to world panels | Unity: only graphics marked as raycast targets block; s&box: similar to Iron | The CSS way, no extra flags |
| Transform setters | Each set sends an op; world values are worked out in JS from the parent chain (shear from non-uniform scale under rotation is dropped) | Native | Rust owns the world; scripts mirror locals |

## Intl backends (compiled TypeScript)

| Area | Browser / bun | Iron today | Notes |
|---|---|---|---|
| ICU | ICU4C: every locale, `availableFormats`, all calendars | ICU4X `compiled_data`: every locale's data is linked (no trimming yet) | Planned: baked data for a main-market locale list (`icu-lite`, default) and an optional `icu-full` cargo feature using `rust_icu`/ICU4C for exact results; see `docs/research/iron-ts-intl-datetime.md` and `tools/iron_ts/TODO.md` |
| Date component options (`{ year, month, day, weekday, hour... }`) | Matched against CLDR `availableFormats` by ICU's pattern generator | Mapped to the nearest ICU4X field set with the requested widths applied (`toLocale*String`, `Intl.DateTimeFormat`); rare combinations differ or are rejected, see "Intl date formatting" below | Exact only with `icu-full` or a generated `availableFormats` table |
| Locale choice | Best fit among the locales the engine has | The first tag in the list, root collation/formatting when the locale has no data | The per-game locale list is not wired from `package.json` yet |

## Intl date formatting (compiled TypeScript, `icu-lite`)

What differs from the reference engine today (measured by `bench/datefmt.ts`, 15 locales, five zones;
the columns below are left out of that test). The cause is either the structural one (ICU4X formats
from fixed field sets, ICU4C picks a pattern from `availableFormats`) or a CLDR version difference.

| Case | Browser / bun | Iron | Cause |
|---|---|---|---|
| `{ month: "short", day: "numeric" }`, `{ weekday: "short", month: "short", day: "numeric" }` in de and ja | `19. Okt.`, `10月19日` | `19.10.`, `10/19`: the month name width is not applied to a numeric field set | structural |
| `{ hour: "numeric", hour12: true }` in de; midnight in ja | `6 AM`, `午前0時` | `6 Uhr morgens`, `午前12時` | structural / CLDR |
| Numeric month, day and hour padding | per locale from `availableFormats` | looked up in a table generated from the reference engine for 31 locales (`intl_pad.rs`); other locales keep ICU4X's padding | structural (table is the stand-in) |
| `dateStyle: "short"` in pl; times in es (`6:30` against `06:30`) | CLDR of the engine | CLDR 48 of ICU4X | CLDR version |
| ja, ko and zh `timeStyle` full/long | `6時30分00秒`, `오전 6시 30분 0초`, `GMT-4 06:30:00` | `6:30:00`, `오전 6:30:00`, `GMT-406:30:00` | CLDR version |
| fr `timeStyle` long | `UTC−4` (U+2212) | `UTC-4` | CLDR version |
| `timeZoneName: "shortGeneric"` / `longGeneric` in en-GB | `United Kingdom Time` | `UK Time` | CLDR version |
| en-GB with `timeZoneName` | `3:30:00 GMT-7` | `03:30:00 GMT-7` | structural |
| Zone names for old instants (Lord Howe 1970) | `Australian Eastern Standard Time` | `Lord Howe Standard Time` | zone history in the data |
| U+202F (narrow no-break space) before AM/PM and in some patterns | plain space | replaced by a plain space, as engines do | compatibility |
| `era`, `dayPeriod`, `calendar`, `numberingSystem`, `formatToParts`, `formatRange`, `resolvedOptions` | supported | `RangeError` / not built | open |
| Default calendar other than Gregorian (th, fa, ar, he, ps) | that calendar's years and names | ICU4X's calendar output, the width adjustment is skipped | open |

Exact results for every option and locale need `availableFormats`: the optional `icu-full` build
(ICU4C through `rust_icu`) or a generated table; see `docs/research/iron-ts-intl-datetime.md`.

## Compiled TypeScript (iron_ts)

The ahead-of-time compiler gives JavaScript's answer, with these exceptions (see
[iron-ts.md](iron-ts.md)):

| Area | Iron | JavaScript | Why |
|---|---|---|---|
| `Object.freeze` | Returns the object; later writes are not rejected | Writes fail silently (or throw in strict mode) | Structs have no frozen flag |
| `typeof` of a nullable | A null value reports `"undefined"`; only the literal `typeof null` is `"object"` | `"object"` for null | The compiler does not tell null from undefined |
| `JSON.parse` | Parses into the static type: fields the type lacks are dropped; a shape mismatch (wrong kind, missing property, string outside a literal union) throws a `TypeError` after the text proved to be valid JSON | Builds whatever the text holds | Values are typed structs, not dictionaries |
| `JSON.stringify` | Fields in declaration order (array-index keys first), not source order; only `?` fields are omitted when null (a `T \| undefined` field prints `null`); no replacer, reviver or `toJSON` | Own properties in insertion order; `undefined` properties omitted | Keys are fixed at compile time |
| `RegExp` groups that did not take part | `m[i]` is null, so `m[1].length` dereferences null (a fault) | `undefined`; `m[1].length` throws a `TypeError` | The compiler does not tell null from undefined |
| `RegExp` `exec` `groups` | `groups === null` is true when the regex has no named groups | `groups` is `undefined` | Same: null stands for undefined |
| `RegExp` match as a tuple | Tuple types are not supported; a match is a `RegExpMatchArray` object seen as its string array | An array with extra properties | No tuples in the type system |
| `RegExp` replacement functions | The parameters after the match (groups, named groups) need the regex's groups known at compile time (a regex literal, `new RegExp("literal")`, or a function-local `const` from one); module-level `const` regexes do not carry them; rest parameters are a compile error | Any function, any regex | The function is inlined and its arity is checked against the groups |
| `matchAll` | Eager: an array of match objects, from a copy of the regex | A lazy iterator | No iterator objects; `for...of`, spread and `map` still work |
| Generator return type | Must be written (`Generator<T>`, `IterableIterator<T>`, ...) | Inferred | The yield type picks the generated classes |
| `yield*` | Only as a statement; a `yield` inside another `yield`'s operand is an error | An expression | Rewritten to a loop of yields |
| `return value` in a generator | Dropped: the final result is `{ done: true, value: <zero> }` | `value` is the returned value | One `IteratorResult` type per yield type |
| `generator.throw()`, `return()` | No `throw()`; `return()` does not run `finally` blocks | Both supported | Not built yet |
| `IteratorResult.value` when done | The zero value (0, null) | `undefined` | The compiler does not tell null from undefined |
| `Iterable<T>` annotations | Accept generators only (not arrays, strings or other iterables) | Any iterable | Maps to the generated classes |
| `Symbol()` values | Not built; only `Symbol.iterator` as a method name | Available | Needed only for the iterator protocol |
| Coroutines on WebAssembly | Not available | n/a | Not a target in the next year |
| `Date()` without `new` | Not supported | Returns the current time as a string | Use `new Date().toString()` |
| `Date.parse` of exotic legacy strings | A few return NaN (a bare number such as `"0"`, `"Oct 19"` without a year) | Engines guess a year or a value | The legacy tokenizer is strict where engines are lax; ISO and the common legacy forms match bun |
| `Date` constructor arguments | `new Date(y, m, ...)` takes plain arguments, at most seven | Spread allowed | Lowered to a fixed-arity runtime call |
| `Date` in typed JSON | Not supported in `JSON.stringify` / `JSON.parse` types yet | Stringifies through `toJSON`; parse gives a string | The JSON descriptors have no `Date` kind |
| `Date` local time | Uses jiff's tz database; years beyond +-9999 are mapped by 400-year Gregorian cycles; a skipped local time uses the offset before the jump | Engines' own zone code | Same results as engines for the zones checked (five, against bun) |
| `Intl.Collator` options and locales | The options argument must be an object literal with literal values, and locales a string or an array of strings, both read at compile time; anything else is a compile error | Any expression | The options become a `key=value;` string for the runtime |
| `Intl.Collator` members | Only `new Intl.Collator(...)` and `.compare`; no `resolvedOptions` or `supportedLocalesOf` | The full API | Step I1 of the `Intl` work |
| Locale data | ICU4X (CLDR) data for every locale is linked in; an unsupported locale id falls back to root collation, not the default locale; the CLDR version can differ from the engine's, so rare orderings may differ | The engine's own ICU, which falls back to the default locale | No data trimming yet; checked against bun for en, de, sv, tr, ja, zh |
| Default locale | `en-US` until the host calls `ts_rt_intl_set_locales`; the game's `package.json` list is not wired to it yet | The environment's locale | Needs `iron_app` |
| `Intl` beyond `Collator`, `NumberFormat`, `DateTimeFormat`, `PluralRules`, `ListFormat` and `RelativeTimeFormat` | `Locale`, `DisplayNames` and `Segmenter` are not built (`Segmenter` needs iterators) | Available | Later steps of the `Intl` work |
| `PluralRules`, `ListFormat`, `RelativeTimeFormat` members | Only the constructor and `select` / `format`; no `supportedLocalesOf`, `resolvedOptions`, `formatToParts`, `formatRange` or `selectRange`; `ListFormat.format` takes a `string[]`, not an iterable; options and locales are read at compile time like the collator's | The full API | Step I4 of the `Intl` work |
| `Error` `cause` | `cause: Error \| null`: only an `Error` can be a cause; an absent cause is null, not told apart from `undefined` (test with `== null`) | Any value; `undefined` when absent | Typed field; null stands for undefined |
| `Error.stack`, `captureStackTrace` | Not built | Available | No stack capture in compiled code |
| `Intl.NumberFormat` options and locales | Same as the collator: an object literal with literal values, locales a string or an array of strings, read at compile time; `Number.prototype.toLocaleString` takes the same arguments | Any expression | Shared `options_spec` |
| `Intl.NumberFormat` members | Only `new Intl.NumberFormat(...)` and `.format`; no `formatToParts`, `formatRange`, `resolvedOptions` or `supportedLocalesOf`; Number values only (no BigInt, no strings) | The full API | Step I2 of the `Intl` work |
| `NumberFormat` `style: "unit"`, compact currency | `style: "unit"` raises a `RangeError`; `notation: "compact"` with `style: "currency"` is not built | Supported | Not yet done |
| `NumberFormat` `useGrouping: "always"` | Does not force grouping in locales that group only from five digits (es, pl, pt-PT) | Forces grouping | ICU4X's decimal formatter treats `always` as `auto` |
| `NumberFormat` NaN text | `"NaN"` in every locale except `ar` and `ru`, which are localized | Localized per locale | Only those two locales were handled |
| `NumberFormat` currency code after the number with NaN or infinity | A space is put before NaN or infinity (`KWD NaN`) | `KWDNaN` | The placeholder swap does not copy the engine's automatic spacing rule |
| Typed arrays | `Int8Array` .. `Float64Array` and `Uint8ClampedArray` own their memory. No `ArrayBuffer`, `DataView`, `subarray`, `buffer` or `new X(buffer, offset, length)` (a compile error), no `BigInt64Array` / `BigUint64Array`; a read past the end gives 0, not `undefined`; no `constructor` property; `X.of` and `Array.from(typed)` are missing | Views share an `ArrayBuffer`; out-of-range reads give `undefined` | Typed arrays are a header plus raw data, with no buffer object to share; missing values are 0 as for `number[]` |
| BigInt | `bigint` is a leaf object on `num-bigint`. Not built: `BigInt64Array` / `BigUint64Array`, `toLocaleString`, bigints as `Map` / `Set` keys, bigints in `JSON` types (a compile error). Mixing a bigint with a number in arithmetic, unary `+` and `>>>` are compile errors. | Mixing, unary `+` and `>>>` throw a `TypeError` at run time; `&&` / `\|\|` give an operand | Types are checked at compile time |
| `Object.keys/values/entries`, `in`, `hasOwn` | Keys are the struct's fields, fixed at compile time; `hasOwn` and `in` need a string literal key | Keys are looked up at run time | The program is typed and closed; run-time dictionaries need `Map` |

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

## Drag and drop (M5.e)

| Area | Iron | Browsers / spec | Why |
|---|---|---|---|
| Drag image | The element drawn live at 0.6 opacity, so it must stay rendered | A snapshot taken at `dragstart` | No offscreen snapshot of a subtree yet |
| `getData` | Works in every drag event | Only in `drop` (protected mode elsewhere) | The data stays in the scripts, so there's nothing to protect |
| `dragover` | Sent when the pointer moves | Also every ~350 ms while still | Nothing changes without movement here |
| `dropEffect` / `effectAllowed` | Kept for scripts; the cursor only shows accepted (`grabbing`, `copy` for files) or not (`no-drop`) | Pick the cursor and filter drops | Enough for the editor; easy to add |
| `dragenter` | Doesn't need to be prevented | Must be prevented (with `dragover`) to accept | Fewer handlers for the same result |
| System drags | Files only (paths); on Windows the position comes from `GetCursorPos`, elsewhere the last pointer position | Files, text, URLs, live position | winit 0.30 reports files without positions |

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
