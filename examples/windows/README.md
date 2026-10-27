# windows

A showcase of Platypus's multi-window API. The main window has a small React UI with four buttons and an event log. From it you
open owned tool windows (a palette and any number of counters) and an isolated Inspector window with no parent. The Inspector
shares no state with the main window: it receives the selected object through named events and edits it by sending events
back. Nothing is drawn in 3D; this is all UI.

APIs used: `Window.main`, `Window.create({ title, style, parent, width, height, x, y, key })`, `Window.find`,
`Window.current`, `Window.isSupported`, `win.onOpened/onClosed/onOpenFailed`, `win.close()`, `win.emit` / `win.on` and
`win.ask` / `win.answer` (messages between windows), `win.openError`, `createRoot(win).render(...)`, and React hooks
(`useState`, `useEffect`). Docs: `docs/windows.md`.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/windows
```

then from the repo root:

```bash
cargo run -p platypus_app -- examples/windows
```

Or package it with `platypus-build package ../examples/windows`. A screenshot run is
`cargo run -p platypus_app -- examples/windows --screenshot out.png --frames N`. In screenshot runs `docs/windows.md` says
secondary windows are virtual (offscreen, no OS window), so only the main window is in the image. `docs/windows.md` also says
the real winit multi-window runner "has not been run on a real display", so treat the interactive run as experimental; the
interactive window may crash some GPUs.

## Structure

| File | What |
|---|---|
| `project.json` | `name` "windows", `title` "Platypus - Windows". |
| `tsconfig.json` | Extends the base config, `jsx: "react-jsx"`, a `paths` entry, includes `src`. |
| `src/main.tsx` | All 245 lines: data, the shared log, selection, `Palette`, `Counter`, `Inspector`, `App`, start-up. |

No assets, scenes, CSS or game components: there is no `register(...)` call at all, only `createRoot().render(<App />)`.

## How it works

**Data and the main window's domain.** `things` is a tiny array of `{ name, position }`. The main window owns the selection
(`let selected`) and notifies listeners. Because windows are separate React roots, the code shares state through plain module
variables plus subscriber lists (`useLines`, `selectionListeners`).

**Message passing.** The main window handles two kinds of messages sent from the Inspector:

```ts
Window.main.on("edit", (m: Edit, from: Window) => {
  for (const t of things) if (t.name === m.name) t.position[1] = m.y;
  note(`edit from window ${from.handle}: ${m.name}.y = ${m.y}`);
  select(selected);
});
Window.main.answer("selected", (): Thing => {
```

and `select(i)` pushes changes to the Inspector with `Window.find("inspector")?.emit("select", thingAt(i))`
(written there as an explicit `!== undefined` check).

**Owned tool windows.** `openPalette()` and `openCounter()` create windows with `style: "tool"` and `parent: Window.main`, each
with a `key` so a hot reload finds it again:

```ts
const win = Window.create({ title: "Palette", style: "tool", parent: Window.main, width: 200, height: 190, x: 80, y: 120, key: "palette" });
win.onOpened((w, h) => note(`Palette opened ${w}x${h}`));
win.onClosed(() => note("Palette closed"));
createRoot(win).render(<Palette />);
```

Each counter has its own `useState`, a `+1` button, and a `Close` button that calls `win.close()`.

**The Inspector.** `openInspector()` uses `style: "normal"` and no parent, so it is a separate family and event domain. Inside it,
`Window.current` is the Inspector. On mount it subscribes to `select` and asks the main window what is selected:

```ts
Window.main.ask<Thing>("selected").then(
  (answer) => {
    setThing(answer);
```

The `y + 0.5` / `y - 0.5` buttons send an `edit` event back to `Window.main`.

**Start-up.** After `createRoot().render(<App />)` and setting `Window.main.title`, if `Window.isSupported` it opens the palette, a
counter and the Inspector; otherwise it creates a "Probe" window that fails to open (`onOpenFailed`) and logs `probe.openError`. The
`App` also shows a red banner when windows are unsupported.

## Controls

All by mouse on UI boxes: **Palette** / **New counter** / **Inspector** open or re-find windows; **Next object** cycles the
selection; clicking a name in the Palette selects it; **y + 0.5** / **y - 0.5** in the Inspector edit the selected object; the
main window's Events box shows the last six log lines.

## Things to try and gotchas

- Build an editor-style game: a main viewport plus palette / inspector windows driven by the same event pattern.
- Add a window `style: "dialog"` for settings, or `"popup"` for a menu; options are listed in `docs/windows.md`.
- Replace the module-level lists with a small store and one `useStore` hook; the pattern here is deliberately minimal.
- Use `key` on every window you want to survive hot reload.
- Gotcha: window creation never throws; failure arrives through `onOpenFailed`/`openError` (phones and web have no extra windows).
- Gotcha: an isolated window (no parent) shares nothing with the main window except messages; module variables here are used only
  from main-domain callbacks, and the Inspector gets data through `ask`/`emit`.
- Gotcha: `thingAt(i)` exists because indexed reads may be `undefined`; `things[i] ?? {...}` supplies a fallback.
- Style objects use `PlatypusStyle`; `palette` is spread into several boxes (`{ ...palette, flexGrow: 1 }`).

Related examples: form, arcade, hello (for plain React UI); clock (`flip.tsx` for UI effects)
