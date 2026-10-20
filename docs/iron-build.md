# iron-build: building a game with iron_ts

`tools/iron_build` (binary `iron-build`) builds a game folder written in TypeScript for `iron_app` with **no bun and no
`node_modules`**. Only `tsc` is optional, for type checking while developing. It lives in the `tools` workspace:

```
cd tools
cargo run -p iron_build -- build ../examples/hello
```

| Command | What |
|---|---|
| `build <game> [--no-tsc] [--aot] [--cook] [--no-compile]` | Builds `<game>/dist`. |
| `types [<game>] [--out file]` | Writes `<game>/types/iron.d.ts` (type declarations for `tsc`). |
| `assets <folder>... [--prune]` | Writes `.meta` GUID files (the port of `scripts/assets.ts`). |
| `dev <game> [--no-tsc]` | Rebuilds on every change and signals the running app. |
| `check <examples folder> [--tsc]` | Builds every game in the folder and prints a table (`bun run check:examples`). |

## What a build does

1. **Config.** `package.json` -> `iron { title, company, identifier, main? }`; `tsconfig.json` (comments and trailing commas allowed)
   -> `include` (default `src`) and `compilerOptions.jsx`. The entry is `iron.main`, else the first of `src/main.tsx`, `src/main.ts`,
   `src/index.tsx`, `src/index.ts`, `main.tsx`, `main.ts`.
2. **Module graph** (`iron_ts::modules`, below): the entry and every file it imports become one source text.
3. **`tsc --noEmit -p .`** when a `tsconfig.json` exists and `tsc` is found (`node_modules/.bin/tsc` in the game folder or any parent,
   else on the `PATH`). A missing `tsc` is only a note; a failing one stops the build. `--no-tsc` skips it.
4. **Compile.** The bundle is compiled with all engine manifests (the lists of `iron_native::manifests()` plus `Ui`), so lowering
   and code generation errors show up now. Errors name `file:line` of the original file. `--aot` also writes `dist/game.o` and
   `dist/game_bindings.rs` (the object file for a release build that links the game into the executable).
5. **Stylesheets.** `import "./x.css"` lines are collected in import order into `dist/game.css` (`css.rs`): `@import` is
   inlined; every `url()` is rewritten: a file up to 32 KiB becomes a `data:` URL, a larger one is copied to `dist/` as
   `name-<hash>.ext` and the URL is relative to `game.css`. A `url()` that does not exist stays as written, with a warning (the
   Charm font is not in the repository).
6. **Assets.** `.meta` files are made for new assets (`assets.rs`; existing ones and their GUIDs are kept; a moved file keeps its
   GUID when the move is unambiguous). Every string literal of the program that names an existing asset file under the game folder
   (`Texture.load("assets/x.png")`, `Model.load`, `loadFont`, `scenes/*.scene.json`, sounds) is copied with its `.meta` to the same
   relative path in `dist/`. Paths follow `iron_core::resolve_asset` (relative, no `..`, no drive letters or schemes). `--cook` then
   runs `iron <game> --cook` (the engine's cooker) when `target/{release,debug}/iron_app` is found.
7. **`dist/game.irbundle`**, written atomically (temporary file and rename).

## `game.irbundle`

JSON, `format` 1:

```json
{ "format": 1, "title": "...", "company": "...", "identifier": "...", "entry": "src/main.tsx",
  "source": "<one TypeScript/TSX text>", "files": ["src/tween.ts", "src/main.tsx"],
  "css": "game.css", "assets": ["assets/coin.png"] }
```

`source` has no `@iron/*` or `react` imports: the host compiles it with the engine manifests and the `Ui` manifest. A host reads
`format`, `entry`, `source`, `title`, `company`, `identifier`, `css` (relative to `dist/`, or `null`) and `assets`.

## Module graph (`tools/iron_ts/src/modules.rs`)

`iron_ts::modules::bundle(entry)` (and `compile_program`) writes the files, dependencies first, into one text, by *editing byte
ranges* of the originals (comments and JSX stay as written; erased statements keep their newlines):

- `import ... from "@iron/..."` and `"react"` are erased. Names come from the manifests and the ambient prelude; an aliased import
  (`import { Color as C }`) is renamed to the real name (`Color`). Default and namespace imports of libraries are erased and the name
  is left as it is.
- Relative imports (`./x`, `../y.ts`, `./dir` -> `dir/index.ts`; `.js` specifiers try `.ts`/`.tsx`) are followed, including
  `export { a as b } from`, `export * from` and chains of re-exports; an import is renamed to the final name of the declaration it
  reaches. `import type` is handled like `import`.
- `export` is dropped from every file except the entry (the entry's exports are the program's). `export default <expression>` becomes
  `const __default_<path> = ...`.
- Top-level names that two files declare are renamed in the dependency (`helper` -> `helper_2`); the entry's names win.
  Shorthand properties keep their key (`{ count }` -> `{ count: n_2 }`).
- `import "./x.css"` is not read; it is listed in `Bundle::css` for the build tool.
- Not supported (an error naming the file): `import * as ns` of a project file, `export * as ns`, anonymous `export default function`
  or `class`, bare package imports other than `react` and `@iron/*`.
- `Bundle::locate_in(message)` rewrites `line N` of an error as `file:line`.

## Type declarations (`iron-build types`)

`types/iron.d.ts` has `declare module "@iron/engine" { ... }` generated from the manifests (`iron_ts::types_dts`: handle and value
classes with constructors and fields, statics, methods, properties, namespaces; callbacks as function types; no doc comments, the
manifests do not carry them) followed by the ambient declarations of the compiler (`tools/iron_ts/src/lower/ambient.d.ts`: `react`,
`@iron/react`, `JSX.IntrinsicElements`, `IronStyle`, ...), verbatim, when that file exists. A game's `tsconfig.json` lists `types` in
`include`; with that, `tsc` needs no `node_modules`, no `@iron/*` packages and no `@types/react`.

## The dev loop (`iron-build dev`)

The watcher polls modification times (200 ms; no dependency) of everything in the game folder except `dist`, `node_modules`,
`target`, `types` and dot folders, waits 100 ms for a second save, rebuilds, and writes `dist/.iron-reload` **after**
`dist/game.irbundle`:

```json
{ "seq": 7, "ok": true }
{ "seq": 8, "ok": false, "error": "text of the build error" }
```

The app watches that one file; a new `seq` means reload `game.irbundle` (on `ok: false` log the error and keep the old code; the old
bundle is untouched). Hot reload keeps state the way the QuickJS reload does (scene save/restore, `onReload`).

## Tests

`M9-TS-1800..1806` module graph, `1810` declarations, `1830..1831` `.meta` files, `1840` CSS bundling, `1850..1852` config and a
full build, `1860..1861` the dev status file and the change fingerprint. `bun run check:examples` (`iron-build check examples`)
builds all 19 examples and prints a table; failures there are missing language or engine features, listed in
`docs/research/iron-examples-on-iron-ts.md`.
