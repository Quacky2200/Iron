// Rules of the game simulation, checked without the engine. `bun test` runs these.
import { describe, expect, test } from "bun:test";

import { addPlayer, buildWalls, createState, damageZombie, step, zombieSpeed, type PlayerInput, type SimState, startLevel, type Walls } from "./sim.ts";

const W = 1022;
const H = 750;

/** No walls at all. */
const open: Walls = buildWalls(W, H, Array.from({ length: H }, () => [W]));
/** Solid walls everywhere. */
const solid: Walls = buildWalls(W, H, Array.from({ length: H }, () => [0, W]));
const maps = (w: Walls) => () => w;

// Aim straight right of the player centre (the centre is at y 375), so the angle is 0.
const idle: PlayerInput = { moveX: 0, moveY: 0, aimX: 900, aimY: 375, fire: false };

/** Steps the game for a whole number of 60 Hz frames. */
function run(state: SimState, seconds: number, input: PlayerInput[] = [idle], walls = open) {
  const frames = Math.round(seconds * 60);
  for (let i = 0; i < frames; i++) step(state, input, 1 / 60, maps(walls));
}

function soloState(): SimState {
  const state = createState(7, 6);
  addPlayer(state);
  startLevel(state, () => open);
  return state;
}

describe("walls", () => {
  test("a baked mask keeps the runs of walls and open floor", () => {
    const walls = buildWalls(4, 2, [
      [1, 1, 2],
      [0, 4],
    ]);
    expect(walls.cells[0]).toBe(0);
    expect(walls.cells[1]).toBe(1);
    expect(walls.cells[2]).toBe(0);
    expect(walls.cells[3]).toBe(0);
    expect(walls.cells[4]).toBe(1);
  });
});

describe("solo play", () => {
  test("zombies spawn in single player (the original never spawned one)", () => {
    const state = soloState();
    run(state, 0.5);
    expect(state.zombies.length).toBeGreaterThan(0);
  });

  test("a level's zombie budget grows by 3 to 5 each level", () => {
    const state = soloState();
    expect(state.zombiesMax).toBeGreaterThanOrEqual(8);
    expect(state.zombiesMax).toBeLessThanOrEqual(10);
  });
});

describe("movement", () => {
  test("a player moves 400 px a second with one key held", () => {
    const state = soloState();
    const p = state.players[0]!;
    const x0 = p.x;
    run(state, 0.5, [{ ...idle, moveX: 1 }]);
    expect(p.x - x0).toBeGreaterThan(195);
    expect(p.x - x0).toBeLessThan(205);
  });

  test("a wall stops the player", () => {
    const state = soloState();
    const p = state.players[0]!;
    const x0 = p.x;
    run(state, 0.5, [{ ...idle, moveX: 1 }], solid);
    expect(p.x).toBe(x0);
  });
});

describe("weapons", () => {
  test("a press fires one pistol round and turns the aim toward the mouse", () => {
    const state = soloState();
    const p = state.players[0]!;
    const before = p.gun.rounds;
    step(state, [{ ...idle, fire: true }], 1 / 60, maps(open));
    expect(p.gun.rounds).toBe(before - 1);
    expect(state.bullets.length).toBe(1);
    expect(p.angle).toBeCloseTo(0, 0);
  });

  test("holding fire with the pistol does not repeat", () => {
    const state = soloState();
    const p = state.players[0]!;
    const before = p.gun.rounds;
    run(state, 1, [{ ...idle, fire: true }]);
    expect(p.gun.rounds).toBe(before - 1);
  });

  test("an empty clip starts a reload that blocks shooting", () => {
    const state = soloState();
    const p = state.players[0]!;
    p.gun.rounds = 16;
    p.bulletsInClip = 1;
    step(state, [{ ...idle, fire: true }], 1 / 60, maps(open));
    expect(p.gun.rounds).toBe(15);
    expect(p.reloadTimer).toBeGreaterThan(0);
    expect(state.events.some((e) => e.kind === "reload")).toBe(true);
  });
});

describe("zombies", () => {
  test("a hit takes health and scores 10; a kill scores 80 once the fade ends", () => {
    const state = soloState();
    const p = state.players[0]!;
    const z = {
      id: 999,
      x: 600,
      y: 300,
      angle: 0,
      hp: 10,
      maxHp: 10,
      dying: false,
      fadeTimer: 0,
      attackTimer: null,
      timeout: 0,
      targetId: p.id,
      dead: false,
    };
    state.zombies.push(z);
    damageZombie(state, z, 4, p);
    expect(z.hp).toBe(6);
    expect(p.score).toBe(10);
    damageZombie(state, z, 6, p);
    expect(z.dying).toBe(true);
    run(state, 0.5);
    expect(p.kills).toBe(1);
    expect(p.score).toBe(10 + 10 + 80);
  });

  test("four zombie hits kill a player with 100 health, and the score is floored at 0", () => {
    const state = soloState();
    const p = state.players[0]!;
    p.score = 50;
    state.zombies.push({
      id: 999,
      x: p.x,
      y: p.y,
      angle: 0,
      hp: 10,
      maxHp: 10,
      dying: false,
      fadeTimer: 0,
      attackTimer: 0,
      timeout: 0,
      targetId: p.id,
      dead: false,
    });
    run(state, 3, [idle]);
    expect(p.dead).toBe(true);
    expect(p.score).toBe(0);
    expect(state.gameOver).toBe(true);
    expect(state.events.length).toBe(0);
  });
});

describe("levels", () => {
  test("the next level starts when the last zombie of this one dies, with 10 x level HP", () => {
    const state = soloState();
    const p = state.players[0]!;
    state.zombiesLeft = 0;
    state.zombies = [];
    const z = {
      id: 999,
      x: 600,
      y: 300,
      angle: 0,
      hp: 10,
      maxHp: 10,
      dying: false,
      fadeTimer: 0,
      attackTimer: null,
      timeout: 0,
      targetId: p.id,
      dead: false,
    };
    state.zombies.push(z);
    state.levelEnding = false;
    damageZombie(state, z, 10, p);
    run(state, 0.5);
    expect(state.level).toBe(2);
  });
});

describe("powerups", () => {
  test("touching an ammo powerup adds 320 rounds", () => {
    const state = soloState();
    const p = state.players[0]!;
    const before = p.gun.rounds;
    state.powerups.push({ id: 1000, type: "ammo", x: p.x, y: p.y, age: 0 });
    step(state, [idle], 1 / 60, maps(open));
    expect(p.gun.rounds).toBe(before + 320);
    expect(state.powerups.length).toBe(0);
  });
});

describe("zombie speed", () => {
  test("zombies speed up a little each level until level 3, then hold", () => {
    expect(zombieSpeed(1)).toBe(1);
    expect(zombieSpeed(2)).toBeCloseTo(1.15);
    expect(zombieSpeed(3)).toBeCloseTo(1.3);
    expect(zombieSpeed(9)).toBeCloseTo(1.3);
  });
});
