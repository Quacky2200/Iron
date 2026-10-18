// The game's rules, with no engine in them: players, zombies, bullets, powerups, levels and
// score, stepped by `step()`. The view (main.tsx) turns this state into GameObjects and feeds
// it input; the events it returns are for sound. Everything is in arena pixels (1024 x 750,
// y down, top-left origin), as in the original.
//
// The state holds a list of players, each driven by an input slot. Single player is one
// slot. Zombies chase the nearest living player, so a second player (from the network later)
// is one more entry in `players`, with no change here.

export const ARENA_W = 1024;
export const ARENA_H = 750;
export const PLAYER_SIZE = 72;
export const ZOMBIE_SIZE = 72;
const POWERUP_SIZE = 74;

/** Fixed step for zombie AI (the original ran it on a 20 ms timer). */
export const AI_TICK = 0.02;
const MAX_ZOMBIES_ALIVE = 10;
const MAX_POWERUPS = 2;

export interface Gun {
  name: "Pistol" | "MP5" | "Sniper";
  /** Rounds in reserve (the original's `Bullets`). */
  rounds: number;
  clip: number;
  damage: number;
  /** Keeps firing while the trigger is held, every `cadence` seconds. */
  auto: boolean;
}

export const GUN_START: Gun = { name: "Pistol", rounds: 750, clip: 15, damage: 10, auto: false };

export const GUN_PICKUPS = {
  MP5: { name: "MP5", rounds: 4200, clip: 120, damage: 40, auto: true },
  Sniper: { name: "Sniper", rounds: 420, clip: 5, damage: 120, auto: false },
  Pistol: { name: "Pistol", rounds: 520, clip: 25, damage: 35, auto: false },
} as const satisfies Record<string, Gun>;

export type PowerupType = "nuke" | "ammo" | "mp5" | "sniper" | "pistol" | "health";
const POWERUP_TYPES: PowerupType[] = ["nuke", "ammo", "mp5", "sniper", "pistol", "health"];

/** Input for one player for one step. */
export interface PlayerInput {
  /** -1..1 per axis; each axis is a key or stick. */
  moveX: number;
  moveY: number;
  /** Where the player is aiming, in arena pixels. */
  aimX: number;
  aimY: number;
  /** Trigger held this step. */
  fire: boolean;
}

export interface Player {
  id: number;
  x: number;
  y: number;
  angle: number;
  health: number;
  dead: boolean;
  gun: Gun;
  bulletsInClip: number;
  reloadTimer: number;
  /** Auto-fire cadence timer. */
  fireTimer: number;
  /** Semi-auto: one shot per press. */
  firedThisPress: boolean;
  regenTimer: number;
  score: number;
  kills: number;
  aim: { x: number; y: number };
  noAmmoTimer: number;
}

export interface Zombie {
  id: number;
  x: number;
  y: number;
  angle: number;
  hp: number;
  maxHp: number;
  dying: boolean;
  fadeTimer: number;
  /** Seconds until the next hit; `null` when not attacking. */
  attackTimer: number | null;
  timeout: number;
  targetId: number;
  /** Set once the death has been handled (points, drops, level end). */
  dead: boolean;
}

export interface Bullet {
  id: number;
  x: number;
  y: number;
  dirX: number;
  dirY: number;
  /** Zombie it is aimed at (chosen when fired). */
  targetId: number | null;
  damage: number;
  ownerId: number;
  /** Seconds left before it is removed. */
  life: number;
}

export interface Powerup {
  id: number;
  type: PowerupType;
  x: number;
  y: number;
  age: number;
}

export type SimEvent =
  | { kind: "shot"; gun: Gun["name"]; player: number }
  | { kind: "reload"; player: number }
  | { kind: "noAmmo"; player: number }
  | { kind: "hit"; zombie: number }
  | { kind: "zombieDied"; zombie: number }
  | { kind: "powerup"; type: PowerupType }
  | { kind: "levelStart"; level: number; map: number }
  | { kind: "gameOver" };

