# The editor

The editor is `iron_editor` (`iron_editor.exe` on Windows): the same program as `iron`, running
an editor UI written with Iron's own React UI, like a game. Its components live in a shared
library, `@iron/editor-ui`, so games and tools can use them too.

The editor's UI (`editor/`) is a QuickJS bundle, a deprecated backend: `iron_editor` passes `--force` implicitly (it logs the deprecation
note once and runs). Porting the editor to iron_ts is part of the removal plan in TODO.md.

| Path | Role |
|---|---|
| [bin/iron_editor.rs](../crates/iron_app/src/bin/iron_editor.rs) | The second binary: includes main.rs as a module and calls its `main` |
| [editor.rs](../crates/iron_app/src/editor.rs) | `ENABLED`, where the UI lives, the project folder, `EditorHost` (log feed, `editor.*` ops), opening files in a code editor |
| [project.rs](../crates/iron_app/src/project.rs) | File operations on the project folder (list, copy in, move), confined to it |
| [logging.rs](../crates/iron_app/src/logging.rs) | The capture layer and ring buffer the console reads |
| [editor/](../editor/src/main.tsx) | The editor's UI (a Bun workspace built to `editor/dist`) |
| [js/editor-ui](../js/editor-ui/src/index.ts) | `@iron/editor-ui`: `MenuBar`, `Dock`, `VirtualList`, `TreeView`, `Console`, `ProjectBrowser`, `SceneView`, `onEditorFrame` and their models |

## One program, two binaries

`crates/iron_app` has two binary targets. `iron_app` is `src/main.rs`. `iron_editor` is
`src/bin/iron_editor.rs`, which includes main.rs with `#[path = "../main.rs"] mod app;` and
re-exports the modules that reach each other through `crate::` (`info`, `models`, `particles`,
which main.rs declares `pub(crate)`). The `[[bin]]` entry sets `test = false`, so the tests don't
run twice.

`editor::ENABLED` compares `env!("CARGO_BIN_NAME")` with `iron_editor` at compile time. Where it
is true:

- `iron_editor [project]`: `parse_args` keeps the folder argument (default: the current folder)
  as the project (`editor::set_project`, made absolute; `__iron_info.app.project`) and runs
  `editor::ui_folder()` instead: `IRON_EDITOR_UI`, else an `editor` folder next to the executable
  (a packaged editor), else the workspace's `editor/`.
  The folder is a game folder like any other (`dist/game.js`, `dist/game.css`, `package.json` for
  the title and window size), so `--screenshot`, hot reload and `bun run dev` work as for games.
- `main` turns on log capture before anything logs.
- `__iron_info.app.isEditor` is true (`Application.isEditor`).
- `editor.*` ops run. In `iron` they log a warning and do nothing, so games can't use them.

## The log feed

`logging::init` adds `CaptureLayer` (filtered like the other layers) to the subscriber. While
`capture(true)` is on, each event becomes a `LogRecord { seq, level, target, message, time_ms }` in
a static ring buffer of 5,000 records; the message is the `message` field followed by the other
fields as ` name=value`. Scripts' `console.*` logs with target `script`, so the buffer has engine
and script lines in order.

`EditorHost::pump` runs once per frame, right after `tick`. Once the scripts have sent
`editor.logHistory`, it calls `__iron_log(flat)` with the records it hasn't sent, flat as
`[seq, level, target, message, timeMs, ...]` (level 0 trace to 4 error). `editor.logHistory`
resets its position to 0, so a reloaded UI gets the whole history again; `log.ts` keeps the
newest `seq` it has seen and skips older ones. Errors from `__iron_log` go to stderr, not the log,
which would feed them back.

## Ops

| Op | Args | Does |
|---|---|---|
| `editor.logHistory` | none | Starts the feed from the first kept record |
| `editor.openFile` | `path, line, col` | Opens the file at that place in the user's code editor |
| `editor.listDir` | `id, path` | Answers `{ path, entries: [{ name, dir, size }] }`: folders first, by name, without dot files |
| `editor.copyIn` | `id, [sources], dir` | Copies files or whole folders from outside into a project folder, renaming (`a (2).png`) instead of overwriting; answers `{ created }` |
| `editor.move` | `id, path, dir` | Moves a project file or folder into a project folder; refuses overwriting and moving a folder into itself; answers `{ path }` |

