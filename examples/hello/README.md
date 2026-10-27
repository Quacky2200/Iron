# hello

The smallest Iron game: one component that logs a greeting once and "tick" every frame. It also shows
how a game splits code into modules: `src/main.ts` imports a function from `src/modules/greet.ts` by its
bare name. There is no UI and no 3D scene; it is a check that the build and the component lifecycle work.

APIs used: `Component` (`onStart`, `onUpdate`), `register({ Hello })`, `console.log`, and the
`modules/` folder import rule of the module graph (`docs/iron-build.md`, "Module graph").

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/hello
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/hello
```

A screenshot run (`cargo run -p iron_app -- examples/hello --screenshot out.png --frames 30`) renders an empty
window and exits; it is the safest way to run on a machine whose GPU is unstable with an open window. To
read the output, start `iron_app` from a terminal: the log goes to stderr (and to `Player.log` next to the
game's `prefs.json` for a normal run). You can also build one executable with
`iron-build package ../examples/hello` (see `docs/iron-build.md`, "Packaging a single executable").

## Structure

| File | What |
|---|---|
| `project.json` | `{ "name": "hello", "title": "Iron - Hello" }`: only `name` is required; everything else takes its default |
| `tsconfig.json` | Extends `../../tsconfig.base.json`; `paths` maps bare names to `./src/modules/*` for `tsc` |
| `src/main.ts` | The entry (a `.ts` file, no JSX): the `Hello` component and `register` |
| `src/modules/greet.ts` | `greeting(name, times)`, a module of this game |

No assets, scenes or CSS.

## How it works

1. `iron-build` finds the entry (`src/main.tsx`, `src/main.ts`, ...), follows its imports and compiles the
   whole module graph with iron_ts. The import below resolves to `src/modules/greet.ts` because bare
   imports are files of the `modules/` folder next to the entry:

```ts
import { greeting } from "greet";
```

2. `Component`, `register` and `console` are globals; the example imports no engine names.
3. `register({ Hello })` makes the class known to the engine. A registered class is spawned once at start
   (the default; `{ spawn: false }` turns that off, as `arcade` does for components it creates itself).
4. The lifecycle hooks run on the spawned instance:

```ts
class Hello extends Component {
  override onStart() {
    console.log(greeting("world", 1));
  }

  override onUpdate(_dt: number) {
    console.log("tick");
  }
}
```

`onStart` prints `hello world` once. `onUpdate(dt)` runs every frame with the time since the last frame in
seconds, so "tick" is printed once per frame (a lot of lines; remove it when you reuse the file).

5. `greeting` builds a string with a template literal and `repeat`, clamping the count to at least 1:

```ts
return `hello ${name}`.repeat(Math.max(1, times));
```

## Controls

None.

## Things to try

- Pass `3` to `greeting` and see the string repeat.
- Add a second module `src/modules/math2.ts` and import it as `"math2"`.
- Use `input.getKeyDown("Space")` in `onUpdate` and log only on a press.
- Add a field (`speed = 5`) and set it when you add the component from code, as in
  `addComponent(Hello, { speed: 9 })` (see `animation`, which passes `{ animator }` to `Driver`).
- Create a `GameObject` in `onStart` and give it a `MeshRenderer` to get a first 3D object (see `cornell`).

## Gotchas

- Relative imports (`./foo`) work too; the `modules/` folder is only for bare names.
- The game is compiled TypeScript, not run in a JS engine: only the supported subset compiles (see
  `docs/iron-ts.md`). This example uses nothing outside it.
- `tsc` (optional type check) needs the generated `types/iron.d.ts`; use `iron-build check --tsc` or
  `iron-build types` rather than plain `tsc`, because the engine names are globals.

Related examples: text, boxes, scenes
