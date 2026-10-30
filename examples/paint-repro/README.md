# paint-repro

The Fluid desktop's paint and layout reports as one page of cells, each case beside a control that
works. What should show is red (row 1: glass that stays one even colour).

1. Two stacked `backdrop-filter: blur(10px) saturate(1.6)` glass boxes over a blue/orange backdrop: each
   blurs only what is behind its own box, so no blue reaches the lower one.
2. A radial gradient in an `overflow: hidden` clip half its size. Boxes are flex rows, so the 96px box
   shrinks to the 48px clip as in CSS (2a, 2b); `flexShrink: 0` (2e) keeps it 96px.
3. Box shadows outside a rounded corner, in the same clip; 3b shrinks the same way, 3g doesn't.
4. An absolute 8x8 box in a menu bar title: with a static title (4a) its containing block is the
   absolute bar, as in CSS; with a relative title (4b-4e), the title.
5. A static grey box with a transform, a filter, a backdrop filter, `willChange: transform` or
   `contain: paint` is still the containing block of its absolute red square (5f: `relative`).
6. A dropdown in a static `overflow: hidden` bar belongs to the relative frame around it, so the bar
   doesn't clip it (6a); a relative (6b) or transformed (6c) bar clips it, and so does a `clipPath`
   on the item holding it (6d).

APIs used: `backdropFilter`, `backgroundImage` gradients, `boxShadow`, `clipPath`, `contain`, `filter`,
`overflow`, `position` and insets, `transform`, `willChange`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/paint-repro
```

Then from the repo root (`cd ..`):

```bash
cargo run --release -p platypus_app -- examples/paint-repro --screenshot out.png --frames 5 --renderer gles
```

## Structure

| File | What |
|---|---|
| `src/main.tsx` | The cells, all with inline styles |

Differences from browsers: `docs/differences.md`.
