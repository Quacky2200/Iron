# fishtank

A calm, full-screen aquarium. A perspective camera drifts slowly in front of layered scenery: a hazy back wall,
rock silhouettes, a sand floor, light shafts, caustics on the sand, kelp at several depths, marine snow and an
air stone's bubbles, all fading into the water colour with linear fog. Four species of fish school in 3D, keep
inside the tank and flee the pointer. Every visible thing is a `LineRenderer2D` shape (outline plus fill); there
are no meshes, textures or audio.

Engine features used: `LineRenderer2D` (`points`, `loop`, `width`, `color`, `fillColor`, `sortingOrder`),
`Shapes` (`ellipse`, `circle`, `rectangle`, `roundedRectangle`), `Spline` (`sample`), `RenderSettings` fog
(`fogMode`, `fogColor`, `fogStartDistance`, `fogEndDistance`), `Camera` (`screenToWorldPoint`),
`Quaternion.lookRotation`, `input` (mouse position and buttons, keys), `QualitySettings.vSyncCount`,
`addStyleSheet` and a React HUD.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/fishtank
```

Then, from the repo root:

```bash
cargo run -p iron_app -- examples/fishtank
```

Package with `iron-build package ../examples/fishtank`. A screenshot run is
`iron examples/fishtank --screenshot out.png --frames N`; it is the safest check, since the window has
crashed some GPUs.

## Structure

| File | What |
|---|---|
| `src/main.tsx` | Water constants, `shape` helpers, `Fish`, `Flock`, the `Tank` component, the React `Hud` |
| `project.json` | `name: fishtank`, `title: Fish Tank` (no assets folder) |
| `tsconfig.json` | Extends `tsconfig.base.json`, `jsx: react-jsx` |

## How it works

**Water.** `Tank.onAwake` creates the camera and turns on linear fog in the water's colour. The back wall, rocks,
sand strips, light shafts, kelp, snow and bubbles are all `shape(...)` calls, each a GameObject with a
`LineRenderer2D`. Depth order is faked through `sortingOrder` (rocks use `Math.round(z * 100)`), and the sand is
built from `flat` strips rotated by `Quaternion.euler(-90, 0, 0)` to lie on the floor. Because fog also tints the
back wall, `unfogged(c)` inverts the fog so the wall shows as the colour you ask for:

```ts
const back = (x: number, h: number) => Math.max(0, (x - h * f) / (1 - f));
```

**Fish.** A `Fish` is a plain class (not a `Component`), a tree of shapes built from a `Species` table: a body
from a closed `Spline`, a tail on a child object, a dorsal fin, stripes and an eye. `place` rotates it to its
heading, flips `localScale.y` when swimming left, beats the tail with a sine and updates `sortingOrder` from depth
only when the rounded value changes.

**Flocking.** Each frame `buildFlock` writes positions, velocities and sizes into a `Float64Array` (7 numbers per
fish) and a grid of cell lists (`CELL = 2.6`). `Fish.step` visits the 27 neighbouring cells and sums separation,
alignment and cohesion with its own species, then adds wandering, tank-keeping, food attraction and fright. The
comment in the source says it uses plain numbers because vectors would allocate per pair.

**Interaction.** `onUpdate` turns the mouse position into a point on the z=0 plane with
`screenToWorldPoint`. Left click drops six food pellets, the nearest fish race to them and grow when they eat
(`grown`, up to 1.5), right click adds a fish, Space starts a "tap" fright wave, V toggles
`QualitySettings.vSyncCount`.

**Animation.** Kelp and caustic outlines are rebuilt each frame (`pl.line.points = pts`); snow, bubbles and shafts
move by position. The camera path is sine-based: `cam.rotation = Quaternion.lookRotation(...)`.

**HUD.** `Hud` is React, with `pointerEvents: "none"`, a radial-gradient vignette, and a readout whose CSS
`opacity` transition (`.hud`) fades it in only while the mouse moved in the last 3 seconds. It re-renders when
`Tank` calls `notify()` four times a second. State is shared by module variables (`tank`, `eaten`) and a listener
set.

## Controls

| Input | Action |
|---|---|
| Move mouse | Fish flee the pointer; the readout appears |
| Left click | Drop food |
| Right click | Add a random fish |
| Space | Tap the glass (fish scatter) |
| V | Toggle vsync |

## Things to try

- Add a predator species that chases the others (extra steer term in `Fish.step`).
- Add more species by adding rows to `SPECIES` and parts in the `Fish` constructor.
- Replace the shape-built scenery with sprites or models and keep the fog.
- Make a screensaver: lower the HUD and let the camera path run.
- Reuse the grid-based flock for birds, bats or crowds.

## Gotchas

- The source avoids features outside the compiled subset: classes with explicit field types, `for` loops over
  indices, and helper functions instead of spreads or optional object shapes (for instance `shape` builds `Look`
  fields one by one). Its fish vectors are replaced with plain numbers in the hot loop to avoid allocation.
- `Look` is an interface passed by value; optional fields are checked with `!== undefined`.
- Pointer, camera and fog are all driven by the single `Tank` component; the module variable `tank` is how the
  HUD finds it.
- Lines with `width: 0` are used for pure fills.
- Sorting uses integers (`order = round(z*10)*10`), so fish at the same depth rely on the per-part index offset.

Related examples: fog, shapes, splines, sine-circle-demo, platformer
