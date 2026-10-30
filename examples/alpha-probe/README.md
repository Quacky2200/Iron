# alpha-probe

Six translucent boxes over an opaque `#3060c0` field, each 100x100 at a known place, so their pixels can be
read back: a 25% white fill, a 25% white border (square and rounded), a 50% red fill, a 25% black border over
it, and a 25% white fill over a `backdrop-filter: blur(4px)`. It shows how Platypus blends translucent colours
(docs/rendering.md "Blending"). By default that is on sRGB values, as browsers do: the white fill reads
`100,136,208`, the red one `152,48,96`. Add `"blending": "linear"` to `project.json` to see linear-light
blending instead (`142,157,210`, `191,69,141`: lighter and greyer). The tests `M2-BLEND-03`/`04` (wgpu and
GLES) and `M2-BLEND-09` (CPU) build the same scene and check those pixels.

APIs used: `createRoot`, inline styles (`backgroundColor`, `border`, `borderRadius`, `backdropFilter`).

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/alpha-probe
```

Then, from the repo root (`cd ..`):

```bash
cargo run --release -p platypus_app -- examples/alpha-probe
```

`--renderer vulkan|gles|cpu` draws it with each renderer; the probes read the same on all three.
