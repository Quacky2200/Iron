# world

The main 3D tour of GameObjects. A camera orbits a lit floor with a ring of coloured panels that spin as one
object, a top-down minimap camera in the corner, a React menu standing in the world on a `WorldPanel`, falling
physics primitives that thud in 3D, a particle fountain, a point light with a halo, a visible spotlight beam,
bloom, a rope with a swinging ball, a glTF house with mesh colliders, and animated pixel-art coins that face
the camera. Screen UI is drawn over all of it.

Engine features used: `GameObject` and `parent` hierarchies, `Camera` (perspective, orthographic, `depth`, `rect`),
`CameraOrbit`, `AudioListener` / `AudioSource`, `MeshRenderer` + `Material`, `Light` (directional, point, spot),
`RenderSettings.bloom`, `Model.load(...).instantiate`, `GameObject.createPrimitive`, `Rigidbody`, `BoxCollider`,
`Rope`, `ParticleSystem`, `SpriteRenderer` + `Sprite.grid`, `PlaneRenderer`, `WorldPanel`, `Texture.create`,
`Random`, `addStyleSheet`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/world
```

Then, from the repo root:

```bash
cargo run -p platypus_app -- examples/world
```

Assets can be prepared with `cargo run -p platypus_app -- examples/world --cook` (textures and models into
`.platypus/cooked/`). A screenshot run is `platypus-game examples/world --screenshot out.png --frames N`; use it first, as
the interactive window has crashed some GPUs.

## Structure

| File | What |
|---|---|
| `src/main.tsx` | Components `Spin`, `Thud`, `SpriteAnimation`, `World` (builds the scene in `onStart`), the `Menu` and `Hud` |
| `project.json` | `name: world`, `title: Platypus - World` |
| `assets/house.glb` | The glTF model |
| `assets/coin.png` | A 4-frame, 16 px coin sprite sheet |
| `assets/thud.wav` | Impact sound for the falling primitives |

## How it works

`register({ World })` makes `World` a startup component; everything is created in `World.onStart`.

**Cameras.** The main camera has a `CameraOrbit` (centre, distance, pitch, yaw, `autoRotate`) and an
`AudioListener`. A second camera looks straight down, orthographic, and draws on top because of a higher `depth`,
into a corner `rect`:

```ts
const map = new GameObject("Minimap", { position: new Vector3(0, 20, 0), rotation: Quaternion.euler(-90, 0, 0) });
map.addComponent(Camera, { orthographic: true, orthographicSize: 6.5, depth: 1, rect: new Rect(0.76, 0.03, 0.21, 0.35) });
```

**Floor, lights, effects.** A `MeshRenderer` plane with a `Material` (colour, emission, smoothness) on a scaled
child, a `BoxCollider` for physics, a soft-shadow directional `Light`, a point `Light` with `haloSize`, a spot
`Light` with `beamIntensity` aimed with `transform.lookAt`, and `RenderSettings.bloom`. The fountain is a
`ParticleSystem` with `shape: { type: "cone" }`, additive blending and `colorOverLifetime`.

**Physics props.** Ten primitives (cube, sphere, capsule, cylinder) from `createPrimitive` get a `Rigidbody`, a
spatial `AudioSource` and the `Thud` component. `Random.initState(3)` keeps the layout repeatable. Cubes use a
texture painted in script (`crateTexture`). A `Rope` hangs a heavy ball from a hook: `Rope` with `end: ballBody`.

**Hierarchy.** Six `PlaneRenderer` panels and floor tiles are children of the `Ring` object, which has a
`Spin` component; rotating the parent moves the children. Child positions are set from a rotated forward vector:

```ts
const dir = Quaternion.euler(0, angle, 0).rotate(Vector3.forward);
```

**Model.** `Model.load("assets/house.glb")` is instantiated twice with different transforms and
`colliders: "mesh"`.

**Sprites.** `Texture.load(...).whenLoaded().then(...)` waits for the sheet before `Sprite.grid(sheet, 4, 1,
undefined, 40)` cuts four frames. `SpriteAnimation` swaps frames in `onUpdate` and copies the camera's rotation in
`onLateUpdate` so the sprite faces the viewer (a billboard).

**UI.** A `WorldPanel` (640x400 px at 170 px per metre) is a GameObject component; `createRoot(panel).render(<Menu />)`
puts a React tree on it. Pointer hover and clicks are routed through the camera. `addStyleSheet` supplies
`:hover` and `transition` rules. The screen `Hud` sets `pointerEvents: "none"` on its full-screen box so the
pointer reaches the world panel.

## Controls

There are no key bindings. The pointer hovers and clicks the three items on the in-world menu (click counter).
The camera orbits by itself (`autoRotate: 10`).

## Things to try

- Make the camera interactive: replace `autoRotate` with mouse look, or follow a player (see `platformer` for a 2D
  follow script).
- Add shoot-and-click: `Camera.main` plus a physics raycast to pick one of the primitives.
- Put real game UI (health, inventory) on a `WorldPanel` attached to an object.
- Swap the house for your own `.glb` and keep `colliders: "mesh"`.
- Turn on `RenderSettings` fog or change `Light.shadows` to compare cost.

## Gotchas

- Planes lie flat by default; the ring panels are stood up with `Quaternion.euler(90, 0, 0)`.
- Texture loads are asynchronous: the coins are created inside `whenLoaded().then(...)`.
- A full-screen UI box blocks the pointer unless it has `pointerEvents: "none"`.
- The HUD refreshes with a 100 ms `setTimeout` loop and `useState`, not each frame.
- The unit scale matters: the floor is a 10 m Unity-style plane scaled by 1.2.

Related examples: physics, platformer, scenes, classics, materials, cornell
