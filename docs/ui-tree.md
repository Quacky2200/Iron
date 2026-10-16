# UI tree, styles, layout, text and the draw list

Crate: [iron_ui](../crates/iron_ui/src). Entry type: `UiTree` in
[tree.rs](../crates/iron_ui/src/tree.rs). Pipeline overview: [README.md](README.md). Design
background: [DESIGN.md](../DESIGN.md) section 5. CSS stylesheets are covered in [css.md](css.md).

```
Commit ops --apply_commit--> Node map + dirty sets
                                  |  layout_if_needed()
                                  v
        restyle_dirty -> shape dirty text -> Taffy compute -> absolute Rects
                                  |  draw_list()
                                  v
                          Vec<DrawCmd>  -->  iron_render
```

## Node model

`UiTree<L: LayoutEngine = TaffyLayout>` stores `HashMap<NodeId, Box<Node>>` (boxed because a node
is large). A `Node` holds:

| Field | Meaning |
|---|---|
| `kind` | `"box"`, `"text"`, `"span"`, `"input"`, `"textarea"`, `"#text"` (`TEXT_NODE`), or `"root"` |
| `parent`, `children` | Tree links (ordered) |
| `style: taffy::Style` | Layout properties |
| `visual: Visual` | Paint-only properties ([style.rs](../crates/iron_ui/src/style.rs)) |
| `text` | Characters of a `#text` node |
| `attrs: Attrs`, `state: ElementState` | `className`, `id`, `disabled`, `checked`, `tabIndex`, text field props (`value`, `defaultValue`, `placeholder`, `readOnly`, `rows`), `data-*`; `:hover`/`:active`/`:focus`/`:focus-visible` ([attrs.rs](../crates/iron_ui/src/attrs.rs)) |
| `inline` | Inline `style` declarations, in the order set (later wins) |
| `generated` | `Some(Before/After)` for stylesheet-generated content |
| `vars` | Computed custom properties (`Rc`, shared with the parent when unchanged) |

Ids: `ROOT` is node 0 (always the window size, see `root_style`). React allocates ids from 1 and
never reuses them. Generated `::before`/`::after` nodes use ids from `0x8000_0000`
(`FIRST_GENERATED_ID`), so they can never collide with React's.

## Commits and opcodes

[opcodes.rs](../crates/iron_ui/src/opcodes.rs) defines eight opcodes shared with
[opcodes.ts](../js/react/src/opcodes.ts) (keep them in sync by hand):

| Opcode | Args | Effect |
|---|---|---|
| `CREATE` 0 | id, type | Detached element |
| `SET_STYLE` 1 | id, (prop, value)* | `null` resets a property |
| `APPEND_CHILD` 2 | parent, child | Also *moves* an attached node |
| `INSERT_BEFORE` 3 | parent, child, before | Also moves |
| `REMOVE_CHILD` 4 | parent, child | Destroys the whole subtree |
| `CREATE_TEXT` 5 | id, text | `#text` node |
| `SET_TEXT` 6 | id, text | |
| `SET_ATTRS` 7 | id, (name, value)* | `null` removes |

`UiTree::apply_commit` decodes ops in order via `apply_op`. A bad op yields a `UiError` and is
skipped; the rest still apply, so the tree is always valid (`attach` refuses cycles, text-node
parents and unknown ids). Per-commit bookkeeping is collected in local sets
(`children_changed`, `styles_changed`, `text_changed`, `text_style_changed`) and flushed **once
per commit**: styles are pushed to the layout engine, child lists are re-sent (inline containers
get none, see below), text blocks are marked dirty. Finally `needs_layout` is set.

`SET_STYLE` applies the value immediately through `apply_style` *and* records it in `node.inline`
so the cascade can re-apply it later (see [css.md](css.md)). Values containing `var(` or
`--custom` props are deferred to restyle because they depend on inherited variables.

## Styles

[style.rs](../crates/iron_ui/src/style.rs) `apply_style(&mut Style, &mut Visual, prop, &Value)` is
the single place where a property name (camelCase) and a `Value` become state. Numbers are logical
pixels; strings are CSS values parsed by Taffy's parser (layout) or Iron's own parsers (paint).
Both inline styles and stylesheet declarations (after kebab-case to camelCase) go through it, so
the two paths cannot drift. On error nothing changes (`StyleError`).

