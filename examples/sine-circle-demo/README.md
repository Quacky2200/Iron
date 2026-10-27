# sine-circle-demo

A port of MiniGameEngine's "SineDemo": a closed curve whose radius wobbles like a sine wave around a circle. Its radius,
frequency, depth, rotation, centre and colour all slide between random targets using a small tween class, and it plays a
looping music track. A text-only React HUD shows the numbers and keys. It is a good example of data-driven 2D line art plus a
simple transition system.

APIs used: `LineRenderer2D` (`points`, `width`, `loop`, `color`), `Camera` (orthographic, `backgroundColor`), `audio.load(..., { stream:
true })`, `Sound.play({ loop: true })`, `Playback.pause/resume`, `input.getKeyDown`, `input.getMouseButtonDown`,
`Screen.fullScreen`, `Color`, React `createRoot`, `useState`, `useEffect`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/sine-circle-demo
```

then from the repo root:

```bash
cargo run -p platypus_app -- examples/sine-circle-demo
```

Or package it with `platypus-build package ../examples/sine-circle-demo`. A screenshot run is
`cargo run -p platypus_app -- examples/sine-circle-demo --screenshot out.png --frames N`. The demo starts paused (see below), so a
screenshot shows the initial curve and the "[paused]" prompt. The interactive window may crash some GPUs; screenshots are safest.

## Structure

| File | What |
|---|---|
| `project.json` | `name` "sine-circle-demo", `title` "Sine Circle Demo". |
| `tsconfig.json` | Base config, `jsx: "react-jsx"`, a `paths` entry, includes `src`. |
| `src/main.tsx` | 385 lines: geometry constants, `SineCircle` (the curve maths), `SineDemo` component, HUD. |
| `src/tween.ts` | 103 lines: `Tween<T>`, `Easings`, `cubicBezier`, `lerpNumber`. Imported with `import { Easings, lerpNumber, Tween } from "./tween.ts"`. |
| `assets/oldskool.mp3` (+ `.meta`) | Music, "Oldskool" by Of Far Different Nature, CC0 (`assets/CREDITS.md`). |

## How it works

**Coordinates.** The original used form pixels (1008 x 729). The same numbers are kept and converted to world units, so it looks
the same at any window size: `UNITS_PER_PX = 10 / FORM_H`, with an orthographic camera of size 5.

**The curve.** `SineCircle.at(d)` returns the point for one degree for one of four formulas (`pi`, `normal`, `doubleSine`,
`doubleCos`). `points()` samples 361 points and joins them with a closed Catmull-Rom spline (GDI+ `DrawClosedCurve` behaves that way),
evaluated `SPLINE_STEPS = 3` times per segment in script. It does not use the native `Spline` class. Output is converted to
world units with y flipped:

```ts
out.push({ x: (x - CENTER.x) * UNITS_PER_PX, y: (CENTER.y - y) * UNITS_PER_PX });
```

**Setup (`SineDemo.onAwake`).** Creates the camera, a `LineRenderer2D` for the curve and a red one for the centre marker, loads the
music (`audio.load("assets/oldskool.mp3", { stream: true })`), and six tweens (radius, frequency, depth, position, rotation,
colour). Each tween picks a random target with `pick` and, with `loop` and `pingPong`, goes there and back:

```ts
this.radius = new Tween<number>({
  start: 50,
  lerp: lerpNumber,
  pick: () => rand(20, FORM_H / 3),
  easing: Easings.easeIn,
  loop: true,
  pingPong: true,
  enabled: true,
});
```

`Tween.update(dt)` eases a value between `a` and `b`; with `pingPong` a new target is picked each time the value returns to the start;
`halfDuration` makes each leg take half the duration. The easings are CSS cubic-bezier curves solved by bisection.

**Update loop.** While not paused, every tween is updated and `applyFrame()` copies values into `SineCircle` and re-sets the
renderer's `points` (about 1,100 segments per frame). Input is read in `onUpdate`. The demo starts paused (`this.paused = true`), and
the music starts only on first unpause. `setType` switches formula and sets fast (1-3 s) or slow (4-10 s) transition durations.

**HUD.** The `Hud` component is a transparent, `pointerEvents: "none"` box with text only. The component notifies listeners every
0.05 s through a module-level `listeners` set, and `useStats()` re-reads `demo.stats`:

```ts
const [view, setView] = useState(read);
useEffect(() => {
  const update = () => setView(read());
  listeners.add(update);
```

## Controls

| Input | Effect |
|---|---|
| Left click | Next sine formula |
| 1, 2, 3, 4 | Pick a formula (Pi sine, Normal sine, Double sine, Double cos) |
| Right click | Flip background white/black (and the colour range's lightness) |
| Space | Pause / resume (and music) |
| F5 | Show the centre point |
| F11 | Fullscreen |

## Things to try and gotchas

- Add a fifth formula: extend `SineType`, `SINE_TYPES`, `SINE_NAMES` and the `switch` in `at()`.
- Reuse `Tween<T>` for UI or game-object animation (any `T` with a `lerp`); it is plain TypeScript with no engine calls.
- Replace the hand-written Catmull-Rom with the native `Spline` (see `splines`) to cut script work per frame.
- Drive the curve from audio levels instead of random targets.
- Gotchas: the script evaluates about 1,100 spline points per frame in TypeScript; the original's double-wave formulas are kept "bugs
  and all" (comment in the source); `switch` in `at()` assigns `tr` in cases that `break`; `normal` returns early.
- Games import only relative files here (`./tween.ts`); engine names are globals.

Related examples: raindrop-demo, splines, shapes
