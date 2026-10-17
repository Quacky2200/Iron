# Physics and audio (M4)

Crates and packages: [iron_physics](../crates/iron_physics/src/lib.rs) (rapier3d 0.36),
[iron_audio](../crates/iron_audio/src/lib.rs) (kira 0.12), their TypeScript sides
[physics.ts](../js/engine/src/physics.ts) and [audio.ts](../js/engine/src/audio.ts) in
`@iron/engine`, and the wiring in [iron_app/main.rs](../crates/iron_app/src/main.rs).
Demo: [examples/physics](../examples/physics/src/main.tsx).

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
`torque`, `torqueImpulse`), `clearForces`, `character`, `move`, `raycast`. Colliders are boxes,
balls, capsules, cylinders, convex hulls and triangle meshes (`mesh`, static geometry).

Audio ops: `load`, `play`, `set` (`volume`, `rate`, `pan`), `pause`, `resume`, `stop`, `seek`,
`unload`, `bus`, `emitter`, `trackVolume`, `trackPause`, `trackResume`, `emitterPosition`,
`removeTrack`, `listener`. Volumes cross as linear gains and are converted to decibels in Rust.

## Per fixed tick

`App::tick` runs fixed 60 Hz ticks (`TICK`, max 5 per frame; see [scripting.md](scripting.md)). Each tick:

```
1. ComponentHost::update(TICK)   component onUpdate code queues commits and ops
2. apply_script_output()         commits -> UiTree, then ops (physics.*, audio.*, ...) in order
3. step_physics()                Physics::step, take_batch, __iron_physics(...) if the game has it
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
| `moved` | 11 | `id, px, py, pz, qx, qy, qz, qw, vx, vy, vz` |
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
- `RigidBody`: `addCollider`, `setPosition`, `setRotation`, `setVelocity`, `setAngularVelocity`,
  `moveTo` (kinematic), `applyImpulse`, `applyTorqueImpulse`, `addForce`, `addTorque`,
  `clearForces`, `setEnabled`, `remove`, plus `position`/`rotation`/`velocity` from the last batch.
  Callbacks: `onCollisionStart`, `onCollisionEnd` get a `CollisionEvent`
  (`collider`, `other`, `sensor`, `speed`).
- `Collider` options: `shape`, `position`, `rotation`, `friction` (0.5), `restitution` (0),
  `density` (1), `sensor`.
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

- There are no GameObject components yet (M5.a). Bodies and sounds are created by scripts
  directly, and nothing ties them to scene objects.
- Body IDs are not preserved across hot reload yet (M4-PHY-04 is pending).
- rapier's character autostep fails if a move pushes down as steeply as it moves forward. A
  strong downward pull can stop a character climbing a step.
- Collision handlers' ops apply one tick late (see above), and ray hits arrive one tick after the ask.
- `physics.mesh` colliders are for static geometry.
