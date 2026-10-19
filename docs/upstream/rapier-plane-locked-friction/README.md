# rapier: friction lost on plane-locked 3D bodies

An upstream report for [dimforge/rapier](https://github.com/dimforge/rapier), ready to post.
Iron works around the bug itself (`iron_physics`, `Physics::set_constraints`), so nothing here
is needed to build Iron.

In Iron it hit every Rigidbody2D (2D runs in the same 3D rapier world, kept in the x-y plane by
this lock) and `examples/physics`: neither had any friction. `rapier2d` itself is unaffected; its
friction solve handles one direction at a time.

- `ISSUE.md`: the issue.
- `PR.md`: the pull request description.
- `fix.patch`: the fix plus a regression test, against the published rapier3d 0.36.1 crate.
  It applies cleanly there (`git apply fix.patch`).

Before opening the PR, check the code against rapier's current `master`; the solve may have
changed since 0.36.1. In the repository, the test also needs registering wherever rapier lists
its tests. The published crate's `Cargo.toml` lists each one, with `autotests = false`:

```toml
[[test]]
name = "plane_locked_friction"
path = "tests/plane_locked_friction.rs"
```

Checked on 2026-10-19:

- No existing issue or PR covers this. Searched open and closed issues and PRs for friction,
  lock, locked, `enabled_translations`, tangent and singular; the nearest, #137 "Incorrect
  friction behavior", is from 2020 and unrelated.
- rapier3d 0.36.1 (the newest release) passes all 443 of its own tests both with and without
  the fix; the new test fails without it and passes with it (444 in all).
