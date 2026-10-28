# postfx-focus

Three ways to blur: `1` depth of field (one ball sharp, far lamps become bokeh discs; `Tab` pulls focus to the next
ball), `2` tilt-shift (a sharp band across the middle, a miniature look), `3` gaussian blur over everything.
`Up` / `Down` change the amount.

`platypus-game examples/postfx-focus -- 2` starts in mode 2 (used for screenshots).
Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx-focus
cd .. && cargo run --release -p platypus_app -- examples/postfx-focus
```
