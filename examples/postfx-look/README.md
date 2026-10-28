# postfx-look

Post-processing looks: several `PostProcess` passes stacked into a finished picture, switched at run time.
Keys `1` Noir (black and white, grain, heavy vignette), `2` Dream (tilt-shift blur, warm grade, fringing),
`3` Comic (cel bands and ink lines), `4` Old TV (CRT glass, scanlines, grain), `5` Arcade (big pixels, few colours),
`0` plain. The look's name and what to look for are on screen.

`platypus-game examples/postfx-look -- 3` starts with look 3 (used for screenshots).
The other post-processing examples: `postfx` (every effect with a menu), `postfx-aberration`, `postfx-focus`,
`postfx-outline`, `postfx-film`. Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx-look
cd .. && cargo run --release -p platypus_app -- examples/postfx-look
```
