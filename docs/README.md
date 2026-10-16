# Iron developer docs

These pages explain **how Iron is implemented today**: data flow, key types, algorithms and where
the code lives. They complement, and do not repeat:

| File | Role |
|---|---|
| [DESIGN.md](../DESIGN.md) | The intended design and decisions (may be ahead of the code) |
| [TODO.md](../TODO.md) | Plan and progress by milestone |
| [TESTS.md](../TESTS.md) | Test plan with IDs such as `M3-EVT-01` |
| [CLAUDE.md](../CLAUDE.md) | Short overview, commands, conventions, gotchas |

Where the code differs from DESIGN.md the pages say so ("Difference from DESIGN.md").

## Index

| Page | Covers |
|---|---|
| [scripting.md](scripting.md) | `iron_script`, `@iron/engine`, ops and commits, `ComponentHost`, bundles and how `iron_app` loads them |
| [ui-tree.md](ui-tree.md) | `iron_ui` tree, opcodes, styles, Taffy layout, Parley text, text fields (`editor.rs`), draw list |
| [css.md](css.md) | Stylesheet parsing, selector matching, cascade, invalidation, variables, `::before`/`::after`, `@font-face`, URL rebasing |
| [react.md](react.md) | `@iron/react`: reconciler host config, style/attr diffing, `createRoot`, `Text`/`Span`/`Input`/`TextArea`, events, refs |
| [input-and-events.md](input-and-events.md) | Platform input, `Pointer`, `Interaction`, hit testing, focus, dispatch to scripts, cursor, scrolling, text fields (keys, clipboard, IME), gamepads, game `input` API, gamepad UI navigation |
| [rendering.md](rendering.md) | `iron_render`: GPU setup, `UiRenderer` planning and passes, shaders, offscreen path |
| [testing.md](testing.md) | Test layout per crate, fixtures, goldens, mapping to TESTS.md |

## Crate map

```
iron_core      Value, ids, Commit/Op, asset path safety          (no deps on other Iron crates)
iron_platform  winit window + event loop, InputEvent (keys, pointer, IME, gamepads), gilrs
iron_script    ScriptRuntime trait, QuickJS backend, prelude.js, ComponentHost
iron_ui        UiTree, styles, CSS, layout, text, draw list, events/interaction
iron_render    wgpu: Gpu, surfaces, UiRenderer (consumes iron_ui::DrawCmd)
iron_app       the `iron` binary that wires everything together
js/engine      @iron/engine   (Component, register, op, loadFont, stylesheets, input)
js/react       @iron/react    (reconciler, events)
```

## The frame pipeline

The binary [main.rs](../crates/iron_app/src/main.rs) is driven by winit's redraw callback
(`PlatformApp::frame`). Input arrives separately, between frames, through `PlatformApp::input`.
The window is created hidden and shown after its first frame; a resize also draws a frame right away.

```
 winit WindowEvent
      |  iron_platform::convert_window_event
      v
 InputEvent --(scale to logical px)--> iron_ui::Input --> Interaction::handle
                                                              |  UiEvent (hover, click, key, ...)
                                                              v
                                       __iron_ui_event(type, path, data) in QuickJS
                                                              |  React handlers run, setState
 frame():                                                     v
   1. ComponentHost::update (fixed 60 Hz ticks)   -> script components (onStart/onUpdate)
   2. ScriptRuntime::tick(now_ms)                 -> timers + microtasks (React scheduling)
   3. drain_commits -> UiTree::apply_commit       -> node tree mutated, dirty sets filled
   4. drain_ops (loadFont, addStyleSheet, focus, scroll...) handled by the app
   5. Interaction::handle(Input::Refresh)         -> re-hit-test under a still pointer
   6. UiTree::set_size(window / scale)
   7. UiTree::draw_list()                         -> layout_if_needed: restyle -> text shaping
                                                     -> Taffy -> absolute rects -> Vec<DrawCmd>
   8. UiRenderer::render(...)                     -> instances, layers, clips -> wgpu passes
   9. present (or --screenshot: offscreen target, PNG, exit)
```

Key ideas to keep in mind:

- **Rust owns the world, JS owns behaviour.** JS only holds numeric node ids. React sends a
  *commit* (list of `[opcode, ...args]`) per React commit; Rust applies it as a unit.
- **Lazy, dirty-driven work.** `apply_commit` only records what changed. The expensive work
  (restyle, text shaping, layout) happens once in `UiTree::layout_if_needed`, called by
  `draw_list()` and `hit_test()`.
- **Logical pixels everywhere above the renderer.** The app divides window sizes and cursor
  positions by the scale factor; `RenderTarget::scale` converts back.
- **The draw list is GPU-free.** `iron_ui` never touches wgpu; tests can inspect `Vec<DrawCmd>`.

## Where to start reading

| I want to... | Read |
|---|---|
| Add a CSS property | `apply_style` in [style.rs](../crates/iron_ui/src/style.rs), then [ui-tree.md](ui-tree.md) |
| Add a selector / pseudo-class | [css.rs](../crates/iron_ui/src/css.rs), [cascade.rs](../crates/iron_ui/src/cascade.rs), [css.md](css.md) |
| Change text field editing | [editor.rs](../crates/iron_ui/src/editor.rs), [input-and-events.md](input-and-events.md#text-fields-m3c-step-5) |
| Add a UI event | [events.rs](../crates/iron_ui/src/events.rs), [events.ts](../js/react/src/events.ts), [input-and-events.md](input-and-events.md) |
| Change gamepad / game input | [gamepad.rs](../crates/iron_platform/src/gamepad.rs), [game_input.rs](../crates/iron_app/src/game_input.rs), [input.ts](../js/engine/src/input.ts), [input-and-events.md](input-and-events.md#gamepads-and-game-input-m3c-step-6) |
| Add a script op | [main.rs](../crates/iron_app/src/main.rs) `tick()`, [scripting.md](scripting.md) |
| Add a draw command | [draw.rs](../crates/iron_ui/src/draw.rs), [ui/mod.rs](../crates/iron_render/src/ui/mod.rs), [rendering.md](rendering.md) |
