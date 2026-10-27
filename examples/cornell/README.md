# cornell

The Cornell box (Cornell University Program of Computer Graphics, 1984) built from cube primitives: a
2 m room with a red left wall, green right wall, white floor, ceiling and back, a tall and a short block,
and a light in the ceiling. Iron has no global illumination or area lights yet (the source comment points
to TODO.md M7), so the example approximates the look with an emissive panel plus bloom, a point light,
a soft-shadow directional light and several tinted "bounce" point lights.

APIs used: `GameObject`, `MeshRenderer`, `Material`, `Light` (point and directional), `Camera`,
`RenderSettings.ambientLight` / `bloom`, `QualitySettings.shadowDistance`, `Quaternion.euler`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/cornell
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/cornell
```

Screenshot run: `cargo run -p iron_app -- examples/cornell --screenshot out.png --frames 30`. This
is the safest way to run; the interactive window may crash some GPUs. Package with
`iron-build package ../examples/cornell`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` ("Iron - Cornell box") only; no quality or layer settings |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | `box()` and `bounce()` helpers, the `CornellBox` component, a one-line caption in React |

No assets or scenes; everything is created in `onStart`.

## How it works

1. **Reflectances.** The original's measured values are `Color` constants in linear space:

```ts
const WHITE = new Color(0.73, 0.73, 0.73, 1);
const RED = new Color(0.63, 0.065, 0.05, 1);
const GREEN = new Color(0.14, 0.45, 0.091, 1);
```

2. **Camera and globals.** The camera sits at `(0, 1, 3.9)` with a 39 degree field of view and a black
   background (no skybox). `RenderSettings.ambientLight` is a faint grey, `RenderSettings.bloom` has a
   threshold of 1.2 so only the emissive panel glows, and `QualitySettings.shadowDistance = 6` keeps
   the shadow range tight for a small room.
3. **The room.** `box(name, position, size, color, yaw, castShadows)` creates a `GameObject` with the
   built-in `"cube"` mesh scaled to `size`. The walls are 0.05 m thick boxes, open at the front. They are
   created with `castShadows` false (`shadowCastingMode: "off"`), because the ceiling would otherwise shade
   everything from the light above. The two blocks are turned by 17 and -17 degrees and do cast shadows.
4. **The light panel.** A thin cube near the ceiling whose material has an emission colour above 1 so
   bloom picks it up:

```ts
material: new Material(Color.white, new Color(4, 3.6, 2.9, 1), 0, true),
```

   The positional form is `Material(color, emission, smoothness, unlit, ...)` (`docs/assets-and-models.md`), so the
   panel is unlit with a bright emission colour.
5. **Lights.** A warm point light under the panel, plus a directional "Shadow light" tilted with
   `Quaternion.euler(-72, 20, 0)`, `shadows: "soft"` and `shadowStrength: 0.8`, so the blocks leave shadows.
6. **Faked bounce light.** `bounce()` places a point light next to a lit surface, tints it with the surface
   colour times the light colour, normalises its brightest channel to 1 and uses `intensity` for strength.
   Red and green bounces give the colour bleeding; a floor bounce lifts undersides; a dim fill from the open front finishes it:

```ts
bounce("Red bounce", new Vector3(-0.75, 0.9, 0.1), RED, warm, 1.6, 2.4);
bounce("Green bounce", new Vector3(0.75, 0.9, 0.1), GREEN, warm, 1.6, 2.4);
```

7. **UI.** `createRoot().render(<Caption />)` draws a small grey credit line at the bottom (`pointerEvents: "none"`).

## Controls

None; the camera is fixed.

## Things to try

- Move the blocks, or change `yaw`, and see the shadows follow.
- Change the wall colours; the bounce lights use the same constants, so colour bleeding changes with them.
- Raise `bloom.intensity` or the panel's emission to compare glow strength.
- Add a `CameraOrbit` (as in `classics`) to inspect the box from other angles.
- Put a `Rigidbody` and `BoxCollider` on new objects to drop things into the room (see `physics`).

## Gotchas

- This is an approximation, not real GI: every bounce is a hand-placed point light. If you move a wall, move
  its bounce light too.
- Room walls have shadows off on purpose; turning them on darkens the whole interior.
- `box()` is a local function, not an engine API.

Related examples: classics, materials, light-units
