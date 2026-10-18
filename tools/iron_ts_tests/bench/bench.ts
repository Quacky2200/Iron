// The benchmark scenario for sim.ts: one player auto-firing an MP5 (never dies, never runs dry),
// ten zombies alive, 50 parked bullets (plus the MP5's own). `bench(n)` steps n times at 1/60 s and returns a checksum
// so every engine can be checked against the others. Run with `bun bench.ts` or bundled into
// QuickJS (`__bench`).
import { addPlayer, buildWalls, createState, startLevel, step, type PlayerInput, type SimState } from "../fixtures/sim.ts";

const W = 1022;
const H = 750;

export function makeState(): SimState {
  const open = buildWalls(W, H, Array.from({ length: H }, () => [W]));
  const state = createState(7, 6);
  const p = addPlayer(state);
  startLevel(state, () => open);
  p.health = 1e9;
  p.gun = { name: "MP5", rounds: 1e9, clip: 1e9, damage: 40, auto: true };
  state.zombiesLeft = 1e9;
  // 50 bullets that stay where they are (the MP5's own bullets last a fraction of a second).
  for (let i = 0; i < 50; i++) {
    state.bullets.push({ id: 5000 + i, x: 100 + i * 10, y: 100, dirX: 0, dirY: 0, targetId: null, damage: 0, ownerId: 1, life: 1e9 });
  }
  return state;
}

export function checksum(s: SimState): number {
  let sum = s.level * 1000 + s.players[0]!.score + s.players[0]!.kills * 7 + s.bullets.length * 13 + s.zombies.length * 17 + s.seed;
  for (const z of s.zombies) sum += z.x + z.y;
  return sum;
}

export function bench(steps: number): number {
  const open = buildWalls(W, H, Array.from({ length: H }, () => [W]));
  const state = makeState();
  const inputs: PlayerInput[] = [{ moveX: 0, moveY: 0, aimX: 900, aimY: 375, fire: true }];
  const walls = () => open;
  for (let i = 0; i < steps; i++) step(state, inputs, 1 / 60, walls);
  return checksum(state);
}

(globalThis as { __bench?: (n: number) => number }).__bench = bench;

if (typeof process !== "undefined" && process.argv[1]?.endsWith("bench.ts")) {
  const steps = Number(process.argv[2] ?? 10000);
  const t = performance.now();
  const sum = bench(steps);
  const ms = performance.now() - t;
  console.log(JSON.stringify({ engine: "bun", steps, ms, checksum: sum }));
}
