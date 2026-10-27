# cloth

Three `Cloth` components on meshes built in code: a flag pinned along its pole edge and flapping in gusty
wind, a tablecloth dropped onto a ball and a capsule, and a curtain pinned at points along its rail. It
shows how to make a mesh from arrays (`Mesh.create`) and attach the cloth simulation to it.

APIs used: `Mesh.create({ vertices, uv, triangles })`, `Cloth` (`pinBox`, `pins`, `pinRadius`,
`externalAcceleration`, `randomAcceleration`, `bendingStiffness`, `damping`, `friction`,
`sphereColliders`, `capsuleColliders`), `SphereCollider`, `CapsuleCollider`, `Material.create` with
`doubleSided`, `CameraOrbit`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/cloth
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/cloth
```

Screenshot run: `cargo run -p iron_app -- examples/cloth --screenshot out.png --frames 120` (use enough
frames for the cloth to settle). The interactive window may crash some GPUs; the screenshot run is
safest. Package with `iron-build package ../examples/cloth`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | `sheet()` mesh builder, the `Stage` component, a caption in React |

No assets. The physics behind `Cloth` is `iron_physics::cloth` (`docs/physics-and-audio.md`).

## How it works

1. **Scene setup.** `Stage.onStart` creates a camera with `CameraOrbit` (centre `(0, 1.4, 0)`, distance 10,
   `autoRotate: 0`, so it holds still), a sky, a soft-shadow sun and a floor plane.
2. **The mesh.** `sheet(w, h, cols, rows)` builds a grid of vertices in the xy plane, matching UVs and two
   triangles per cell. The cloth simulates the mesh's vertices, so more cells means smoother cloth and more cost:

```ts
const stride = cols + 1;
for (let r = 0; r < rows; r++) {
  for (let c = 0; c < cols; c++) {
    const a = r * stride + c;
    triangles.push(a, a + stride, a + 1, a + 1, a + stride, a + stride + 1);
  }
}
```

3. **Double-sided fabric.** `fabric(color)` returns a `Material.create({ color, smoothness: 0.15, doubleSided: true })`
   so both faces of the sheet render.
4. **Flag.** A `MeshRenderer` with `sheet(2, 1.2, 24, 14)` and a `Cloth` that pins vertices inside a box (`pinBox`,
   in the object's local space, along the left edge), blown by a steady push plus random gusts:

```ts
pinBox: { min: new Vector3(-1.05, -1, -1), max: new Vector3(-0.95, 1, 1) },
externalAcceleration: new Vector3(6, 0, 1.5),
randomAcceleration: new Vector3(8, 2, 6),
```

5. **Tablecloth.** A 2.6 m sheet, rotated flat with `Quaternion.euler(-90, 0, 0)`, 2.4 m up. It has no pins, so it
   falls. The ball and capsule have a normal collider each (`SphereCollider`, `CapsuleCollider`), and the cloth
   lists them: `sphereColliders: [ball], capsuleColliders: [post]` with `friction: 0.8`.
6. **Curtain.** A 2.2 m sheet pinned only at five points (`pins`, positions along the top row, each with a
   `pinRadius` of 0.06), swaying under a light `randomAcceleration`.
7. **UI.** A caption box with `pointerEvents: "none"` so it does not block the mouse.

## Controls

None; the camera is fixed (`autoRotate: 0`).

## Things to try

- Raise `externalAcceleration.x` on the flag to see it stand out.
- Pin the tablecloth's four corners with `pins` to make a hammock.
- Add more colliders to the `sphereColliders` list and drop the sheet over them.
- Drive wind from input: set the `Cloth` component's acceleration from `input.getAxis` in an `onUpdate`
  (check `docs/physics-and-audio.md` for which properties can change at run time).
- Use `sheet()` with a texture (`mainTexture` in `Material.create`) for a printed flag.

## Gotchas

- The sheet faces +z by construction and is rotated into place by the object's transform; pins and `pinBox` are in local space.
- `doubleSided: true` is needed or one side of the cloth is invisible.
- Cloth self-collision is not implemented yet (CLAUDE.md lists it as a later item), so a sheet can pass through itself.
- Cell counts (24x14, 26x26, 22x22) are chosen for the demo; very fine meshes cost more per frame.

Related examples: animation, physics, classics
