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
| `package <game> [flags]` | One executable for the game: `dist/<exe>.exe` (see "Packaging a single executable"). |

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

## Packaging a single executable (`iron-build package`)

```
iron-build package <game> [--host iron_app.exe] [--out file] [--no-build] [--no-tsc]
    [--exe name] [--title t] [--company c] [--version v] [--copyright c] [--description d]
    [--icon file.png|.ico] [--sign-cmd "signtool sign /fd sha256 ... {exe}"]
```

It builds the game (unless `--no-build`), then writes `<game>/dist/<exe>.exe` (no extension on other systems): a copy of the
engine host (`iron_app`) with the game's **icon and version info** written into its PE resources (Windows hosts only) and the game's
**files appended as a payload**. The result runs from any folder with no loose files; `iron_app <game folder>` is unchanged.

**The host** is `--host`, else `$IRON_HOST`, else the newest of `iron_app` next to `iron-build` and `target/{release,debug}/iron_app`
found going up from the game folder and the current folder (release first). A host that already carries a payload is stripped first, so
a packaged exe can be re-packaged. A debug host is ~77 MB, a release one is much smaller: ship release.

**Metadata** (command line, then `package.json` `iron.<field>`, then the package.json field of the same meaning, then the default):

| Field | `iron.` key / flag | Falls back to | Used for |
|---|---|---|---|
| title | `title` / `--title` | `name` | window title, `ProductName`, settings folder |
| exe | `exe` / `--exe` | the title (made file-name safe; `.exe` is dropped) | file name, `InternalName`, `OriginalFilename` |
| company | `company` / `--company` | `author` | `CompanyName`, settings folder |
| version | `version` / `--version` | `version` of package.json, else `0.0.0` | `FileVersion`, `ProductVersion` (`1.2.3` -> 1.2.3.0; a `-pre` suffix is dropped from the numbers, kept in the text) |
| copyright | `copyright` / `--copyright` | `Copyright (c) <company>` (empty without a company) | `LegalCopyright` |
| description | `description` / `--description` | `description` of package.json, else the title | `FileDescription` (what Task Manager shows) |
| icon | `icon` / `--icon` | none (the host's own icon stays) | exe icon; window and taskbar icon at run time |
| identifier | `identifier` | `dev.iron.<name>` | `Application.identifier` |

The icon is a `.png` (any size; centred on a transparent square when not square) or an `.ico`. A PNG is converted in-process
(`icon.rs`, `image` crate) to an `.ico` of 16, 32, 48 and 256 px, and `pe.rs` writes it with the `editpe` crate (the whole `.rsrc`
section is parsed and rebuilt; no `rc.exe`, no other tool). The version info is a VS_VERSIONINFO block with the string table
`040904b0` (US English, Unicode) and the matching `Translation`. The same icon file goes into the payload (`iron-icon.png|ico`) and
`iron.icon` of the payload's `package.json` points to it, so the window and taskbar icon come from the same file.

**The payload** (`iron_core::payload`, format version 1) is appended after the host (and after the resource edit):

```
[host exe][entry data...][table of contents][32-byte trailer]
trailer: "IRONPAYL" | version u32 | reserved u32 | payload_len u64 | toc_len u64   (little endian)
toc:     count u32; per entry: path_len u16, path, offset u64, stored_len u64, raw_len u64, method u8 (0 stored, 1 deflate), crc32 u32
```

Entries are deflate-compressed (`flate2`, stored when it does not help or under 64 bytes); every one has a CRC-32 that is checked on
read. Paths are relative, `/` separated and checked (`payload::safe_path`: no `..`, empty or `.` parts, drive letters, backslashes);
a table with an unsafe path is refused. What goes in (paths relative to the game folder, so the extracted folder looks like the game
folder):

- `dist/`: `game.irbundle`, `game.css`, the files the CSS references (hashed fonts and images). Not `game.o`, `game_bindings.rs`,
  `*.exe` or dot files.
- Every asset the bundle names (`assets` in `game.irbundle`), `iron.startScene`, and everything under `iron.include` (a list of
  files or folders; **default `["assets"]`**, the whole folder, because the build finds assets only by string literals and misses
  paths built at run time or named inside CSS text), each with its `.meta`. The game folder's copy is used, not `dist/`'s.
- `package.json`, rewritten with the resolved metadata (so `Application`, the window, the icon and the settings folder agree with
  the exe's version info), and `iron-icon.*`.

**How the exe finds its payload.** At startup `iron_app` (`payload.rs`, called from `parse_args` in `main.rs`; not in `iron_editor`)
opens its own executable and looks for the trailer at the end of the file (a plain `iron_app` has none: `Ok(None)`, normal start). If
the file is a signed PE it also looks right before the Authenticode certificate table (data directory 4, 8-byte aligned), so a payload
survives signing. The payload's table of contents is hashed into an id; the files are extracted once into
`<per-user cache>/Iron/games/<exe name>-<id>/` (`%LOCALAPPDATA%`, `~/Library/Caches`, `$XDG_CACHE_HOME` or `~/.cache`) beside a
`.iron-extracted` marker (extracted to a temporary folder and renamed, so a crash or a second instance never sees half a game), and
the game runs with that folder as its game folder: `--screenshot` and `--frames` still apply, any folder argument is ignored. A new
build has a new id, extracts again and removes the older folders of the same exe name. An unchanged game starts at once.

*Why a cache folder and not an in-memory file system:* about a hundred places in `iron_app`, `iron_render` and `iron_audio` read
textures, models, sounds, fonts, scenes and CSS from disk (and the asset watcher and the cooker walk the folder). One real folder keeps
every one of them unchanged; the costs are the first-start extraction (a few
files, milliseconds) and the disk space of a second copy. An in-memory layer under `resolve_asset` stays possible later.

`--sign-cmd "<command>"` runs after the file is complete: `{exe}` is replaced by the quoted path and the command goes through
`cmd /S /C` (Windows) or `sh -c`. A failing command fails the package. Nothing is signed by Iron itself; see
`docs/research/iron-code-signing.md` (order: resources and payload first, sign last).

**Not done** (also in `tools/iron_ts/TODO.md`): macOS `.app` bundle with `Info.plist` and `.icns`, Linux `.desktop` file and icon
theme entry, the X11/Wayland window class (winit's default is used), the Windows `AppUserModelID` for taskbar grouping, a Windows
application manifest (DPI awareness, long paths), cooked assets (`.iron/cooked`) in the payload, an optional in-memory file system.

### The same bundle on iOS and Android (plan only)

Nothing here is built yet. The inputs are the same: `game.irbundle` (or the `--aot` object `dist/game.o` + `game_bindings.rs`),
`game.css`, assets and the metadata of the table above.

- **iOS** forbids writable-executable memory, so the Cranelift JIT that `iron_app` uses to compile `game.irbundle` at start cannot run:
  the game must be built with `--aot` (an object file for `aarch64-apple-ios`, linked into the app binary with the engine host as a
  static library) and the bundle's source is not shipped at all. The app is an `.app` folder (host binary, `dist/`, `assets/`,
  `Info.plist` from the metadata: `CFBundleName` = title, `CFBundleIdentifier` = identifier, `CFBundleShortVersionString` = version,
  an asset catalog from the icon), assets read from the app bundle (read-only, so no extraction), signed with a provisioning profile and
  uploaded as an `.ipa`. Needs a Mac for the linker, `codesign` and Xcode's tools, and an Apple developer account.
- **Android** takes the host as a `cdylib` per ABI (`arm64-v8a`, ...) in an APK or AAB (`NativeActivity` or GameActivity glue from
  `winit`'s android backend), with the `--aot` object linked in for release (JIT is allowed there, so a debug build can still compile
  the bundle at start). Assets go into the APK's `assets/` (read through the NDK `AAssetManager`: the asset reader needs a small
  seam, since there is no folder to extract to cheaply), the icon into `mipmap-*` densities, the identifier is the `applicationId`
  and the version fills `versionName` / `versionCode`. Packaging (`aapt2`, zip alignment, `apksigner` or bundletool) can be done
  from Rust or by calling the SDK tools; signing needs a keystore.
- Both need the `--aot` path to cover all of the engine API first (today the examples run through the JIT) and a mobile windowing and
  input layer in `iron_platform`.

## Tests

`M9-API-1000..1005` the payload format (round trip, no trailer, unsafe paths, damage and duplicates, extraction and id, a payload before
a certificate table), `M9-API-1010..1011` the runtime side (`iron_app/src/payload.rs`), `M9-TS-2400..2402` metadata, `2410..2412` file
collection and a full package with the sign hook, `2420..2421` icon conversion, `2430` the PE resource round trip, `2440` (ignored
unless asked: `cargo test -p iron_build -- --ignored standalone`, needs a built host and a GPU) the standalone run.

`M9-TS-1800..1806` module graph, `1810` declarations, `1830..1831` `.meta` files, `1840` CSS bundling, `1850..1852` config and a
full build, `1860..1861` the dev status file and the change fingerprint. `bun run check:examples` (`iron-build check examples`)
builds all 19 examples and prints a table; failures there are missing language or engine features, listed in
`docs/research/iron-examples-on-iron-ts.md`.