The file ops answer a frame later through `__iron_editor_fs([id, ok, json or error, ...])`,
sent by `EditorHost::pump`; `project.ts` turns them into promises. Their paths are relative to the
project and go through `project::resolve`: `iron_core::resolve_asset`'s rules (no absolute paths,
drives, schemes or `..`), and through symlinks the real path must still be inside. `copyIn`'s
sources are the exception, being outside: only paths dropped on the window this session
(`EditorHost::allow_sources`, fed by the file drag code) are allowed. A copy creates at most
10,000 files and skips symlinks.

`editor.openFile` resolves a relative path against the current folder, the UI's folder, then the
source tree the program was built from, taking the first where it exists. The command comes from
`IRON_EDITOR_OPEN` if set: split on spaces (double quotes group), with `{file}`, `{line}` and
`{col}` filled in, e.g. `subl "{file}:{line}:{col}"`. Otherwise it is `code -g file:line:col` when
`code` is on PATH (`which` tries PATHEXT's extensions on Windows, where it is `code.cmd`, and the
process gets `CREATE_NO_WINDOW`). With neither, it logs how to set one up.

## `@iron/editor-ui`

Each component has a model module of plain functions (tested with `bun test` without React) and a
React component that draws it. Styles are in `editor-ui.css` (bundled into the app's `game.css`),
coloured through `--iron-*` custom properties.

### MenuBar ([menu.ts](../js/editor-ui/src/menu.ts), [MenuBar.tsx](../js/editor-ui/src/MenuBar.tsx))

`menus: Menu[]`, each with `MenuItem`s: `label`, `accelerator`, `action`, `submenu`, `checked`,
`disabled`, `separator`. The state is a `MenuState { top, open, path }`: the open top menu and the
highlighted index at each level; a submenu shows while its item is highlighted.

- **Mouse:** pressing a title opens or closes it; while one is open, hovering another title opens
  that one; hovering an item highlights it; a click runs it. When open, a transparent backdrop
  (an absolutely positioned child of the bar, far larger than the window, below the titles and
  menus) catches presses anywhere else and closes the menus.
- **Keys:** the bar takes focus (`tabIndex={-1}`) when a menu opens. `menuKey` handles
  arrows (Left/Right also open and close submenus and move between menus), Home/End, Enter/Space
  and Escape (one level at a time). Separators and disabled items are skipped.
- **From anywhere:** `input.on("keydown")` (gameplay keys, which arrive whatever has focus, except
  text fields) runs accelerators (`findAccelerator`, matched by `code` and modifiers), Alt+letter
  opens the menu starting with that letter, and F10 highlights the bar.

Accelerators are written like `Ctrl+Shift+S`, `F5`, `Delete`, `Ctrl++`; `acceleratorLabel` shows
them normalized.

### Dock ([dock.ts](../js/editor-ui/src/dock.ts), [Dock.tsx](../js/editor-ui/src/Dock.tsx))

A layout is a tree of `DockSplit { direction: "row" | "column", children, sizes }` (sizes are
fractions adding to 1) with `DockGroup { tabs, active }` leaves. Every operation returns a new
layout: `activate`, `removeTab`, `dockTab(layout, tab, group, zone, index?)` and `resize`.
After each change, empty groups are dropped, single-child splits are replaced by the child, and a
child split in the same direction as its parent merges into it, so docking left of a group in a
row adds a column instead of nesting a new row. `serializeLayout` / `parseLayout` save and load
JSON; `parseLayout` drops unknown tabs and returns null for malformed input.

`Dock` draws splits as flex rows or columns whose children have `flexGrow` = their fraction and
`flexBasis: 0`, with `SPLITTER`-pixel splitters between them, and groups as a `TAB_BAR`-high tab
strip over the active panel. Iron has no layout read-back for scripts, so `computeRects` works out
the same rects in JS from the `rect` prop (the dock's place on screen); panels get their content
box from it (`render({ x, y, width, height })`: the size below the tab strip, and where it is on screen in UI
pixels, for a camera drawing into it), and drags use it:

- **Seeing through.** A panel with `seeThrough: true` gets a transparent group background (the
  `see-through` class), and `.iron-dock` has no background of its own, so what is drawn under the UI (a
  camera's view of the world) shows in the panel's box. Other groups keep the panel colour.
- **Splitters** take pointer capture on press and resize from the layout as it was when the drag
  started (pixels moved divided by the split's free space).
- **Tabs** take pointer capture on press. Past 5 pixels the press becomes a drag: `dropTarget`
  finds the group under the pointer and the zone (tab strip or middle: `center`; within a quarter
  of an edge: that side), a translucent box previews the zone (`zoneRect`) and a ghost of the tab
  follows the pointer. Releasing calls `dockTab`. The drag state is kept in a ref too, since
  pointer moves render a tick later.

### VirtualList ([virtual.ts](../js/editor-ui/src/virtual.ts), [VirtualList.tsx](../js/editor-ui/src/VirtualList.tsx))

Fixed row height. The list is a scrolling box (`overflowY: auto`, `height` from props) with a
spacer as tall as all rows; only rows in `visibleRange(scrollTop, height, rowHeight, count,
overscan)` are mounted, absolutely placed at `index * rowHeight`. The engine scrolls the box
(wheel, scrollbar, keys) and the `scroll` event updates `scrollTop`, so the overscan rows cover the
frame before React catches up. The handle (`ref`) has `scrollToIndex(index, align)`
(`scrollForIndex`: only as far as needed, or start / center / end), `scrollToEnd` and `focus`.
With `keyboard` on (the default), arrows, Page Up/Down and Home/End move `selected` (`keyIndex`)
and scroll it into view.

### TreeView ([tree.ts](../js/editor-ui/src/tree.ts), [TreeView.tsx](../js/editor-ui/src/TreeView.tsx))

`roots: TreeNode[]` (`id`, `label`, `children`, `data`) and a `TreeState { expanded, selected,
focus, anchor }` the parent keeps. `flatten` turns the expanded part into rows (iteratively, so
deep trees are fine), drawn by a `VirtualList`. Clicks go through `clickSelect` (Ctrl toggles,
Shift ranges from the anchor, both add a range); keys through `treeKey` (Left collapses or goes to
the parent, Right expands or goes to the first child, Shift extends, Ctrl moves focus only,
Ctrl+Space toggles). Slots: `renderIcon`, `renderLabel`, and renaming with F2 (or a double-click
without `onActivate`): `renderRename(node, done)` or a default text field that commits on Enter or
blur and cancels on Escape, then `onRename(node, name)`. Expanded nodes show a CSS chevron (a
rotated square with two borders), since system fonts may lack the triangle glyphs.

### Console ([console.ts](../js/editor-ui/src/console.ts), [log.ts](../js/editor-ui/src/log.ts), [Console.tsx](../js/editor-ui/src/Console.tsx))

`log.ts` installs `__iron_log`, keeps up to 10,000 `LogEntry`s and is read through
`useSyncExternalStore` (subscribing sends `editor.logHistory` once). The panel has Clear,
Collapse (identical level and message shown once with a count, `collapse`), a search box over
message and target, and Info / Warn / Error toggles with counts (`countLevels`, debug counts as
info). Rows show a level dot, the time and the first line of the message, coloured by level. It
follows new lines while scrolled to the bottom; scrolling up pauses that until the end is in view
again.

`parseLinks` finds `path/file.ext:line[:col]` (relative, absolute, Windows drives, backslashes),
skipping hosts right after a URL scheme. Each link is its own `Text` element (spans aren't hit
targets of their own), and clicking it calls `openInEditor`, the `editor.openFile` op.

### ProjectBrowser ([project.ts](../js/editor-ui/src/project.ts), [ProjectBrowser.tsx](../js/editor-ui/src/ProjectBrowser.tsx))

The project folder's files on a `VirtualList`: breadcrumbs (the project's name, then each
folder), a status and Refresh; rows with a folder or file icon, the name and the size, and `..`
inside a folder. Double-click or Enter opens a folder, Backspace goes up. It uses drag and drop
([input-and-events.md](input-and-events.md#drag-and-drop-m5e)):

- Rows are `draggable`; `onDragStart` puts the item's path in the transfer
  (`PROJECT_PATH_TYPE`).
- Folder rows, breadcrumbs, `..` and the list itself (the folder shown) are drop targets: their
  `onDragOver` accepts system files (copy) and project items that `canMoveInto` that folder
  (move), highlights the folder, and stops propagation so the innermost one wins.
- `onDrop` calls `copyIn` with the dropped paths or `moveInto` with the item's path, then logs
  the result and lists the folder again.

`useScreenSize()` gives the window size in UI pixels, re-rendering on `Screen.onChange` (added to
`@iron/engine` for this).

## The Scene view ([SceneView.tsx](../js/editor-ui/src/SceneView.tsx), [scene-camera.ts](../js/editor-ui/src/scene-camera.ts), [frame.ts](../js/editor-ui/src/frame.ts))

The editor's world is the engine's: the Scene view is a camera in it, drawing into the panel's box.

**The camera.** `SceneView` makes a `Scene Camera` object with `hideFlags = HideAndDontSave` (not listed,
never saved, never `Camera.main`) and `dontDestroyOnLoad`, with a skybox clear, a dark background and
`depth: -100` so game cameras draw over it. The panel is `seeThrough`, so the view shows in its box. Each
frame `place` sets the camera's position, rotation, projection, clip planes and `rect` (the panel's box
over the screen size, written only when it changed), and `sceneGrid` follows the `grid` prop. The
camera is destroyed when the panel unmounts.

**The model** (scene-camera.ts) is plain functions from one `SceneCameraState { pivot, yaw, pitch,
distance, orthographic }` to the next, tested without React (`M5-EDT-22`). The eye sits `distance` behind the
pivot along the view direction (an orthographic view sits farther back and takes its size from
`distance`, so nearby objects aren't cut by the near plane). Yaw 0 looks along -z; positive pitch looks down.

| Control | Function |
|---|---|
| Right-drag | `look`: turns about the eye (the pivot moves to keep the distance) |
| Right-drag + WASD / QE | `fly`: right, forward, down and up for the frame's `unscaledDeltaTime`; speed grows with `distance`, x4 with Shift |
| Middle-drag | `pan`: slides the pivot so the point under the cursor at its depth follows |
| Wheel | `zoom`: `distance` times 1.1 per 40 units, clamped |
| Alt + left-drag | `orbit`: turns about the pivot |
| F | `frame`: the pivot at the selection's centre, far enough back to see its padded bounds |
| Axis gizmo | `lookAlong`: looks along an axis about the same pivot |

Pointer capture keeps a drag going outside the panel. A left click that doesn't move selects nothing
(`onSelect(null)`), and the context menu is suppressed.

**Frame callbacks.** `onEditorFrame(fn)` ([frame.ts](../js/editor-ui/src/frame.ts)) calls `fn` every frame
after scripts and animation have moved things. It makes one hidden object (`HideAndDontSave`,
`dontDestroyOnLoad`) with a component whose `onLateUpdate` calls the listeners, and returns the function
that unsubscribes. The Scene view and the live Hierarchy use it.

**Orientation gizmo.** An SVG in the top-right corner draws the six axes from `gizmoAxes` (farthest
first, so near ones paint over): positive axes are coloured discs with a line and a label, negative ones
small grey rings, and axes pointing away are dimmer. SVG shapes are not hit targets, so transparent boxes
sit over each axis end: clicking one looks along the opposite direction (towards the centre), and the
centre box toggles perspective and orthographic, shown as `Persp` / `Iso` under it.

**SceneIcons.** Cameras, directional, point and spot lights, audio sources and reflection probes (`iconKind`;
`visibleObjects()` lists the objects, depth first, leaving out `HideInHierarchy` ones) get a round 22 px
button at their projected position (`worldToScreenPoint`, dropped behind the near plane or outside the
panel), so icons keep one size at any distance. The art is a 16 px SVG per kind with fixed markup, so each
is rasterized once. A click selects the object (Ctrl or Shift adds); selected icons have an accent
border.

The icons move every frame but React doesn't: each frame's result is reduced to a key of the camera
state and each icon's id, kind and position rounded to whole pixels, and `setView` is called only when
the key changes. A still view costs a few projections and a string compare per frame.

**Wire gizmos.** Each frame, for the selected objects (when Show Gizmos is on), `drawGizmos` calls
`Gizmos` for what Unity draws: a camera's frustum (or a box for orthographic), a point light's sphere, a
spot light's cone, a directional light's rays, a probe's box, and an audio source's min and max
distance spheres. `Gizmos.present(camera)` then sends them to the Scene camera, so they draw over the
view (faint where hidden) and are replaced next frame. The lines belong to that camera, so the Game view
never shows them. See [world-rendering.md](world-rendering.md#editor-grid-and-gizmo-lines).

**Transform handles** ([handles.ts](../js/editor-ui/src/handles.ts)). Like Unity: Q is the view tool, W move,
E rotate and R scale (keys, or the toolbar in the panel). The handles sit on the last selected object and
keep a constant size on screen (`HANDLE_PIXELS`). `handles.ts` is plain math over an abstract `HandleView`
(world to panel pixels, rays, units per pixel), so it is tested without a camera (`M5-EDT-24`). It lays out
the parts, finds the one under the pointer (the centre, an axis, or a ring's front arc, within
`GRAB_PIXELS`) and turns a drag into the object's new pose: move along a local axis or in the view plane,
rotate about an axis through the ring's plane, scale by the ratio along an axis or uniformly from the
centre. Ctrl snaps (1 m, 15 degrees, 0.1). `SceneView` draws the parts as SVG in a box just around them,
lights up the hovered part, and reports each finished drag with `onTransformed(object, before, after)`
so the editor can undo it.

## The editor UI ([editor/src/main.tsx](../editor/src/main.tsx))

A menubar (File, Edit, View, Window, Help with accelerators and an Open Recent submenu), a dock
and a status bar. The default layout: Hierarchy on the left, Scene and Game in the middle,
Inspector on the right, Console and Project along the bottom. `.editor` has no background, so the
see-through panels show the world.

- **Demo scene.** `buildDemoScene()` ([demo-scene.ts](../editor/src/demo-scene.ts)) runs at startup: a
  floor, four shapes, the three kinds of light, a `Main Camera` (its `rect` is empty, so it draws
  nowhere until the Game panel gives it one), an audio source and a reflection probe, so every icon
  shows. It stands in until scenes open from the project.
- **Hierarchy.** A `TreeView` over `visibleObjects()`, with `GameObject.id` as node ids and an icon by
  component (`kindOf`). `useSceneTree` computes a key of `id:parentId:name` per object in `onEditorFrame`
  and rebuilds the tree only when it changes, so adding, moving or renaming objects shows at once.
  Renaming sets `GameObject.name`.
- **Selection** is the tree's selected ids, shared with the Scene view: selecting in either updates the
  other (`select`), and the Inspector (a placeholder) names the selected objects.
- **Game.** `GameView` sets `Camera.main`'s `rect` to the panel's box while it is shown and to an empty
  rect when it is hidden or unmounted, so the game's camera draws only inside the Game tab.
- **Project** is the project browser.
- **View > Show Grid / Show Gizmos** toggle the Scene view's `grid` and `gizmos` props.
- Window > Save / Load / Reset Layout keeps the layout in PlayerPrefs (`editor.layout`), and the saved
  layout loads at start. Unimplemented menu items log what they would do.

## Tests and checks

- `js/editor-ui/test/models.test.ts`: the models (`M5-EDT-06` to `-14`, `-18`, `-19`);
  `menubar.test.tsx`: the React menubar through the real reconciler with dispatched events
  (`M5-EDT-14`); `log.test.ts`: the log feed (`M5-EDT-20`).
- `js/editor-ui/test/scene-camera.test.ts`: the Scene camera's controls and axis gizmo (`M5-EDT-22`);
  `handles.test.ts`: the transform handles' math (`M5-EDT-24`);
  `js/engine/test/gizmos.test.ts` (`Gizmos`, `M5-RND-23`) and the `HideFlags` cases in `scenes.test.ts` (`M5-SCN-06`).
  The Scene view in the window is a manual check (`M5-EDT-23`); the grid and line passes have goldens (`M5-RND-22`, `-23`).
- Rust units in `editor.rs` (`M5-EDT-16`, `-17`) and `logging.rs` (`M5-EDT-15`); `project.rs`
  (`M5-DND-07`); `project.test.ts` and `project-browser.test.tsx` (`M5-DND-08`, `-10`).
- `bun run screenshots editor` renders `tests/output/editor.png` with `iron_editor` (`M5-EDT-21`,
  checked by eye). There is no golden: the editor uses system fonts.

## Limits

- Fixed row heights only; no tab reordering by position in the strip (a drop in a strip appends).
- The Scene view has no picking in the 3D view (only icons select) and snaps only with Ctrl;
  gizmos and handles are drawn for the selection only.
- Panels can't be closed or reopened yet, nor float in windows of their own.
- Accelerators don't run while a text field has focus (its keys don't reach `input`).
- No native menus yet (TODO.md M5.e).
