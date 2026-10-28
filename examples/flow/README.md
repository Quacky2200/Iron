# flow

Floats: an article whose picture floats left and whose pull-quote floats right, the text wrapping around
both, a second picture inside a paragraph of bare text (click it to swap sides), and `clear` to end the
wrap. Containers that hold floats are `display: block`, because boxes are flex containers by default.

APIs used: `float`, `clear` (CSS and `PlatypusStyle`), `Image`, `Text`, `useState`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/flow
```

Then from the repo root (`cd ..`):

```bash
cargo run -p platypus_app -- examples/flow --screenshot out.png --frames 30
```

## Structure

| File | What |
|---|---|
| `src/main.tsx` | The page: article with a floated `Image` and quote, the bare-text paragraph |
| `src/ui.css` | `.figure` / `.quote` floats, `.rule { clear: both }` |
| `assets/images/landscape.png` | The picture |

How floats work in Platypus: `docs/ui-tree.md` ("Floats"); differences from browsers: `docs/differences.md`.
