# postfx

Post-processing: a sunlit colonnade seen down its avenue, with every built-in `PostProcess` effect switchable at run
time and one effect the game writes itself in WGSL. Three balls (near, middle, far) are there to focus on; bright
lamps far down the avenue turn into bokeh discs when the depth of field blurs them.

- **Menu** (or keys): `1` depth of field, `2` tilt-shift, `3` ink outlines, `4` cel bands, `5` colour grading,
  `6` chromatic aberration, `7` vignette, `8` film grain, `9` blur, `0` pixelate, `-` CRT, `=` ripple (custom
  WGSL). Each row has `-` / `+` for its main parameter.
- `Tab` moves the depth of field's focus to the next ball (the example eases `focus_distance` as a lens does),
  `A` turns everything off, `T` tours the effects one at a time.
- `platypus-app examples/postfx -- outline vignette` starts with those effects on (names as in `PostEffect`; used for
  screenshots).

Engine features exercised: `PostProcess.create`, `createCustom` with typed parameters (`"tint:color"`),
`enabled`, `order`, `setFloat` / `setColor` / `setVector`, `scene()`, `view_uv()`, `post_time()` and `fx.rect` in
the custom WGSL, `Application.args`, `RenderSettings.tonemapping` (passes run on the tonemapped picture), a sky,
soft sun shadows, `Texture.create`, React menu. Guide: [docs/postprocessing.md](../../docs/postprocessing.md).

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/postfx
cd .. && cargo run --release -p platypus_app -- examples/postfx
```

Screenshot run: `cargo run --release -p platypus_app -- examples/postfx --screenshot out.png --frames 60 -- outline`
(a window opens for a moment).