export interface SimState {
  /** The seed for `random`, advanced by every random pick (kept for lock-step play later). */
  seed: number;
  players: Player[];
  zombies: Zombie[];
  bullets: Bullet[];
  powerups: Powerup[];
  level: number;
  map: number;
  mapCount: number;
  /** Zombies still to spawn this level, and the total for this level. */
  zombiesLeft: number;
  zombiesMax: number;
  maxAmountOfZombies: number;
  spawnTimer: number;
  /** Nuke: no drops while it runs. */
  overrideTimer: number;
  /** Set when the last zombie of the level died, so the next level starts after its drop check. */
  levelEnding: boolean;
  nextId: number;
  aiAccumulator: number;
  aiTicks: number;
  gameOver: boolean;
  events: SimEvent[];
}

// ---- Random (mulberry32, from a seed in the state) ----

function nextRandom(state: SimState): number {
  state.seed = (state.seed + 0x6d2b79f5) >>> 0;
  let t = state.seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 0x100000000;
}

/** A random number in [min, max). */
const rand = (state: SimState, min: number, max: number) => min + nextRandom(state) * (max - min);
/** A random integer in [min, max) (the original's `Random.Next(min, max)`). */
const randInt = (state: SimState, min: number, max: number) => Math.floor(rand(state, min, max));

// ---- Geometry ----

const dist = (ax: number, ay: number, bx: number, by: number) => Math.hypot(ax - bx, ay - by);

/** Degrees 0..360, 0 = +x, clockwise on screen (y down). */
export function angleBetween(ax: number, ay: number, bx: number, by: number): number {
  const a = (Math.atan2(by - ay, bx - ax) * 180) / Math.PI;
  return (a + 360) % 360;
}

/** The arena's walkable mask: `walls(x, y)` is true for a wall. Built from a map's baked mask. */
export interface Walls {
  width: number;
  height: number;
  /** One byte per pixel: 1 for a wall. */
  cells: Uint8Array;
}

export function buildWalls(width: number, height: number, rows: number[][]): Walls {
  const cells = new Uint8Array(width * height);
  rows.forEach((runs, y) => {
    let x = 0;
    runs.forEach((count, i) => {
      if (i % 2 === 1) cells.fill(1, y * width + x, y * width + x + count);
      x += count;
    });
  });
  return { width, height, cells };
}

export function isWall(walls: Walls, x: number, y: number): boolean {
  const px = Math.floor(x);
  const py = Math.floor(y);
  if (px < 0 || py < 0 || px >= walls.width || py >= walls.height) return true;
  return walls.cells[py * walls.width + px] === 1;
}

/** The original's `WithinWindow`: strictly inside the 1022 x 750 playable area. */
const withinWindow = (x: number, y: number) => x > 0 && x < 1022 && y > 0 && y < 750;

/** The original's `WithinBounds`: inside the window and not in a wall. */
const withinBounds = (walls: Walls, x: number, y: number) => withinWindow(x, y) && !isWall(walls, x, y);

// ---- Setup ----

export function createState(seed: number, mapCount: number): SimState {
  const state: SimState = {
    seed,
    players: [],
    zombies: [],
    bullets: [],
    powerups: [],
    level: 0,
    map: -1,
    mapCount,
    zombiesLeft: 0,
    zombiesMax: 0,
    maxAmountOfZombies: 5,
    spawnTimer: 0.1,
    overrideTimer: 0,
    levelEnding: false,
    nextId: 1,
    aiAccumulator: 0,
    aiTicks: 0,
    gameOver: false,
    events: [],
  };
  return state;
}

export function addPlayer(state: SimState): Player {
  const player: Player = {
    id: state.nextId++,
    x: 476,
    y: 339,
    angle: 0,
    health: 100,
    dead: false,
    gun: { ...GUN_START },
    bulletsInClip: GUN_START.clip,
    reloadTimer: 0,
    fireTimer: 0,
    firedThisPress: false,
    regenTimer: 0,
    score: 0,
    kills: 0,
    aim: { x: 0, y: 0 },
    noAmmoTimer: 0,
  };
  state.players.push(player);
  return player;
}

