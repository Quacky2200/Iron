# Windows: the step list

Design: [iron-windows.md](iron-windows.md) (section numbers below refer to it). Written 2026-10-21 against base `c3ffc1b`.
Each step is one commit. Test IDs: `M9-API-1400..1499` (Rust and QuickJS), `M9-TS-2800..2899` (compiled programs and bun).
Add every ID to `TESTS.md` (under "M9.b", a new "Windows API" table) in the step that adds the test, tagged in a comment on the
test as usual (`// M9-API-1401`).

## Ground rules for the implementer

- **Never launch an interactive window.** The user's PC has hard-reset with the Iron window open. Verification is unit tests,
  fake-platform tests and `--screenshot` runs only. Screenshot runs still show the main window for a moment, as they do today;
  run at most one at a time and only in the steps that say so.
- Shell: `export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"`. Edit files with the Edit/Write tools or a Python script in the
  scratchpad (heredocs mangle quotes here).
- Checks before each commit (CLAUDE.md): `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `bun test`, `bun run typecheck`, `bun run build`, `bun run build:fixtures` (when `js/` changed). For
  `tools/` changes also `cd tools && cargo test -p iron_ts_tests --test all windows` and `cargo clippy --workspace --all-targets
  -- -D warnings` there. The disk is tight: use `CARGO_INCREMENTAL=0` for one-off checks, and delete corrupt `.pdb` files after an
  interrupted build.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Update `TODO.md` and `tools/iron_ts/TODO.md`
  ("Windows API") as items finish. Docs refresh after step 11 (CLAUDE.md: a Sonnet subagent given only the diff).
- Match the surrounding style: short doc comments that say why, tests named after behaviour.

Which steps touch which crates:

| Step | iron_platform | iron_engine_api | iron_api(_macros) | tools/ (iron_ts, iron_ts_ui, iron_ts_tests) | iron_render | iron_app | js/ | examples |
|---|---|---|---|---|---|---|---|---|
| 1 | x | | | | | | | |
| 2 | | x | | | | | | |
| 3 | | | x | | | | | |
| 4 | | | | x | | | | |
| 5 | x | | | | | | | |
| 6 | | | | | x | | | |
| 7 | | | | | | x | | |
| 8 | | | | | | x | | |
| 9 | | x | | | | x | | |
| 10 | | | | | | x | x | |
| 11 | | | | x (types only) | | | | x |
| 12 | docs only | | | | | | | |

Steps 1 to 4 are new code only (no existing behaviour changes). Steps 5 to 10 change `iron_platform`, `iron_render` and
`iron_app` (allowed now). Steps D1 to D5 wait for the user's decision.

---

## Step 1: platform window types, the plan and the tree (pure, `iron_platform`)

**Files.** New `crates/iron_platform/src/windows.rs`; `lib.rs` exports `windows::{WindowKey, WindowKind, WindowSpec,
NativePlan, Os, WindowTree, WindowCommand, WindowHost, plan, place}`.

**Code** (design 7.1):

- `WindowKey(u32)`, `WindowKey::MAIN = WindowKey(1)`.
- `WindowKind` with `parse(&str) -> Option<WindowKind>` ("normal", "tool", "dialog", "popup", "borderless", "child") and
  `name()`.
- `WindowSpec` with `impl Default` (title "", size 400 x 300, position None, kind Normal, parent None, all buttons and
  resizable true, closable true, always_on_top false, visible true, active true, icon None) and `WindowSpec::main(title, w, h)`.
- `Os { Windows, MacOs, X11, Wayland, Other }`, `Os::current()` (`cfg!(windows)`, `target_os = "macos"`, Linux: X11 unless
  `WAYLAND_DISPLAY` is set and `WINIT_UNIX_BACKEND` is not `x11`).
- `plan(spec, os) -> NativePlan` per the table in design section 4:
  - `Tool`: decorations, `tool_caption` (Windows only), `skip_taskbar` (Windows), owner = parent or MAIN, buttons (close only,
    if closable), `x11_type: Some("utility")`.
  - `Dialog`: owner, `skip_taskbar` (Windows), buttons close only, `x11_type: Some("dialog")`.
  - `Popup`: no decorations, `tool_caption` (Windows), `skip_taskbar` (Windows), owner, `active: false` always (a popup never
    takes focus when shown; `win.focus()` still works), `x11_type: Some("popupmenu")`.
  - `Borderless`: no decorations, no owner.
  - `Child`: `embed_in = parent or MAIN` on Windows and X11; on MacOs, Wayland, Other: kind becomes Tool (that row of the
    table) and `warning = Some("style \"child\" is not available on <os>; using \"tool\"")`.
  - `minimizable` / `maximizable` only count for Normal and Borderless.
  - `level_on_top = spec.always_on_top`.
- `place(parent_pos, parent_size, size, n) -> (x, y)`: `parent_pos + (parent_size - size) / 2 + 24 * (n % 8)` per axis.
- `WindowTree`: `insert`, `remove`, `close_order` (deepest first, then the key itself), `children` (creation order),
  `parent`. Inserting a parent that is the key or a descendant of it stores None.
- `WindowCommand` and `WindowHost` as in 7.1 (`WindowHost::Os(Arc<winit::window::Window>)`, `Virtual { width, height, scale }`).

**Tests** (`#[cfg(test)] mod tests` in `windows.rs`):

