# classics

A gallery of the classic computer-graphics test models: the Stanford Bunny and the Newell teaset
(teapot, teacup, teaspoon) as glTF binaries. Each stands on a pedestal and turns under its own spotlight,
while the camera orbits. Behind them is a row of six spheres with `smoothness` from 0 to 1 (the classic
material sweep).

APIs used: `Model.load(path).instantiate(...)`, `Material`, `Light` (directional and spot), `CameraOrbit`,
`RenderSettings.skybox`, `QualitySettings.shadowDistance`, `Transform.rotate` and `lookAt`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/classics
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/classics
```

Screenshot run: `cargo run -p iron_app -- examples/classics --screenshot out.png --frames 30`. The
interactive window may crash some GPUs, so the screenshot run is safest. Package with
`iron-build package ../examples/classics`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | `Turntable` and `Gallery` components, a credits overlay |
| `assets/bunny.glb`, `teapot.glb`, `teacup.glb`, `spoon.glb` | The models (plus `.meta` GUID files) |
| `tools/make_models.py` | Offline script that converted the source models to `.glb` |

`make_models.py <bun_zipper_res2.ply> <newell_teaset dir>` needs the sources downloaded separately (they are not in the
repo). It scales the bunny to 1 m tall and the teaset to a shared scale (teapot 1 m tall), puts them on
y = 0, centres them in x and z and smooths the bunny's normals.

## How it works

1. **Camera.** `Gallery.onStart` creates a camera and adds `CameraOrbit`, an engine component that circles a
   centre point. `autoRotate: 6` sets the automatic rotation speed:

```ts
cam.addComponent(CameraOrbit, { center: new Vector3(0, 0.8, 0), distance: 6.5, pitch: 18, yaw: 10, autoRotate: 6 });
```

2. **Sky.** `RenderSettings.skybox` is set to a dusk gradient (`zenith`, `horizon`, `ground`) with a very
   bright, low `sunColor` (12, 7, 4). The sky also lights and is reflected by the glossy models.
3. **Lights.** A soft-shadow directional key light (`Quaternion.euler(-45, 35, 0)`), a weaker blue rim
   light from behind, and `QualitySettings.shadowDistance = 14`.
4. **Material sweep.** A loop makes six spheres, each with `new Material(Color.hex("#c23a3a"), Color.black, i / 5)`
   (the third argument is smoothness, as the file's comment says).
5. **Pedestals.** A `pieces` array lists file, position and scale (the spoon is shown at scale 3 because
   it is small next to the teapot). For each piece the code creates a cylinder pedestal, a spotlight placed
   above and in front and aimed with `lookAt`, then loads the model:

```ts
const model = Model.load(p.file).instantiate({ position: p.at.add(new Vector3(0, 0.3, 0)), scale: Vector3.one.scale(p.scale) });
model.addComponent(Turntable, { degreesPerSecond: 15 + i * 5 });
```

6. **Turntable.** The component turns its object about y every frame with `this.transform.rotate(new Vector3(0, degreesPerSecond * dt, 0))`.
7. **Credits.** A React overlay (`createRoot().render(<Credits />)`) credits the Stanford lab and Martin Newell.

## Controls

None; the camera orbits by itself. (`CameraOrbit` may also respond to input; the example does not use it.)

## Things to try

- Swap in your own `.glb` in `pieces` and set `scale` until it fits the pedestal.
- Add a `metallic` sweep next to the smoothness row (see `materials`).
- Stop the orbit with `autoRotate: 0` and place the camera yourself.
- Add a `Cloth` or `Rigidbody` to a model (see `cloth`, `physics`).
- Add `quality` levels to `project.json` and compare shadow and MSAA settings (see `quality`).

## Gotchas

- Model paths are relative to the game folder. `.meta` files (GUIDs, import settings) sit beside the assets and
  are written by `iron-build assets`.
- The example keeps the models' credits in an overlay; keep that if you reuse them.
- The spoon is small in the file, so the example scales it per piece (`scale: 3`) instead of editing the model.
- `Material` has a positional form `new Material(color, emission, smoothness)` (used here) and an options form
  `Material.create({ ... })` (used in `animation`).

Related examples: cornell, materials, animation