- Layout properties land in `taffy::Style` (flex, grid, margin, padding, border widths, inset...).
- Paint properties land in `Visual`: colours (with `currentColor` flags), border radius
  (`Corners<Len>`, px or percent), `shadows`, gradients and images, `filter_blur`,
  `backdrop_blur`, `opacity`, `pointer_events`, `overflow`, `clip_path`, `clip_rect`,
  `scrollbar_width/color`, `cursor`, and `text: TextProps`.
- [paint.rs](../crates/iron_ui/src/paint.rs) holds the parsers: radii, shadow lists, blur filters,
  backgrounds (`linear-gradient`, `radial-gradient`, `url()`), `clip-path` shapes (`ClipPath`
  resolved to a `ClipShape` against the box), `Overflow`.
- [color.rs](../crates/iron_ui/src/color.rs): `Rgba::parse` for hex, `rgb()`, `hsl()`, named colours.
- [text_style.rs](../crates/iron_ui/src/text_style.rs): `TextProps` are the *specified* text
  properties of one node; `ComputedText::child(&TextProps)` applies CSS inheritance.
  `UiTree::computed_text(id)` folds root-to-node. `is_text_prop` lets the tree know a style change
  affects text shaping.

`Visual` has a field-wise `PartialEq` so restyle can detect "no visible change".

## Layout

[layout.rs](../crates/iron_ui/src/layout.rs) hides Taffy behind `trait LayoutEngine`
(`create_node`, `set_style`, `set_children`, `set_measured`, `mark_dirty`, `compute`, `layout`).
`taffy::Style` doubles as the backend-neutral style type. Why the trait: DESIGN.md section 5.3 keeps
Yoga as a fallback, and the JSON fixtures in `tests/layout` are generic over the engine (see
[testing.md](testing.md)). `TaffyLayout` maps `NodeId` to Taffy ids; Taffy caches results, so only
dirty subtrees recompute.

`layout_if_needed()` (tree.rs) is the heart of the frame:

1. `restyle_dirty()`: recompute cascaded styles for dirty nodes (no-op without stylesheets/vars).
2. For each dirty text root, `build_text_layout` shapes a new Parley layout.
3. `layout.compute(ROOT, w, h, measure)`. Measured leaves (text blocks) call back into
   `text::measure`.
4. Walk the tree top-down turning Taffy's parent-relative `NodeLayout` into absolute `Rect`s in
   `self.rects`. Children of a scrolled element are shifted by its (clamped) scroll offset. For
   text blocks, `text::finish` re-breaks lines at the final content width and applies `text-align`.

`NodeLayout` also carries resolved `border`, `padding` and `scroll_max` (Taffy's scrollable
overflow), used for clipping, scrollbars and event data.

## Text

[text.rs](../crates/iron_ui/src/text.rs) wraps Parley (`TextEngine` = `FontContext` +
`LayoutContext<Brush>`; the brush is the run's `Rgba`). Fonts: bundled Inter (variable, upright and
italic) is always registered and put first for the generic sans-serif families; `with_system_fonts`
adds platform fonts for fallback (CJK, emoji). Tests use `UiTree::new` (no system fonts) to stay
deterministic. `register_font` accepts overrides for family/weight/style (like `@font-face`), then
replaces the `LayoutContext` because Parley caches family lookups, including misses.

`iron_ui` enables Parley's `complex-scripts` feature, so word and line segmentation use ICU4X's
dictionary segmenters. Japanese/Chinese word moves (Ctrl+Arrow) and double-click selection follow
dictionary word boundaries, and Thai, Lao, Khmer and Burmese text without spaces wraps between
words (M3-TXT-08). It adds about 3.8 MB to the release binary.

How inline content works:

- A **text root** is a `text` element, or a `span`/`#text` that is *not* inside a text container
  (`is_text_root`; it becomes an anonymous text block).
- Children of `text`/`span` are **not** sent to Taffy as layout children (`is_inline_container`);
  they are flattened by `build_text_layout` into `(string, ComputedText)` runs, each carrying its
  node's inherited style, and handed to `TextEngine::build`. The root is a Taffy leaf marked
  `set_measured` so Taffy asks for its size.
- `text::measure` breaks lines for the available width (`MinContent` uses width 0, `MaxContent`
  none) and returns a ceil'd size. `text::finish` does the final break + alignment.
