# boxes

A UI-only showcase of the box model as Platypus draws it: flexbox and grid layout, linear and radial
gradients, background images, per-corner radii, box shadows (outer and inset), `filter: blur`, a
frosted-glass header with `backdropFilter`, group `opacity`, and a timer-driven counter that adds and
removes circles. It is the M2 milestone demo (React -> Taffy layout -> renderer).

APIs used: `createRoot().render`, the `box` element, `PlatypusStyle`, `useState` / `useEffect`, `setTimeout`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/boxes
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p platypus_app -- examples/boxes
```

A screenshot run is `cargo run -p platypus_app -- examples/boxes --screenshot out.png --frames 30`; it is the
safest option because the interactive window may crash some GPUs. For one executable use
`platypus-build package ../examples/boxes`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | All components: `Card`, `Swatch`, `Counter`, `Blobs`, `App` |
| `assets/stripes.png` (+ `.meta`) | A small striped image used as a tile and as a `contain` background |

## How it works

1. **Shared style object.** `card` is a `PlatypusStyle` (radius, border, shadow, padding, column flex); the
   `Card` component spreads it and lets callers override:

```tsx
function Card({ style, children }: { style?: PlatypusStyle; children?: ReactNode }) {
  return <box style={{ ...card, ...style }}>{children}</box>;
}
```

2. **Backdrop for the glass.** `Blobs` draws absolutely positioned circles with a `radial-gradient` that
   fades to transparent, plus a pill using `url(assets/stripes.png)`. They sit behind the header so the
   blur has something to show.
3. **Frosted header.** A translucent box with `backdropFilter: "blur(18px)"`, a border and a shadow that
   combines an inset highlight with an outer drop shadow.
4. **Grid.** The main area is `display: "grid"` with `gridTemplateColumns: "repeat(4, 1fr)"` and
   `gridTemplateRows: "1fr 1fr auto"`. Eight swatch cards cover: linear gradient, image tile with
   `backgroundSize: "24px"`, inset shadow, `borderRadius: "40px 4px"` with a thick border, radial gradient,
   `filter: "blur(6px)"` (blurs the box and its child), and `opacity: 0.5` (the white child is drawn over the
   blue parent first and then both fade together). One card shows the image with `backgroundSize: "contain"`.
5. **Counter.** The last card spans the row (`gridColumn: "1 / -1"`). A hook counts 1 -> 8 -> 1 with one
   timeout per step, and `Array.from` renders that many circles with keyed children:

```tsx
const id = setTimeout(() => {
  const next = n + dir;
  if (next > 8 || next < 1) setDir(-dir);
  setN(Math.min(8, Math.max(1, next)));
}, 400);
return () => clearTimeout(id);
```

   Each circle uses `hsl(...)` for its colour and glow (`boxShadow`), so keys and diffing add and remove boxes
   at the end.

## Controls

None.

## Things to try

- Swap the 4-column grid for `flexWrap` rows and compare.
- Animate a style value (a width, a colour) by changing it from a timer, using the counter's pattern.
- Use a card as a reusable panel in your own HUD (see `arcade`, which uses `rgba` panels instead of `backdropFilter` for cost).
- Put `backdropFilter` on a box over the 3D world (see `docs/world-rendering.md`; the arcade comment says it costs
  a full-screen copy and blur each frame).
- Move the style objects into a `.css` file (see `css`).

## Gotchas

- Styles are inline objects; there is no stylesheet here.
- Image paths such as `url(assets/stripes.png)` are relative to the game folder.
- Blur and backdrop filters are the most expensive effects here; `docs/differences.md` lists where Platypus's
  filter order and defaults differ from a browser.

Related examples: css, text, form
