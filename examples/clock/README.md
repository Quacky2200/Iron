# clock

Twelve different clock faces for the wall-clock time, switched with side arrows, dots at the bottom or the keyboard. Most are
drawn in 2D with `LineRenderer2D` and `TextMesh`; two use 3D (a perspective camera for the mileage-style meter, and meshes, a
directional light with shadows and a procedural skybox for the sundial); one (Flip) is drawn by Platypus's UI with 3D CSS
transforms. Several faces glow through bloom by using colours brighter than white. It is the largest 2D example and shows how to
organise many scenes in one game.

APIs used: `Component`, `GameObject` (with `parent`), `LineRenderer2D`, `TextMesh`, `Shapes.arc/circle`, `Spline` (hands, aura, fish,
glyph morphing), `DateTime.now`, `Camera` (orthographic and perspective), `RenderSettings.bloom`, `RenderSettings.skybox`, `Light`,
`MeshRenderer`, `Material.create`, `Mesh.create`, `Quaternion.euler/lookRotation`, `input` (keys, `mousePosition`),
`Camera.screenToWorldPoint`, `QualitySettings.vSyncCount`, `addStyleSheet`, React `createRoot` with inline SVG.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/clock
```

then from the repo root:

```bash
cargo run -p platypus_app -- examples/clock
```

Or package one exe with `platypus-build package ../examples/clock`. A screenshot run is
`cargo run -p platypus_app -- examples/clock --screenshot out.png --frames N`; it always shows the first face (Analog), because
switching needs input. The interactive window may crash some GPUs; screenshots are safest.

## Structure

`project.json` is just `name: "clock"`, `title: "Clock"`; `tsconfig.json` sets `jsx: "react-jsx"`. No assets, scenes or CSS files
(one stylesheet string in `main.tsx`). All files are under `src/` (2,415 lines in total).

| File | Lines | What |
|---|---|---|
| `main.tsx` | 189 | The `Clocks` component (switching, FPS readout, V), the React `Hud` (arrows, name, dots), `FlipFace` placeholder |
| `common.ts` | 116 | Colours, easings, the `Face` base class, builders `under` / `obj` / `shape` / `label`, `perspective()`, `two()` |
| `analog.ts` | 181 | Sweeping rings that unwind, spline leaf hands, ticking second hand with overshoot, pulsing spline aura |
| `dials.ts` | 124 | Three rings of numbers turn under a window |
| `meter.ts` | 140 | Odometer drums in perspective, bloom |
| `sundial.ts` | 231 | 3D garden sundial, real latitude maths, directional light shadow, sky follows the sun |
| `hourglass.ts` | 178 | Sand timer that drains each minute and turns over |
| `lava.ts` | 414 | Lava lamp: noise-pushed blobs that bud, merge, sink; mouse pushes them |
| `underwater.ts` | 191 | Porthole clock on the sea bed, kelp, bubbles, fish |
| `chandelier.ts` | 162 | Classic dial in front of spirograph rosettes, candles, crystals |
| `rainbow.ts` | 114 | Colour bands, hue-wheel ticks, confetti, per-character colours |
| `morph.ts` | 147 | Material-style digits that morph via splines resampled to 64 points |
| `flip.tsx` | 116 | Flip cards in UI (`FlipClock`, `Card`, `Half`) |
| `segments.ts` | 112 | Seven-segment alarm clock glowing through bloom |

## How it works

**One root per face.** Every face extends the abstract `Face` in `common.ts` (a `name`, a `root: GameObject`, `update(now, t, dt)`,
and optional `show(camera)` / `hide(camera)`). A face builds its objects inside its constructor with `under(this.root, ...)`, and the
`obj`/`shape`/`label` helpers parent new objects to the root being built:

```ts
export function under(root: GameObject, build: () => void): void {
  const before = parent;
  parent = root;
  build();
  parent = before;
}
```

**Switching.** `Clocks.onAwake` creates the camera and all twelve faces, enables only the current root, and calls `show`. `select(i)`
wraps the index, calls `old.hide(camera)`, disables the old root, enables the new one and calls `show`. Faces that change global state
in `show` (background colour, bloom, camera projection, skybox, ambient light) undo it in `hide`. For example
`SegmentFace`:

```ts
RenderSettings.bloom = { threshold: 1.0, intensity: 1.4, scatter: 0.72 };
```

and `RenderSettings.bloom = null` on hide. `meter.ts` uses `perspective(camera)`, which returns a function that restores the camera.

**Time.** `onUpdate` reads `DateTime.now` once and passes `now`, the accumulated `t` and `dt` to the current face only. Faces cache their
last shown text and only assign `TextMesh.text` when it changed (see `analog.ts`'s `shownTime`).

**Analog face.** `Ring.update` draws a sweeping arc with `Shapes.arc`, and when the fraction wraps, the tail races to the head. Hands
are filled closed splines (`handShape`: six knots, `sample(10)`). The second hand turns with
`(now.second - 1 + easeOutBack(...)) * 6` degrees for the tick overshoot.

**The flip face is UI.** `FlipFace` has an empty root; `Hud` renders `FlipClock` when it is current. Each `Card` stores the previous digit
in a `useRef`, and for 0.6 s renders two `Half` flaps with `rotateX` and `perspective: "900px"`; `onUpdate` calls `notify()` every frame while it is
shown so React redraws. The arrows are `svg` polylines with a white copy that fades in on `:hover` through the stylesheet.

**Other highlights.** `sundial.ts` computes the sun's altitude and azimuth for latitude 51.5 degrees N and the day of the year,
moves a directional `Light` (`shadows: "soft"`), sets `RenderSettings.skybox` each frame and builds a gnomon `Mesh.create({
vertices, normals, triangles })`; `D` runs a day-passing time lapse (it starts by itself at night). `lava.ts` pools blob line renderers and moves
them with a noise field (credit to the "Lava Lamp" codepen in its header); `morph.ts` defines glyph knots per digit, builds
splines, and resamples by distance to the same point count so digits can lerp.

## Controls

| Input | Effect |
|---|---|
| Left / Right arrow, `,` / `.` | Previous / next face |
| Click the side arrows or the bottom dots | Previous / next, or jump to a face |
| V | Toggle vsync (readout top right) |
| D (Sundial) | Day time lapse on/off |
| Mouse (Lava lamp) | Pushes blobs away |

## Things to try and gotchas

- Use the `Face` pattern for a game's screens (menu, levels, credits): a root object per screen and `enabled` toggled.
- Copy the bloom trick: a `Color` with channels above 1 (like `new Color(4, 0.16, 0.08, 1)`) glows once `RenderSettings.bloom` is set.
- Build your own face: extend `Face`, build in the constructor, add it to the list in `Clocks.onAwake`.
- Pool objects like `lava.ts` (`free` list, empty `points` to hide) rather than creating and destroying them each frame.
- Swap the clock for a countdown or stopwatch by feeding `update` something other than `DateTime.now`.
- Gotchas: `show`/`hide` must restore every global it changes or later faces inherit it; shapes avoid translucent strokes because
  overlapping segments look beaded (comment in `mix()` in `common.ts`); `main.tsx` imports faces with relative `.ts` paths; exported
  helpers (`fishBody`, `sunPosition`) are shared between modules.
- Not fully read in detail: the inner maths of `lava.ts`, `hourglass.ts`, `chandelier.ts` and `underwater.ts` (headers and structure only).

Related examples: shapes, splines, sine-circle-demo, raindrop-demo, windows
