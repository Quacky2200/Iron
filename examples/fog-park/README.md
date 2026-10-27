# fog-park

An old London park in a painted, storybook look, built entirely from primitives and a few generated meshes. Gravel paths curve
between lawns with hoop-top railings, a pond with lily pads and reeds narrows under an arched footbridge, and there are
trees, two weeping willows, benches, a bandstand, gas lamps, flower beds and an entrance gate. Old London (terraced streets,
chimneys, spires, a dome) sinks into linear distance fog beyond the trees. Two moods: a misty morning, and night with the gas
lamps' light shafts. The camera strolls the loop path at head height, over the bridge.

Engine features exercised:

- `RenderSettings`: `fogMode = "linear"` (start/end per mood), `fogColor`, `heightFog` (`HeightFogSettings`), `volumetricFog`
  (`VolumetricFogSettings`), `bloom` (`BloomSettings`), `ambientMode`, `ambientLight`, `tonemapping = "neutral"`.
- `FogVolume` with `shape: "ellipsoid"`, `size`, `density`, `color`, `blendDistance`: soft mist lying on the pond.
- `Light`: directional sun (soft or hard shadows by mood), point lamps with `volumetricIntensity`, `haloSize`, `shadows` set at run time.
- `Mesh.createFlat({ positions, normals, uv, triangles })` for swept tubes (railings, willow fronds), shared across many `MeshRenderer`s.
- `Quaternion.lookRotation`, `fromToRotation`, `slerp`, `Vector3.cross`, `Random.initState(1898)`, `MeshRenderer.shadowCastingMode`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/fog-park
cd .. && cargo run -p platypus_app -- examples/fog-park
```

Packaged: `platypus-build package ../examples/fog-park`. Screenshot run: `platypus-game examples/fog-park --screenshot out.png --frames 60`
(the interactive window has crashed some GPUs; the screenshot run is safest). The scene creates many objects; the start-up log
prints `fog-park: N mesh renderers`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only. |
| `tsconfig.json` | `tsc` options. |
| `src/main.tsx` | 1058 lines. Moods and `apply()`; material palette; `shape`/`beam`/`rod`/`walk` helpers; the `Path` (Catmull-Rom) class; layout constants (pond discs, bridge, paths); piece builders (`pond`, `bridge`, `bench`, `lamp`, `tree`, `willow`, `bandstand`, `gate`, `bed`, `skyline`); the `Park` component; `Menu`. |

No assets.

## How it works

1. **Moods as data.** `MOODS` holds, per mood (`morning`, `night`), the sky colour, ambient, sun direction/colour/intensity, fog
   start/end, height fog, shafts, pond-mist colour, bloom and whether the lamps are lit. `apply()` pushes one mood to
   `RenderSettings` and to the objects registered in `world` (camera, sun, lamps, mists):

   ```ts
   for (const mist of world.mists) {
     mist.volume.color = m.mist;
     mist.volume.density = state.mist ? mist.density : 0;
   }
   ```

2. **Fog toggles.** With the distance fog off, `apply()` keeps `RenderSettings.fog = true` but switches to `"exponential"` with
   `fogDensity = 0`, so height fog and shafts still show. The sky colour is also the fog colour and the camera background.
3. **Helpers.** `beam(a, b, ...)` places a cube between two points; `rod(a, b, ...)` a cylinder or capsule using
   `Quaternion.fromToRotation(Vector3.up, d)`; `walk(line, step, fn)` repeats a callback along a polyline.
4. **Paths.** `Path` evaluates a Catmull-Rom curve (24 samples per segment) and exposes `at(d)`, `dir(d)`, `distOf(i)` and `line(...)`
   (offset points to a side). Paths drive gravel boxes, railing placement, tree/shrub exclusion (`pathDist`) and the camera stroll.
5. **Pond and bridge.** The pond is a union of discs (`POND`), tested by `pondDist`; the bridge's height is a sine over its
   length (`deck(x, z)`), reused to lift the camera over it.
6. **Meshes shared by many objects.** `hoop()` and `panel()` build one swept-tube mesh each, cached in module variables, and every railing
   object reuses it; `willow()` merges its boughs and fronds into three meshes to keep renderer counts low.
7. **Lamps.** Gas lamps are created with intensity 0 and registered in `world.lamps`; `apply()` turns them on at night (intensity 28,
   hard shadows, halo) and swaps the glass material between `GLASS` and `GLOW`. Lamp parts have `shadowCastingMode = "off"` or the light
   would be blocked by its own lantern.
8. **Camera.** `onUpdate` moves along `LOOP` at 1.1 m/s, aims between the path ahead (72%) and the pond (28%) and smooths the rotation with
   `slerp(target, 1 - Math.exp(-dt * 1.8))`.

## Controls

| Input | Action |
|---|---|
| `N` | Mood: misty morning / night |
| `M` | Pond mist (height fog and ellipsoid volumes) on/off |
| `D` | Distance fog on/off |
| `L` | Light shafts on/off |

The menu buttons do the same.

## Try / adapt

- Add a third mood to `MOODS` (dusk, snow) and a button; everything else already reads from that table.
- Replace the stroll with a first-person controller and add colliders to the bridge.
- Reuse `Path` for any road, rail or patrol route.
- Use `FogVolume` `shape: "ellipsoid"` for ground mist, breath-of-the-lake effects, or smoke pockets.
- Merge many similar objects into one `Mesh.createFlat` (as `willow` does) when renderer counts matter.

## Gotchas

- All geometry is made of primitives, which is why helper functions like `beam` and `rod` exist; there is no model loading.
- The roof of each terrace is shortened by 0.06 m (comment: to avoid z-fighting at the end walls); similar nudges appear elsewhere.
- `uv` arrays in `Arrays` are left empty for tubes (no textures used); `Mesh.createFlat` accepts them that way here.
- `Random.initState(1898)` makes the layout repeatable; changing the order of random calls changes the whole park.
- Generic helper `pick = <T,>(list: T[])` uses a trailing comma in the type parameter (TSX syntax) in this file.

Related examples: fog, fog-world, fog-blocks, godrays-forest, godrays-sunset
