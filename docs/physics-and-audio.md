# Physics and audio (M4)

Crates and packages: [iron_physics](../crates/iron_physics/src/lib.rs) (rapier3d 0.36),
[iron_audio](../crates/iron_audio/src/lib.rs) (kira 0.12), their TypeScript sides
[physics.ts](../js/engine/src/physics.ts) and [audio.ts](../js/engine/src/audio.ts) in
`@iron/engine`, and the wiring in [iron_app/main.rs](../crates/iron_app/src/main.rs). On top of the
handles there are Unity-style components ([physics-components.ts](../js/engine/src/physics-components.ts),
[physics2d.ts](../js/engine/src/physics2d.ts), [audio-components.ts](../js/engine/src/audio-components.ts)),
and [physics_sync.rs](../crates/iron_app/src/physics_sync.rs) ties bodies to GameObjects. Demos:
[examples/physics](../examples/physics/src/main.tsx) (handles), [examples/world](../examples/world/src/main.tsx)
and [examples/platformer](../examples/platformer/src/main.tsx) (components).

## Ops and IDs

Both follow the core rule: Rust owns the world, JS owns behaviour. Scripts never hold rapier or
kira handles. They **choose numeric IDs** themselves (module counters in `physics.ts`/`audio.ts`,
like UI node ids), so creating a body or sound needs no round trip, and queue ops with
`__iron_op("physics.<name>", ...)` or `__iron_op("audio.<name>", ...)`.

`App::apply_script_output` routes by prefix: `physics.*` goes to `Physics::apply(name, args)`,
`audio.*` to `App::audio_op`. A bad op (unknown name, wrong arguments, unknown ID) returns an
`Err`; the app logs a warning and carries on.