/** Starts level 1 (or the next level): picks a map other than the last, and the zombie budget. */
export function startLevel(state: SimState, walls: (map: number) => Walls): void {
  state.level += 1;
  // The original's budget: 3 to 5 more zombies each level.
  state.maxAmountOfZombies += randInt(state, 3, 6);
  state.zombiesMax = state.maxAmountOfZombies;
  state.zombiesLeft = state.maxAmountOfZombies;
  state.levelEnding = false;
  state.spawnTimer = 0.1;

  // A new map, never the one just played (the original could never pick the last map).
  let map = randInt(state, 0, state.mapCount);
  if (state.mapCount > 1 && map === state.map) map = (map + 1) % state.mapCount;
  state.map = map;

  // Powerups outside the new map's walls go; the others stay.
  const mask = walls(map);
  state.powerups = state.powerups.filter((p) => withinBounds(mask, p.x + POWERUP_SIZE / 2, p.y + POWERUP_SIZE / 2));
  for (const p of state.players) {
    p.x = 476;
    p.y = 339;
  }
  state.events.push({ kind: "levelStart", level: state.level, map });
}

// ---- Input ----

function emit(state: SimState, event: SimEvent) {
  state.events.push(event);
}

/** Moves one player by its input, one pixel at a time so walls stop it cleanly. */
function movePlayer(p: Player, input: PlayerInput, dt: number, walls: Walls) {
  const sx = Math.sign(input.moveX);
  const sy = Math.sign(input.moveY);
  // 400 px/s along one axis. Both held: 4 px per axis, so about 377 px/s overall.
  const axisSpeed = sx !== 0 && sy !== 0 ? 400 / Math.SQRT2 : 400;
  let left = axisSpeed * dt;
  while (left > 0 && (sx !== 0 || sy !== 0)) {
    const s = Math.min(1, left);
    left -= s;
    const nx = p.x + sx * s;
    const ny = p.y + sy * s;
    // The original tested one sample point per direction (see its movement code).
    const blocked =
      (sx < 0 && isWall(walls, nx + 7, ny + 4)) ||
      (sx > 0 && isWall(walls, nx + PLAYER_SIZE - 7, ny + 7)) ||
      (sy < 0 && isWall(walls, nx + 7, ny + 6)) ||
      (sy > 0 && isWall(walls, nx + 7, ny + PLAYER_SIZE - 6));
    if (blocked) break;
    p.x = Math.min(ARENA_W - PLAYER_SIZE, Math.max(0, nx));
    p.y = Math.min(ARENA_H - PLAYER_SIZE, Math.max(0, ny));
  }
}

/** Aims at the mouse point and fires, reloads and auto-fires for one player. */
function handleWeapon(state: SimState, p: Player, input: PlayerInput, dt: number) {
  p.aim = { x: input.aimX, y: input.aimY };
  p.angle = angleBetween(p.x + 36, p.y + 36, input.aimX, input.aimY);

  if (p.reloadTimer > 0) p.reloadTimer = Math.max(0, p.reloadTimer - dt);
  if (p.noAmmoTimer > 0) p.noAmmoTimer = Math.max(0, p.noAmmoTimer - dt);
  if (p.fireTimer > 0) p.fireTimer = Math.max(0, p.fireTimer - dt);
  if (!input.fire) {
    p.firedThisPress = false;
    return;
  }
  // Semi-auto guns fire once per press (a press during a reload fires when it ends, if still held).
  if (!p.gun.auto && p.firedThisPress) return;
  if (p.gun.auto && p.fireTimer > 0) return;
  if (p.reloadTimer > 0) return;
  if (p.gun.rounds <= 0) {
    if (p.noAmmoTimer <= 0) emit(state, { kind: "noAmmo", player: p.id });
    p.noAmmoTimer = 1.6;
    return;
  }

  p.gun.rounds -= 1;
  p.bulletsInClip = p.gun.rounds % p.gun.clip === 0 ? p.gun.clip : p.gun.rounds % p.gun.clip;
  p.firedThisPress = true;
  p.fireTimer = 0.12;
  emit(state, { kind: "shot", gun: p.gun.name, player: p.id });
  fireBullet(state, p);

  // The clip ran out: the reload starts and blocks shooting for 1.5 s.
  if (p.gun.rounds > 0 && p.gun.rounds % p.gun.clip === 0) {
    p.reloadTimer = 1.5;
    emit(state, { kind: "reload", player: p.id });
  }
}

