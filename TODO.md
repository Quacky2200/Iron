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
- [x] Bun workspace: root `package.json`, `js/engine`, `examples/hello`, `tests/fixtures`, shared `tsconfig.base.json` (strict) — `M1-BLD-03`, `M1-BLD-04` (Bun 1.4.3)
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
- [x] Clean exit on window close — `M1-PLT-03` (manual, passed 2026-10-16 on Windows)
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
- [x] `bun test` unit tests for the JS host (`js/engine/test/`, 4 tests)

### 7. `iron_app` and the example
- [x] `iron` binary: game folder (`dist/game.js`) or `.js` path; opens window; runs loop
- [x] Fixed 60 Hz tick with catch-up limit; clear colour each frame
- [x] `examples/hello`: `Hello` logs `hello` on start and `tick` on update
- [x] `tests/fixtures/{hello,throws}.js` built by Bun from `tests/fixtures/src/*.ts` (`bun run build:fixtures`; CI checks they're up to date)
- [x] `iron examples/hello` runs the Bun-built game end to end

### 8. CI
- [x] GitHub Actions workflow (`.github/workflows/ci.yml`): Windows + Linux Rust jobs, JS job with Bun
- [x] Software rendering on CI runners (WARP on Windows, Mesa llvmpipe on Linux)
- [ ] Push to GitHub and see it go green (not yet run)

**M1 done when:** all `M1-*` tests pass in CI and `M1-PLT-03` passes manually.

**Status:** all 24 Rust tests and 4 JS tests pass locally (Windows, RTX 5080); build and typecheck are clean. `iron examples/hello` runs on Vulkan: `hello` once, ~60 ticks/s. The manual close test passed. Remaining: a CI run.

---

## M2 — Boxes (React → Taffy → screen)

**Goal:** `iron examples/boxes` shows a React-rendered layout of coloured, rounded, bordered boxes that relayouts when the window resizes and when React state changes (a timer-driven counter that adds/removes boxes).

**Versions checked 2026-10-16:** React 19.3.1, react-reconciler 0.34.0, Taffy 0.15.0, criterion 0.8.2.

### M2 decisions (proposed, confirm before starting)
- **D1 — Op transport:** the reconciler collects ops in a JS array during a commit and calls a native `__iron_commit(ops)` **once** in `resetAfterCommit`. Ops are flat arrays: `[opcode, ...args]`. Start with the existing `Value` conversion; switch to a binary `ArrayBuffer` encoding only if `M2-BEN-04` says so.
- **D2 — Style values:** numbers mean logical pixels; strings are CSS values parsed in Rust (Taffy's `parse` feature for layout properties, a small parser for colours). Styles travel as flat `[prop, value, prop, value, ...]` lists, so `Value` needs no object type. M3's CSS files reuse the same parsers.
- **D3 — Elements in M2:** `<box>` only. Text arrives in M3; `createTextInstance` throws a clear "text isn't supported until M3" error.
- **D4 — Scheduling:** React 19 + react-reconciler 0.34. The script runtime gains `setTimeout`/`clearTimeout`/`queueMicrotask` and runs the QuickJS job queue every frame. Root renders use `updateContainerSync` + `flushSyncWork` so a frame's state changes commit in the same frame.
- **D5 — Crate split:** a new `iron_ui` crate owns the node tree, op decoding, style parsing, the `LayoutEngine` trait with the Taffy backend, and produces a GPU-free **draw list** of quads. `iron_render` gains a `QuadRenderer` that draws any draw list. UI stays testable without a GPU, and the quad renderer is reusable for world panels later.
- **D6 — Units:** layout in logical pixels; the window's scale factor is applied only in the renderer.

### 1. Script runtime additions (`iron_script`)
- [ ] `setTimeout` / `clearTimeout` / `queueMicrotask` natives; timers fire from `ScriptRuntime::tick(now)` — `M2-SCR-01`
- [ ] Run pending QuickJS jobs (promises, microtasks) after every call into JS — `M2-SCR-02`
- [ ] `__iron_commit(ops)` native: one call per commit, collected with `drain_commits()` — `M2-SCR-03`
- [ ] Bundle defines: `process.env.NODE_ENV` set by `bun build --define` (dev for local runs, production for release)
- [ ] Add the new tests to the backend conformance suite so every future backend gets them

### 2. `iron_ui` crate: tree, styles, layout (no GPU)
- [ ] Opcodes shared with JS: `create`, `setStyle`, `appendChild`, `insertBefore`, `removeChild`, `appendToRoot`, `insertInRootBefore`, `removeFromRoot`, `destroy`
- [ ] `UiTree`: applies a commit; unknown IDs or bad ops are reported and skipped — `M2-UI-01`, `M2-UI-02`
- [ ] Style parsing: Taffy `parse` for layout props; colours as `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()` and a few names — `M2-STY-01`, `M2-STY-02`
- [ ] M2 style subset: `display` (flex/grid/none), flex props, `width/height/min/max`, `padding`, `margin`, `borderWidth`, `gap`, `position` + insets, grid templates and placement, `backgroundColor`, `borderColor`, `borderRadius`, `opacity`
- [ ] `LayoutEngine` trait + `TaffyLayout` backend; layout only reruns when dirty — `M2-LAY-01`..`M2-LAY-04`
- [ ] Layout fixtures as data (`tests/layout/*.json`: tree + styles + expected rects) so a Yoga backend can run the same cases — `M2-LAY-05`
- [ ] Root resize relayouts the tree — `M2-UI-03`
- [ ] Draw list: one quad per visible box (rect, colour, border, radius, opacity), in paint order — `M2-UI-04`

### 3. `@iron/react` package (`js/react`)
- [ ] Host config for react-reconciler 0.34 (mutation mode). React 19 has many required host functions (update priority, suspend-commit hooks, etc.); stub them with sensible defaults and keep the list in one file
- [ ] Node IDs: numbers from a counter, never reused in a session — `M2-REC-05`
- [ ] `commitUpdate` diffs `style` and emits only changed properties — `M2-REC-02`
- [ ] `createRoot().render(<App/>)` API; JSX typing for `<box>` (`JSX.IntrinsicElements`) with a typed `style` prop
- [ ] `bun test` suite with a fake `__iron_commit` that records ops — `M2-REC-01`..`M2-REC-05`

### 4. Integration in QuickJS
- [ ] React fixture (`tests/fixtures/src/boxes.tsx` → `boxes.js`) runs in `QuickJsRuntime`; its commits applied to `UiTree` give the expected tree and layout — `M2-INT-01`
- [ ] A state change driven by `setTimeout` commits the right incremental ops — `M2-INT-02`

### 5. Quad renderer (`iron_render`)
- [ ] Instanced quads with an SDF fragment shader for rounded corners and borders, with premultiplied alpha
- [ ] Golden tests offscreen — `M2-DRW-01`, `M2-DRW-02`
- [ ] Whole draw list in a fixed number of draw calls — `M2-DRW-03`
- [ ] Scale factor applied at draw time — `M2-DRW-04`

### 6. App wiring and example
- [ ] `iron_app`: each frame call `tick(now)`, apply commits, relayout if dirty, draw quads over the clear colour
- [ ] Window resize → root size in logical pixels → relayout
- [ ] Load `@iron/react` apps alongside components (a bundle can register components and render UI)
- [ ] `examples/boxes`: grid/flex showcase plus a timer-driven counter that adds and removes boxes
- [ ] Manual check: resize the window and watch it relayout — `M2-MAN-01`

### 7. Benchmarks and the Taffy/Yoga decision
- [ ] `criterion` benches in `iron_ui` — `M2-BEN-01`..`M2-BEN-03`
- [ ] JS→Rust commit transfer bench (QuickJS) — `M2-BEN-04`
- [ ] Record results in TESTS.md
- [ ] **Decision point:** if Taffy misses targets badly, prototype a Yoga backend against the `M2-LAY-05` fixtures

**Suggested order:** 1 → 2 → 3 → 4 → 5 → 6 → 7. Steps 2 and 3 are independent once the opcode list (step 2's first item) is agreed, so they can run in parallel.

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