- `text::draw(layout, origin)` produces a `TextCmd`: `GlyphRun`s (font data, size, variation
  coordinates, synthetic bold/italic, colour, positioned glyphs) plus underline/strike rectangles.
- `dirty_text` is the set of roots to reshape; `text_build_count()` lets tests assert how often
  shaping ran (used in [text.rs tests](../crates/iron_ui/tests/text.rs) to check that unrelated
  changes do not reshape text).

## Draw list

[draw.rs](../crates/iron_ui/src/draw.rs): `Rect` (logical px, with `intersect/union/expand`,
rounded hit testing), `ClipShape` (rect with elliptical corners), and `enum DrawCmd`:

| Command | Meaning |
|---|---|
| `BackdropBlur{rect,radii,blur}` | Blur what is already drawn behind the box |
| `BoxShadow(ShadowShape)` | Outer shadow, before its box |
| `Quad(Quad)` | Background colour, optional `ImageFill`, border, radii |
| `InsetShadow{shadow,border}` | Inside the padding box, over the background |
| `PushLayer{bounds,blur,opacity}` / `PopLayer` | Offscreen group (filter blur, group opacity) |
| `PushClip(ClipShape)` / `PopClip` | Clip following commands |
| `Text(TextCmd)` | Glyph runs and decorations |

