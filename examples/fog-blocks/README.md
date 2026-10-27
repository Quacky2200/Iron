# fog-blocks

A hill of grass blocks in pale sky-blue fog, after foggy Minecraft. The pixel-art tile atlas is made in code, the terrain
is meshed in 16x16-column chunks (one mesh each, only visible faces), and the camera flies a 70-second loop: low among
the blocks, up through a deck of cloud to look down on the hilltop, and back.

Engine features exercised:

- `RenderSettings`: `fogColor`, `fogMode = "exponentialSquared"`, `fogDensity`, `heightFog`, `fog`, `ambientMode`,
  `ambientLight`, `tonemapping = "none"`.
- `FogVolume` component (`size`, `density`, `color`, `blendDistance`): a flat box of fog used as a cloud deck.
- `Texture.create(w, h, Color[], { filterMode: "point", mipmaps: false, wrapMode: "clamp" })`: a procedural atlas.
- `Mesh.createFlat({ positions, normals, uv, triangles })` with typed arrays, `MeshRenderer`, `Material.create({ mainTexture, smoothness })`.
- `Light` directional with `shadows: "soft"`, `QualitySettings.shadowDistance`, `Mathf.smoothStep` / `lerp`, `Random`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-blocks
cd .. && cargo run -p platypus_app -- examples/fog-blocks
```

Packaged executable: `platypus-build package ../examples/fog-blocks`. Screenshot run (safest on GPUs that crash with the
window): `platypus-game examples/fog-blocks --screenshot out.png --frames 60`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 409 lines: constants and `state`, `apply()`, terrain `height()`, atlas, face tables, chunk buffers, the `Blocks` component, `Menu`. |

No asset files; the atlas is drawn in code.

## How it works

1. **Terrain function.** `height(x, z)` adds a Gaussian hill, a ridge and a sine swell and rounds to whole blocks;
   `columnHeight(i, j)` returns 0 outside the 44x44 grid so edge faces get drawn down to ground.
2. **Atlas.** Eight 16x16 tiles (`const enum Tile`: GrassTop, GrassSide, Dirt, Stone, Sand, Water, Wood, Leaves) go into
   one 128x16 texture with a seeded LCG for per-pixel shading. `Texture.create` takes the bottom row first, hence
   `pixels[(PX - 1 - y) * TILES * PX + ...]`.
3. **Faces.** `makeFaces()` derives the six cube faces (normal, corners wound outward, UVs) once. `face()` appends one
   quad to growable typed arrays reused between chunks; the comment notes that indexed writes into typed arrays are
   much faster in QuickJS than `push`. UVs are inset a hair so tiles do not bleed:

   ```ts
   buf.uv[p2 + i * 2] = (tile + 0.02 + t[i * 2]! * 0.96) / TILES;
   buf.uv[p2 + i * 2 + 1] = 0.01 + t[i * 2 + 1]! * 0.98;
   ```

4. **Chunks.** For each column: a top face, then side faces only from the lowest neighbour's height up (so hidden
   faces are never emitted); depth 0 is the grass-rimmed side, depth 4 or more is stone. Trees are kept in a `Map`
   keyed by a packed integer and meshed into the chunk that contains them, with faces skipped against `solid()`.
   Each chunk is one `Mesh.createFlat` and one `GameObject("Chunk")`.
5. **Fog.** `apply()` sets exponential-squared fog in the sky colour, and a height fog (`baseHeight: 1`) that pools in
   valleys. The cloud is a `FogVolume` whose density is toggled:

   ```ts
   cloud = deck.addComponent(FogVolume, { size: CLOUD.size, density: CLOUD.density, color: Color.hex("#e4edfd"), blendDistance: CLOUD.blend });
   ```

6. **Flight.** `fly()` computes `up` from `(1 - cos(phase)) / 2` through `smoothStep`, lerps radius 16 to 34 and height
   from `this.low` to 30, and looks at the hilltop. `this.low` is computed once so the low pass is level above the
   tallest column within 2 blocks of the circle.

## Controls

| Input | Action |
|---|---|
| `F` | Distance fog on/off |
| `H` | Height fog on/off |
| `C` | Cloud deck on/off |
| `Space` | Pause/resume the flight |
| Menu buttons | Same four toggles |

## Try / adapt

- Make the terrain your own: swap `height()` for noise, or read blocks from a saved array.
- Add per-chunk lifetime (create/destroy `Chunk` objects around a moving camera): see fog-world for that.
- Use more `FogVolume`s (low ground mist, a fog bank) positioned at different heights.
- Add a water tile pass: `Tile.Water` and `Tile.Sand` are in the atlas but not used by this scene.
- Change `FLIGHT_SECONDS` or replace `fly()` with player input.

## Gotchas

- Only the grid at chunk granularity is meshed once at start; no collision or editing.
- `Tile.Water` / `Tile.Sand` are unused here (in this source).
- The `switch (tile as Tile)` and `const enum` forms are what the compiled subset accepts here; typed-array growth is
  done by hand (`grow()`) rather than a dynamic list.
- `RenderSettings.tonemapping = "none"` keeps the flat pastel look.

Related examples: fog-world, fog, fog-park, fog-teapots
