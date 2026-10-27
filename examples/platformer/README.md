# platformer (Fox Run)

A small 2D side-scrolling platformer. An orthographic camera follows a fox (a `Rigidbody2D` with a
`CapsuleCollider2D` and a frame-animated `SpriteRenderer`) across tiled ground, stone platforms and a stack of
crates that tumble. Coins are trigger colliders that burst into sparks (`ParticleSystem`) when collected, and a
React HUD counts them. It is a good first look at the 2D side of the engine: sprites, 2D physics, particles and a
HUD over the world, all in one 246-line file.

Engine features used: `Texture.load`, `Sprite.create`, `SpriteRenderer` (`sortingOrder`, `flipX`, `color`),
`Rigidbody2D`, `BoxCollider2D`, `CircleCollider2D`, `CapsuleCollider2D`, `EdgeCollider2D`,
`onCollisionEnter2D` / `onTriggerEnter2D`, `Camera` (orthographic), `ParticleSystem`, `Texture.create` (pixels made
in script), `input.getAxis` / `getButtonDown`, `Application` and `SystemInfo`, `createRoot` with React hooks.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/platformer
```

Then, from the repo root:

```bash
cargo run -p platypus_app -- examples/platformer
```

A single-file executable can be made with `platypus-build package ../examples/platformer` (it uses `icon` from
`project.json`; see `docs/platypus-build.md`). A screenshot run is
`platypus examples/platformer --screenshot out.png --frames N`. It is the safest way to check the result: the
interactive window has crashed some GPUs.

## Structure

| File | What |
|---|---|
| `src/main.tsx` | Everything: sprites, the `Fox`, `Coin`, `CameraFollow2D` and `Level` components, and the `Hud` |
| `project.json` | `name: platformer`, `title: Fox Run`, `icon: assets/icon.png` |
| `tsconfig.json` | Extends `tsconfig.base.json`, `jsx: react-jsx` |
| `assets/tiles.png` | 16x16 tiles in a row: grass, dirt, crate, stone (indices 0-3) |
| `assets/fox.png` | 5 frames of 16x16: four run frames and an idle frame |
| `assets/coin.png` | 4 spin frames of 16x16 |
| `assets/icon.png` | The exe/window icon |

The `.png.meta` files hold the asset GUIDs and import settings (written by `scripts/assets.ts`).

## How it works

**Sprites.** Textures are loaded with point filtering and no mipmaps so the pixel art stays crisp. Each frame is
a `Sprite` cut from a sheet at 16 pixels per unit:

```ts
const PPU = 16;
const tiles = Texture.load("assets/tiles.png", { filterMode: "point", mipmaps: false });
const tile = (i: number) => Sprite.create(tiles, new Rect(i * 16, 0, 16, 16), new Vector2(0.5, 0.5), PPU);
```

**Setup.** `register({ Level })` makes `Level` a component that is spawned when the game starts. Its `onStart`
builds everything: a camera (`orthographic: true, orthographicSize: 5`), hills and clouds drawn from a 256x256
circle texture made in script (`circleTexture`, via `Texture.create(n, n, px)`), tiled grass and dirt, an
`EdgeCollider2D` for the ground line, end walls (`BoxCollider2D`), four stone platforms, nine crates, eight
coins and the fox. Draw order comes from `sortingOrder` (hills -20, clouds -10, crates 1, coins 2, fox 5), not from
distance.

**The fox.** `Fox.onUpdate` reads input and sets the body velocity; the jump is deferred to `onFixedUpdate` so it
applies on a physics tick. Ground contact is counted with enter/exit callbacks, ignoring coins:

```ts
onCollisionEnter2D(c: Collision2D) {
  if (c.gameObject.name !== "coin") this.ground++;
}
```

A run cycle (`FOX[Math.floor(this.t * 12) % 4]`) plays while moving on the ground, and `flipX` follows the
direction. The body is `freezeRotation: true, gravityScale: 2.2, collisionDetectionMode: "continuous"`.

**Attract mode.** If there is no input for 3 seconds, or during the first 6 seconds (`Time.time < 6`), the fox
runs and jumps by itself, so a screenshot run shows it moving.

**Coins.** `Coin` cycles four frames, and in `onTriggerEnter2D` it adds to `score`, notifies the HUD through a
`Set` of listeners, spawns a one-shot `ParticleSystem` (`bursts: [{ time: 0, count: 24 }]`,
`blendMode: "additive"`, `stopAction: "destroy"`, a `colorOverLifetime` fade) and destroys itself.

**Camera.** `CameraFollow2D.onLateUpdate` eases towards the fox with an exponential lerp
(`1 - Math.exp(-dt * 4)`) and clamps x to the level (-5 to 31).

**HUD.** A React component keeps the score in `useState` and subscribes in `useEffect`; the full-screen box has
`pointerEvents: "none"`. It also shows `Application.productName`, `Application.version` and
`SystemInfo.operatingSystem` (from `project.json`).

## Controls

| Input | Action |
|---|---|
| Arrow keys / A, D | Run (the `Horizontal` axis) |
| Space | Jump (the `Jump` button), only while touching the ground |

## Things to try

- Add enemies: a component that patrols with `Rigidbody2D.velocity` and kills the fox on `onCollisionEnter2D`.
- Add a goal: stop `Fox` and show a React message when `score` reaches 8.
- Replace the hand-placed level arrays with a tile map string and loop over it (the code already builds ground and
  platforms from arrays).
- Add sound with `AudioSource` or `audio.play` when a coin is collected (see `physics` and `world`).
- Move the level into a scene file and load it with `SceneManager` (see `scenes`).

## Gotchas

- Components get engine values by name, not by import: the code uses `Sprite`, `Texture`, `Rigidbody2D`, `input`
  and so on as globals.
- Every array index uses `!` (`FOX[4]!`) because indexing is typed as possibly undefined.
- Object literals like `[x, y, n] as const` are used for tables of numbers.
- Score is shared with the HUD through module state plus a listener set, not through the engine.
- Ground detection by counting collisions is simple; it would miscount if the fox touched a wall and nothing else.

Related examples: world, physics, scenes, shapes, splines
