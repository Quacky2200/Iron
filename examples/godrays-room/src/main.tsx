// God rays: a derelict hall on a late afternoon. The sun pours in through a tall window with a
// cross mullion, through the louvres of a half-shuttered window and through a broken patch of
// the roof, in crisp beams (`VolumetricFogSettings.sharpDistance`) that land as bright shapes on
// the floorboards, a table, a sheet-covered armchair and a stack of crates. A few hundred dust
// motes drift in the air and glow where they cross the sunlight. The camera drifts along the far
// wall at eye height, looking across the beams towards the window. The menu (or keys) switches
// the sharp god rays (G), the fog and its shafts (F) and the dust (D).
import {
  Camera,
  Color,
  Component,
  GameObject,
  input,
  Light,
  Material,
  MeshRenderer,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Vector3,
  register,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

// The room: x -6..6 (the window wall at -6), z -7..7, floor at 0, roof planks from 5.2.
const ROOM_X = 6;
const ROOM_Z = 7;
const WALL_H = 5.2;
const RAFTER_Y = 4.95;
const RAFTER_STEP = 1.4;
const PLANK_W = 0.5;
const PLANKS_X = (ROOM_X * 2) / PLANK_W;
const PLANKS_Z = (ROOM_Z * 2) / RAFTER_STEP;

// Where sunlight travels: from the north-west, about 34 degrees above the horizon.
const SUN = new Vector3(0.75, -0.55, -0.35).normalized;
const SUN_COLOR = new Color(1, 0.74, 0.46, 1);

// The openings. The tall window (cross mullion at MULLION_Z / TRANSOM_Y) and the shuttered one
// sit in the west wall; the roof hole is a set of missing roof plank cells.
const WIN = { z0: -0.2, z1: 2.2, y0: 0.9, y1: 4.3 };
const MULLION_Z = 1;
const TRANSOM_Y = 3.0;
const SHUT = { z0: -4.8, z1: -3.2, y0: 1.0, y1: 3.2, split: -4.0 };
const SLAT_STEP = 0.2;
const MISSING_SLATS = new Set([3, 7]);
const HOLE = { x0: -5.5, x1: -2.5, z0: 1.4, z1: 5.6 };

const DUST_COUNT = 400;
const GOD_RAYS = { intensity: 1.2, anisotropy: 0.5, steps: 4, distance: 30, albedo: new Color(1, 0.86, 0.68, 1), sharpSamples: 96 };
const SHARP_DISTANCE = 22;
const FOG_COLOR = new Color(0.055, 0.046, 0.036, 1);

/** What the menu shows and changes; `apply` sends it to the engine. */
const state = { sharp: true, fog: true, dust: true };
let refreshMenu = () => {};
let dustRoot: GameObject | null = null;

function apply(): void {
  RenderSettings.fogMode = "exponential";
  RenderSettings.fogColor = FOG_COLOR;
  RenderSettings.fogDensity = 0.035;
  RenderSettings.volumetricFog = { ...GOD_RAYS, sharpDistance: state.sharp ? SHARP_DISTANCE : 0 };
  RenderSettings.fog = state.fog;
  if (dustRoot) dustRoot.enabled = state.dust;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
/** A box between two corners. */
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material, parent?: GameObject) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material, parent);
}
const mat = (hex: string, smoothness = 0.1) => Material.create({ color: Color.hex(hex), smoothness });
const pick = <T,>(list: T[]): T => list[Random.rangeInt(0, list.length)]!;

// Roof cells (plank across x, segment between rafters along z) that are gone.
const missing: boolean[] = new Array(PLANKS_X * PLANKS_Z).fill(false);
const cellOf = (x: number, z: number) => {
  const i = Math.floor((x + ROOM_X) / PLANK_W);
  const j = Math.floor((z + ROOM_Z) / RAFTER_STEP);
  return i < 0 || j < 0 || i >= PLANKS_X || j >= PLANKS_Z ? -1 : j * PLANKS_X + i;
};

