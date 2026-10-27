# raindrop-demo

A port of MiniGameEngine's RaindropDemo. Rain lands as rings of soft discs that spread out and dissolve, over looping rain music.
Each disc is a `GameObject` with a `SpriteRenderer`, grown and faded by its own `Droplet` component. The weather (Drizzle, Rain,
Stormy) sets how often drops land. The only UI is a text HUD in React.

APIs used: `Sprite.create`, `Texture.create` (a procedurally generated texture), `SpriteRenderer` (`sprite`, `color`, `sortingOrder`),
`GameObject` creation and `destroy()`, `Component` hooks `onAwake` / `onUpdate`, `audio.load(..., { stream: true })`, `Sound.play({
loop: true })`, `Playback.pause/resume`, `input`, `Screen.fullScreen`, `Camera` (orthographic), React HUD.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/raindrop-demo
```

then from the repo root:

```bash
cargo run -p platypus_app -- examples/raindrop-demo
```

Or package it with `platypus-build package ../examples/raindrop-demo`. A screenshot run is
`cargo run -p platypus_app -- examples/raindrop-demo --screenshot out.png --frames N` (the default weather is Stormy, so a few
dozen frames already show drops). The interactive window may crash some GPUs; screenshots are safest.

## Structure

| File | What |
|---|---|
| `project.json` | `name` "raindrop-demo", `title` "Raindrop Demo". |
| `tsconfig.json` | Base config, `jsx: "react-jsx"`, a `paths` entry, includes `src`. |
| `src/main.tsx` | 264 lines: constants, colour helpers, `Droplet`, `RaindropDemo`, HUD. |
| `assets/rains-falling-on-my-head-loop.mp3` (+ `.meta`) | The looping music, streamed. |

## How it works

**Units.** As in the original, sizes are in form pixels (1008 x 729) and converted with `UNITS_PER_PX = 10 / FORM_H`; the camera's
orthographic size is 5, so drops look the same at any window size.

**Weather as a bit mask.** The original's trick is kept: a drop lands when `roll & weather == weather`.

```ts
enum Weather {
  Drizzle = 7,
  Rain = 3,
  Stormy = 0,
}
```

`RaindropDemo.onUpdate` accumulates time and calls `roll()` every `CHECK_INTERVAL = 0.025` s. `roll()` draws `randInt(0, 256)`, and
returns unless `(chance & this.weather) === this.weather`.

**Drop kinds.** One in ten drops is large (radius 100-200), slow (`movement` 1.6) and dark (`modifyBrightness(BASE_COLOUR, 0.25)`),
and lands where the last drop did. The rest are radius 20-100, `movement` 0.8, and land at a random position.

**A ring of discs.** `spawn()` makes `DROPLETS = 8` objects at the same spot. Disc `i` lasts `movement + rate * i` seconds and is
dimmer by `(DROPLETS - i) / DROPLETS`, with `order: i` as its sorting order. Components are given their settings through
`addComponent`'s options:

```ts
go.addComponent(Droplet, { maxRadius: radius, duration, colour: shade, order: i });
```

**The sprite.** `getDiscSprite()` builds one 256 x 256 white disc texture with an antialiased edge, once, and every ripple shares it:

```ts
discSprite = Sprite.create(Texture.create(n, n, px), null, new Vector2(0.5, 0.5), n);
```

**Droplet.** In `onAwake` it adds the `SpriteRenderer`. In `onUpdate`, progress `t = elapsed / duration` sets the scale (the disc is one
unit at scale 1, so scale equals diameter) and fades the alpha by `1 - t`; at `t >= 1` it calls `this.gameObject.destroy()`.

**Music.** `onAwake` loads `assets/rains-falling-on-my-head-loop.mp3` as a streamed sound and plays it looping at once; `M` pauses or
resumes the playback.

**HUD.** `Hud` is a `pointerEvents: "none"` box. `RaindropDemo` calls `notify()` on changes and every 0.05 s; `useStats()` subscribes
through a module-level `listeners` set. The weather name is coloured with a red-to-blue pulse (`textPhase`).

## Controls

| Input | Effect |
|---|---|
| Left or right click | Next weather (Drizzle, Rain, Stormy) |
| 1, 2, 3 | Drizzle, Rain, Stormy |
| M | Mute / unmute the music |
| F11 | Toggle fullscreen |

## Things to try and gotchas

- Make the rings follow the mouse (`input.mousePosition`, `camera.screenToWorldPoint` as in `splines`).
- Replace the disc with a ring texture, or add a splash sound per drop with `audio.load(...).play()`.
- Pool the droplet objects instead of creating and destroying up to 8 per drop (Stormy rolls every 25 ms, so this makes up to
  8 objects every roll).
- Use the same ripple pattern for UI feedback or sonar pings.
- Gotcha: procedural textures use `Texture.create(w, h, Color[])`, which fills a plain array of 65,536 `Color`s; build it once.
- Gotcha: the HUD talks to the component through module-level `demo` and `listeners` (React and components are separate worlds).
- Gotcha: `WEATHER_NAME: Record<Weather, string>` uses computed enum keys; the source shows that working in this subset.

Related examples: sine-circle-demo, shapes, arcade (React HUD over a game)