/** The zombie in the aim cone (±25° of the aim) that is closest, or none. */
function pickTarget(state: SimState, p: Player): Zombie | null {
  const cx = p.x + 36;
  const cy = p.y + 36;
  let best: Zombie | null = null;
  let bestDist = Infinity;
  for (const z of state.zombies) {
    if (z.dying) continue;
    const a = angleBetween(cx, cy, z.x + 36, z.y + 36);
    const diff = Math.min(Math.abs(a - p.angle), 360 - Math.abs(a - p.angle));
    if (diff > 25) continue;
    const d = dist(p.x, p.y, z.x, z.y);
    if (d < bestDist) {
      best = z;
      bestDist = d;
    }
  }
  return best;
}

function fireBullet(state: SimState, p: Player) {
  const cx = p.x + 36;
  const cy = p.y + 36;
  const target = pickTarget(state, p);
  // The original aimed a point far out along the aim line; the bullet flies there.
  const rad = (p.angle * Math.PI) / 180;
  state.bullets.push({
    id: state.nextId++,
    x: cx,
    y: cy,
    dirX: Math.cos(rad),
    dirY: Math.sin(rad),
    targetId: target ? target.id : null,
    damage: p.gun.damage,
    ownerId: p.id,
    life: 4,
  });
}

// ---- Zombies ----

function nearestLivingPlayer(state: SimState, z: Zombie): Player | null {
  let best: Player | null = null;
  let bestDist = Infinity;
  for (const p of state.players) {
    if (p.dead) continue;
    const d = dist(z.x, z.y, p.x, p.y);
    if (d < bestDist) {
      best = p;
      bestDist = d;
    }
  }
  return best;
}

function spawnZombie(state: SimState) {
  state.zombiesLeft -= 1;
  const side = randInt(state, 1, 5);
  let x = 0;
  let y = 0;
  if (side === 1) {
    x = rand(state, 0, ARENA_W);
    y = rand(state, -200, 0);
  } else if (side === 2) {
    x = rand(state, -200, 0);
    y = rand(state, 0, ARENA_H);
  } else if (side === 3) {
    x = rand(state, ARENA_W, ARENA_W + 200);
    y = rand(state, 0, ARENA_H);
  } else {
    x = rand(state, 0, ARENA_W);
    y = rand(state, ARENA_H, ARENA_H + 200);
  }
  const hp = state.level * 10;
  const zombie: Zombie = {
    id: state.nextId++,
    x,
    y,
    angle: 0,
    hp,
    maxHp: hp,
    dying: false,
    fadeTimer: 0,
    attackTimer: null,
    timeout: 0,
    targetId: state.players[0]?.id ?? 0,
    dead: false,
  };
  state.zombies.push(zombie);
}

/** One AI step for all zombies (the original's `ZM_Elapsed`). */
/** Zombies get a little faster each level until level 3 (+15% at level 2, +30% from 3 on). */
export function zombieSpeed(level: number): number {
  return Math.min(1 + 0.15 * (level - 1), 1.3);
}

function zombieStep(state: SimState) {
  state.aiTicks += 1;
  const speed = zombieSpeed(state.level);
  const separate = state.aiTicks % 3 === 0;
  state.zombies.forEach((z, i) => {
    const target = nearestLivingPlayer(state, z);
    if (target) z.targetId = target.id;
    if (!target && !z.dying) {
      z.attackTimer = null;
      return;
    }
    const tx = target ? target.x : z.x;
    const ty = target ? target.y : z.y;
    const dx = z.x - tx;
    const dy = z.y - ty;

    if (separate && i >= 1) {
      const prev = state.zombies[i - 1]!;
      if (dist(z.x + 36, z.y + 36, prev.x + 36, prev.y + 36) < 72) {
        const otherDist = dist(prev.x, prev.y, tx, ty);
        const myDist = dist(z.x, z.y, tx, ty);
        if (myDist > otherDist) z.timeout = 4;
        else prev.timeout = 4;
      }
    }

    if (Math.abs(dx) >= 65 || Math.abs(dy) >= 65) {
      z.attackTimer = null;
      if (z.timeout > 0) {
        z.timeout -= 1;
      } else {
        // One pixel per axis per step (the original never checks walls for zombies).
        z.x -= Math.sign(dx) * speed;
        z.y -= Math.sign(dy) * speed;
      }
      z.angle = target ? angleBetween(z.x + 36, z.y + 36, tx + 36, ty + 36) : z.angle;
    } else if (!z.dying) {
      // In contact: the first hit lands at once, then every 0.85 s.
      if (z.attackTimer === null) z.attackTimer = 0;
    }
  });
}

