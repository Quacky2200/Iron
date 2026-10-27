# Particles

Code: [particles.ts](../js/engine/src/particles.ts) (the `ParticleSystem` component),
[particles.rs](../crates/platypus_app/src/particles.rs) (the simulation),
[world_view.rs](../crates/platypus_app/src/world_view.rs) (ops, per-frame update, quads) and the
`__platypus_particles` call in [main.rs](../crates/platypus_app/src/main.rs). Particles are drawn as ordinary
camera-facing `WorldQuad`s, so sorting, blending and textures are in
[world-rendering.md](world-rendering.md) and [assets-and-models.md](assets-and-models.md).

The simulation runs in Rust on the CPU. Scripts only send settings and play/stop commands, and never see
individual particles.

## Settings

`ParticleSystem` has Unity's fields: main (`duration`, `loop`, `playOnAwake`, `startDelay`,
`startLifetime`, `startSpeed`, `startSize`, `startRotation`, `angularVelocity`, `startColor`,
`gravityModifier`, `drag`, `simulationSpace`, `maxParticles`, `randomSeed`), emission (`rateOverTime`,
`bursts`), `shape`, `sizeOverLifetime`, `colorOverLifetime`, `texture`, `blendMode` (`alpha` or
`additive`) and `textureSheet`. A `MinMax` is a number or `[min, max]` (random per particle; `startColor`
takes two colours the same way).

The component serialises the lot into one flat list of key/value pairs and sends
`render.particles component object [key, value, ...] playOnAwake`. It sends again before a frame
(`onPreRender`) when the serialised list changed, so edits like `ps.shape.radius = 2` just work.
`Settings::parse` in Rust reads the pairs over the defaults; an unknown key or bad value is an error
that logs a warning. New settings **keep the live particles and the clock** (`set_settings`).

Shapes emit along the object's **up (+y)**, unlike Unity's +z cones, so a fountain needs no rotation:
`point`, `sphere`, `hemisphere`, `cone { angle, radius }` (spread from straight up, from a disc) and
`box`.

## Control and reports

`play`, `stop(clear?)`, `pause`, `clear` and `emit(count)` send `render.particleControl component action
[count]`. `emit` is queued and runs at the next update, when the object's position is known.

Each frame, in `App::tick` right after the component hooks, `WorldViews::update_particles(dt, scene)`
advances every system on an active object by the frame's scaled `dt` (`ComponentHost::frame_dt`, so
`Time.timeScale` applies). It returns `[id, count, alive, ...]` for systems whose particle count or
alive state changed, and the app calls `__platypus_particles` with that list. The component uses it for
`particleCount`, `isPlaying` and, when a one-shot system finishes, `onStopped` and `stopAction:
"destroy"`.

## Simulation

`ParticleSystem::update(dt, world)` in `particles.rs`:

1. If playing, the clock advances. While emitting, `emit_due` works out the cycle (`duration`, after
   `startDelay`), fires every burst that is due and not yet fired this cycle (`cycles` 0 repeats every
   `interval`), and adds `rateOverTime * dt` to a fractional counter, emitting whole particles. A
   non-looping system stops emitting after one cycle. Emission is capped by `maxParticles`.
2. Each particle gets gravity (`9.81 * gravityModifier` down), drag (a fraction of speed lost per
   second), its angular velocity and its position. With `simulationSpace: "local"` positions are in
   the object's space and move with it, and gravity is converted into that space so it still points down
   in the world. In `world` space particles stay where they were emitted.
3. Particles past their lifetime are dropped.

Random values come from a small seeded xorshift32 (`randomSeed`), so the same seed repeats.

## Drawing

For each camera `WorldViews::views` calls `quads(world, camera_rotation, out)`: one `WorldQuad` per
particle, rotated to face that camera (plus its own spin about the view axis). Size and colour are the
particle's start value times the over-lifetime curves, which are linear between keys (`(time, value)`
or `(time, r, g, b, a)` rows in the op). With a texture sheet the frame is picked from the particle's
life: `life * cycles * frames`. The texture defaults to `DEFAULT_PARTICLE_TEXTURE`, the soft spot.
`additive` quads add light and write no depth; alpha ones sort with the other see-through things.

## Limits

- Particles are quads only (no meshes, ribbons or trails), with no collisions, force fields or
  sub-emitters.
- Every particle is its own `WorldQuad`, rebuilt per camera per frame.
- The simulation does not run on inactive objects, and particles in flight are not kept across hot reload (the emitters are, so they start emitting again).

## Tests

Unit tests in `particles.rs` (emission rate and bursts, lifetimes, gravity and cone direction, curves
and sheets) and `js/engine/test/particles.test.ts`.
