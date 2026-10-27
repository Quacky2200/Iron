# physics

A side-on physics toy. Crates and balls fall onto a floor and a tilted ramp, simulated by the engine's rapier
world, and are drawn as React UI boxes that follow each body. Clicking drops another one, a sensor zone counts
what passes through it, and impacts play a thud that is louder for harder hits and panned to where they happen.
It shows that physics bodies and UI can be combined without any 3D rendering.

Engine features used: `GameObject`, `Rigidbody` (with `RigidbodyConstraints`), `BoxCollider`, `SphereCollider`
(`bounciness`, `dynamicFriction`, `isTrigger`), `onCollisionEnter(Collision)` with `relativeSpeed`,
`onTriggerEnter(Collider)`, `Quaternion.euler`, `audio.play` with `volume` / `rate` / `pan`, React `createRoot`
and the UI element `box` / `text`, CSS gradient, `transform: rotate()`, `box-shadow`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/physics
```

Then, from the repo root:

```bash
cargo run -p platypus_app -- examples/physics
```

Or package one exe with `platypus-build package ../examples/physics`. A screenshot run is
`platypus-game examples/physics --screenshot out.png --frames N`; prefer it, since the interactive window has crashed
some GPUs.

## Structure

| File | What |
|---|---|
| `src/main.tsx` | Components `Thing`, `Zone`, `Spawner`; `buildLevel`, `drop`; the React `World` view |
| `project.json` | `name: physics`, `title: Platypus - Physics` |
| `assets/thud.wav` | The impact sound |

## How it works

**Scale.** The world is in metres; the screen is in pixels. Two constants map between them: 60 pixels per metre,
with the world origin at the bottom centre of a 1280x720 layout (`ORIGIN = { x: 640, y: 660 }`).

**Level.** `Spawner` is registered with `register({ Spawner })` and runs `buildLevel()` and 14 `drop` calls in
`onStart`. The level is a ground box, a ramp rotated 20 degrees about z, and a trigger box:

```ts
const ramp = new GameObject("Ramp", { position: new Vector3(-4, 3, 0), rotation: Quaternion.euler(0, 0, -20) });
ramp.addComponent(BoxCollider, { size: new Vector3(6, 0.3, 4), dynamicFriction: 0.2 });
```

**Bodies.** `drop` makes a sphere collider for every third body and a box otherwise, with a size from `i % 4`.
The scene is 3D but seen from the side, so each body freezes z position and x/y rotation:

```ts
go.addComponent(Rigidbody, {
  angularDrag: 0.2,
  constraints: RigidbodyConstraints.FreezePositionZ | RigidbodyConstraints.FreezeRotationX | RigidbodyConstraints.FreezeRotationY,
});
```

**Sound.** `Thing.onCollisionEnter` ignores hits under 0.8 m/s relative speed and plays `thud.wav` with volume
`speed / 8`, a pitch from the size, and `pan` from the body's x position.

**Drawing.** There is no mesh. `World` is a React component that re-renders every 16 ms (`useFrame`, a
`setTimeout` loop that bumps a state counter) and maps each `Thing` to an absolutely positioned `box` from
`t.transform.position`. The angle is derived from the quaternion (`angle`). Static pieces (floor, ramp, zone) are
boxes whose pixel sizes mirror the colliders. The ramp is drawn with `transform: "rotate(20deg)"`, matching the
physics rotation.

**Input.** The root `box` has an `onClick` that converts `e.clientX` / `e.clientY` back to metres and calls
`drop`.

**Zone.** `Zone.onTriggerEnter` increments `passes`, which the headline text shows.

## Controls

| Input | Action |
|---|---|
| Left click | Drop a new crate or ball at the cursor |

## Things to try

- Swap the UI drawing for `MeshRenderer` primitives (`GameObject.createPrimitive`) as in `world`.
- Make the zone count only balls, or award score and show it in the headline text.
- Add a `Rigidbody` kinematic paddle that follows the mouse.
- Add joints or a `Rope` (see `world`) to hang things.
- Use `bounciness` and `dynamicFriction` values to tune different materials.

## Gotchas

- Physics runs on fixed ticks; `transform.position` is the pose from the last tick, so the UI is a tick behind
  at most.
- The UI polls (`useFrame`) rather than receiving events from physics; fine for a demo, costly for hundreds of
  bodies.
- Pixel numbers in the view (ramp size, zone size) duplicate the collider numbers; they must be kept in step by hand.
- `things` is module state, not scene state, so it is not saved with the scene.

Related examples: world, platformer, scenes, classics