/** Zombie hits and the fade of dying zombies, in seconds. */
function zombieTimers(state: SimState, dt: number) {
  for (const z of state.zombies) {
    if (z.dying) {
      z.fadeTimer -= dt;
      continue;
    }
    if (z.attackTimer === null) continue;
    z.attackTimer -= dt;
    if (z.attackTimer > 0) continue;
    const target = state.players.find((p) => p.id === z.targetId && !p.dead);
    z.attackTimer = 0.85;
    if (!target) continue;
    target.health = Math.max(0, target.health - 25);
    if (target.health <= 0) killPlayer(state, target);
  }
}

/** A zombie took damage: +10 points, and it dies at zero health. */
export function damageZombie(state: SimState, z: Zombie, amount: number, byPlayer: Player | null) {
  if (z.dying) return;
  z.hp -= amount;
  emit(state, { kind: "hit", zombie: z.id });
  if (byPlayer) byPlayer.score += 10;
  if (z.hp <= 0) {
    z.dying = true;
    z.fadeTimer = 0.3;
    z.hp = 0;
    z.attackTimer = null;
    // The death is handled once the fade ends (see `finishDeaths`).
  }
}

function finishDeaths(state: SimState, walls: Walls) {
  const remaining: Zombie[] = [];
  for (const z of state.zombies) {
    if (!z.dying || z.fadeTimer > 0) {
      remaining.push(z);
      continue;
    }
    if (z.dead) continue;
    z.dead = true;
    emit(state, { kind: "zombieDied", zombie: z.id });
    const killer = state.players.find((p) => p.id === z.targetId) ?? state.players[0];
    if (killer) {
      killer.score += 80;
      killer.kills += 1;
    }
    const lastOfLevel = state.zombiesLeft === 0 && state.zombies.every((o) => o === z || o.dying);
    if (lastOfLevel) {
      state.levelEnding = true;
    } else if (state.overrideTimer <= 0 && withinBounds(walls, z.x + 36, z.y + 36)) {
      dropPowerup(state, z);
    }
  }
  state.zombies = remaining;
}

// ---- Powerups ----

/** 1 in 11 kills, at most two powerups on screen, never the type already held. */
function dropPowerup(state: SimState, z: Zombie) {
  if (state.powerups.length >= MAX_POWERUPS) return;
  if (randInt(state, 0, 11) !== 3) return;
  // A gun you already hold is not dropped again (the original's rule).
  const holding = new Set(state.players.map((p) => p.gun.name));
  const choices = POWERUP_TYPES.filter((t) => !((t === "mp5" && holding.has("MP5")) || (t === "sniper" && holding.has("Sniper")) || (t === "pistol" && holding.has("Pistol"))));
  if (choices.length === 0) return;
  const type = choices[randInt(state, 0, choices.length)]!;
  state.powerups.push({ id: state.nextId++, type, x: z.x, y: z.y, age: 0 });
}

function activatePowerup(state: SimState, p: Player, type: PowerupType) {
  emit(state, { kind: "powerup", type });
  if (type === "nuke") {
    state.overrideTimer = 0.85;
    for (const z of state.zombies) {
      if (z.dying) continue;
      if (!withinWindow(z.x + 36, z.y + 36)) continue;
      damageZombie(state, z, z.hp, p);
      p.score += 120;
    }
  } else if (type === "ammo") {
    p.gun.rounds += 320;
  } else if (type === "mp5" || type === "sniper" || type === "pistol") {
    const pick = type === "mp5" ? GUN_PICKUPS.MP5 : type === "sniper" ? GUN_PICKUPS.Sniper : GUN_PICKUPS.Pistol;
    p.gun = { ...pick };
    p.bulletsInClip = pick.clip;
  } else if (type === "health") {
    p.regenTimer = 0.65;
  }
}

