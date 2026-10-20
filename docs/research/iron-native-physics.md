# Physics in Rust for compiled scripts: design note

Status: **decisions taken, two questions open** (2026-10-20). Next after scenes in the order the user gave (components, math, physics,
audio, rendering). `physics.ts` (bodies, colliders, joints, raycasts), `physics-components.ts` (Rigidbody, the collider
components, CharacterController, joints, Rope) and `physics2d.ts` are 1,860 lines over `crates/iron_physics`, which already
simulates everything. What is left is the script surface, and three pieces of boundary the compiler does not have yet.

## 1. What is needed that does not exist

| Need | Used by | Proposal |
|---|---|---|
| **Arrays of numbers / `Vector3` into the engine** | `convex` and `mesh` shapes, `LineRenderer` points, `AnimationCurve.fromPoints` | `ExtTy::F64Arr` and `ExtTy::ValueArr(Vector3)`: a typed array / `Vector3[]` is passed as pointer and length. Results come back through a count and an indexer, as `Scene.children` does. |
| **Callbacks into script** | `onCollisionEnter/Exit/Stay`, `onTriggerEnter/Exit` | They are component hooks, not closures: `NativeHost` gets hooks 9 to 13 (taking the other collider's component or object). The physics step returns contact events and the host dispatches them after the fixed tick, like `FiredEvent` for animation. `PhysicsBody.onCollisionStart` closures stay QuickJS-only until closure APIs exist. |
| **`raycast` as a Promise** | `physics.raycast` | The world is in process, so the native `Physics.raycast(origin, dir, max)` answers at once (the QuickJS op path keeps its one-tick delay). A hit is read through `RayHit` handles (`collider`, `distance`, `point`, `normal`). This is a difference to list for review. |

## 2. Shape

- `iron_physics` keeps the rapier world and script-chosen ids. A new `crates/iron_physics_api` (or a module of `iron_engine_api`)
  declares `Rigidbody`, `BoxCollider`, `SphereCollider`, `CapsuleCollider`, `MeshCollider`, `CharacterController`, the joints
  and `Physics` (`gravity`, `raycast`, `overlap*`) with `#[iron_api]`, as handle classes whose objects are the GameObjects
  (like `Camera`): `go.addComponent(Rigidbody, { mass: 2 })`, `go.getComponent(BoxCollider)` through `findOn`.
- Each registers a **kind** (`iron_kinds`), so scene files with physics components load, and `instantiate` copies them.
- A fixed tick is `physics::step(dt)` and then `sync` writes the moved bodies back into `iron_scene` (the existing
  `take_moved` / `attach` path) and returns the contact events for the host.
- 2D physics is the same registry with a `2D` suffix and the z axis locked, as `physics2d.ts` does today.

## 3. Order

1. Arrays across the boundary (compiler, macro, runtime) with tests against bun.
2. Component hooks for contacts in `NativeHost` and the `Collision` argument.
3. `Rigidbody` + colliders + `Physics.raycast` (compared by a fixed-seed simulation against the QuickJS path: the same
   rapier world, so the numbers must match).
4. Joints, `Rope`, `CharacterController`, then 2D.
5. Kinds for scene files.

## 4. Open questions for review

- A raycast that answers at once changes when a script sees a hit (same tick instead of the next). Keep the one-tick delay for
  compatibility, or take the simpler answer?
- Should `Collision` be a handle class (`collision.other`, `collision.speed`) or the two numbers `(otherObject, speed)`?

## 5. Decisions (user, 2026-10-20)

- `raycast` is synchronous and returns the hit (or none): no Promise, no one-tick delay on the native path.
- Arrays are stored natively (typed arrays and `Vector3[]` are real arrays in the engine and in compiled code), not copied into
  QuickJS-style lists. `points` of a shape is such an array.
- Vectors and points live on the stack: value classes (`Vector3`, `Quaternion`, `Color`, ...) stop being heap objects in compiled
  code (scalar replacement; passed and returned in registers or in a caller-owned slot). This is its own compiler step, done
  before physics because every physics call passes them.
- JSON is as native as possible: `JSON.parse` / `stringify` in compiled code are Rust (they already are); on the QuickJS side
  the bridge copies once per call and gets a native-speed path later with rquickjs work.
- Contact events are component hooks called by name (`onCollisionEnter`, `onCollisionExit`, `onTriggerEnter`, `onTriggerExit`,
  plus `onCollisionStay`).

## 6. Order now

1. Stack value classes in the compiler (with a benchmark against the heap version).
2. Native arrays across the boundary.
3. Collision hooks in `NativeHost`.
4. Rigidbody, colliders and `Physics` queries; then joints, character controller, 2D; then kinds for scene files.

Open: the shape of the `Collision` argument (a value type `Collision { other, collider, speed, sensor }`, recommended, or two
loose parameters), and how value-class `@property` fields are written back once values are on the stack (a fresh value, not
in place).
