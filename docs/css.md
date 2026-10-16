# CSS: stylesheets, selectors, cascade and invalidation

Files: [css.rs](../crates/iron_ui/src/css.rs) (parsing), [cascade.rs](../crates/iron_ui/src/cascade.rs)
(matching, cascade, rule index, dependencies), and the restyle code in
[tree.rs](../crates/iron_ui/src/tree.rs). Plan and backlog: TODO.md "M3b" and "M3d". Prefer
standard CSS over s&box extensions (CLAUDE.md).

## Overview

```
game.css / addStyleSheet(css)
   |  parse_stylesheet (cssparser)         css.rs
   v
Stylesheet { rules: [selectors + declarations], font_faces, warnings }
   |  UiTree::add_stylesheet_with_base: rebase url()s, drop invalid declarations
   v
UiTree.sheets  --restyle_everything-->  RuleIndex + Deps  (cascade.rs)
   |
   |  per dirty node: matched_declarations -> ordered declarations -> apply_style
   v
Node.style (Taffy) + Node.visual
```

Inline `style` props and stylesheet rules end up in the same `apply_style` call
([ui-tree.md](ui-tree.md)), so every property works in both.

## Parsing ([css.rs](../crates/iron_ui/src/css.rs))

- Uses `cssparser` for syntax and Servo's `selectors` crate for selector parsing. `IronSelectors`
  is the `SelectorImpl`; `CssString` is the string newtype for names and values (with a stable
  `precomputed_hash`, which the ancestor bloom filter needs).
- `parse_stylesheet(css) -> Stylesheet`. Rules keep declarations as **CSS text**
  (`Declaration { prop, value, important }`), with `prop` converted by `camel_case` (kebab to
  camelCase; `--custom` names kept). Values are re-parsed later by `apply_style`, which is why
  they can contain `var()`.
- Failures never abort the sheet: an invalid selector, unsupported at-rule (`@media`, `@keyframes`
  currently), or unknown pseudo-class yields a message in `Stylesheet.warnings` and the next rule
  still applies. `@font-face` becomes a `FontFace`; its `src` is the first `url(...)`.
- Supported selector features (all parsed by `selectors`): type, class, id, attribute selectors,
  combinators (descendant, `>`, `+`, `~`), `:first-child`/`:last-child`/`:only-child`,
  `:nth-child(.. of S)`, `:nth-last-child`, `*-of-type`, `:root`, `:empty`, `:not`, `:is`, `:where`,
  `:has` (relative selectors). Iron-specific pseudo-classes (`PseudoClass`): `:hover`, `:active`,
  `:focus`, `:focus-visible`, `:focus-within`, `:enabled`, `:disabled`, `:checked`, `:blank`.
  Pseudo-elements: `::before`, `::after` only.
- Element type selectors use the Iron element names: `box`, `text`, `span`, `root`.

## Matching ([cascade.rs](../crates/iron_ui/src/cascade.rs))

`NodeRef<'a, L>` implements `selectors::Element` over `UiTree`: parent/sibling navigation skips
`#text` nodes and generated nodes (`is_element`), `has_class`/`has_id`/`attr_matches` read
[Attrs](../crates/iron_ui/src/attrs.rs), and `match_non_ts_pseudo_class` reads `ElementState`
and attrs. `:focus-within` scans the subtree for a focused node.

### Cascade order

`matched_declarations(tree, sheets, index, id, target, caches)` returns two lists, `normal` and
`important`, each sorted by `(specificity, (sheet, rule, declaration))`, lowest priority first.
A rule's specificity is the best among its matching selectors. `restyle` then builds the final
order:

```
normal rule declarations  ->  inline style declarations  ->  !important rule declarations
```

and applies them in that order (later wins). This gives "inline beats rules, `!important` beats
inline", matching browsers. Notes: there is no origin/layer system, no general CSS-wide keywords
(`inherit`/`initial`), and a value like `0.5` is turned into a number by `css_value` so `opacity: .5` equals
the inline form `opacity: 0.5`.

### Rule index

`RuleIndex::build` files each selector once, by the **rightmost compound's** most selective key:
id, else class (first one), else type, else "universal". Selectors with a pseudo-element go to a
separate `pseudo` list. For an element, `candidates(kind, attrs)` gathers universal + type bucket +
id bucket + every class bucket, so most elements test only a few selectors. Candidates are
deduplicated and sorted back into rule order.

## Invalidation and restyle

Goal: after one class or state change, restyle only what can be affected (M3-BEN-01 targets
< 1 ms on a 2,246-node tree).

**Keys.** A `Key` is something a selector can depend on that changes at runtime: `Class(name)`,
`Id`, `Attr(name)`, `Hover`, `Active`, `Focus`, `FocusVisible`, `Disabled`, `Checked`.

**Deps.** `Deps::add_sheet` walks every selector right-to-left (`iter_raw_match_order`), tracking
the *position* of each compound relative to the subject:

| Position | Reached by | Set | Affects when the key changes |
|---|---|---|---|
| Subject | the rightmost compound | (not stored) | only the node itself |
| Ancestor | left of `>` or descendant combinator | `deps.ancestor` | the node's subtree |
| Sibling | left of `+` / `~`, or inside `:nth-child(.. of S)` | `deps.sibling` | its later siblings (and their subtrees) |
| Has | inside `:has()` | `deps.has` | its ancestors |

