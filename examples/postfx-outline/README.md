# postfx-outline

Ink outlines from the picture's depth and normals: lines on silhouettes and creases (balls, columns, rails), none
on the flat floor or sky. `O` outlines on / off, `P` cel bands on / off, `C` also ink brightness edges (shadows,
texture detail), `Up` / `Down` line thickness.

`platypus-app examples/postfx-outline -- color` starts with brightness edges on (used for screenshots).
Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx-outline
cd .. && cargo run --release -p platypus_app -- examples/postfx-outline
```