| ID | Test | Checks |
|---|---|---|
| M9-API-1400 | `window_kinds_parse_and_name_round_trip` | every name parses and `name()` gives it back; "Tool" and "" do not parse |
| M9-API-1401 | `tool_windows_are_owned_skip_the_taskbar_and_get_the_small_caption_on_windows` | `plan(tool, Windows)`: owner MAIN, `tool_caption`, `skip_taskbar`, buttons (true, false, false) |
| M9-API-1402 | `tool_windows_on_macos_are_child_windows_without_taskbar_rules` | `plan(tool, MacOs)`: owner MAIN, no `tool_caption`, no `skip_taskbar` |
| M9-API-1403 | `embedded_children_fall_back_to_tool_where_the_platform_has_none` | `Child` on Windows: `embed_in`; on MacOs and Wayland: kind Tool, a warning |
| M9-API-1404 | `popups_have_no_decorations_and_do_not_take_focus` | `plan(popup spec with active true, Windows)`: decorations false, active false |
| M9-API-1405 | `new_windows_are_centred_on_their_parent_and_cascade` | `place` for n = 0, 1, 8 |
| M9-API-1406 | `closing_a_window_closes_its_children_deepest_first` | tree MAIN -> 2 -> 3, MAIN -> 4: `close_order(2) == [3, 2]`, `close_order(MAIN) == [3, 2, 4, MAIN]` |
| M9-API-1407 | `a_parent_that_would_make_a_cycle_is_dropped` | insert 2 under 3 after 3 under 2 |

**Acceptance.** `CARGO_INCREMENTAL=0 cargo test -p iron_platform` passes; nothing else uses the new module yet.

---

## Step 2: the `Window` script API (`iron_engine_api::window`)

**Files.** New `crates/iron_engine_api/src/window.rs`; `lib.rs`: `pub mod window;`, add `window::window_api::MANIFEST.to_json()`
to `manifests()` and `window::window_api::register_quickjs(ctx)?` to `register_quickjs`. New test file
`crates/iron_engine_api/tests/window.rs`.

**Code.** Exactly design section 5: `WindowRef`, `MAIN`, `Style` (its own enum, no `iron_platform` dependency), `Flags`,
`WindowState`, `WindowRequest`, `Opened`, the host functions and `#[iron_api(namespace = "Window", quickjs, handles(Window =
WindowRef))] pub mod window_api`. Without step 3, leave `#[options]` out (add it in step 3).

Behaviour details the tests pin down:

- State: `thread_local! { static STATE: RefCell<State> }` with `windows: BTreeMap<WindowRef, WindowState>`, `next: u32` (starts
  at 2), `requests: Vec<WindowRequest>`, `listeners: HashMap<WindowRef, WindowListeners>` (`Listeners<()>` x2,
  `Listeners<(f64, f64)>` x2, `Listeners<(bool,)>`, `Listeners<(f64,)>`), `current: Option<WindowRef>`, `cancel: bool`,
  `in_close_request: Option<WindowRef>`, `claim: BTreeMap<String, WindowRef>` (reload). `reset()` leaves only MAIN (title
  "Iron", 1280 x 720, scale 1, open, created).
- `create`: NaN width/height -> 400 x 300; NaN x/y -> `position: None`; empty style -> Normal; unknown style -> Normal plus
  `iron_engine_api::log` warning; `focus` is stored as given and popups ignore it (they never take focus when shown, step 1).
  A `parent` of 0 for Tool / Dialog / Popup / Child -> MAIN; for Normal / Borderless -> None. With a key that
  an open window already has (outside reload): returns the new window anyway and logs a warning that the key is taken (find
  returns the first).
- Setters push requests and update the state at once (scripts read back what they set), except `width` / `height`: they push
  `Size` and keep the old value until `resized` (the OS may clamp). Document this on the getter.
- `close(MAIN)` pushes nothing and calls `app::request_quit()` (whatever `Application.quit` uses; look it up in `app.rs`).
- `close_requested(w)`: sets `in_close_request`, emits `on_close_requested` from a snapshot, returns `!cancel`, clears both.
- `closed(w)`: idempotent; marks `open = false`, emits `on_closed`, removes the listeners.
- `enter_current(w) -> CurrentGuard` restores the previous `current` on drop.

**Tests** (`crates/iron_engine_api/tests/window.rs`, Rust only, no GPU, no display):

| ID | Test |
|---|---|
| M9-API-1410 | `the_main_window_exists_from_the_start_and_takes_the_manifest_title` (`set_main("Game", ..)`; `WindowRef::main().title()`) |
| M9-API-1411 | `setting_the_title_asks_the_host_and_reads_back_at_once` |
| M9-API-1412 | `create_returns_a_handle_at_once_and_queues_one_request` (handle 2, `Create(2)`, `state(2).created == false`) |
| M9-API-1413 | `create_fills_defaults_per_style` (tool parent MAIN, normal parent none, NaN size 400 x 300, popup not focusing) |
| M9-API-1414 | `unknown_styles_fall_back_to_normal` |
| M9-API-1415 | `setters_of_the_same_field_coalesce_but_create_and_close_keep_their_order` |
| M9-API-1416 | `opened_fills_the_native_handle_platform_and_size` |
| M9-API-1417 | `resize_move_focus_and_scale_events_reach_their_listeners_once_per_change` |
| M9-API-1418 | `a_close_request_listener_can_cancel_the_close` |
| M9-API-1419 | `cancel_close_outside_a_close_request_does_nothing` |
| M9-API-1420 | `closed_runs_on_closed_once_and_drops_the_listeners` |
| M9-API-1421 | `a_closed_window_reads_its_last_values_and_ignores_setters` |
| M9-API-1422 | `count_and_at_list_open_windows_main_first`; `find(key)` |
| M9-API-1423 | `parent_children_and_is_child` (`isChild` per U1, `childCount`, `childAt`) |
| M9-API-1424 | `current_is_the_window_whose_event_is_dispatched_else_main` |
| M9-API-1425 | `off_removes_any_listener_by_token` |
| M9-API-1426 | `closing_the_main_window_from_script_asks_to_quit` |