// ---- Players ----

function killPlayer(state: SimState, p: Player) {
  p.dead = true;
  p.health = 0;
  p.score = Math.max(0, p.score - 200);
  if (state.players.every((o) => o.dead)) {
    state.gameOver = true;
    emit(state, { kind: "gameOver" });
  }
}

// ---- The step ----

/**
 * Advances the game by `dt` seconds. `inputs` is indexed like `state.players`. Returns
 * nothing: read `state.events` (cleared at the start of each step) for sounds.
 */
export function step(state: SimState, inputs: PlayerInput[], dt: number, walls: (map: number) => Walls): void {
  state.events = [];
  if (state.gameOver) return;
  const mask = walls(state.map);

  state.players.forEach((p, i) => {
    if (p.dead) return;
    const input = inputs[i] ?? { moveX: 0, moveY: 0, aimX: p.x, aimY: p.y, fire: false };
    movePlayer(p, input, dt, mask);
    handleWeapon(state, p, input, dt);
    if (p.regenTimer > 0) {
      p.regenTimer -= dt;
      if (p.regenTimer <= 0 && p.health < 100) {
        p.health = Math.min(100, p.health + 25);
        p.regenTimer = 0.65;
        if (p.health >= 100) p.regenTimer = 0;
      }
    }
  });

  // Bullets fly toward their aim point and hit their chosen zombie within 75 px.
  for (const b of state.bullets) {
    b.x += b.dirX * 1500 * dt;
    b.y += b.dirY * 1500 * dt;
    b.life -= dt;
    if (b.targetId === null) continue;
    const z = state.zombies.find((o) => o.id === b.targetId);
    if (!z || z.dying) {
      b.targetId = null;
      continue;
    }
    if (dist(b.x, b.y, z.x, z.y) < 75) {
      const owner = state.players.find((p) => p.id === b.ownerId) ?? null;
      damageZombie(state, z, b.damage, owner);
      b.life = 0;
    }
  }
  state.bullets = state.bullets.filter((b) => b.life > 0 && b.x >= 0 && b.y >= 0 && b.x <= ARENA_W && b.y <= ARENA_H);

  // Zombie AI on a fixed 20 ms tick.
  state.aiAccumulator += dt;
  while (state.aiAccumulator >= AI_TICK) {
    state.aiAccumulator -= AI_TICK;
    zombieStep(state);
  }
  zombieTimers(state, dt);
  finishDeaths(state, mask);

  // Spawns: a zombie every 1.25 to 2.5 s, up to ten alive, while budget is left.
  if (state.zombiesLeft > 0 && state.zombies.filter((z) => !z.dying).length < MAX_ZOMBIES_ALIVE && state.players.some((p) => !p.dead)) {
    state.spawnTimer -= dt;
    if (state.spawnTimer <= 0) {
      spawnZombie(state);
      state.spawnTimer = rand(state, 1.25, 2.5);
    }
  }

  if (state.overrideTimer > 0) state.overrideTimer = Math.max(0, state.overrideTimer - dt);

  // Powerups age, blink out and are picked up by touching them.
  for (const p of state.powerups) p.age += dt;
  state.powerups = state.powerups.filter((p) => p.age < 19.2);
  for (const pl of state.players) {
    if (pl.dead) continue;
    const hit = state.powerups.find((pw) => pl.x < pw.x + POWERUP_SIZE && pl.x + PLAYER_SIZE > pw.x && pl.y < pw.y + POWERUP_SIZE && pl.y + PLAYER_SIZE > pw.y);
    if (hit) {
      state.powerups = state.powerups.filter((pw) => pw !== hit);
      activatePowerup(state, pl, hit.type);
    }
  }

  // The level ends once the last zombie is gone.
  if (state.levelEnding && state.zombies.length === 0) {
    startLevel(state, walls);
  }
}

/** A quick way to see whether a point is in the arena's walkable area (for tests and the view). */
export const walkable = (walls: Walls, x: number, y: number) => withinBounds(walls, x, y);
