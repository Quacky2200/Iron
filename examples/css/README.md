# css

A stylesheet showcase. `src/ui.css` is imported by the entry and built into `build/game.css`; React
components pick rules with `className`. The cards show combinators, structural pseudo-classes, `:has`,
`:not`, `:empty`, attribute states (`:checked`, `:disabled`), `::before` / `::after`, custom properties
with `var()`, `@font-face`, `clip-path`, `object-fit`, a scrolling list, text fields and right-to-left
text. A stylesheet added at run time changes the accent colour every second.

APIs used: `import "./ui.css"`, `addStyleSheet` / `removeStyleSheet`, `Input`, `TextArea`, `Image`, `Text`,
`Span`, pointer and keyboard props (`onClick`, `onDoubleClick`, `onKeyDown`, `onScroll`, `tabIndex`).

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/css
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/css
```

Screenshot run (safest; the interactive window may crash some GPUs):
`cargo run -p iron_app -- examples/css --screenshot out.png --frames 30`. Package with
`iron-build package ../examples/css`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | Components: `Card`, `Toggle`, `Fields`, `ScrollList`, `Accent`, `App` |
| `src/ui.css` | 329 lines of rules: variables on `:root`, `.card`, `.item`, `.toggle`, `.slot`, shapes, `.scroller`, inputs |
| `assets/images/landscape.png` | A photo for the clipping and `object-fit` demos (path `PHOTO` in `main.tsx`) |

The CSS also references the Charm font (`../assets/fonts/Charm-*.ttf`, relative to the CSS file). That
font is not committed; without it the "@font-face" card falls back to the default font.

## How it works

1. **CSS import.** `import "./ui.css";` is the only import. The builder writes `build/game.css` and the
   engine loads it next to the compiled game. Rules use standard CSS (cascade, specificity, `!important`).
2. **Variables.** `:root` in `ui.css` defines `--accent`, `--card` and so on; rules use `var(--accent)`.
3. **Run-time stylesheet.** `Accent` adds a rule every second and removes it on cleanup; every rule that
   reads `--accent` restyles:

```tsx
useEffect(() => {
  const id = addStyleSheet(`:root { --accent: ${accents[tick % accents.length]} }`);
  return () => removeStyleSheet(id);
}, [tick]);
```

4. **Structural selectors.** The five list items are plain `Text` elements; the stylesheet styles them with
   `:nth-child(even)`, `:first-child`, `:last-child`, `.item + .item`, and `.item.on ~ .item`. The
   `highlighted` index comes from the ticker, or from a click that pins it (`pinned`).
5. **States.** `:hover` and `:active` are set by the pointer; `.toggle` rules use `:enabled`, `:disabled`,
   `:checked` and `:focus-visible`. The `Toggle` component sets the `checked` and `disabled` attributes
   from state and handles Space / Enter so it works from the keyboard:

```tsx
onKeyDown={(e) => {
  if (e.key === " " || e.key === "Enter") {
    e.preventDefault();
    toggle();
  }
}}
```

6. **Generated content.** `.tag::before { content: "#" }`, `.quote::before` / `::after`, and `.bullet::before`.
   `.slot:empty` and `.slot:not(:empty)` show different styles for an empty and a filled box.
7. **Clipping and images.** `.circle`, `.ticket` and `.lens` use `clip-path: circle()`, `inset(... round ...)`
   and `ellipse()`. Three `Image` elements show `objectFit` default, `contain` and `cover` with
   `objectPosition`.
8. **Scrolling.** `.scroller` holds 20 focusable rows; `onScroll` reports `e.scrollTop`, shown in the heading.
9. **Text fields.** `Fields` has a controlled `Input` (`value` + `onChange`) and a `TextArea`; an `Input` with
   `dir="rtl"` and an Arabic `Text` test right-to-left shaping.
10. **Double click.** The title counts `onDoubleClick` events.

## Controls

| Input | Effect |
|---|---|
| Click an item in the first card | Pins the highlight (click again to release) |
| Click a toggle, or Tab to it and press Space / Enter | Toggles it (the disabled one does nothing) |
| Double-click the title | Adds "(double-clicked Nx)" |
| Mouse wheel, keys, scrollbar on the list | Scrolls it |
| Tab / Shift+Tab | Moves focus; `:focus-visible` rings appear |

## Things to try

- Change the `accents` array or the colours in `:root` to re-theme the whole screen.
- Add a rule using `:has()` (the stylesheet has `.card:has(.item.on)`) to style a parent from its children.
- Build a light/dark switch by adding or removing a stylesheet with `addStyleSheet`.
- Turn the toggle into a switch with `::after` and a `:checked` rule.
- Compare with `form`, which uses the same techniques in a working form.

## Gotchas

- `url()` in CSS is relative to the CSS file, so the font path starts with `../assets`.
- The stylesheet accepts standard CSS; s&box-only extensions are not the goal (`CLAUDE.md`: "prefer standard CSS").
  Differences from browsers are listed in `docs/differences.md`.
- Interactive elements other than inputs need `tabIndex={0}` to receive keyboard focus.

Related examples: form, text, boxes