Plus flags: `focus_within`, `uses_has`, and `structural` (any of `:nth-*`, `:first-child`, `+`, `~`,
`:empty`, `:blank`, `:has`). `:not/:is/:where` recurse with the same position.

**Recording changes.** Attribute edits (`SET_ATTRS`, `attr_keys_changed`) and state changes
(`UiTree::set_state`) push `(node, Vec<Key>)` into `key_changes`. Child-list changes put the
parent in `structure_changed`.

**Resolving them.** At the start of `layout_if_needed`, `restyle_dirty`:

1. Returns early (cheap) when there are no stylesheets and no variable usage: inline styles were
   already applied directly in `apply_op`.
2. `invalidate_keys` marks the node, then its subtree / later siblings / ancestors according to
   which `Deps` set contains a changed key. `invalidate_structure` (only if `deps.structural`)
   marks the parent and its children's subtrees (and ancestors when `:has` is used).
3. Walks the tree in document order with an explicit stack, calling `restyle(id)` for each node in
   `style_dirty` and stopping once the set is empty. Document order matters: parents are restyled
   before children, so inherited variables are ready.

`restyle` recomputes `style` and `visual` from scratch for that node, sets the layout style only
if it changed, marks text dirty if text properties changed, and if the node's custom properties
changed it dirties all descendants.

Adding or removing a stylesheet calls `restyle_everything`: rebuilds the index and deps and dirties
every node. New nodes are inserted into `style_dirty` at `CREATE`.

## Custom properties and `var()`

- Declarations named `--x` are collected per node into `Node.vars`, an `Rc<HashMap>` that starts as
  the parent's map (inheritance) and is copy-on-write (`Rc::make_mut`) only if this node changes a
  value. Inline `--x` set through `SET_STYLE` takes part in the cascade like any declaration.
- `substitute_vars(value, lookup)` in css.rs expands `var(--name, fallback)`, supporting nested
  fallbacks and variables that reference variables (cycle and depth protected). It returns `None`
  when a variable is undefined without a fallback; `restyle` then **skips that declaration**
  ("invalid at computed time").
- Because validity depends on the element, `add_stylesheet_with_base` does not pre-validate
  declarations containing `var(`, `--` props or `content`; it validates everything else with a
  throwaway `apply_style` and drops bad declarations with a warning, so they are not re-reported on
  every restyle.

## `::before` / `::after`

Handled in `UiTree::update_pseudo`, called at the end of `restyle` for every element when any
sheet exists:

1. `matched_declarations(.., Target::Pseudo(p))` finds the rules for the pseudo-element (matched with
   `MatchingMode::ForStatelessPseudoElement`).
2. The last `content` declaration wins (`parse_content`: quoted strings, numbers, `attr(name)`,
   concatenation). No `content` means no generated node; an existing one is destroyed.
3. Otherwise `insert_generated` creates a `span` containing a `#text` child, with ids from
   `next_generated` (>= `0x8000_0000`), inserted as first (`Before`) or last (`After`) child.
   `attach` keeps an existing `::after` as the last child when React appends siblings.
4. The span gets the pseudo's declarations (including `var()` resolved from the originating
   element's variables); the text is updated in place if the content changed.

Generated nodes are excluded from selector matching, from the tab order, and from hit-test results:
a hit on generated content is reported as the originating element (`hit` returns the parent).

## `@font-face`, loading and URL rebasing

The tree does not know the game folder, so font loading is the app's job:

- `AddedStylesheet { id, warnings, font_faces }` is returned from `add_stylesheet*`. In
  [main.rs](../crates/iron_app/src/main.rs), `add_stylesheet()` logs the warnings (target `css`) and
  calls `load_font` for each `FontFace`, which reads the file via `resolve_asset` (or a `data:` URL)
  and calls `UiTree::register_font(data, family, weight, italic)`. Text re-lays out when fonts load.
- **`game.css`**: Bun writes it next to `game.js`. The app loads `bundle.with_extension("css")` and
  passes `base` = its folder relative to the game folder (`"dist"`).
- **Rebasing**: Bun emits `url()`s relative to the CSS file. With a non-empty `base`,
  `add_stylesheet_with_base` rewrites every declaration through `css::rebase_urls` (relative URLs
  become `"dist/..."`; `data:` URLs and anything with a scheme or leading slash are untouched) and
  prefixes `@font-face` sources. The renderer later resolves the rewritten path against the game
  folder.
- **`addStyleSheet(css)`** (CSS-in-JS) goes through the `addStyleSheet` op with `base = ""`, so its
  `url()`s are already game-folder-relative. JS ids map to tree ids in `App.sheets`; re-adding an
  id replaces the old sheet.
- Small assets are inlined as `data:` URLs by Bun; larger ones are copied with hashed names
  (see the Bun CSS gotchas in [CLAUDE.md](../CLAUDE.md)).

## Known limits

- No `@media`, `@keyframes`, transitions, or general `inherit`/`initial` (see TODO.md M3d).
- Only `::before`/`::after`; `content` is limited to text.
- Descendant-combinator invalidation is conservative: a changed ancestor key restyles the whole subtree.
