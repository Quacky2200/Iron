# postfx-film

Vignette, film grain and colour grading: the subtle effects that make a render feel like film. Look for darker
corners, shimmering noise in the midtones and a warm tint. `V` vignette, `G` grain, `C` grading (each on / off),
`A` all on / off to compare, `Up` / `Down` strength of all three.

`platypus-app examples/postfx-film -- 1.5` starts at that strength (used for screenshots).
Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx-film
cd .. && cargo run --release -p platypus_app -- examples/postfx-film
```
