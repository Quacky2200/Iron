# fog-world

A Minecraft-like endless world. Terrain comes from seeded value noise, is built per 16x16-column chunk as one textured
mesh containing only visible faces, and chunks stream in and out in a ring around an autopilot camera. The fog is the
render distance, so the world's edge never shows. A spare ring of chunks lies beyond the fog's end, and fog culling
skips drawing it; the HUD shows how much that saves.

Engine features exercised:

- `RenderSettings.fog`, `fogMode = "linear"`, `fogStartDistance`, `fogEndDistance`, `fogColor`, and
  `RenderSettings.fogCulling`.
- `RenderStats` (`meshes`, `culledMeshes`, `fogCulled`), `QualitySettings.vSyncCount`, `QualitySettings.shadowDistance`.
- `Mesh.createFlat({...})` and `mesh.setFlat(data)` to rebuild a mesh in place, `Texture.create` (point filter atlas).
- `GameObject.enabled` to park pooled chunks, `Debug.log`, `Date.now()` for a per-frame build budget.
- React HUD with live counters.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-world
cd .. && cargo run -p platypus_app -- examples/fog-world
```

Packaged: `platypus-build package ../examples/fog-world`. Screenshot run: `platypus-app examples/fog-world --screenshot out.png --frames 120`
(the interactive window has crashed some GPUs; the screenshot run is safest).

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 585 lines: presets, noise (`hash`, `noise`, `fbm`, `height`, `treeAt`), tile atlas, face tables and buffers, `buildChunk`, fog `state`/`applyFog`, the `World` component (flight, streaming, stats log), HUD. |

No asset files. Sibling example fog-blocks reuses the same meshing code on a fixed hill.

## How it works

1. **Presets.** `PRESETS` holds four render distances in chunks (2, 6, 10, 14 with fog off). `START_PRESET = 1`.
   `applyFog()` makes the fog end at `distance * CHUNK` and the start at 60% of that, "as in Minecraft":

   ```ts
   RenderSettings.fogEndDistance = state.distance * CHUNK;
   RenderSettings.fogStartDistance = state.distance * CHUNK * 0.6;
   RenderSettings.fogCulling = state.culling;
   ```

2. **Terrain.** `height(x, z)` is 4-octave fbm for hills plus a mountain term, floored to whole blocks, minimum 2.
   Trees grow where `treeAt()` says so, never within 2 columns of a chunk edge so they stay inside it.
3. **Meshing.** `buildChunk(cx, cz)` reads heights for the chunk plus a one-column border, emits a top face per column,
   side faces down to each lower neighbour, a water surface for low ground, and tree blocks skipping faces hidden by
   neighbours. Faces are written by index into reused typed arrays (the comment says this is much faster than
   `push`) and sliced at the end.
4. **Streaming.** `stream(budget)` drops chunks outside radius `r + 1` (disables the object and pushes it to a
   `pool`), lists missing chunks nearest first, and builds as many as fit in `BUILD_BUDGET_MS = 6`. A pooled chunk is
   refilled with `c.mesh.setFlat(data)`; otherwise a new `GameObject("Chunk")` is made. The radius is `distance + 2`
   with fog on (the spare ring), or `distance` with fog off.
5. **Autopilot.** The heading drifts using `noise(this.t * 0.05, 0.5, 4)`; `fly()` moves at 9 m/s and eases the
   altitude toward `groundAhead(dir) + 9`.
6. **Stats.** Every 300 frames a `Debug.log` line prints the preset, chunks, blocks, drawn/culled meshes, meshes culled
   by fog, chunks missing in view range and the average ms per frame, so presets can be compared.

## Controls

| Input | Action |
|---|---|
| `1` to `4` | Render preset (very small, medium, large, far with no fog) |
| `[` / `]` | Render distance -/+ 1 chunk (2 to 16) |
| `C` | Fog culling on/off |
| `F` | Fog on/off |
| `V` | Toggle vsync (`QualitySettings.vSyncCount`) |
| `Space` | Pause the flight |
| HUD buttons | Same, plus live counters |

## Try / adapt

- Replace the autopilot with first-person input and add a character controller.
- Add a block edit: change `height()` to read from a modified-blocks map and rebuild the one chunk.
- Move `buildChunk` work to spread over more frames if the budget (6 ms) is too high for your target.
- Use `RenderStats` for your own performance HUD.
- Swap the noise for biome-dependent tiles (the atlas has eight tiles; Dirt/Stone are used for depth).

## Gotchas

- Chunks are built on the main thread; the time budget is what keeps frames steady, so the world fills in over several
  frames after a big jump (the log notes "still loading").
- `interface ChunkMesh` is declared for the build result; `Mesh.createFlat` and `setFlat` take that same object.
- Tree keys pack local coordinates into an integer with offsets of 8, so trees must stay within a chunk.
- No collision or lighting per block: faces are lit by the directional sun and ambient only.

Related examples: fog-blocks, fog, fog-park, quality
