# launcher

A game launcher mock-up, to see whether a store / launcher front end works in Platypus's UI: the
examples as a cover-flow carousel or a grid, browsed with the mouse, the keys or a gamepad. It
launches nothing yet: Play shows a loading spinner, then an alert that says so.

## Running it

```bash
cd tools && cargo run -p platypus_build -- build ../examples/launcher
```

```bash
cargo run -p platypus_app -- examples/launcher
```

The list of games is made at build time by `gen-games.ts`, which `project.json`'s `prebuild` runs
with bun: each example's folder, title and description (from its `project.json`) go into
`src/games.gen.ts`, and its screenshot from `bun run screenshots`
(`tests/output/examples-<name>.png`) is copied into `content/shots/` with a background gradient
in its colours (`palette.ts`). Both outputs are git-ignored. An example without a screenshot
shows its title on a colour picked from its name; run `bun run screenshots` first for pictures.

## Using it

| Action | Mouse | Keys | Gamepad |
|---|---|---|---|
| Browse | Arrows, wheel, click a card | ← → (and ↑ ↓ in the grid), WASD | D-pad, left stick, bumpers |
| Open a game's details | Click the selected card | Enter, Space | A (Cross on PlayStation, B on Nintendo) |
| Back | Click outside | Esc, Backspace | B (Circle, A on Nintendo) |
| Options (favourites, details) | Right-click, the ⋯ button | C, the menu key | the top face button (Y, Triangle, X) |
| Carousel / grid | The view buttons top right | G | Click the right stick |
| All games / Favourites | The tabs | Tab, Shift+Tab | LT / RT |
| Menu (views, settings, Exit) | The ☰ button | Esc in the library | Start |

Button prompts follow the last device used: a key or the mouse shows keyboard keys and the
cursor; a pad's first button or stick push (or plugging one in) shows that pad's buttons, with
Xbox, PlayStation or Nintendo icons from its name, and hides the cursor. Favourites, the view and
the category are saved (`PlayerPrefs`). After three seconds on a game its picture fades in behind
everything.

## Command-line arguments

Arguments after `--` reach the launcher as `Application.args`, mostly for screenshot runs:

```bash
cargo run -p platypus_app -- examples/launcher --screenshot out.png --frames 40 -- --view grid --select clock --show context --pad playstation
```

| Argument | What |
|---|---|
| `--select <example>` | Start on that game |
| `--show details\|loading\|alert\|menu\|context` | Start on that screen |
| `--view grid` | Start in the grid view |
| `--category favourites` | Start on the favourites |
| `--favourites a,b` | Use these favourites instead of the saved ones |
| `--pad xbox\|playstation\|nintendo` | Start in gamepad mode with those icons (and keep them) |
| `--step-at <ms> [--steps n]` | Move on a game (n games, 60 ms apart) at that time |

## Files

| File | What |
|---|---|
| `gen-games.ts` | Lists the examples and copies their screenshots (bun, run by `prebuild`) |
| `palette.ts` | A screenshot's colours for the background gradient (used by `gen-games.ts`) |
| `src/main.tsx` | The launcher: carousel, grid, details, menus, input |
| `src/prompts.tsx` | Button prompts that follow the input device |
| `src/launcher.css` | The look |
