# md-reader

A Markdown reader for Iron's own docs, drawn entirely by Iron's UI (React through Iron's
reconciler, laid out by Taffy, text by Parley, drawn by Iron's renderer). It shows `docs/` and
the repository's top-level `.md` files.

## Running it

Build the example (this also bundles the docs), then run it. A release build scrolls and loads
long pages much faster than a debug one.

```bash
cd examples/md-reader && bun run build
```

```bash
cargo run --release -p iron_app -- examples/md-reader
```

The docs are bundled at build time by `gen-docs.ts`: every `.md` under `docs/` and in the
repository root goes into `src/docs.gen.ts`, and the images they can show are copied into
`content/` (both git-ignored). Run `bun run build` again after editing the docs, or use
`bun run dev`, which rebuilds on every change and reloads the running window in place.

## Using it

| Action | How |
|---|---|
| Open a doc | Click it in the file tree, or follow a link |
| Show or hide the file tree | The ☰ button, or Ctrl+B |
| Back / forward | The ← / → buttons, Alt+← / Alt+→, or the mouse's side buttons |
| Home (`docs/README.md`) | The house button |
| Open a folder's listing | Click a folder in the path bar (`Iron / docs / …`), or a `folder/` link |
| Light / dark | The sun / moon button on the right (remembered between runs) |

- Links between docs and to headings (`page.md#heading`) work like a browser's. Links to source
  files and external sites can't be opened from here yet: hovering or clicking one shows its
  target in the status bar.
- Going back returns to where you had scrolled on that page.
- Long pages show their first screenful at once and finish loading over the next frames.

## What it renders

Headings, paragraphs, **bold**, *italic*, ~~strikethrough~~, `inline code`, links, images (PNG,
JPEG and SVG; GIFs show their caption until Iron decodes them), lists (nested, numbered, task
lists), quotes, GitHub tables, rules and code blocks. Code is highlighted for TypeScript /
JavaScript, Rust, WGSL, CSS, JSON, TOML and shell, in Atom's One Dark or One Light colours.

## Command-line arguments (planned)

Iron doesn't pass extra arguments to game scripts yet, so these are planned, not available
(see [TODO.md](TODO.md)):

```bash
cargo run --release -p iron_app -- examples/md-reader --root ../my-project/docs --goto "guide/setup.md#install"
```

- `--root <folder>`: the folder to read instead of Iron's own docs.
- `--goto "<doc>#<heading-id>"`: open a doc at a heading. The doc path is relative to the root.
  `#L42` jumps to line 42 instead.

## Files

| File | What |
|---|---|
| `gen-docs.ts` | Bundles the docs and copies their images |
| `src/markdown.ts` | The Markdown parser (the subset the docs use) |
| `src/highlight.ts` | Syntax highlighting for code blocks |
| `src/main.tsx` | The reader: file tree, history, path bar, progressive page mounting |
| `src/reader.css` | The look, with a light theme under `.app.light` |
