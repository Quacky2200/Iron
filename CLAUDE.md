# Iron

An s&box-like game engine. The core is Rust. Gameplay is TypeScript running in QuickJS, bundled by Bun.
UI is React through a custom reconciler, drawn by our own renderer. The editor will be built the same
way. Full design: `DESIGN.md`. Plan and progress: `TODO.md`. Test IDs (e.g. `M3-EVT-01`): `TESTS.md`.

**Core rule: Rust owns the world, JS owns behaviour.**
- Scripts never hold engine references, only numeric IDs.
- Scripts talk to Rust through ops (`__iron_op(name, ...args)`) and batched UI commits (`__iron_commit(ops)`).
- Rust calls into JS through `__iron_*` globals.

## Layout

| Path | What |
|---|---|
| `crates/iron_core` | Shared types: `Value` (crosses the JS boundary), `NodeId`, `Commit`, ops, asset path safety (`resolve_asset`, `decode_data_url`) |
| `crates/iron_platform` | winit window + event loop (`Platform`, `PlatformApp`), `InputEvent` conversion, gilrs gamepads |
| `crates/iron_script` | `ScriptRuntime` trait + QuickJS backend (rquickjs 0.14), timers prelude, `ComponentHost` (component lifecycle) |
| `crates/iron_ui` | UI tree built from React commits (opcodes in `opcodes.rs`), styles (`style.rs`, `text_style.rs`, `paint.rs`), Taffy layout (`layout.rs`), Parley text (`text.rs`), CSS (`css.rs` parser, `cascade.rs` matching/cascade/invalidation), draw list (`draw.rs`), hit testing (`tree.rs`), pointer events + `:hover`/`:active` (`events.rs`), focus/keys/default actions (`interaction.rs`) |
| `crates/iron_render` | wgpu renderer for the draw list: quads, SDF rounded boxes, shadows, blur layers, gradients, images, glyph atlas (swash). `testing.rs` has golden helpers and a shared test GPU |
| `crates/iron_app` | The `iron` binary: `iron <game folder> [--screenshot out.png] [--frames N]`. Loads `dist/game.js` + `dist/game.css` and wires scripts → UI → renderer |
| `js/engine` | `@iron/engine`: `Component`, `register`, `op`, `loadFont`, `addStyleSheet`/`removeStyleSheet` |
| `js/react` | `@iron/react`: reconciler (`host.ts`), UI event dispatch (`events.ts`), style/attr diffing, `createRoot`, `Text`/`Span` |
| `examples/*` | `hello`, `boxes`, `text`, `css` games (each has `bun run build`) |
| `tests/fixtures` | JS fixtures for Rust integration tests; `tests/layout` JSON layout fixtures; `tests/golden` PNG goldens |

### UI model
- **Elements:** `box`, `text`, `span` and `#text` nodes. `ROOT` is node 0. Generated `::before`/`::after` ids start at `0x8000_0000`.
- **Styles:** inline `style` props (camelCase) go through `apply_style`. Stylesheets go through `cssparser` + `selectors`, with cascade (specificity, inline, then `!important`), custom properties and `var()`, and a rule index plus dependency-aware restyle.
- **Units:** layout is in logical pixels. The app divides by the window scale factor.

## Commands

On Windows the shell needs `export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`.

```bash
cargo fmt --all
```
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
```bash
cargo test --workspace
```
```bash
bun test
```
```bash
bun run typecheck
```
```bash
bun run build
```
```bash
bun run build:fixtures
```
```bash
cargo run -p iron_app -- examples/css --screenshot out.png --frames 30
```

- `bun run build` builds all the examples.
- `bun run build:fixtures` rebuilds `tests/fixtures`. Run it after changing js/.
- The `cargo run` line renders a screenshot and exits; check it by eye.
- Benchmarks: `cargo bench -p iron_ui --bench restyle` (and the other benches in `crates/iron_ui/benches`).

## Working agreements
- **Commits:** work milestone by milestone, one commit per step, without asking.
  - Run every check above before each commit.
  - End commit messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - Update `TODO.md` checkboxes, and `TESTS.md` when adding test IDs.
- **Charm font:** never commit it. It lives in `/Charm/` with local copies under `examples/*/assets/fonts/`. It's excluded via `.git/info/exclude` and `dist/` is ignored. Check `git status` before committing.
- **Code style:** match the surrounding code. Comments are short and say why. Tests are named after behaviour and tagged with their `TESTS.md` ID in a comment.
- **CSS:** prefer standard CSS over s&box's custom extensions. The CSS parity backlog is TODO.md "M3d".

## Gotchas
- **Shell edits:** bash heredocs mangle apostrophes and backslashes on this machine. For multi-line edits use the Edit/Write tools, or a Python script written to the scratchpad.
- **Interrupted builds:** if cargo was killed mid-build, delete the corrupt `.pdb` files under `target/` before rebuilding.
- **GPU tests:** use `iron_render::testing::shared_gpu()`. Creating devices in parallel deadlocks.
- **Bun CSS:** importing CSS needs `--outdir dist --entry-naming "game.[ext]"`.
  - It emits `game.js` + `game.css`.
  - Small assets are inlined as `data:` URLs.
  - Larger ones are copied with hashed names, and their `url()` is relative to the CSS file. The engine rebases them.
- **Library versions:** cssparser 0.38, selectors 0.41, parley 0.12, taffy 0.15, rquickjs 0.14, edition 2024, Rust 1.99. APIs differ from older docs.
- **Parley fonts:** after registering a font, Parley needs a fresh `LayoutContext`, or its resolver cache keeps the old fonts.

## Status (2026-10-16)
- **Done:**
  - M1 scaffold
  - M2 React → Taffy → boxes
  - M3a text and fonts
  - M3b CSS stylesheets
  - M3c step 1: hit testing (`UiTree::hit_test`, `event_path`)
  - M3c step 2: pointer events. `iron_ui::Pointer` turns input into DOM-style events and sets
    `:hover`/`:active`; the app sends them to `__iron_ui_event(type, path, data)`, which
    `@iron/react` (`events.ts`) runs through capture/bubble over `on*` props. `cursor` → OS cursor
  - M3c step 3: keyboard and focus. `iron_ui::Interaction` wraps `Pointer`, owns focus and runs
    default actions (Tab, focus on press) using the "prevented" flag `dispatch` returns;
    `ref.focus()`/`blur()` arrive as `focus`/`blur` ops
- **Next:** the rest of M3c. See TODO.md "M3c" for the steps:
  3. `overflow` clipping and scrolling
  4. `<input>`/`<textarea>` on Parley `PlainEditor` with IME and clipboard (arboard)
  5. gilrs gamepads to scripts
  6. `examples/form`
- **GPU crashes:** the user's PC has hard-reset with the window open (an existing NVIDIA
  instability, also seen before Iron). Warn before launching the interactive window; prefer
  `--screenshot` runs for checks.
- **Needs the user:** the manual checks M2-MAN-01 (resize) and M3-MAN-01 (IME), and the CI run.