/** Whether the sun reaches `p` through one of the openings (furniture is ignored). */
function sunlit(px: number, py: number, pz: number): boolean {
  // Back towards the sun to the west wall.
  if (px > -ROOM_X) {
    const s = (px + ROOM_X) / SUN.x;
    const y = py - SUN.y * s;
    const z = pz - SUN.z * s;
    if (z > WIN.z0 && z < WIN.z1 && y > WIN.y0 && y < WIN.y1) return Math.abs(z - MULLION_Z) > 0.07 && Math.abs(y - TRANSOM_Y) > 0.07;
    if (z > SHUT.z0 && z < SHUT.z1 && y > SHUT.y0 && y < SHUT.y1) {
      if (z > SHUT.split) return true;
      const k = Math.floor((y - SHUT.y0) / SLAT_STEP);
      return MISSING_SLATS.has(k) || (y - SHUT.y0) / SLAT_STEP - k > 0.55;
    }
  }
  // Up to the roof, past the rafters.
  if (py < RAFTER_Y - 0.15) {
    const s = (WALL_H - py) / -SUN.y;
    const c = cellOf(px - SUN.x * s, pz - SUN.z * s);
    if (c < 0 || !missing[c]) return false;
    const r = (RAFTER_Y - py) / -SUN.y;
    const rz = pz - SUN.z * r + ROOM_Z;
    const off = rz - Math.round(rz / RAFTER_STEP) * RAFTER_STEP;
    return Math.abs(off) > 0.14;
  }
  return false;
}

/** A point somewhere along a sunbeam inside the room, or null. */
function pointInBeam(): Vector3 | null {
  const r = Random.value;
  let q: Vector3;
  if (r < 0.45) q = new Vector3(-ROOM_X, Random.range(WIN.y0, WIN.y1), Random.range(WIN.z0, WIN.z1));
  else if (r < 0.65) q = new Vector3(-ROOM_X, Random.range(SHUT.y0, SHUT.y1), Random.range(SHUT.z0, SHUT.z1));
  else q = new Vector3(Random.range(HOLE.x0, HOLE.x1), WALL_H, Random.range(HOLE.z0, HOLE.z1));
  const p = q.add(SUN.scale(Random.range(0.2, q.y / -SUN.y)));
  return Math.abs(p.x) < ROOM_X - 0.3 && Math.abs(p.z) < ROOM_Z - 0.3 && p.y > 0.1 && p.y < RAFTER_Y - 0.2 ? p : null;
}

class Hall extends Component {
  private camera!: Camera;
  private t = 0;
  // Dust: positions, velocities, ages and lifetimes, and which material each mote shows.
  private motes: GameObject[] = [];
  private renderers: MeshRenderer[] = [];
  private pos = new Float32Array(DUST_COUNT * 3);
  private vel = new Float32Array(DUST_COUNT * 3);
  private age = new Float32Array(DUST_COUNT);
  private life = new Float32Array(DUST_COUNT);
  private size = new Float32Array(DUST_COUNT);
  private lit = new Int8Array(DUST_COUNT).fill(-1);
  private bright: Material[] = [];
  private dim!: Material;
  private roof: Material[] = [];

  override onStart() {
    QualitySettings.shadowDistance = 25;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.05, 0.042, 0.034, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1.1, intensity: 0.55, scatter: 0.6 };

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    sun.addComponent(Light, { type: "directional", color: SUN_COLOR, intensity: 3.2, shadows: "hard" });

    Random.initState(7);
    this.buildRoom();
    this.buildFurniture();
    this.buildIvy();
    this.buildDust();

