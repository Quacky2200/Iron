# splines

A two-panel demo of Iron's `Spline` class. The left half has splines that move (a drifting closed ribbon with a car and a ball
riding it, and a wave whose tension swings between straight lines and loose curves). The right half is a small spline editor:
drag knots and handles, add, insert and remove knots, change modes and tension.

APIs used: `Spline` (constructor with `{ closed }`, `knots`, `count`, `segmentCount`, `setPosition`, `setTangents`, `setMode`,
`insertKnot`, `removeKnot`, `handleIn`, `handleOut`, `sample`, `segmentPoint`, `nearest`, `locate`, `tension`, `closed`,
`length`, `version`), `SplineAnimate` (`spline`, `duration`, `place()`), `LineRenderer2D`, `Shapes.circle/triangle`, `TextMesh`,
`Camera.screenToWorldPoint`, `input`, and `QualitySettings.vSyncCount`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/splines
```

then from the repo root:

```bash
cargo run -p iron_app -- examples/splines
```

Or package one exe with `iron-build package ../examples/splines`. A screenshot run is
`cargo run -p iron_app -- examples/splines --screenshot out.png --frames N`. The interactive window may crash some GPUs; the
screenshot run is safest (the editor half needs the mouse, so a screenshot only shows the moving half and the initial curve).

## Structure

| File | What |
|---|---|
| `project.json` | `name` "splines", `title` "Splines". |
| `tsconfig.json` | Extends `../../tsconfig.base.json`, includes `src`. |
| `src/main.ts` | All of it (338 lines): helpers `shape`, `label`, `Dots`, `Drag`, `nearKnot`, and the `SplineDemo` component. |

No assets, scenes, CSS or React UI: all text is `TextMesh`, all graphics are `LineRenderer2D`.

## How it works

`onAwake` builds an orthographic camera (`orthographicSize: 5`) and two halves split at x = 0 by a divider line. Two helpers wrap
the engine classes: `shape()` creates a `LineRenderer2D` on its own object, and `label()` a `TextMesh`. `Dots` keeps a pool of
small filled circles that follow positions; unneeded ones get empty `points` rather than being destroyed.

**The ribbon.** Six knots round an ellipse form a closed spline. Each frame `moveRibbon()` pushes them in and out and re-samples:

```ts
this.ribbon.setPosition(i, base.add(out.scale(wobble)).add(new Vector3(0, Math.sin(t * 0.7 + i) * 0.15, 0)));
...
this.ribbonLine.points = this.ribbon.sample(20);
this.car.place();
```

The car is a triangle with a `SplineAnimate` (`duration: 7`), which moves by distance (steady speed). The ball is moved by hand
with `segmentPoint(segment, fraction)`, so it spends the same time on every segment, fast on long ones and slow on short ones.

**The wave.** Seven knots bob, and `this.wave.tension = 0.5 + 0.5 * Math.sin(t * 0.6)` goes between 0 (straight lines) and 1.

**The editor.** `editInput()` does hit testing on the z = 0 plane using the camera:

```ts
const p = this.camera.screenToWorldPoint({ x: m.x, y: m.y, z: 10 });
return new Vector3(p.x, p.y, 0);
```

A left press near the selected knot's out or in handle starts a handle drag; near a knot it selects and drags the knot; elsewhere it
calls `s.nearest(m)` and, if the curve is within 0.25, inserts a knot after `s.locate(curve.t).segment`, otherwise appends one.
Dragging a handle writes mirrored tangents with `setTangents` unless Alt is held. `drawEditable()` redraws only when
`s.version`, the selection or the fill flag changed, then updates the knot dots, the handle dots and lines, and a status line.

## Controls

Moving half: none (V works everywhere).

Editing half (clicks only count where `m.x > 0.3`):

| Input | Effect |
|---|---|
| Left drag on a knot or handle | Move it. Dragging a handle mirrors the other one; Alt (left or right) moves one alone |
| Left click on empty space | Add a knot at the end; near the curve, insert one there |
| Right click on a knot, Delete or Backspace | Remove the knot (never below 2 knots) |
| C | Open or close the spline |
| T | Cycle the selected knot: auto, bezier, linear |
| `[` and `]` | Tension down and up by 0.1 (0 to 1) |
| F | Fill on or off |
| V | Toggle `QualitySettings.vSyncCount` (the FPS readout is top right) |

## Things to try and gotchas

- Use `Spline` for paths: camera rails, roads, enemy routes. `SplineAnimate` gives constant speed along it.
- Export an edited spline: `s.knots[i].position`, `.tangentIn`/`.tangentOut` and `.mode` hold what you need to save.
- Use `s.nearest(point)` for picking, snapping or "closest point on a path" AI.
- Replace the editor's `LineRenderer2D` dots with sprites or UI for nicer handles.
- Gotcha: `Spline.knots` is a read-only snapshot in compiled games (a fresh array each time; see `docs/differences.md`). The
  example edits through `setPosition`, `setTangents`, `setMode`, `insertKnot` and `removeKnot`.
- Gotcha: array reads use `!` (`s.knots[i]!`) because indexing can be `undefined` under the strict config.
- The code avoids `??` and checks optional fields with `!== undefined`. `Drag` is a small class instead of an object literal.

Related examples: shapes, clock, sine-circle-demo
