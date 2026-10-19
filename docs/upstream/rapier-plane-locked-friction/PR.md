**Title:** Keep friction when the tangent effective-mass matrix is singular (plane-locked bodies)

Fixes #<issue number>.

The 3D friction solve inverts the 2×2 tangent effective-mass matrix `K` to solve both friction directions together. If one tangent has no effective mass, `K` is singular. That happens, for example, for a body whose z translation and x/y rotations are locked. `simd_inv(det)` then returns 0, so friction along *both* tangents becomes 0, and the body slides as if on ice.

That lock is how 3D bodies are kept in a plane, and how engines run 2D physics inside a 3D rapier world (2D bodies sharing a simulation with 3D ones, Unity-style `Rigidbody2D`). All such 2D bodies currently have no friction. `rapier2d` is unaffected.

### Change

- The shared 2×2 solve moves into `tangent_delta_impulse`. Both `ContactConstraintTangentPart::solve` (Coulomb) and `ContactConstraintTangentPartSlim::solve` (Simplified) now call it.
- When `det(K) ≤ 1e-6 · k11 · k22`, each tangent is solved on its own: `dvel_i / k_ii`, which is a no-op for a zero `k_ii`. Otherwise the result is exactly the same as before.
- The 2D build and the generic (multibody) constraints already solve per tangent, so they're unchanged.

### Test

`tests/plane_locked_friction.rs`: a cuboid on a floor is shoved at 3 m/s along x, with and without the plane lock, under both friction models, with the shove both at creation and after settling. The locked body now stops within a few centimetres of where the free one stops (0.91 m; Coulomb free 0.97 m). Before the fix it slid 9 m.

The existing test suite passes unchanged: 443 tests, plus the new one.
