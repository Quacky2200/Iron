**Title:** Friction is lost entirely for 3D bodies locked to a plane (locked translation + perpendicular rotations)

**Version:** rapier3d 0.36.1, both `FrictionModel::Simplified` (the default) and `FrictionModel::Coulomb`.

### What happens

A dynamic body with `enabled_translations(true, true, false)` and `enabled_rotations(false, false, true)` gets no friction at all.

That lock is the usual way to keep 3D bodies in a plane. It's also how engines run 2D physics inside a 3D rapier world, so 2D and 3D objects share one simulation (Unity-style `Rigidbody2D` components, side-on 2D scenes, 2.5D games). In that setup *every* 2D body is affected: crates and characters slide off platforms and across floors as if on ice. `rapier2d` itself isn't affected (see below).

A 1×1×1 cuboid resting on a fixed cuboid floor, both with friction 0.5, given a linear velocity of 3 m/s along x:

| | Simplified | Coulomb |
|---|---|---|
| no locks | stops after 0.91 m | stops after 0.97 m |
| plane-locked | slides 9.00 m in 180 steps (never slows) | slides 9.00 m in 180 steps |

The expected distance is v² / (2μg) ≈ 0.92 m.

<!-- Attach plane-locked-friction.gif here: drag it into the GitHub editor. -->

### Repro

```rust
use rapier3d::prelude::*;

let mut world = PhysicsWorld::new();
let ground = world
    .bodies
    .insert(RigidBodyBuilder::fixed().translation(Vector::new(0.0, -0.5, 0.0)));
world.colliders.insert_with_parent(
    ColliderBuilder::cuboid(20.0, 0.5, 20.0).friction(0.5),
    ground,
    &mut world.bodies,
);
let body = world.bodies.insert(
    RigidBodyBuilder::dynamic()
        .translation(Vector::new(0.0, 0.5, 0.0))
        .linvel(Vector::new(3.0, 0.0, 0.0))
        .enabled_translations(true, true, false)
        .enabled_rotations(false, false, true),
);
world.colliders.insert_with_parent(
    ColliderBuilder::cuboid(0.5, 0.5, 0.5).friction(0.5),
    body,
    &mut world.bodies,
);
for _ in 0..180 {
    world.step();
}
// ≈ 9.0 (no friction); ≈ 0.9 without the two `enabled_*` calls.
println!("{}", world.bodies[body].translation().x);
```

### Cause

Both 3D tangent solves, `ContactConstraintTangentPart::solve` and `ContactConstraintTangentPartSlim::solve` in `src/dynamics/solver/contact_constraint/contact_constraint_element.rs`, solve the two friction directions together by inverting the 2×2 tangent effective-mass matrix `K`.

With the z translation and the x/y rotations locked, the tangent along z has no effective mass: `im·t = 0`, and `dp × t` lies in the plane of the locked rotations. So `k22 = 0` and `k12 = 0`, which makes `det(K) = 0`. `simd_inv(0)` returns 0, so both tangent impulses become 0, and friction along the free x tangent is dropped as well.

Locking only the tilts (z translation free), or only the z translation, keeps friction, because `K` stays invertible. The 2D build and the generic (multibody) path solve each tangent on its own, so they don't have this problem.

### Expected

Friction along the free tangent as usual, and no impulse along the locked one.

I have a fix that falls back to per-tangent solves when `K` is singular, with a regression test, and can open a PR.