**Acceptance.** `cargo test -p iron_engine_api` passes. `M9-API-120` (every manifest function exists in QuickJS,
`crates/iron_native/tests`) passes with the new module: run `cargo test -p iron_native`.

---

## Step 3: options objects in QuickJS (`#[options]`)

**Why.** `Window.create({ title, style })` maps by name in compiled code already (`call_ext_args`), but the QuickJS bindings
take positional arguments only (`audio.createBus(0.5)`).

**Files.** `crates/iron_api_macros/src/lib.rs` (the QuickJS argument reading of a function), `crates/iron_api/src/qjs.rs` (a helper
`options_object(ctx, value) -> Option<Object>`: a plain object whose prototype is `Object.prototype`, not an array, not a handle
or value class instance), test `crates/iron_api/tests/options.rs`, `docs/iron-api.md` (one paragraph under "Optional
parameters").

**Rule.** On a function marked `#[options]` (it must also have `#[optional(..)]`, else a compile error): when the call has
exactly `optional_from + 1` arguments and the last is a plain object, each optional parameter is read from the property with
its camelCase script name (`min_width` -> `minWidth`); a missing or `undefined` property is "left out" (the same default as
today). Positional calls keep working. The manifest does not change (the compiler already does this by name).

**Tests.**

| ID | Test |
|---|---|
| M9-API-1430 | `an_options_object_fills_optional_parameters_by_name` (a demo `#[iron_api]` module in the test: `make(a: f64, #[optional(b = 2, c, s)] ...)`) |
| M9-API-1431 | `left_out_keys_take_the_declared_defaults` |
| M9-API-1432 | `positional_calls_still_work_and_a_value_class_argument_is_not_an_options_object` |
| M9-API-1433 | `window_create_takes_an_options_object_in_quickjs` (`crates/iron_native/tests/window.rs`: `Window.create({ title: "T", style: "tool", width: 200 })`, then `Window.main().title`, `w.isChild`, `w.parent.handle === 1`) |
| M9-API-1434 | `window_listeners_run_from_quickjs_and_can_cancel_a_close` (same file: `w.onCloseRequested(() => w.cancelClose())`, `window::close_requested(w) == false`) |

Then put `#[options]` on `WindowRef::create`.

**Acceptance.** `cargo test -p iron_api -p iron_native -p iron_engine_api` passes.

---

## Step 4: compiled programs, types and `createRoot(window)` (`tools/`)

**Files.**

- `tools/iron_ts/src/lower/engine_ts.d.ts`: add (format as the file's header describes)
  ```ts
  type WindowStyle = "normal" | "tool" | "dialog" | "popup" | "borderless" | "child";
  interface WindowOptions { title?: string; width?: number; height?: number; x?: number; y?: number; minWidth?: number;
    minHeight?: number; style?: WindowStyle; parent?: Window | null; resizable?: boolean; minimizable?: boolean;
    maximizable?: boolean; closable?: boolean; alwaysOnTop?: boolean; visible?: boolean; focus?: boolean; key?: string; }
  interface Window {
    style: WindowStyle;
    readonly parent: Window | undefined;
  }
  declare namespace Window {
    const main: Window;
    const focused: Window | undefined;
    const current: Window;
    const count: number;
    function create(options?: WindowOptions): Window;
    function at(index: number): Window | undefined;
    function find(key: string): Window | undefined;
  }
  ```
- `tools/iron_ts/src/lower/ambient.d.ts`: `declare function createRoot(target?: WorldPanel | Window | number): ...`.
- `tools/iron_ts_ui/src/mount.rs`: `WINDOW_MOUNTS`, `#[unsafe(no_mangle)] pub extern "C" fn ts_ui_mount_window(window: u64,
  element: u64)`, `pub fn take_window_mounts() -> Vec<(u64, u64)>`; a third `mount` overload in `root_manifest_json()` with
  `{"name": "window", "type": "handle:Window"}`; the same in `tools/iron_ts/src/lower/ui_root_manifest.json`; export
  `take_window_mounts` from `lib.rs`. Make sure the symbol is listed by `iron_ts_ui::symbols()`.
- `tools/iron_ts_tests/bench/windows.ts` (new program, prefix `win_`, struct base 7400, engine manifests; add it to the table
  in `tools/iron_ts_tests/build.rs` and to the `engine_manifests()` match arm list, plus `pub mod windows_t` in
  `tools/iron_ts_tests/src/lib.rs` as the others) exporting functions the test calls:
  - `win_createTool()`: `Window.create({ title: "Palette", style: "tool", width: 220, height: 320, key: "palette" })`, returns
    the handle;
  - `win_describe(h)`: finds the window by handle through `Window.at` and returns
    `` `${w.title}|${w.style}|${w.isChild}|${w.parent?.title ?? "-"}|${w.width}x${w.height}` ``;
  - `win_setMainTitle(t)`, `win_mainTitle()`;
  - `win_listen(h)`: registers `onResized`, `onClosed`, `onCloseRequested` (cancels when a module flag is set) and counts calls;
  - `win_counts()`: returns the counters as text; `win_list()`: titles of `Window.at(i)` for `i < Window.count`, joined by `,`;
  - `win_mount(h)`: `createRoot(w).render(<box><text>{w.title}</text></box>)` (JSX in a `.ts` bench needs the `.tsx` path:
    name the file `windows.tsx` if the build script supports it, else build the element with the `Ui` helpers the other UI
    benches use; check `bench/ui.ts`).
- New `tools/iron_ts_tests/tests/all/windows.rs` (`mod windows;` in `main.rs`), using the `in_every_gc_mode` pattern of
  `render_native.rs` with `iron_engine_api::window::reset()`.

**Tests.**

| ID | Test |
|---|---|
| M9-TS-2800 | `create_with_an_options_object_maps_keys_by_name` (`win_describe` = `Palette\|tool\|true\|Iron\|220x320`) |
| M9-TS-2801 | `window_main_reads_as_a_property_and_its_title_goes_to_the_host` (`WindowRequest::Title(MAIN, ..)`) |
| M9-TS-2802 | `static_and_instance_focused_both_resolve` (`Window.focused` undefined, `w.focused` false; after `focus_changed(w, true)` both say so) |
| M9-TS-2803 | `window_listeners_from_compiled_closures_run_and_cancel` |
| M9-TS-2804 | `count_and_at_list_the_windows_in_order` |
| M9-TS-2805 | `create_root_of_a_window_queues_a_window_mount` (`iron_ts_ui::take_window_mounts()` has `(handle, element)`; `take_mounts()` is empty) |
| M9-TS-2806 | `iron_build_types_declare_window_and_window_options` (in `tools/iron_build` tests: `types` output contains `declare class Window`, `interface WindowOptions`, `const main: Window`) |

**Acceptance.** `cd tools && cargo test -p iron_ts_tests --test all windows` and `cargo test -p iron_build` pass, also with
`IRON_TS_GC=stress` (the `in_every_gc_mode` helper covers it).

---

## Step 5: multi-window platform (`iron_platform`)

**Files.** `crates/iron_platform/src/platform.rs` (Runner), `windows.rs` (backend trait, `run_commands`, `attributes`, win32
helper), new `crates/iron_platform/src/fake.rs`, `input.rs` (two variants), `Cargo.toml` (`windows-sys` 0.52 features add
`"Win32_UI_Shell"`), `lib.rs` exports (`fake::{FakePlatform, FakeWindow}`).

**Code** (design 7.2 to 7.4):

1. `InputEvent::Moved { x: i32, y: i32 }`, `InputEvent::ScaleChanged(f64)`; `convert_window_event` maps `WindowEvent::Moved` and
   `WindowEvent::ScaleFactorChanged { scale_factor, .. }`.
2. `PlatformApp` gains the default methods of 7.2 (`take_window_commands`, `window_opened`, `window_open_failed`,
   `window_input`, `close_requested`, `window_closed`, `events_done`, `take_windows_to_show`).
3. `WindowConfig` gains `pub virtual_windows: bool` (default false).
4. `trait Backend { fn open(&mut self, key, spec: &WindowSpec, plan: &NativePlan) -> Result<WindowHost, String>;
   fn apply(&mut self, key, cmd: &WindowCommand); fn destroy(&mut self, key); fn show(&mut self, key); }` and
   `pub(crate) fn run_commands<A: PlatformApp, B: Backend>(app: &mut A, backend: &mut B, tree: &mut WindowTree, os: Os)`, which
   drains `app.take_window_commands()`: `Open` -> `plan`, log `plan.warning`, `backend.open` -> `window_opened` or
   `window_open_failed`, `tree.insert`; `Close(k)` (k != MAIN) -> for each of `tree.close_order(k)`: `app.window_closed`,
   `backend.destroy`, `tree.remove`; others -> `backend.apply`.
5. `WinitBackend<'a> { event_loop: &'a ActiveEventLoop, windows: &'a mut BTreeMap<WindowKey, Arc<Window>>, ids: &'a mut
   HashMap<WindowId, WindowKey>, virtual_windows: bool, main_scale: f64 }` implementing `Backend` with
   `fn attributes(spec, plan, owner: Option<&Window>, embed: Option<&Window>) -> WindowAttributes` (design 7.3), and a
   `VirtualBackend` used when `virtual_windows` (records sizes, answers `WindowHost::Virtual`; `apply` of `SetSize` /
   `SetPosition` queues `Resized` / `Moved` events for the runner to send through `window_input`).
6. Runner: `ids`, `windows`, `tree` fields; main window inserted as MAIN in `resumed`; `window_event` routes by id
   (`RedrawRequested` of MAIN only; `CloseRequested` -> `close_requested(key)` -> if true: MAIN exits, others go through
   `Close` handling; `Destroyed` of a key still in `windows` -> the same close path); after input and after each frame
   `run_commands` and `take_windows_to_show()` -> `set_visible(true)`; `about_to_wait` calls `events_done()` first.
7. `#[cfg(windows)] fn set_tool_window(window: &Window, on: bool)` with the subclass (design 2.1 and 7.3), plus a no-op on other
   platforms. Keep `unsafe` blocks small with `// SAFETY:` comments as `set_black_background` does.
8. `fake.rs`: `FakePlatform` over `run_commands` with a `FakeBackend` (records `FakeWindow`s, applies setters to their spec,
   `show` sets a `shown` flag; `user_close` runs `close_requested` then `Close`; `focus` sends `Focused` events; `step` runs
   commands, `frame()`, commands, `take_windows_to_show`, `events_done()`).

**Tests** (`crates/iron_platform/tests/windows.rs` with a small recording `PlatformApp`; the pure `attributes` test in
`windows.rs`):

| ID | Test |
|---|---|
| M9-API-1440 | `moved_and_scale_factor_events_convert` (in `input.rs` tests) |
| M9-API-1441 | `one_window_apps_get_main_window_events_through_input` (default `window_input`) |
| M9-API-1442 | `open_commands_open_windows_and_report_them` (FakePlatform: `Open(2, tool)` -> `window_opened(2, Virtual)`; `window(2).plan.owner == Some(MAIN)`) |
| M9-API-1443 | `windows_stay_hidden_until_the_app_says_they_were_drawn` |
| M9-API-1444 | `a_cancelled_close_request_keeps_the_window` and `closing_the_main_window_exits` |
| M9-API-1445 | `closing_a_parent_closes_its_children_first` (`window_closed` order `[3, 2]`) |
| M9-API-1446 | `setters_reach_the_window` (title, size, position, kind -> plan recomputed) |
| M9-API-1447 | `winit_attributes_follow_the_plan` (`attributes(..)`: public fields `title`, `inner_size`, `decorations`, `window_level`, `resizable`, `enabled_buttons`, `visible == false`, `active`) |
| M9-API-1448 | `focus_moves_between_windows_and_events_done_runs_once_per_step` |

The winit runner itself is not run in tests (it needs a display). `cargo check -p iron_platform` plus the fake tests cover the
command path it shares.

**Acceptance.** `cargo test -p iron_platform`; `cargo test --workspace` unchanged elsewhere (iron_app compiles with the default
methods); **no screenshot run in this step.**

---

## Step 6: more surfaces on one GPU (`iron_render`)

**Files.** `crates/iron_render/src/gpu.rs`.

```rust
/// A surface for another window on this GPU (same instance, adapter and device).
pub fn create_window_surface(
    &self,
    target: impl Into<wgpu::SurfaceTarget<'static>>,
    width: u32,
    height: u32,
) -> Result<WindowSurface, RenderError>
```

It calls `self.instance.create_surface(target)?`, checks `self.adapter.is_surface_supported(&surface)` (else a new
`RenderError::Unsupported(String)` variant), then `WindowSurface::new(self, surface, width, height)` (present mode
`AutoNoVsync`, the same frame latency).

**Tests.** None can create a surface without a window; the function is covered by step 8's manual path only. Add a doc comment
saying so. (`cargo test -p iron_render` must still pass; no new goldens.)

---

## Step 7: secondary windows in the app: state, routing, closing (`iron_app`)

**Files.** New `crates/iron_app/src/windows.rs`; `main.rs` (fields, `PlatformApp` methods, `Surface::Window(u32)`,
`handle_on`, `update_cursor`, `update_ime`); `native_render.rs` (window runtimes beside panel runtimes); `native.rs` (collect
`iron_ts_ui::take_window_mounts()` into `window_mounts`, like `panel_mounts`); `iron_core` gets `pub const WINDOW_TARGET: u32 =
0x4000_0000;` (`crates/iron_core/src/lib.rs`).

**Code** (design 8.1 to 8.4):

- `windows.rs`: `AppWindow`, `AppWindows`, and pure helpers:
  - `fn requests_to_commands(reqs: Vec<WindowRequest>, state: impl Fn(WindowRef) -> Option<WindowState>) -> (Vec<WindowCommand>,
    Option<String> /* main title */)`: maps `Style` -> `WindowKind`, `Create` -> `Open(key, spec)` (position None -> resolved by
    `place` with the parent's last known outer position and size and the number of its open children), `Close(MAIN)` dropped.
  - `fn to_ui(physical: (u32, u32), scale: f64) -> (f64, f64)`.
  - `fn slug(title: &str, handle: u32, used: &mut HashSet<String>) -> String` (design 8.5).
  - `fn app_focused(focused: &BTreeSet<WindowKey>) -> bool`.
- `App` fields: `windows: windows::AppWindows`, `window_commands: Vec<WindowCommand>`, `windows_to_show: Vec<WindowKey>`.
  `App::new`: `iron_engine_api::window::set_main(&manifest.title, (w, h), 1.0)`; `window_ready` calls it again with the real
  scale. `run()` passes `virtual_windows: args.screenshot.is_some()`.
- After every pump point that runs scripts (`tick`, `native_after`, `apply_script_output`), call `self.pump_windows()`: take the
  requests, convert, apply the main title (`gfx.window.set_title`), queue commands.
- `window_opened(key, host)`: make the `AppWindow` (tree via `new_panel_tree(root, fonts, sheets)`; `Os` -> `gpu.create_window_surface`,
  `Virtual` -> `OffscreenTarget::with_format(gpu, w, h, Rgba8UnormSrgb)`), then `window::opened`. QuickJS: replay pending
  commits for `WINDOW_TARGET | key`. Native: nothing (mounts arrive by `take_window_mounts`).
- QuickJS commit routing: in the code that routes commits by target to panels, a target `>= WINDOW_TARGET` goes to
  `windows.map[target - WINDOW_TARGET].tree` (or a pending list).
- `window_input(key, ev)`: MAIN -> existing `input`; others -> `self.window_event(key, ev)`: pointer/wheel/keys/IME through
  `handle_on(Surface::Window(key), input)` (which uses that window's `Interaction` and dispatches UI events with
  `window::enter_current`), keys also to game input unless typing (U2), `Resized` -> surface/offscreen resize + `window::resized`,
  `Moved` -> `window::moved` (logical), `ScaleChanged` -> `window::scale_changed`, `Focused` -> the focused set +
  `window::focus_changed`, `CloseRequested` never arrives here (the platform calls `close_requested`).
- `close_requested(key)`: `window::close_requested(WindowRef(key.0))` inside `iron_scene::api::enter` (and the QuickJS
  equivalent: the callback runs inside the runtime already); MAIN: if true, set `done` as today.
- `window_closed(key)`: drop the native runtime of that window (`native_render` state), then the `AppWindow`, then
  `window::closed`.
- `events_done()`: `lifecycle.set_focused(app_focused(..))` when it changed, then `sync_lifecycle()`; the main window's own
  `Focused` handling no longer calls `lifecycle.set_focused` directly.

**Tests** (`#[cfg(test)]` in `windows.rs`, no GPU):

| ID | Test |
|---|---|
| M9-API-1460 | `script_requests_become_platform_commands` (Create tool -> `Open` with kind Tool and parent MAIN; Title(MAIN) -> main title) |
| M9-API-1461 | `windows_without_a_position_are_placed_on_their_parent` |
| M9-API-1462 | `screenshot_names_are_slugs_of_the_titles_and_unique` (`"Counter 1"` -> `counter-1`, `"!!"` -> `window3`, a repeat -> `counter-1-4`) |
| M9-API-1463 | `the_game_is_focused_while_any_of_its_windows_is` |
| M9-API-1464 | `window_targets_do_not_collide_with_panels` (`WINDOW_TARGET \| 2` vs a panel id) |

**Acceptance.** `cargo test -p iron_app` (unit tests), `cargo clippy`. Existing screenshots unchanged: run **one**
`cargo run -p iron_app -- examples/css --screenshot tests/output/css.png --frames 30` and compare with the previous render by eye
(single-window path untouched).

---

## Step 8: drawing secondary windows and screenshots (`iron_app`)

**Files.** `crates/iron_app/src/main.rs` (`frame`, `save_screenshot`), `windows.rs` (`draw_window`).

- `fn draw_window(gpu, renderer, w: &mut AppWindow, now_ms)`: `w.tree.set_size(ui)`, `set_media_env` with that window's scale
  and theme, `load_images`, `draw_list`, then into the surface (`present_with`) or the offscreen target:
  `renderer.render(gpu, &RenderTarget { view, format, size, scale }, CLEAR, &cmds)`. Sets `drawn`; the first time with
  `spec.visible` it pushes the key to `windows_to_show`.
- `frame()`: after the main present, for each open window (not minimised, non-zero size) `draw_window`. Native window runtimes
  flush first (`sync_window_roots`, next to `sync_panels`).
- `save_screenshot(path)`: after writing the main PNG, for each open `AppWindow` draw it offscreen (Virtual windows already are)
  and `write_png(path.with_file_name(format!("{stem}-{slug}.png")))`, with `slug` from step 7.

**Tests.** A GPU test with `iron_render::testing::shared_gpu()` in `crates/iron_app/src/windows.rs`:

| ID | Test |
|---|---|
| M9-API-1470 | `a_virtual_window_draws_its_ui_offscreen` (an `AppWindow` with a Virtual host, a tree with one 40 x 40 red box; `draw_window`; read back: the pixel at (10, 10) is red) |

**Acceptance (screenshot).** Write a throwaway game in the scratchpad (not committed), `window_probe/src/main.tsx`, built with
`cd tools && cargo run -p iron_build -- build <scratchpad>/window_probe`:
```tsx
Window.main.title = "Probe main";
const w = Window.create({ title: "Probe tool", style: "tool", width: 200, height: 120 });
createRoot(w).render(<box style={{ width: "100%", height: "100%", backgroundColor: "#c03030" }} />);
createRoot().render(<box style={{ width: "100%", height: "100%", backgroundColor: "#3030c0" }} />);
```
Then **one** run: `cargo build -p iron_app && cargo run -p iron_app -- <scratchpad>/window_probe --screenshot
tests/output/probe.png --frames 10`. Expect `probe.png` (blue) and `probe-probe-tool.png` (200 x 120 at the main scale, red),
and the log line `saved screenshot`. No second window appears on screen (Virtual).

---

## Step 9: hot reload keeps keyed windows (`iron_engine_api`, `iron_app`)

**Files.** `crates/iron_engine_api/src/window.rs` (`reload_begin`, `reload_end`, claiming in `create`), `native.rs`
(`Native::reload`: call `window::reload_begin()` before the resets and close what it returns; after the first frame of the new
program `reload_end()`), `main.rs` `reload_scripts` (QuickJS: same two calls), `native_render.rs` (drop window runtimes on
reload; new mounts refresh like panels).

Rules: design section 9. `window::reset()` is **not** added to the reload reset list (it would close nothing on screen but forget
the windows); `reload_begin` replaces it there. `Native::shutdown` calls `window::reset()`.

**Tests.**

| ID | Test |
|---|---|
| M9-API-1480 | `reload_drops_listeners_and_returns_keyless_windows_to_close` |
| M9-API-1481 | `create_with_the_key_of_a_kept_window_returns_that_window_with_the_new_title` |
| M9-API-1482 | `keyed_windows_nobody_claimed_are_closed_after_the_reload` |
| M9-API-1483 | `the_main_window_survives_reload_with_its_title` |

**Acceptance.** Unit tests; then one screenshot run of the step 8 probe (with a `key: "tool"` added) with
`IRON_RELOAD_AT_FRAME=5 --frames 10`: both PNGs written, the log shows one reload and no "window closed" line for the keyed tool.

---

## Step 10: QuickJS games (`iron_app`, `js/`)

**Files.** `crates/iron_app/src/main.rs` (where the QuickJS runtime is built and on every reset: `scripts.add_native(|ctx|
iron_engine_api::window::window_api::register_quickjs(ctx))`, or the `NativeInit` shape `add_native` takes), new
`js/engine/src/window.ts` (`export type WindowStyle`, `export interface WindowOptions`, `export declare class Window` mirroring the
design listing with QuickJS's `main()` / `focused()` / `current()` / `count()` functions, and `export const Window =
(globalThis as any).Window`), `js/engine/src/index.ts` export, `js/react/src/index.ts` (`createRoot` accepts an object with a
numeric `handle` getter that is not the main window: `id = WINDOW_TARGET | handle`; `handle === 1` is the screen; the error
message for a second root names the window), test files.

**Tests.**

| ID | Test |
|---|---|
| M9-TS-2820 | `createRoot of a window commits to the window target` (`js/react/test/window_root.test.tsx`, with a fake `Window` object `{ handle: 3 }`: commits carry target `0x40000003`) |
| M9-TS-2821 | `createRoot of the main window is the screen root` |
| M9-TS-2822 | `@iron/engine re-exports the global Window` (`js/engine/test/window.test.ts`, with `globalThis.Window` stubbed) |

**Acceptance.** `bun test`, `bun run typecheck`, `bun run build:fixtures`, `cargo test --workspace`.

---

## Step 11: `examples/windows` (native, globals style)

**Files.** `examples/windows/package.json`, `examples/windows/tsconfig.json` (copy `examples/raindrop-demo`'s), and
`examples/windows/src/main.tsx`. **No** bun `build` / `dev` / `typecheck` scripts in `package.json` (the root
`bun run build` filters by name and `typecheck` runs only scripts that exist; this example is built by `iron-build`). Do not add it
to the root `build` filter. No imports: engine and UI names are globals (`iron-build types` writes `types/iron.d.ts` for tsc).

`package.json`:
```json
{
  "name": "windows",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "iron": { "title": "Windows", "company": "Iron examples", "identifier": "dev.iron.examples.windows" }
}
```

**What it shows.** The main window, titled by the game (`Window.main.title = "Windows demo: N tool windows"`, updated whenever
the count changes, so the manifest title "Windows" is only seen for the first frame), with:

1. **A toolbar** (a row at the top, dark `#20242c`, 44 px): buttons "New counter", "New colours", "Close all", and a label
   "Main handle 1 · isChild false".
2. **A list of open windows** (below the toolbar): one row per `Window.at(i)`: handle, title, style, `isChild`, parent title
   (`-` for none), a green dot when `focused`, and per row (not for the main window) buttons "Focus" (`w.focus()`) and
   "Close" (`w.close()`).
3. **A swatch area** filling the rest, its background the colour last picked in any colours window (starts `#2b3a55`), with
   the text "Picked in <window title>".

**Tool windows** (all `style: "tool"`, parent `Window.main`, keyed `counter-<n>` / `colours-<n>` so they survive hot reload):

- **Counter <n>** (240 x 200): title row (the window's title, 16 px bold), "Parent: <w.parent.title>", "isChild: <w.isChild>",
  "Handle: <w.handle>", "Size: <w.width> x <w.height>" (updated by `onResized`), a big number, buttons "-" and "+", and a
  "Close" button at the bottom (`w.close()`).
- **Colours <n>** (260 x 240): the same header lines, a 4 x 2 grid of swatches (`#e74c3c #e67e22 #f1c40f #2ecc71 #1abc9c #3498db
  #9b59b6 #ecf0f1`, 48 x 48 each, a white 2 px border on the selected one); clicking one sets the shared colour (the main window's
  swatch area and its text update) and a "Close" button.
- Each tool window's `onClosed` removes it from the module's `tools` map and re-renders the main list; `onFocusChanged` re-renders
  the list (the green dot).
- At start the game opens **Counter 1** and **Colours 1** at fixed offsets: `x: Window.main.x + Window.main.width + 16`, `y:
  Window.main.y` and `y + 230`, so a screenshot shows both (and they sit to the right of the main window when run by hand).

**Code shape** (globals style, as `raindrop-demo`):

```tsx
type ToolKind = "counter" | "colours";
interface Tool { win: Window; kind: ToolKind; count: number; }

const tools = new Map<number, Tool>();
let picked = "#2b3a55";
let pickedIn = "nowhere yet";
let serial = 0;
const listeners = new Set<() => void>();
function notify() { for (const f of listeners) f(); updateMainTitle(); }
function useVersion(): number { /* useState + useEffect subscribing to `listeners`, as raindrop's useStats */ }

function updateMainTitle() {
  Window.main.title = `Windows demo: ${tools.size} tool window${tools.size === 1 ? "" : "s"}`;
}

function openTool(kind: ToolKind, x?: number, y?: number): Window {
  serial += 1;
  const title = kind === "counter" ? `Counter ${serial}` : `Colours ${serial}`;
  const win = Window.create({ title, style: "tool", parent: Window.main, width: kind === "counter" ? 240 : 260,
    height: kind === "counter" ? 200 : 240, x, y, key: `${kind}-${serial}` });
  const tool: Tool = { win, kind, count: 0 };
  tools.set(win.handle, tool);
  win.onClosed(() => { tools.delete(win.handle); notify(); });
  win.onFocusChanged(() => notify());
  win.onResized(() => notify());
  createRoot(win).render(<ToolPanel handle={win.handle} />);
  notify();
  return win;
}

function ToolPanel(props: { handle: number }) { useVersion(); const t = tools.get(props.handle); if (!t) return null; /* header + body per kind */ }
function MainPanel() { useVersion(); /* toolbar, list over Window.count / Window.at(i), swatch area */ }

createRoot().render(<MainPanel />);
const m = Window.main;
openTool("counter", m.x + m.width + 16, m.y);
openTool("colours", m.x + m.width + 16, m.y + 230);
```

Check while writing: optional numbers in an options object (`x` undefined) must reach the API as "left out" (NaN); if the
compiler passes 0 for an undefined `number | undefined` variable, pass the keys only when defined (two `create` calls), and note
it in `tools/iron_ts/TODO.md`. If handle-typed props are not supported in compiled JSX, the `handle: number` prop above already
avoids them.

**Build and check (one screenshot run):**
```bash
cd tools && cargo run -p iron_build -- build ../examples/windows && cd ..
cargo build -p iron_app
cargo run -p iron_app -- examples/windows --screenshot tests/output/windows.png --frames 30
```
Expect `tests/output/windows.png` (the main window: toolbar, three rows: main, Counter 1, Colours 1), and
`windows-counter-1.png`, `windows-colours-1.png`. Look at all three by eye. Add the example to `bun run check:examples` (it picks up
every folder; confirm it builds there) and to `scripts/screenshots.ts` only if that script handles extra PNGs (else leave it).

**Tests.** `M9-TS-2830` `examples_windows_compiles` (in `tools/iron_build` tests, as the other examples' compile checks are done
there, or in `check examples`: build `examples/windows` with `--no-tsc` and assert success).

---

## Step 12: docs

Delegate to a Sonnet subagent (CLAUDE.md), with the diff since the design commit. Pages:

- New `docs/windows.md`: the API (from design section 3), styles per platform (section 4), how the app draws and routes windows,
  screenshot files, hot reload, limits; linked from `docs/README.md`.
- `docs/platform.md`: multi-window runner, `PlatformApp` additions, `FakePlatform`, `virtual_windows`, the tool-window subclass.
- `docs/iron-api.md`: the `Window` module in "The engine API beyond the scene", `#[options]` under "Optional parameters", the
  QuickJS `Window.main()` difference.
- `docs/rendering.md`: `Gpu::create_window_surface`, secondary windows' present mode.
- `docs/editor.md`: one line in the limits: floating panels can use `Window` (design section 11).
- `docs/differences.md` if anything differs from browsers/Unity worth listing (no browser equivalent: probably nothing).
- `CLAUDE.md` layout table: `iron_engine_api` row gains `Window`; `examples/*` row gains `windows`.
- `TODO.md`: a "Windows API" item (done parts ticked, D1 to D5 open); `tools/iron_ts/TODO.md` "Windows API" item updated.

---

## Steps waiting for the user

- **D1 Embedded `child` windows** (Windows `WS_CHILD`, X11 reparent): `WinitBackend::open` passes `with_parent_window` for
  `plan.embed_in`; positions are relative to the parent's client area; input and drawing are as for other windows. Test with the
  fake platform only (`M9-API-1490`) and one screenshot (Virtual). Risk: two swapchains overlapping on one screen area.
- **D2 Modal dialogs**: `WindowOptions.modal` (Windows: `WindowExtWindows::set_enable(false)` on the owner while open, re-enabled
  before the dialog is destroyed so Windows does not activate another program). `M9-API-1491`.
- **D3 `Camera.targetWindow`** (design section 10): `camera.rs` field, `render_sync`/`world_view` filter by target, `draw_window`
  uses `draw_frame` when the window has cameras. `M9-API-1492` (views grouped by window), one screenshot.
- **D4 Transparent windows**: `transparent` option, `with_transparent(true)`, surface `alpha_mode` `PreMultiplied` when offered,
  clear colour alpha 0. `M9-API-1493`.
- **D5 Hidden main window in screenshot runs**: the runner never shows the window and drives `frame()` from `about_to_wait`
  when `WindowConfig::hidden` (screenshot runs). Removes the last visible window from automated checks. `M9-API-1494`.
- **U1..U6** in the design (section 14) are defaults this plan builds; changing one changes the step named there.

## Order and stopping points

1 -> 2 -> 3 -> 4 can be done and committed without touching the app (stop and report if any of `M9-TS-2800..2806` shows the
compiler cannot map the options object or resolve `focused`: those decide the API's shape). 5 -> 6 -> 7 -> 8 -> 9 -> 10 change
the platform and app; after 8 the screenshot probe must pass before going on. 11 is the example, 12 the docs.

## Note for the coordinator (not part of this plan)

A message about `project.json` (default company `"${firstName}'s Company"`, a `license` field with `private` / SPDX values,
`iron-build init --license`, license files in `iron-build package`) arrived during this design task. This worktree's base
(`c3ffc1b`) has no `project.json`, no `iron-build init` and no packaging metadata to change, so nothing was done here; it belongs
to the branch that has those. The only overlap with this plan: the main window's starting title comes from the manifest title
(`project.json` `title` there, `package.json` `iron.title` here).
