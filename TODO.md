# Iron — TODO

Work plan by milestone. Each task lists the tests from [TESTS.md](TESTS.md) that prove it's done. Architecture lives in [DESIGN.md](DESIGN.md).

Legend: `[ ]` to do · `[~]` in progress · `[x]` done

---

## Decisions needed before / during M1

- [x] Window/input: `winit` + `gilrs`
- [x] Audio: `kira`
- [x] Layout: Taffy first, Yoga fallback behind `LayoutEngine`
- [x] **First script backend:** QuickJS (`rquickjs` 0.14). `deno_core` follows as a second backend.
- [x] Rust edition 2024, latest stable (1.99 at time of writing)
- [ ] Licence: private for now (see README) until a working demo; open-source licence to be chosen then

---

## Proposed repository layout

```
Iron/
├─ Cargo.toml              # workspace
├─ package.json            # Bun workspace root
├─ DESIGN.md  TESTS.md  TODO.md
├─ crates/
│  ├─ iron_core/           # IDs, math re-exports, op buffer types, logging
│  ├─ iron_platform/       # Platform layer: winit + gilrs
│  ├─ iron_render/         # wgpu device, surfaces, offscreen targets
│  ├─ iron_script/         # ScriptRuntime trait + QuickJS backend + shared test suite
│  ├─ iron_ui/             # (M2) UI tree, LayoutEngine trait, Taffy backend, UI renderer
│  └─ iron_app/            # the `iron` binary: main loop wiring everything together
├─ js/
│  ├─ engine/              # @iron/engine: Component base class, engine API, .d.ts
│  └─ react/               # (M2) @iron/react: custom reconciler
├─ examples/
│  └─ hello/               # TS game: a Hello component
└─ tests/
   ├─ fixtures/            # tiny TS bundles, glTF files
   └─ golden/              # reference PNGs
```

---

## M1 — Hello Iron (scaffold)

**Goal:** `cargo run -p iron_app -- examples/hello` opens a window with a clear colour, loads a Bun-built TS bundle, and a `Hello` component logs `hello` once and `tick` every frame.

### 1. Repository and tooling
- [x] `git init`, `.gitignore` (target/, node_modules/, dist/), `.gitattributes` (LF line endings, PNGs as binary)
- [x] Cargo workspace with crates from the layout above — `M1-BLD-01`
- [x] `rustfmt.toml`; `cargo fmt --check` / `cargo clippy -D warnings` clean — `M1-BLD-02`
- [~] Bun workspace: root `package.json`, `js/engine`, `examples/hello`, shared `tsconfig.base.json` (strict) — `M1-BLD-03`, `M1-BLD-04`. Files written; **not yet run: Bun isn't installed on the dev machine**
- [x] Logging via `tracing` + `tracing-subscriber` (`RUST_LOG` to override)

### 2. `iron_core`
- [x] Typed IDs (`EntityId`, `ComponentId`, `NodeId`) as `u32` newtypes
- [x] `Op` and `OpBuffer` (name + `Value` args; binary encoding comes later)
- [x] `Value` enum for values crossing the script boundary
- [x] Re-export `glam` for math

### 3. `iron_platform`
- [x] `Platform` wrapping the winit event loop (`ApplicationHandler`)
- [x] Headless mode for tests — `M1-PLT-01`
- [x] Convert winit events to Iron `InputEvent`s — `M1-PLT-02`
- [ ] Clean exit on window close — `M1-PLT-03` (**manual check needed**)
- [x] `gilrs` polling stub (wired up properly in M3)

### 4. `iron_render`
- [x] wgpu instance/adapter/device; software fallback when no hardware adapter — `M1-RND-01`
- [x] Window surface (sRGB, handles outdated/lost/minimized) + offscreen target
- [x] Clear to colour, read back pixels (with row padding) — `M1-RND-02`
- [x] Resize and 0×0 (minimized) are safe — `M1-RND-03`
- [x] Golden image helper (`testing::assert_golden`, `IRON_UPDATE_GOLDEN=1`)

### 5. `iron_script`
- [x] `ScriptRuntime` trait: `load_bundle`, `eval`, `call_global`, `drain_ops`, `reset`, `set_log_sink`
- [x] `ScriptError` with message, file, line, stack
- [x] QuickJS backend (`rquickjs`) — `M1-SCR-01`..`M1-SCR-03`
- [x] `console.*` → log sink (default: `tracing`, target `script`) — `M1-SCR-04`
- [x] `reset()` creates a fresh context, keeps natives and log sink — `M1-SCR-05`
- [x] Native `__iron_op(name, ...args)` queues ops — `M1-SCR-06`
- [x] UTF-8 ↔ UTF-16 round-trip test — `M1-SCR-07`
- [x] Backend-generic suite (`runtime_conformance_tests!` in `iron_script::conformance`) — `M1-SCR-08`