`UiTree::draw_list()` runs `layout_if_needed` then `collect(ROOT)` recursively. Per node, in
CSS painting order: `PushClip` for the element's own `clip-path`/`clip: rect()`, `BackdropBlur`,
`PushLayer` if needed, outer shadows (last listed first, so the first shadow ends on top), the
`Quad`, inset shadows, text shadows then text (a text field draws its [editor content](#text-fields) here), then the children, scrollbars and `PopLayer`/`PopClip`.

Decisions worth knowing:

- `display: none` and `opacity <= 0` subtrees are skipped.
- **Group opacity**: a layer is only created if the node has children or more than one own command;
  otherwise the alpha is multiplied into the single command (`scale_alpha`). The layer's `bounds`
  are patched afterwards to the union of what was actually drawn, expanded by `3 * blur`.
- **Clipping** (committed): `clip-path` and legacy `clip` clip the element *and* its children;
  `overflow` clips only the children, to the padding box with inner corner radii (`overflow_clip`,
  `inner_corners`). An axis that does not clip gets an effectively unbounded clip edge
  (`UNCLIPPED`). Hit testing uses the same clip functions, so clipped-away parts cannot be hit.
- `currentColor` is resolved here (`current()`), using the node's computed text colour.

## Scrolling

- **Extent**: `NodeLayout.scroll_max` comes from Taffy's scrollable overflow (`scroll_width()` /
  `scroll_height()`): how far content reaches beyond the padding box. `ScrollInfo::of` derives
  `scrollWidth = clientWidth + max`.
- **Offsets**: `UiTree.scroll: HashMap<NodeId, (f32, f32)>`. `scroll_offset`, `scroll_max`,
  `scroll_axes(id, by_user)` (user: `scroll`/`auto`; scripts also `hidden`) and
  `scroll_to(id, x, y, by_user)`, which clamps to `0..=scroll_max` per scrollable axis, sets
  `needs_layout` and returns whether the offset changed. Taffy's layout is cached; only the
  absolute rects are recomputed.
- **Applied in `layout_if_needed`**: while building `rects`, an element's offset is first clamped
  to the current `scroll_max` (content may have shrunk), then subtracted from the origin passed to
  its children. Hit testing and drawing both read these shifted rects, and the `overflow` clip is
  on the padding box (unshifted), so scrolled content stays clipped and hittable consistently.
  `destroy_subtree` drops offsets of removed nodes.
- **Scrollbars**: `scrollbars(id)` returns up to two `Scrollbar { node, vertical, track, thumb,
  thumb_color, track_color }` in absolute px. Shown for `scroll`, or `auto` with content to scroll,
  unless `scrollbarWidth: none` (thickness: `auto` 8 px, `thin` 5 px). They overlay the content
  (no layout space), inset by `SCROLLBAR_MARGIN` 2 px; when both show, the corner is left free. Thumb
  length is proportional to `client / (client + max)` with a 24 px minimum. `scrollbarColor`
  (thumb, track) is inherited, default a translucent grey thumb and transparent track.
  `collect_scrollbars` draws them as rounded `Quad`s after the children, inside the element's layer
  so they are subject to its opacity but not its overflow clip. `scrollbar_at(path, x, y)` hit-tests
  the track expanded by the margin so thin bars are easy to grab.
- Input handling, wheel chaining and the `scroll` event are in
  [input-and-events.md](input-and-events.md#scrolling-m3c-step-4b).

## Text fields

`<input>` and `<textarea>` are `box`-like elements (styled, bordered, clipped by CSS like any
other) whose text is owned by an `Editor` in [editor.rs](../crates/iron_ui/src/editor.rs):
`UiTree.editors: HashMap<NodeId, Editor>`, created in `CREATE` for `is_editor_kind` and removed with
the node. Each `Editor` wraps a Parley `PlainEditor<Brush>` (text, selection, IME composition; DESIGN
D7) plus `multiline`, `rows`, an optional placeholder layout, the field's own `scroll` and the last
content-box `size`.

- **Sizing**: a field is a *measured leaf* like a text block (`set_measured`); the measure callback
  falls back to `Editor::intrinsic_size`: `known_width` or a default (`INPUT_WIDTH` 160 px,
  `TEXTAREA_WIDTH` 240 px) by one line (textarea: `rows`, default `TEXTAREA_ROWS` 2) of the editor's
  line height. Explicit `width`/`height` win. Attribute changes mark the node dirty.
- **Attributes** (`SET_ATTRS` in `apply_commit`): a `value` attribute (controlled) replaces the text
  if it differs (`Editor::set_value`; skipped while composing; the caret goes to the end at the next
  layout, like browsers). `defaultValue` only fills a field that never had a value or default.
  `placeholder` and `rows` are passed on. `readOnly` and `disabled` are read from `Attrs` when keys
  arrive.
- **Style sync**: `refresh_editors` runs at the end of `layout_if_needed`. For every field it takes
  `computed_text(id)` (so `color`, `font*`, `line-height`, spacing come from the cascade), calls
  `Editor::set_style` (restyles only if the `ComputedText` changed, which also drops the placeholder
  layout), computes `content_box(id)` (rect minus border and padding) and calls `Editor::refresh`:
  sets the wrap width (textarea only; an input never wraps), moves the caret to the end if a script
  just set the text, refreshes the Parley layout, builds the placeholder layout in the text colour at
  half alpha, and runs `keep_caret_visible`.
- **Caret in view**: `Editor::scroll` is the field's own offset, one axis only (x for input, y for
  textarea), adjusted to show the caret rectangle and clamped to `content + caret width - view`.
  It is not a CSS scroll offset and creates no scrollbars.
- **Blink**: `UiTree::set_time(now_ms)` (the app calls it every frame) feeds `caret_on`: on for the
  first `CARET_BLINK_MS` (530 ms) after the last `touch` (any key, press, paste, compose), then
  alternating. Editing calls set `needs_layout`.
- **Drawing** (`editor_cmds`, after the field's own quad and shadows, before children): `PushClip`
  to the padding box with inner corner radii; if focused and not disabled, selection quads
  (`SELECTION`, blue at 0.4 alpha); the text (or the placeholder while empty) as one `Text` command
  at the content origin minus `scroll`; the caret quad (1.5 px wide, text colour) when in the "on"
  half of the blink; `PopClip`. Unfocused fields show no selection and no caret.
- **Cursor**: `auto` is `text` over fields too.
- **API used by [Interaction](input-and-events.md#text-fields-m3c-step-5)**: `is_editor`,
  `editor_value`, `editor_selection`, `editor_key` (refuses disabled), `editor_insert` (refuses
  disabled and read-only), `editor_pointer` (absolute px to content-box px), `editor_compose`,
  `editor_end_compose`, `editor_ime_area` (caret or pre-edit box in absolute px, shifted by the
  field scroll).

## Gotchas

- `needs_layout` is a coarse flag: any commit, state change or scroll sets it, and Taffy's own
  cache keeps the recompute cheap. Hover changes call `set_state`, which records a `Key` change so
  the restyle only touches nodes the stylesheets depend on.
- `hit_test` also calls `layout_if_needed`, so it is safe right after a commit.
- The `uses_vars` flag is set once an inline style uses variables and never cleared.