    const eye = new GameObject("Main Camera", { position: new Vector3(4.6, 1.65, -4) });
    // The outdoors, seen through the openings: a hazy, overexposed afternoon.
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: new Color(2.4, 2.0, 1.5, 1), farClipPlane: 100 });
    apply();
  }

  private buildRoom() {
    // Floorboards along z, in a few shades, with staggered joints.
    const boards = ["#5a4030", "#634634", "#4f3829", "#6b4d36", "#553b2b"].map((h) => mat(h, 0.18));
    const boardW = 0.32;
    for (let x = -ROOM_X; x < ROOM_X - 0.01; x += boardW) {
      let z = -ROOM_Z - Random.range(0, 2.5);
      while (z < ROOM_Z) {
        const len = Random.range(3.5, 6);
        const z0 = Math.max(z, -ROOM_Z);
        const z1 = Math.min(z + len, ROOM_Z);
        slab("Board", x + 0.006, -0.1, z0 + 0.006, x + boardW - 0.006, Random.range(-0.004, 0.004), z1 - 0.006, pick(boards));
        z += len;
      }
    }
    // A dark gap under the boards so the joints read.
    slab("Subfloor", -ROOM_X, -0.2, -ROOM_Z, ROOM_X, -0.09, ROOM_Z, mat("#1a120c"));

    // Plaster walls; the west one is cut around the two windows.
    const plaster = mat("#b5a68c", 0.05);
    const T = 0.3;
    const W = -ROOM_X - T;
    slab("Wall E", ROOM_X, 0, -ROOM_Z - T, ROOM_X + T, WALL_H, ROOM_Z + T, plaster);
    slab("Wall N", -ROOM_X - T, 0, ROOM_Z, ROOM_X + T, WALL_H, ROOM_Z + T, plaster);
    slab("Wall S", -ROOM_X - T, 0, -ROOM_Z - T, ROOM_X + T, WALL_H, -ROOM_Z, plaster);
    slab("Wall W", W, 0, -ROOM_Z, -ROOM_X, WALL_H, SHUT.z0, plaster);
    slab("Wall W", W, 0, SHUT.z1, -ROOM_X, WALL_H, WIN.z0, plaster);
    slab("Wall W", W, 0, WIN.z1, -ROOM_X, WALL_H, ROOM_Z, plaster);
    for (const o of [WIN, SHUT]) {
      slab("Sill wall", W, 0, o.z0, -ROOM_X, o.y0, o.z1, plaster);
      slab("Lintel", W, o.y1, o.z0, -ROOM_X, WALL_H, o.z1, plaster);
    }
    // Worn patches: damp stains, newer plaster, bare brick.
    const patches = ["#8f8068", "#cbbfa6", "#7a5040", "#9a8a6e", "#6e6250"].map((h) => mat(h, 0.05));
    const patch = (x: number, y: number, z: number, w: number, h: number, alongZ: boolean, m: Material) =>
      shape("Patch", "cube", new Vector3(x, y, z), alongZ ? new Vector3(0.02, h, w) : new Vector3(w, h, 0.02), m);
    for (let i = 0; i < 14; i++) {
      const m = pick(patches);
      const w = Random.range(0.5, 1.8);
      const h = Random.range(0.4, 1.4);
      const y = Random.range(0.5, 4.2);
      const side = Random.rangeInt(0, 3);
      if (side === 0) patch(Random.range(-5, 5), y, ROOM_Z - 0.005, w, h, false, m);
      else if (side === 1) patch(ROOM_X - 0.005, y, Random.range(-6, 6), w, h, true, m);
      else patch(-ROOM_X + 0.005, Random.range(0.3, 0.8), Random.range(-6.5, 6.5), w, h * 0.5, true, m);
    }
    // Skirting boards.
    const skirt = mat("#2e1f15", 0.2);
    slab("Skirting", -ROOM_X, 0, ROOM_Z - 0.03, ROOM_X, 0.18, ROOM_Z, skirt);
    slab("Skirting", ROOM_X - 0.03, 0, -ROOM_Z, ROOM_X, 0.18, ROOM_Z, skirt);
    slab("Skirting", -ROOM_X, 0, -ROOM_Z, ROOM_X, 0.18, -ROOM_Z + 0.03, skirt);

    // The tall window: frame, sill and a cross mullion; no glass left.
    const frame = mat("#3a291c", 0.2);
    const D = 0.36;
    slab("Frame", W - 0.03, WIN.y0 - 0.05, WIN.z0, W + D, WIN.y0, WIN.z1, frame);
    slab("Sill", -ROOM_X - 0.05, WIN.y0 - 0.08, WIN.z0 - 0.1, -ROOM_X + 0.18, WIN.y0 - 0.02, WIN.z1 + 0.1, frame);
    slab("Frame", W - 0.03, WIN.y1, WIN.z0, W + D, WIN.y1 + 0.06, WIN.z1, frame);
    slab("Frame", W - 0.03, WIN.y0, WIN.z0, W + D, WIN.y1, WIN.z0 + 0.07, frame);
    slab("Frame", W - 0.03, WIN.y0, WIN.z1 - 0.07, W + D, WIN.y1, WIN.z1, frame);
    slab("Mullion", W + 0.08, WIN.y0, MULLION_Z - 0.06, W + 0.24, WIN.y1, MULLION_Z + 0.06, frame);
    slab("Transom", W + 0.08, TRANSOM_Y - 0.06, WIN.z0, W + 0.24, TRANSOM_Y + 0.06, WIN.z1, frame);

    // The shuttered window: louvres on one half (two slats gone), the other leaf folded back.
    slab("Frame", W - 0.03, SHUT.y0 - 0.05, SHUT.z0, W + D, SHUT.y0, SHUT.z1, frame);
    slab("Frame", W - 0.03, SHUT.y1, SHUT.z0, W + D, SHUT.y1 + 0.05, SHUT.z1, frame);
    slab("Stile", W + 0.02, SHUT.y0, SHUT.split - 0.03, W + 0.1, SHUT.y1, SHUT.split + 0.03, frame);
    for (let k = 0; (k + 0.5) * SLAT_STEP < SHUT.y1 - SHUT.y0; k++) {
      if (MISSING_SLATS.has(k)) continue;
      const y = SHUT.y0 + (k + 0.25) * SLAT_STEP;
      shape("Slat", "cube", new Vector3(W + 0.08, y, (SHUT.z0 + SHUT.split) / 2), new Vector3(0.2, 0.025, SHUT.split - SHUT.z0), frame, undefined, Quaternion.euler(0, 0, -30));
    }
    slab("Open leaf", W - 0.07, SHUT.y0, SHUT.z1 + 0.02, W - 0.02, SHUT.y1, SHUT.z1 + 0.8, frame);

    // Rafters across the room and roof planks over them, some gone: a broken patch near the
    // window wall and a few single gaps.
    const beam = mat("#33241a", 0.12);
    for (let j = 1; j < PLANKS_Z; j++) slab("Rafter", -ROOM_X, RAFTER_Y - 0.15, -ROOM_Z + j * RAFTER_STEP - 0.12, ROOM_X, WALL_H, -ROOM_Z + j * RAFTER_STEP + 0.12, beam);
    slab("Ridge", -0.15, RAFTER_Y - 0.32, -ROOM_Z, 0.15, RAFTER_Y - 0.15, ROOM_Z, beam);
    for (let i = 0; i < PLANKS_X; i++) {
      for (let j = 0; j < PLANKS_Z; j++) {
        const x = -ROOM_X + (i + 0.5) * PLANK_W;
        const z = -ROOM_Z + (j + 0.5) * RAFTER_STEP;
        const inside = x > HOLE.x0 && x < HOLE.x1 && z > HOLE.z0 && z < HOLE.z1;
        const edge = x > HOLE.x0 - 0.6 && x < HOLE.x1 + 0.6 && z > HOLE.z0 - 1.4 && z < HOLE.z1 + 1.4;
        if (inside || (edge && Random.value < 0.35) || (x < 1 && Random.value < 0.025)) missing[j * PLANKS_X + i] = true;
      }
    }
    const roof = (this.roof = ["#3d2c1f", "#46331f", "#36271b"].map((h) => mat(h, 0.08)));
    for (let c = 0; c < missing.length; c++) {
      if (missing[c]) continue;
      const i = c % PLANKS_X;
      const j = Math.floor(c / PLANKS_X);
      const x = -ROOM_X + i * PLANK_W;
      const z = -ROOM_Z + j * RAFTER_STEP;
      slab("Roof plank", x, WALL_H, z, x + PLANK_W, WALL_H + 0.1, z + RAFTER_STEP, pick(roof));
    }
    // Broken planks hanging from the hole's edge.
    for (let k = 0; k < 4; k++) {
      const x = Random.range(HOLE.x0, HOLE.x1);
      const z = k % 2 === 0 ? HOLE.z0 + 0.1 : HOLE.z1 - 0.1;
      const tilt = Random.range(35, 70) * (k % 2 === 0 ? 1 : -1);
      shape("Hanging plank", "cube", new Vector3(x, WALL_H - 0.4, z), new Vector3(0.45, 0.06, 1.1), pick(roof), undefined, Quaternion.euler(tilt, Random.range(-15, 15), 0));
    }
  }

  private buildFurniture() {
    const oak = mat("#4a3322", 0.25);
    const pine = mat("#8a6a45", 0.15);
    const pineDark = mat("#5e4630", 0.15);

    // A faded rug under the table in the window's beam.
    slab("Rug", -4.3, 0.004, -2.4, -1.0, 0.02, 0.6, mat("#5a2a22", 0.02));
    slab("Rug border", -4.15, 0.02, -2.25, -1.15, 0.024, 0.45, mat("#6e4a2a", 0.02));
    slab("Rug field", -4.0, 0.024, -2.1, -1.3, 0.028, 0.3, mat("#4a2420", 0.02));

    // A table with a few things on it.
    const table = new GameObject("Table", { position: new Vector3(-2.6, 0, -0.9), rotation: Quaternion.euler(0, 8, 0) });
    slab("Top", -1.0, 0.74, -0.5, 1.0, 0.8, 0.5, oak, table);
    for (const [x, z] of [[-0.9, -0.4], [0.9, -0.4], [-0.9, 0.4], [0.9, 0.4]] as const) slab("Leg", x - 0.04, 0, z - 0.04, x + 0.04, 0.74, z + 0.04, oak, table);
    slab("Book", -0.5, 0.8, -0.1, -0.2, 0.86, 0.12, mat("#3c4a5a"), table);
    slab("Book", -0.48, 0.86, -0.08, -0.24, 0.9, 0.1, mat("#6a2c24"), table);
    shape("Bottle", "cylinder", new Vector3(0.4, 0.95, 0.1), new Vector3(0.08, 0.15, 0.08), Material.create({ color: Color.hex("#2d4a32"), smoothness: 0.8 }), table);
    shape("Candlestick", "cylinder", new Vector3(0.1, 0.86, -0.2), new Vector3(0.07, 0.06, 0.07), mat("#6a5a3a", 0.6), table);
    shape("Candle", "cylinder", new Vector3(0.1, 1.0, -0.2), new Vector3(0.04, 0.1, 0.04), mat("#d8cfb8"), table);
    shape("Bowl", "sphere", new Vector3(0.6, 0.83, -0.25), new Vector3(0.22, 0.08, 0.22), mat("#8a8478", 0.3), table);

    // Chairs: one pulled up, one knocked over.
    const chair = (at: Vector3, rotation: Quaternion) => {
      const c = new GameObject("Chair", { position: at, rotation });
      slab("Seat", -0.23, 0.43, -0.23, 0.23, 0.48, 0.23, oak, c);
      for (const [x, z] of [[-0.19, -0.19], [0.19, -0.19], [-0.19, 0.19], [0.19, 0.19]] as const) slab("Leg", x - 0.025, 0, z - 0.025, x + 0.025, 0.43, z + 0.025, oak, c);
      for (const x of [-0.19, 0.19]) slab("Post", x - 0.025, 0.48, 0.17, x + 0.025, 1.0, 0.22, oak, c);
      for (const y of [0.62, 0.78, 0.94]) slab("Rail", -0.2, y, 0.18, 0.2, y + 0.05, 0.21, oak, c);
    };
    chair(new Vector3(-2.7, 0, -1.9), Quaternion.euler(0, 190, 0));
    chair(new Vector3(-1.1, 0.23, 0.3), Quaternion.euler(0, 60, 0).mul(Quaternion.euler(-90, 0, 0)));

    // A bookcase on the north wall.
    const bc = new GameObject("Bookcase", { position: new Vector3(0.6, 0, ROOM_Z - 0.22) });
    slab("Side", -1.05, 0, -0.2, -0.99, 2.4, 0.2, oak, bc);
    slab("Side", 0.99, 0, -0.2, 1.05, 2.4, 0.2, oak, bc);
    slab("Back", -1.0, 0, 0.18, 1.0, 2.4, 0.21, pineDark, bc);
    slab("Top", -1.1, 2.4, -0.23, 1.1, 2.46, 0.22, oak, bc);
    const spines = ["#5a2420", "#2a3a4e", "#3a4a2a", "#6a5030", "#4a2a3a", "#7a6a50", "#2a2a2a"].map((h) => mat(h, 0.2));
    for (let s = 0; s < 5; s++) {
      const y = 0.08 + s * 0.48;
      slab("Shelf", -0.99, y - 0.03, -0.2, 0.99, y, 0.18, oak, bc);
      if (s === 4) break;
      let x = -0.95;
      while (x < 0.9) {
        if (Random.value < 0.12) {
          x += Random.range(0.1, 0.3);
          continue;
        }
        const w = Random.range(0.03, 0.07);
        const h = Random.range(0.25, 0.4);
        if (x + w > 0.95) break;
        const leaning = Random.value < 0.08;
        shape("Book", "cube", new Vector3(x + w / 2, y + h / 2, 0), new Vector3(w, h, Random.range(0.18, 0.26)), pick(spines), bc, leaning ? Quaternion.euler(0, 0, -18) : undefined);
        x += w + (leaning ? 0.1 : 0.003);
      }
    }

    // Crates by the east side, under the roof hole's beam.
    const crate = (at: Vector3, yaw: number, size: number) => {
      const c = new GameObject("Crate", { position: at, rotation: Quaternion.euler(0, yaw, 0) });
      const h = size / 2;
      slab("Box", -h, 0, -h, h, size, h, pine, c);
      for (const y of [0.04, size - 0.04, size / 2]) slab("Band", -h - 0.012, y - 0.04, -h - 0.012, h + 0.012, y + 0.04, h + 0.012, pineDark, c);
    };
    crate(new Vector3(3.3, 0, 2.6), 12, 0.7);
    crate(new Vector3(4.15, 0, 2.2), -8, 0.6);
    crate(new Vector3(3.4, 0.7, 2.55), 30, 0.55);
    crate(new Vector3(2.6, 0, 3.6), 40, 0.5);
    slab("Plank", 2.0, 0, 1.2, 2.2, 0.04, 2.6, pine);

    // An armchair under a dust sheet, by the shuttered window.
    const sheet = mat("#c9c2b2", 0.05);
    const ac = new GameObject("Armchair", { position: new Vector3(-3.4, 0, -5.0), rotation: Quaternion.euler(0, 115, 0) });
    shape("Skirt", "cube", new Vector3(0, 0.24, 0), new Vector3(1.0, 0.48, 0.95), sheet, ac);
    shape("Seat", "sphere", new Vector3(0, 0.5, -0.05), new Vector3(0.9, 0.2, 0.85), sheet, ac);
    shape("Back", "cube", new Vector3(0, 0.82, 0.38), new Vector3(0.96, 0.75, 0.24), sheet, ac, Quaternion.euler(-8, 0, 0));
    shape("Back top", "sphere", new Vector3(0, 1.18, 0.4), new Vector3(0.98, 0.22, 0.3), sheet, ac);
    for (const x of [-0.5, 0.5]) shape("Arm", "cube", new Vector3(x, 0.58, 0), new Vector3(0.18, 0.32, 0.92), sheet, ac);
    shape("Fold", "cube", new Vector3(0.3, 0.06, -0.55), new Vector3(0.5, 0.12, 0.25), sheet, ac, Quaternion.euler(0, 20, 0));

    // Rubble and fallen planks under the roof hole.
    const rubble = ["#6a5a48", "#5a4a3a", "#7a6a58"].map((h) => mat(h, 0.05));
    for (let i = 0; i < 9; i++) {
      const at = new Vector3(Random.range(HOLE.x0 - 0.5, HOLE.x1 + 0.5), 0.03, Random.range(HOLE.z0 - 0.5, HOLE.z1));
      shape("Fallen plank", "cube", at, new Vector3(0.45, 0.05, Random.range(0.5, 1.4)), pick(this.roof), undefined, Quaternion.euler(Random.range(-12, 12), Random.range(0, 180), Random.range(-6, 6)));
    }
    for (let i = 0; i < 14; i++) {
      const s = Random.range(0.06, 0.2);
      shape("Rubble", "cube", new Vector3(Random.range(HOLE.x0, HOLE.x1), s * 0.4, Random.range(HOLE.z0, HOLE.z1)), new Vector3(s, s * 0.7, s * 1.2), pick(rubble), undefined, Quaternion.euler(0, Random.range(0, 90), Random.range(-20, 20)));
    }

    // A crooked picture on the north wall.
    const pic = new GameObject("Picture", { position: new Vector3(-2.8, 2.6, ROOM_Z - 0.04), rotation: Quaternion.euler(0, 0, 4) });
    slab("Frame", -0.55, -0.4, -0.02, 0.55, 0.4, 0.02, mat("#4a3a20", 0.4), pic);
    slab("Canvas", -0.47, -0.32, -0.03, 0.47, 0.32, -0.02, mat("#2a3a34", 0.2), pic);
  }

  private buildIvy() {
    // Vines creeping in: down the wall by the window and hanging through the roof hole.
    const leaves = ["#2e4a22", "#3a5a26", "#25401c", "#4a6a2e"].map((h) => mat(h, 0.2));
    const vine = (x: number, y: number, z: number, dx: number, dy: number, dz: number, n: number) => {
      for (let i = 0; i < n; i++) {
        x += dx + Random.range(-0.06, 0.06);
        y += dy + Random.range(-0.05, 0.05);
        z += dz + Random.range(-0.06, 0.06);
        const s = Random.range(0.07, 0.14);
        shape("Leaf", "cube", new Vector3(x, y, z), new Vector3(s, s * 0.9, 0.012), pick(leaves), undefined, Quaternion.euler(Random.range(-50, 50), Random.range(0, 360), Random.range(-40, 40)));
      }
    };
    // Over the window's lintel and down its sides, inside.
    vine(-ROOM_X + 0.05, 4.9, WIN.z0 - 0.3, 0, -0.12, -0.04, 22);
    vine(-ROOM_X + 0.05, 5.0, WIN.z1 + 0.2, 0.01, -0.09, 0.03, 18);
    vine(-ROOM_X + 0.05, 4.6, WIN.z0, 0, 0.0, 0.11, 20);
    // Hanging through the hole: these cut the roof beam.
    for (const [x, z] of [[-5.2, 2.1], [-3.9, 1.5], [-3.0, 5.4], [-4.6, 5.2], [-2.7, 3.4]] as const) vine(x, WALL_H + 0.1, z, 0, -Random.range(0.08, 0.13), 0, Random.rangeInt(7, 13));
    // Along the floor under the hole.
    vine(HOLE.x0, 0.03, HOLE.z1, 0.1, 0, -0.08, 14);
  }

  private buildDust() {
    dustRoot = new GameObject("Dust");
    this.dim = Material.create({ color: Color.black, emission: new Color(0.035, 0.03, 0.024, 1), unlit: true });
    this.bright = [1.6, 2.4, 3.6].map((k) => Material.create({ color: Color.black, emission: new Color(SUN_COLOR.r * k, SUN_COLOR.g * k, SUN_COLOR.b * k, 1), unlit: true }));
    for (let i = 0; i < DUST_COUNT; i++) {
      const go = new GameObject("Mote", { parent: dustRoot });
      const r = go.addComponent(MeshRenderer, { mesh: "sphere", material: this.dim });
      r.shadowCastingMode = "off";
      this.motes.push(go);
      this.renderers.push(r);
      this.size[i] = Random.range(0.01, 0.026);
      this.spawn(i);
      this.age[i] = Random.range(0, this.life[i]!);
    }
  }

  /** A new mote: mostly somewhere along a beam, else anywhere in front of the camera. */
  private spawn(i: number) {
    const p = (Random.value < 0.65 ? pointInBeam() : null) ?? new Vector3(Random.range(-5.7, 5), Random.range(0.1, 4.6), Random.range(-6.7, 5));
    this.pos[i * 3] = p.x;
    this.pos[i * 3 + 1] = p.y;
    this.pos[i * 3 + 2] = p.z;
    this.vel[i * 3] = this.vel[i * 3 + 1] = this.vel[i * 3 + 2] = 0;
    this.age[i] = 0;
    this.life[i] = Random.range(14, 40);
  }

  private updateDust(dt: number) {
    const kick = 0.05 * Math.sqrt(dt);
    const damp = Math.max(0, 1 - 0.7 * dt);
    const fade = 1.5;
    for (let i = 0; i < DUST_COUNT; i++) {
      const a = (this.age[i]! += dt);
      const life = this.life[i]!;
      const o = i * 3;
      // Brownian wander, a slow settle and a faint draught away from the window.
      const vx = (this.vel[o] = this.vel[o]! * damp + (Math.random() - 0.5) * kick);
      const vy = (this.vel[o + 1] = this.vel[o + 1]! * damp + (Math.random() - 0.5) * kick);
      const vz = (this.vel[o + 2] = this.vel[o + 2]! * damp + (Math.random() - 0.5) * kick);
      const x = (this.pos[o] = this.pos[o]! + (vx + 0.006) * dt);
      const y = (this.pos[o + 1] = this.pos[o + 1]! + (vy - 0.01) * dt);
      const z = (this.pos[o + 2] = this.pos[o + 2]! + vz * dt);
      if (a > life || y < 0.03 || y > RAFTER_Y - 0.2 || Math.abs(x) > ROOM_X - 0.1 || Math.abs(z) > ROOM_Z - 0.1) {
        this.spawn(i);
        continue;
      }
      const tr = this.motes[i]!.transform;
      tr.localPosition = new Vector3(x, y, z);
      const s = this.size[i]! * Math.min(1, a / fade, (life - a) / fade);
      tr.localScale = new Vector3(s, s, s);
      const lit = sunlit(x, y, z) ? 1 + (i % 3) : 0;
      if (lit !== this.lit[i]) {
        this.lit[i] = lit;
        this.renderers[i]!.material = lit ? this.bright[lit - 1]! : this.dim;
      }
    }
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyG")) toggle("sharp");
    if (input.getKeyDown("KeyF")) toggle("fog");
    if (input.getKeyDown("KeyD")) toggle("dust");
    if (state.dust) this.updateDust(Math.min(dt, 0.05));
    // A slow walk along the east side, looking across the beams towards the window.
    const t = this.t;
    const cam = this.camera.transform;
    cam.position = new Vector3(4.6 + Math.sin(t * 0.05) * 0.4, 1.65 + Math.sin(t * 0.13) * 0.05, -4 + Math.sin(t * 0.04) * 2.3);
    const target = new Vector3(-5, 2.1 + Math.sin(t * 0.07) * 0.3, 0.4 + Math.sin(t * 0.05 + 1) * 2.2);
    cam.rotation = Quaternion.lookRotation(target.sub(cam.position));
  }
}

function toggle(key: keyof typeof state) {
  state[key] = !state[key];
  apply();
}

register({ Hall });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#c3b8a6", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on ? "#c98a3a" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f6efe4", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#857a6a", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(20, 15, 10, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the room</text>
        <Button label="God rays" value={state.sharp ? `Sharp (${SHARP_DISTANCE} m)` : "Soft (volume only)"} on={state.sharp} keyName="G" onClick={() => toggle("sharp")} />
        <Button label="Fog" value={state.fog ? "On" : "Off"} on={state.fog} keyName="F" onClick={() => toggle("fog")} />
        <Button label="Dust" value={state.dust ? `${DUST_COUNT} motes` : "Off"} on={state.dust} keyName="D" onClick={() => toggle("dust")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
