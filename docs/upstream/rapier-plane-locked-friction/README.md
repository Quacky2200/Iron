# rapier: friction lost on plane-locked 3D bodies

An upstream report for [dimforge/rapier](https://github.com/dimforge/rapier). Posted on
2026-10-19 as [issue #1047](https://github.com/dimforge/rapier/issues/1047) and
[PR #1048](https://github.com/dimforge/rapier/pull/1048) (branch `fix/plane-locked-friction` on
Quacky2200/rapier).
Platypus works around the bug itself (`platypus_physics`, `Physics::set_constraints`), so nothing here
is needed to build Platypus.

In Platypus it hit every Rigidbody2D (2D runs in the same 3D rapier world, kept in the x-y plane by
this lock) and `examples/physics`: neither had any friction. `rapier2d` itself is unaffected; its
friction solve handles one direction at a time.

- `ISSUE.md`: the issue.
- `PR.md`: the pull request description.
- `plane-locked-friction.gif` (and its last frame, `.png`): the test's scenario drawn from
  traces of rapier itself, stock 0.36.1 ("Before") and with the fix ("After"). Drag it into the
  GitHub issue or PR to attach it.
- `fix-master.patch`: the fix plus a regression test, laid out for the rapier repository
  (`src/...`, the test in `crates/rapier3d/tests/`, where tests are found automatically). Use
  this one for the PR: `git apply fix-master.patch` from the repository's root.
- `fix.patch`: the same change laid out for the published rapier3d 0.36.1 crate (the test in
  `tests/`). That crate's `Cargo.toml` lists each test (`autotests = false`), so running it
  there also needs:

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
- On `master`, `contact_constraint_element.rs`, `contact_with_coulomb_friction.rs` and
  `contact_with_twist_friction.rs` are identical to 0.36.1. The bug is still there, and
  `fix-master.patch` applies cleanly. The file's last change was the soft-body work (#1010,
  2026-09-19). None of the 12 open PRs touches the contact constraints.