### 6. `@iron/engine` (TS) and components
- [x] `Component` base class with `onStart()` / `onUpdate(dt)`
- [x] `register({ Hello })` registry (object form keeps names stable under minification) — `M1-CMP-01`
- [x] `ComponentHost` calls methods by `(component id, method name)` via `__iron_dispatch` — `M1-CMP-02`, `M1-CMP-03`
- [x] Exceptions disable only the failing component — `M1-CMP-04`
- [x] Hand-written globals `.d.ts` (`js/engine/src/globals.ts`; generate from Rust later)
- [~] `bun test` unit tests for the JS host (`js/engine/test/`) — written, not run yet (needs Bun)

### 7. `iron_app` and the example
- [x] `iron` binary: game folder (`dist/game.js`) or `.js` path; opens window; runs loop
- [x] Fixed 60 Hz tick with catch-up limit; clear colour each frame
- [x] `examples/hello`: `Hello` logs `hello` on start and `tick` on update
- [x] `tests/fixtures/{hello,throws}.js` hand-built from `tests/fixtures/src/*.ts`
- [ ] Regenerate fixtures with `bun run build:fixtures` once Bun is installed

### 8. CI
- [x] GitHub Actions workflow (`.github/workflows/ci.yml`): Windows + Linux Rust jobs, JS job with Bun
- [x] Software rendering on CI runners (WARP on Windows, Mesa llvmpipe on Linux)
- [ ] Push to GitHub and see it go green (not yet run)

**M1 done when:** all `M1-*` tests pass in CI and `M1-PLT-03` passes manually.

**Status:** all 24 Rust tests pass locally (Windows, RTX 5080). The app ran on Vulkan: `hello` once, ~60 ticks/s. Remaining: Bun-side checks, the manual close test, and a CI run.

---

## M2 — Boxes

- [ ] `@iron/react`: `react-reconciler` host config emitting ops — `M2-REC-01`..`M2-REC-05`
- [ ] Op buffer flush once per commit; binary encoding if JSON-ish is too slow — `M2-REC-04`, `M2-BEN-04`
- [ ] `iron_ui`: node tree from ops, error-tolerant — `M2-UI-01`, `M2-UI-02`
- [ ] `LayoutEngine` trait + Taffy backend — `M2-LAY-01`..`M2-LAY-05`
- [ ] Shared layout fixtures (usable by a future Yoga backend) — `M2-LAY-05`
- [ ] Batched quad/SDF renderer: rects, rounded corners, borders — `M2-DRW-01`..`M2-DRW-03`
- [ ] `criterion` benchmarks — `M2-BEN-01`..`M2-BEN-04`
- [ ] **Decision point:** if Taffy misses targets badly, prototype Yoga backend

## M3 — Real UI

- [ ] Choose text library (`cosmic-text` vs `parley`); text rendering + measurement cache — `M3-TXT-*`
- [ ] Choose styling approach (CSS files / style objects / both); CSS subset parser — `M3-CSS-*`
- [ ] Hit testing, bubbling, focus, keyboard — `M3-EVT-*`
- [ ] IME support through winit — `M3-IME-01`
- [ ] `gilrs` gamepad events to scripts — `M3-PAD-01`

## M4 — Hot reload

- [ ] Bun watch → rebuild → engine notified
- [ ] Wipe-and-rebuild with property serialization — `M4-HOT-01`..`M4-HOT-05`
- [ ] React Fast Refresh for panels — `M4-HOT-06`
- [ ] Reload benchmark — `M4-HOT-07`

## M5 — Scene, assets, rendering

- [ ] **Decision:** own renderer vs reuse Bevy crates (DESIGN §7.4)
- [ ] **Decision:** ECS vs GameObject + Component
- [ ] Scene serialization, GUID `.meta` files — `M5-SCN-*`
- [ ] glTF import + cook, platform texture formats, asset hot reload — `M5-AST-*`
- [ ] PBR, shadows, IBL, post-processing — `M5-RND-*`

## M6 — Physics and audio

- [ ] `rapier` integration + TS components — `M6-PHY-*`
- [ ] `kira` integration — `M6-AUD-*`

## M7 — Editor shell

- [ ] Docking, tree view, virtualized list, inspector, undo/redo — `M7-EDT-*`

## Later

- [ ] Steam multiplayer (`steamworks`), `@sync` / `@rpc`, QUIC transport
- [ ] Baked lighting, light/reflection probes
- [ ] Android, Quest VR, PC VR (OpenXR)
- [ ] Steam Audio spatial sound
- [ ] Ray tracing tier
- [ ] iOS (QuickJS backend, Xcode wrapper)
- [ ] Packaging and distribution (pack files, Steam Workshop?)
- [ ] `deno_core` and Bun script backends