Physics ops: `gravity`, `timestep`, `body`, `collider`, `removeBody`, `removeCollider`, `set`
(`position`, `rotation`, `velocity`, `angularVelocity`, ...), `force` (`force`, `impulse`,
`torque`, `torqueImpulse`, `impulseAt`), `clearForces`, `character`, `move`, `raycast`, `colliderSet`
(`friction`, `restitution`, `sensor`, `offset`), `joint`, `removeJoint` and `attach`.
`set` also takes `targetRotation`, `enabled`, `kind` (change the body type), `mass`, `gravityScale`,
`linearDamping`, `angularDamping`, `ccd`, `constraints` (six bools: translations then rotations
allowed), `sleep` and `wake`. Colliders are boxes, balls, capsules, cylinders, convex hulls and
triangle meshes (`mesh`, static geometry). A collider whose shape is `renderMesh` is rewritten by the
app into a `mesh` or `convex` one from a render mesh's data
([assets-and-models.md](assets-and-models.md#materials-and-meshes)).

`attach` is not a physics op: `App::apply_script_output` gives each `physics.*` op to
`Attachments::op` first, which handles `attach` and notes `removeBody`, and only the others reach
`Physics::apply`.

Audio ops: `load`, `play`, `set` (`volume`, `rate`, `pan`), `pause`, `resume`, `stop`, `seek`,
`unload`, `bus`, `emitter`, `trackVolume`, `trackPause`, `trackResume`, `emitterPosition`,
`removeTrack`, `listener`. Volumes cross as linear gains and are converted to decibels in Rust.

## Per fixed tick

`App::tick` runs fixed 60 Hz ticks (`TICK`, max 5 per frame; see [scripting.md](scripting.md)). Each tick:

```
1. ComponentHost::fixed_update   component onFixedUpdate code queues commits and ops
2. apply_script_output()         commits -> UiTree, then ops (physics.*, audio.*, ...) in order
3. step_physics()                Attachments::step: bodies follow objects scripts moved, Physics::step,
                                 take_batch, objects follow the bodies that moved;
                                 then __iron_physics(...) if the game has it
```

So what a component asks for in a tick is applied before that tick's step. Collision handlers
run inside step 3 and may queue more ops; those apply on the **next** tick. After the tick loop
the frame runs timers, `apply_script_output()` again, then `update_audio`, which calls
`Mixer::advance(elapsed)` and `poll()` and sends ended playbacks to `__iron_audio(ended)`.

`physics_events` and `audio_events` are probed once at startup
(`typeof __iron_physics === 'function'`, same for `__iron_audio`). A game that never imports the
APIs gets no calls. An empty batch is not sent.

## Batch format

`Physics::take_batch` returns a `Batch` (moved bodies, contacts, ray answers, characters).
`Batch::to_args` turns it into four **flat number arrays**, so one tick crosses the boundary as
four arrays rather than many objects: `__iron_physics(moved, contacts, rays, characters)`.

| Array | Stride | Fields per entry |
|---|---|---|
| `moved` | 14 | `id, px, py, pz, qx, qy, qz, qw, vx, vy, vz, wx, wy, wz` (`w` is angular velocity, rad/s) |
| `contacts` | 5 | `started (1/0), a, b, sensor (1/0), speed` |
| `rays` | 6 | `query, collider (-1 for a miss), distance, nx, ny, nz` |
| `characters` | 2 | `id, grounded (1/0)` |

- `speed` is the relative speed (m/s) of the two colliders going into the contact (rapier's events
  fire before the solver). It is 0 when the contact ends. Sensor overlaps also produce contacts.
- Raycasts queued with `physics.raycast` are answered in the batch after the step, so the
  promise resolves one tick later.
- `__iron_audio(ended)` receives an array of playback IDs.

## TypeScript API

All of it is exported from `@iron/engine`. IDs and ops are hidden behind small handle classes.

```ts
import { physics, audio, Component, register } from "@iron/engine";

physics.addCollider({ shape: { box: { x: 10, y: 0.5, z: 2 } }, position: { x: 0, y: -0.5, z: 0 } });
const body = physics.createBody({ position: { x: 0, y: 5, z: 0 } }); // dynamic by default
body.addCollider({ shape: { ball: 0.5 }, restitution: 0.5 });
body.onCollisionStart = (e) => {
  if (!e.sensor && e.speed > 1) audio.play("assets/thud.wav", { volume: e.speed / 8 });
};
// Updated after each tick that moved the body:
console.log(body.position, body.rotation, body.velocity);
```

Physics:

- `physics.createBody({ kind, position, rotation, velocity, gravityScale, linearDamping,
  angularDamping, ccd, lockRotations })`. `kind` is `dynamic`, `fixed` or `kinematic`.
  `physics.addCollider` with no body makes static world geometry.
- The handle classes are `PhysicsBody` and `PhysicsCollider` (the names `Rigidbody` and `Collider` now
  belong to the components below). `PhysicsBody`: `addCollider`, `setPosition`, `setRotation`, `setVelocity`, `setAngularVelocity`,
  `moveTo` (kinematic), `applyImpulse`, `applyTorqueImpulse`, `addForce`, `addTorque`,
  `clearForces`, `setEnabled`, `remove`, plus `applyImpulseAtPoint`, `setKind`, `setMass`,
  `setGravityScale`, `setDamping`, `setCcd`, `setAllowedAxes`, `sleep`, `wakeUp`, `attachTo(objectId)`,
  and `position`/`rotation`/`velocity`/`angularVelocity` from the last batch.
  Callbacks: `onCollisionStart`, `onCollisionEnd` get a `CollisionEvent`
  (`collider`, `other`, `sensor`, `speed`).
- `PhysicsCollider` options: `shape`, `position`, `rotation`, `friction` (0.5), `restitution` (0),
  `density` (1), `sensor`. `setFriction`, `setRestitution`, `setSensor` and `setOffset` change it later.
  A shape can also be `{ renderMesh: { mesh, convex, scale } }`.
- `physics.createJoint(kind, a, b | null, anchorA, anchorB, params)` returns a `PhysicsJoint`
  (`remove()`); kinds are `fixed`, `ball`, `hinge`, `spring` and `rope`.
- `physics.setGravity(v)`, `physics.body(id)`.
- `physics.raycast(origin, direction, maxDistance = 1000)` returns a `Promise<RayHit | null>`
  (`collider`, `distance`, `point`, `normal`).
- Characters: `body.makeCharacter({ maxSlope, snapToGround, stepHeight })` on a kinematic body
  with a capsule, then `body.move(delta)` each tick (moves in one tick add up). Include a small
  downward pull; `grounded` updates after the tick.

Audio (volumes are linear gains, times are seconds; fades are an optional last argument):

```ts
const music = audio.load("assets/music.ogg", { stream: true }).play({ loop: true, fadeIn: 2 });
const sfx = audio.createBus({ volume: 0.8 });
const lamp = audio.createEmitter({ x: 3, y: 1, z: 0 }, { minDistance: 1, maxDistance: 30 });
audio.play("assets/hum.wav", { on: lamp });
audio.setListener(player.position, player.rotation); // emitters pan and fade from here
music.setVolume(0.2, 1.5);
```

- `audio.load(src, { stream })` returns a `Sound` (`play`, `unload`). `audio.play(src, options)`
  loads (cached) and plays in one call. `PlayOptions`: `volume`, `rate`, `pan`, `loop`, `fadeIn`,
  `start`, `on` (a `Bus` or `Emitter`).
- `Playback`: `setVolume`, `setRate`, `setPan`, `pause`, `resume`, `stop`, `seek`.
- `Bus` (`audio.createBus({ volume, parent })`): `setVolume`, `pause`, `resume`, `remove`.
- `Emitter` (`audio.createEmitter(position, { minDistance, maxDistance, bus })`): `setPosition`,
  `setVolume`, `remove`.
- `audio.setListener(position, rotation)`, `audio.setMasterVolume(volume, fade)`.

`audio.load` sends `audio.load id src stream`. `load_sound` in `main.rs` reads the file from the
game folder through `resolve_asset` (relative paths, no `..`) or decodes a `data:` URL, then
hands the bytes to `Mixer::load`. Static sounds are decoded up front; `stream: true` decodes
while playing.

## Components on GameObjects (M5.a2)

Scripts normally use components rather than the handles above. They are built on the handles and live in
[physics-components.ts](../js/engine/src/physics-components.ts); nothing in Rust knows about
components, only about bodies and the `attach` link.

**One body per object.** `physicsFor(go)` makes a `fixed` body at the object's pose the first time any
physics component needs it and sends `physics.attach body object`. `Rigidbody` switches that body to
`dynamic` (or `kinematic`); colliders become `PhysicsCollider`s on it, with the object's scale applied
when added (changing the scale later does not resize them). An object with colliders and no Rigidbody
is static. `releasePhysics` removes the body when nothing on the object needs it.

**Sync both ways** ([physics_sync.rs](../crates/iron_app/src/physics_sync.rs)). `Attachments` keeps
`body <-> object` maps. In `Attachments::step`:

1. `ScriptScene::take_script_moved` lists the objects scripts moved or re-parented since the last step
   (and everything under them). Each attached one gets `Physics::set_pose` from its world transform: a
   kinematic body travels there over the step (pushing things on the way), others teleport and keep
   their velocity, like setting a Unity transform.
2. `Physics::step`, then `take_batch`.
3. For each moved body in the batch, `ScriptScene::set_world_from_engine` moves its object, which feeds
   the scene move batch to scripts (see [scene-and-objects.md](scene-and-objects.md#the-move-batch)).

The result is returned so `step_physics` can still call `__iron_physics`.

**Components.**

- `Rigidbody`: `mass`, `drag`, `angularDrag`, `useGravity`, `isKinematic`, `constraints`
  (`RigidbodyConstraints` flags), `collisionDetection` (`continuous` turns CCD on), `velocity`,
  `angularVelocity`, `addForce(force, ForceMode)` (`Force`, `Acceleration`, `Impulse`,
  `VelocityChange`), `addTorque`, `addForceAtPosition`, `addExplosionForce`, `movePosition`,
  `moveRotation`, `sleep`, `wakeUp`. Setters send `physics.set` ops.
- `BoxCollider`, `SphereCollider`, `CapsuleCollider` (`size`/`radius`/`height`, `center`), `MeshCollider`
  (`points` and `indices`, or `sharedMesh` for a render mesh; `convex` makes a hull, which can move
  with a Rigidbody), all with `isTrigger` and a `PhysicsMaterial` (`friction`, `restitution`). Changes
  send `physics.colliderSet` or rebuild the collider.
- `CharacterController`: a capsule on a kinematic body (`radius`, `height`, `slopeLimit`, `stepOffset`)
  moved with `move(motion)` or `simpleMove(speed)` (which adds gravity). `isGrounded` and `velocity`
  come from the last step. It is a handle-level character underneath (`makeCharacter`).
- Joints (`Joint` subclasses): `FixedJoint`, `CharacterJoint` (ball and socket), `HingeJoint` (axis,
  limits, motor), `SpringJoint` (rest length, stiffness, damping) and `RopeJoint` (max distance). Each
  joins the object's Rigidbody to `connectedBody`, or to the world when it is null. `anchor` is in the
  object's space; `connectedAnchor` is worked out from where the bodies are unless
  `autoConfigureConnectedAnchor` is off. They are made in `onStart` and send `physics.joint`. In Rust
  `add_joint` builds a rapier impulse joint with `contacts_enabled(false)`, so joined bodies do not
  collide with each other. A joint to the world hangs from `world_anchor`, a fixed body made when first
  needed.
- `Rope`: a chain of capsule segments (each a GameObject with a CapsuleCollider and Rigidbody)
  joined by ball joints, from this object to an optional `end` Rigidbody. By default it's drawn as
  one smooth tube: a `LineRenderer` (`js/engine/src/lines.ts`) on its own object, given the links'
  joint points each `onPreRender`. It rebuilds a script `Mesh` (Catmull-Rom smoothed path, rings
  carried along by parallel transport so they don't twist) only when the points change.
  `renderMode: "segments"` draws each link as a capsule instead.

**Callbacks.** The handle layer calls every function in `contactListeners` for each contact. The
component layer's listener looks both colliders up in `colliderOwners` and calls these on every
component of both objects: `onCollisionEnter(collision)`/`onCollisionExit` (a `Collision` with
`gameObject`, `collider`, `rigidbody`, `relativeSpeed`) and, for sensor contacts,
`onTriggerEnter(other)`/`onTriggerExit`. 2D colliders get the `...2D` versions instead (Unity keeps
them apart). A throwing listener is logged and the others still run.

### 2D physics

[physics2d.ts](../js/engine/src/physics2d.ts) has `Rigidbody2D` (`bodyType` dynamic, kinematic or
static; `mass`, `gravityScale`, `drag`, `freezeRotation`, `velocity`, `addForce(ForceMode2D)`),
`BoxCollider2D`, `CircleCollider2D`, `CapsuleCollider2D`, `PolygonCollider2D` and `EdgeCollider2D`, with
`Vector2` APIs and angles in degrees about z.

There is no separate 2D world. 2D runs in the **same rapier world**: a Rigidbody2D's body can only move
in x and y and turn about z (`constraints`), and each 2D collider is its outline extruded into a **slab**
`Physics2D.depth` (default 10 m) deep, centred on the object, so 2D objects collide whatever their z.
Box, circle, capsule and polygon colliders become convex hulls of the outline's points at both faces; `EdgeCollider2D` is a triangle mesh strip along its line, so it is for static scenery and kinematic platforms. `Physics2D.gravity` sets the shared
world gravity (z stays 0). `Physics2D.raycast(origin, direction, distance)` casts in the XY plane
through `physics.raycast` and resolves one tick later with a `RaycastHit2D` (the component, point,
normal, distance). Callbacks are `onCollisionEnter2D`/`Exit2D` (`Collision2D`) and
`onTriggerEnter2D`/`Exit2D`.

### Audio components

[audio-components.ts](../js/engine/src/audio-components.ts):

- `AudioSource`: `clip` (a `Sound` or a path; paths are loaded once and shared), `playOnAwake`, `volume`,
  `pitch`, `panStereo`, `loop`, `mute`, `spatialBlend`, `minDistance`, `maxDistance`, `outputBus`, and
  `play`, `stop`, `pause`, `unPause`, `playOneShot(clip, volumeScale)`. With `spatialBlend` 0 it plays flat
  on `outputBus`; above 0 (there is no partial blend) it makes an `Emitter` that follows the object and
  plays from there.
- `AudioListener`: sends `audio.setListener(position, rotation)` from its object on `onAwake` and every
  `onPreRender`, only when the pose changed. Put one on the camera's object. With none, the listener
  stays at the origin facing -z. `AudioListener.volume` is the master volume.

## The mixer and offline mode

`iron_audio::Mixer` is the trait the app holds (`load`, `apply`, `poll`, `advance`).
`open_mixer(offline)` returns a kira device mixer, or an offline one when `offline` is true or no
output device could be opened (a warning is logged).

The app opens the mixer **lazily**, on the first `audio.*` op, so games without sound never touch
a device. `--screenshot` runs mix offline (`self.screenshot.is_some()`), silently.

[offline.rs](../crates/iron_audio/src/offline.rs) `OfflineBackend` has no device. `render(frames)`
mixes stereo frames on demand, which is how the audio tests measure output. `advance(seconds)` on
an offline mixer renders and drops that much audio, so sounds still progress and end (and
`__iron_audio` still fires) with no device.

## Known limits

- Colliders follow scale changes: setting `localScale` or reparenting calls `scaleListeners`
  (`scene.ts`), and every collider on the object and its children whose world scale changed since
  it was made rebuilds its shape. Joint anchors don't follow.
- Physics is reset on hot reload (a new `Physics` and empty `Attachments`), so body IDs are not preserved (M4-PHY-04 waits for hot reload part 2; see [scripting.md](scripting.md#hot-reload)).
- rapier's character autostep fails if a move pushes down as steeply as it moves forward. A
  strong downward pull can stop a character climbing a step.
- Collision handlers' ops apply one tick late (see above), and ray hits arrive one tick after the ask.
- Character moves wait a step after colliders are added or removed (`dirty`): rapier's query structures
  only learn them in a step, and moving through a floor made this tick would be wrong. Setting a body's
  properties no longer marks the world dirty.
- `physics.mesh` colliders are for static geometry; use `convex` for moving ones.
- `Time.timeScale` does not scale physics.
- `PolygonCollider2D` is convex: a concave outline becomes the hull of its points.
