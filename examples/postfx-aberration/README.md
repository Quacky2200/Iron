# postfx-aberration

Chromatic aberration alone. Look for red and blue fringes on the bright lamps, the rails and the columns, growing
towards the edges of the picture. `Space` turns it on and off, `Up` / `Down` change `amount`.

`platypus-game examples/postfx-aberration -- 0.03` starts with that amount (used for screenshots).
Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx-aberration
cd .. && cargo run --release -p platypus_app -- examples/postfx-aberration
```
