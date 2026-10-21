// God rays: the cell. A medieval basement prison cell of rough stone, lit only by daylight
// through a small barred window high in the back wall. The sun slants down through the
// dusty air in crisp beams, the bars' shadows striping the beams and the pool of light on
// the flagstones; dust motes drift through the dark and flare up where the sun catches them.
// A heavy door, a straw pallet, a bucket and chains on the wall; a drain grate in the
// ceiling. The camera sways slowly inside the cell, looking across the beams to the window.
// The menu (or keys) switches the sharp god rays (G), the fog (F), the dust (D) and the
// sunlight's tint (T: warm or teal).
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
  type PrimitiveMesh,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

// The cell's inside: x -3..3, z -4..4, floor at y 0, ceiling at y 4; walls 0.5 thick.
const HX = 3;
const HZ = 4;
const H = 4;
const T = 0.5;
// The window in the back wall (z = 4 .. 4.5), seen from inside.
const WIN = { x0: -0.65, x1: 0.65, y0: 2.6, y1: 3.7 };
const BAR_XS = [-0.44, -0.22, 0, 0.22, 0.44];
const BAR_YS = [2.95, 3.35];
const BAR_Z = HZ + T / 2;
const BAR_R = 0.028;
// The door on the right wall (x = 3), along z.
const DOOR = { z0: 1.25, z1: 2.35, y1: 2.15 };

// Where the sunlight travels: in through the window, down and towards the camera.
const SUN_DIR = new Vector3(0.25, -0.55, -0.8).normalized;
const TO_SUN = SUN_DIR.scale(-1);

const TINTS = {
  warm: {
    sun: new Color(1, 0.86, 0.68, 1),
    sky: new Color(3.2, 2.9, 2.4, 1),
    albedo: new Color(1, 0.94, 0.86, 1),
    bounce: new Color(1, 0.8, 0.6, 1),
    mote: new Color(3, 2.6, 2, 1),
  },
  teal: {
    sun: new Color(0.55, 0.88, 1, 1),
    sky: new Color(1.6, 2.7, 3.2, 1),
    albedo: new Color(0.8, 1, 1, 1),
    bounce: new Color(0.45, 0.8, 0.9, 1),
    mote: new Color(1.6, 2.8, 3.2, 1),
  },
};
type Tint = keyof typeof TINTS;

const SUN_INTENSITY = 6;
const FOG_COLOR = new Color(0.006, 0.006, 0.007, 1);

/** What the menu shows and changes; `apply` sends it to the engine. */
const state = { godRays: true, fog: true, dust: true, tint: "warm" as Tint };
let refreshMenu = () => {};
let scene: Cell | null = null;

function apply(): void {
  const tint = TINTS[state.tint];
  RenderSettings.fog = state.fog;
  RenderSettings.fogMode = "exponential";
  RenderSettings.fogColor = FOG_COLOR;
  RenderSettings.fogDensity = 0.05;
  RenderSettings.volumetricFog = {
    intensity: 0.8,
    anisotropy: 0.5,
    steps: 8,
    distance: 20,
    albedo: tint.albedo,
    sharpDistance: state.godRays ? 10 : 0,
    sharpSamples: 128,
  };
  scene?.applyTint(tint);
  if (scene) scene.dust.enabled = state.dust;
  refreshMenu();
}

function shape(name: string, mesh: PrimitiveMesh, at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}

/** A cube spanning two corners. */
function slab(name: string, a: Vector3, b: Vector3, material: Material) {
  const centre = new Vector3((a.x + b.x) / 2, (a.y + b.y) / 2, (a.z + b.z) / 2);
  const size = new Vector3(Math.abs(b.x - a.x), Math.abs(b.y - a.y), Math.abs(b.z - a.z));
  return shape(name, "cube", centre, size, material);
}

/** A ring of short capsules in its local xy plane: chain links' ends, shackles, door rings. */
function ring(name: string, at: Vector3, radius: number, thickness: number, material: Material, rotation: Quaternion, segments = 8) {
  const go = new GameObject(name, { position: at, rotation });
  const len = ((2 * Math.PI * radius) / segments) * 1.25;
  for (let k = 0; k < segments; k++) {
    const a = (k / segments) * Math.PI * 2;
    const at = new Vector3(Math.cos(a) * radius, Math.sin(a) * radius, 0);
    shape("Segment", "capsule", at, new Vector3(thickness, len / 2, thickness), material, go, Quaternion.euler(0, 0, (a * 180) / Math.PI));
  }
  return go;
}

/** A chain link: two long sides and two short ends, in its local xy plane. */
function link(at: Vector3, yaw: number, material: Material) {
  const go = new GameObject("Link", { position: at, rotation: Quaternion.euler(0, yaw, 0) });
  const t = 0.013;
  for (const s of [-1, 1]) {
    shape("Side", "capsule", new Vector3(s * 0.018, 0, 0), new Vector3(t, 0.04, t), material, go);
    shape("End", "capsule", new Vector3(0, s * 0.036, 0), new Vector3(t, 0.022, t), material, go, Quaternion.euler(0, 0, 90));
  }
  return go;
}

interface Rect {
  u0: number;
  u1: number;
  v0: number;
  v1: number;
}

/**
 * Rough stone blocks over a wall's inside face, in rows of uneven heights and widths, each
 * standing a little proud of the wall by its own depth. `origin` is the face's corner at the
 * floor, `along` its direction, `inward` the face's normal; `holes` are left open.
 */
function stoneWall(origin: Vector3, along: Vector3, inward: Vector3, length: number, holes: Rect[], stones: Material[]) {
  const alongX = Math.abs(along.x) > 0.5;
  const gap = 0.025;
  let v = 0;
  while (v < H - 0.05) {
    const rowH = Math.min(Random.range(0.42, 0.56), H - v);
    let u = Random.range(-0.3, 0);
    while (u < length) {
      let end = Math.min(u + Random.range(0.55, 1.0), length);
      let next = end;
      let skip = false;
      for (const r of holes) {
        if (r.v1 <= v || r.v0 >= v + rowH) continue;
        if (u >= r.u0 && u < r.u1) {
          next = r.u1;
          skip = true;
        } else if (r.u0 > u && r.u0 < end) {
          end = r.u0;
          next = r.u1;
        }
      }
      const u0 = Math.max(u, 0);
      if (!skip && end - u0 > 0.12) {
        const depth = Random.range(0.05, 0.12);
        const w = end - u0 - gap;
        const h = rowH - gap;
        const mid = origin
          .add(along.scale(u0 + (end - u0) / 2))
          .add(new Vector3(0, v + rowH / 2, 0))
          .add(inward.scale(depth / 2));
        const size = alongX ? new Vector3(w, h, depth) : new Vector3(depth, h, w);
        shape("Stone", "cube", mid, size, Random.pick(stones)!);
      }
      u = next;
    }
    v += rowH;
  }
}

interface Mote {
  go: GameObject;
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  phase: number;
  lit: boolean;
}

/** Whether the sun reaches `p` through the window, between the bars. */
function inSun(x: number, y: number, z: number): boolean {
  if (z >= HZ) return false;
  for (const zp of [HZ, HZ + T]) {
    const t = (zp - z) / TO_SUN.z;
    const wx = x + TO_SUN.x * t;
    const wy = y + TO_SUN.y * t;
    if (wx < WIN.x0 || wx > WIN.x1 || wy < WIN.y0 || wy > WIN.y1) return false;
  }
  const t = (BAR_Z - z) / TO_SUN.z;
  const bx = x + TO_SUN.x * t;
  const by = y + TO_SUN.y * t;
  for (const b of BAR_XS) if (Math.abs(bx - b) < BAR_R) return false;
  for (const b of BAR_YS) if (Math.abs(by - b) < BAR_R) return false;
  return true;
}

class Cell extends Component {
  private camera!: Camera;
  private sun!: Light;
  private bounce!: Light;
  private t = 0;
  dust!: GameObject;
  private motes: Mote[] = [];
  private moteDim!: Material;
  private moteLit: Record<Tint, Material> = {} as Record<Tint, Material>;

  override onStart() {
    scene = this;
    QualitySettings.shadowDistance = 16;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 2;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.012, 0.012, 0.014, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1, intensity: 0.35, scatter: 0.6 };
    Random.initState(1347);

    const sunGo = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN_DIR) });
    this.sun = sunGo.addComponent(Light, { type: "directional", color: TINTS.warm.sun, intensity: SUN_INTENSITY, shadows: "hard" });
    // A faint glow off the pool of light on the floor, standing in for bounced light; it
    // stays out of the fog so the beams are the sun's alone.
    const pool = new Vector3(1.4, 0.35, -0.5);
    const bounceGo = new GameObject("Bounce", { position: pool });
    this.bounce = bounceGo.addComponent(Light, { type: "point", color: TINTS.warm.bounce, intensity: 2.5, range: 6, shadows: "none", volumetricIntensity: 0 });

    this.buildShell();
    this.buildWindow();
    this.buildFloor();
    this.buildCeiling();
    this.buildDoor();
    this.buildProps();
    this.buildDust();

    const eye = new GameObject("Main Camera", { position: new Vector3(-2, 1.6, -3.3) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 70, backgroundColor: TINTS.warm.sky, farClipPlane: 60, nearClipPlane: 0.05 });
    apply();
  }

  /** The thick outer walls, floor and roof that keep the sun out, and the stones on them. */
  private buildShell() {
    const core = Material.create({ color: Color.hex("#2e2a26"), smoothness: 0.05 });
    const stones = ["#5a544c", "#4f4a44", "#625a50", "#544e48", "#48443f", "#5c5248"].map((h) => Material.create({ color: Color.hex(h), smoothness: Random.range(0.08, 0.2) }));

    slab("Floor slab", new Vector3(-HX - T, -0.5, -HZ - T), new Vector3(HX + T, 0, HZ + T), core);
    slab("Left wall", new Vector3(-HX - T, -0.5, -HZ - T), new Vector3(-HX, H + T, HZ + T), core);
    slab("Right wall", new Vector3(HX, -0.5, -HZ - T), new Vector3(HX + T, H + T, HZ + T), core);
    slab("Front wall", new Vector3(-HX, -0.5, -HZ - T), new Vector3(HX, H + T, -HZ), core);
    // The back wall, around the window.
    slab("Back wall low", new Vector3(-HX, -0.5, HZ), new Vector3(HX, WIN.y0, HZ + T), core);
    slab("Back wall high", new Vector3(-HX, WIN.y1, HZ), new Vector3(HX, H + T, HZ + T), core);
    slab("Back wall left", new Vector3(-HX, WIN.y0, HZ), new Vector3(WIN.x0, WIN.y1, HZ + T), core);
    slab("Back wall right", new Vector3(WIN.x1, WIN.y0, HZ), new Vector3(HX, WIN.y1, HZ + T), core);

    const winHole: Rect = { u0: WIN.x0 + HX - 0.18, u1: WIN.x1 + HX + 0.18, v0: WIN.y0 - 0.2, v1: WIN.y1 + 0.25 };
    const doorHole: Rect = { u0: DOOR.z0 + HZ - 0.15, u1: DOOR.z1 + HZ + 0.15, v0: 0, v1: DOOR.y1 + 0.2 };
    const px = new Vector3(1, 0, 0);
    const pz = new Vector3(0, 0, 1);
    stoneWall(new Vector3(-HX, 0, HZ), px, pz.scale(-1), 2 * HX, [winHole], stones);
    stoneWall(new Vector3(-HX, 0, -HZ), px, pz, 2 * HX, [], stones);
    stoneWall(new Vector3(-HX, 0, -HZ), pz, px, 2 * HZ, [], stones);
    stoneWall(new Vector3(HX, 0, -HZ), pz, px.scale(-1), 2 * HZ, [doorHole], stones);
  }

  /** The window: a thick sill and lintel, deep reveals, and iron bars set in the stone. */
  private buildWindow() {
    const sill = Material.create({ color: Color.hex("#6a6258"), smoothness: 0.15 });
    const reveal = Material.create({ color: Color.hex("#57514a"), smoothness: 0.1 });
    // Reveal linings, so the sunlit sides of the opening read as dressed stone.
    slab("Reveal left", new Vector3(WIN.x0 - 0.12, WIN.y0, HZ - 0.12), new Vector3(WIN.x0, WIN.y1, HZ + T), reveal);
    slab("Reveal right", new Vector3(WIN.x1, WIN.y0, HZ - 0.12), new Vector3(WIN.x1 + 0.12, WIN.y1, HZ + T), reveal);
    slab("Sill", new Vector3(WIN.x0 - 0.22, WIN.y0 - 0.16, HZ - 0.22), new Vector3(WIN.x1 + 0.22, WIN.y0, HZ + T + 0.08), sill);
    slab("Lintel", new Vector3(WIN.x0 - 0.22, WIN.y1, HZ - 0.15), new Vector3(WIN.x1 + 0.22, WIN.y1 + 0.22, HZ + T), sill);

    const iron = Material.create({ color: Color.hex("#2a2826"), metallic: 0.7, smoothness: 0.3 });
    const height = WIN.y1 - WIN.y0 + 0.1;
    for (const x of BAR_XS) shape("Bar", "cylinder", new Vector3(x, (WIN.y0 + WIN.y1) / 2, BAR_Z), new Vector3(BAR_R * 2, height / 2, BAR_R * 2), iron);
    const width = WIN.x1 - WIN.x0 + 0.1;
    for (const y of BAR_YS) {
      shape("Crossbar", "cylinder", new Vector3(0, y, BAR_Z), new Vector3(BAR_R * 2, width / 2, BAR_R * 2), iron, undefined, Quaternion.euler(0, 0, 90));
    }
  }

  /** Flagstones in uneven rows, each a slightly different colour and height, and grit. */
  private buildFloor() {
    const flags = ["#4a4640", "#55504a", "#3f3c38", "#4e4842", "#504a40"].map((h) => Material.create({ color: Color.hex(h), smoothness: Random.range(0.1, 0.25) }));
    const gap = 0.025;
    let z = -HZ;
    while (z < HZ - 0.05) {
      const d = Math.min(Random.range(0.6, 0.9), HZ - z);
      let x = -HX + Random.range(-0.4, 0);
      while (x < HX) {
        const end = Math.min(x + Random.range(0.6, 1.05), HX);
        const x0 = Math.max(x, -HX);
        const top = 0.02 + Random.range(0, 0.014);
        shape("Flagstone", "cube", new Vector3((x0 + end) / 2, top - 0.03, z + d / 2), new Vector3(end - x0 - gap, 0.06, d - gap), Random.pick(flags)!);
        x = end;
      }
      z += d;
    }
    // Grit and pebbles, gathered more under the window where the light falls.
    for (let i = 0; i < 24; i++) {
      const near = i < 15;
      const px = near ? 1.4 + Random.range(-1.2, 1.2) : Random.range(-2.8, 2.8);
      const pz = near ? -0.5 + Random.range(-1.2, 1.2) : Random.range(-3.8, 3.8);
      const s = Random.range(0.02, 0.07);
      shape("Grit", "sphere", new Vector3(px, 0.03, pz), new Vector3(s, s * 0.5, s * Random.range(0.7, 1.3)), Random.pick(flags)!);
    }
  }

  /** The roof slab with a drain grate over a dark recess, and wooden beams. */
  private buildCeiling() {
    const core = Material.create({ color: Color.hex("#2e2a26"), smoothness: 0.05 });
    const g = { x0: -1.35, x1: -0.65, z0: 1.15, z1: 1.85 };
    slab("Roof west", new Vector3(-HX - T, H, -HZ - T), new Vector3(g.x0, H + T, HZ + T), core);
    slab("Roof east", new Vector3(g.x1, H, -HZ - T), new Vector3(HX + T, H + T, HZ + T), core);
    slab("Roof south", new Vector3(g.x0, H, -HZ - T), new Vector3(g.x1, H + T, g.z0), core);
    slab("Roof north", new Vector3(g.x0, H, g.z1), new Vector3(g.x1, H + T, HZ + T), core);
    slab("Recess cap", new Vector3(g.x0 - 0.2, H + T, g.z0 - 0.2), new Vector3(g.x1 + 0.2, H + T + 0.3, g.z1 + 0.2), core);

    const iron = Material.create({ color: Color.hex("#2a2826"), metallic: 0.7, smoothness: 0.3 });
    const cx = (g.x0 + g.x1) / 2;
    const cz = (g.z0 + g.z1) / 2;
    for (const k of [-1, 0, 1]) {
      shape("Grate bar", "cube", new Vector3(cx + k * 0.2, H - 0.02, cz), new Vector3(0.04, 0.04, 0.76), iron);
      shape("Grate bar", "cube", new Vector3(cx, H - 0.05, cz + k * 0.2), new Vector3(0.76, 0.04, 0.04), iron);
    }
    const wood = Material.create({ color: Color.hex("#3a2a1c"), smoothness: 0.1 });
    for (const z of [-2.6, 0, 2.6]) shape("Beam", "cube", new Vector3(0, H - 0.13, z), new Vector3(2 * HX, 0.26, 0.28), wood);
  }

  /** A heavy plank door with iron straps, in a stone frame on the right wall. */
  private buildDoor() {
    const frame = Material.create({ color: Color.hex("#635b51"), smoothness: 0.12 });
    const iron = Material.create({ color: Color.hex("#262422"), metallic: 0.75, smoothness: 0.35 });
    const planks = ["#4a3524", "#3e2c1e", "#45311f"].map((h) => Material.create({ color: Color.hex(h), smoothness: 0.12 }));
    const x = HX;
    slab("Jamb", new Vector3(x - 0.14, 0, DOOR.z0 - 0.18), new Vector3(x, DOOR.y1, DOOR.z0), frame);
    slab("Jamb", new Vector3(x - 0.14, 0, DOOR.z1), new Vector3(x - 0, DOOR.y1, DOOR.z1 + 0.18), frame);
    slab("Door lintel", new Vector3(x - 0.16, DOOR.y1, DOOR.z0 - 0.22), new Vector3(x, DOOR.y1 + 0.24, DOOR.z1 + 0.22), frame);
    const n = 5;
    const w = (DOOR.z1 - DOOR.z0) / n;
    for (let i = 0; i < n; i++) {
      const z = DOOR.z0 + w * (i + 0.5);
      shape("Plank", "cube", new Vector3(x - 0.05, DOOR.y1 / 2, z), new Vector3(0.08 + Random.range(0, 0.015), DOOR.y1 - 0.02, w - 0.012), Random.pick(planks)!);
    }
    for (const y of [0.35, 1.08, 1.82]) {
      shape("Strap", "cube", new Vector3(x - 0.1, y, (DOOR.z0 + DOOR.z1) / 2), new Vector3(0.02, 0.08, DOOR.z1 - DOOR.z0 - 0.04), iron);
      shape("Hinge pin", "cylinder", new Vector3(x - 0.08, y, DOOR.z1 - 0.05), new Vector3(0.05, 0.07, 0.05), iron);
    }
    shape("Peephole grille", "cube", new Vector3(x - 0.095, 1.55, (DOOR.z0 + DOOR.z1) / 2), new Vector3(0.015, 0.2, 0.26), iron);
    shape("Lock plate", "cube", new Vector3(x - 0.1, 1.0, DOOR.z0 + 0.14), new Vector3(0.02, 0.18, 0.12), iron);
    ring("Door ring", new Vector3(x - 0.13, 0.9, DOOR.z0 + 0.28), 0.06, 0.016, iron, Quaternion.euler(0, 90, 0));
  }

  /** The straw pallet, the bucket, and two chains with shackles on the left wall. */
  private buildProps() {
    const wood = Material.create({ color: Color.hex("#3e2d1f"), smoothness: 0.12 });
    const straws = ["#b89a52", "#a88a46", "#9c8040", "#c2a560"].map((h) => Material.create({ color: Color.hex(h), smoothness: 0.2 }));
    const iron = Material.create({ color: Color.hex("#2c2622"), metallic: 0.65, smoothness: 0.3 });
    const rust = Material.create({ color: Color.hex("#3d2c22"), metallic: 0.5, smoothness: 0.25 });

    // Straw pallet in the back left corner.
    const px = -2.35;
    const pz = 2.5;
    shape("Pallet", "cube", new Vector3(px, 0.1, pz), new Vector3(1.0, 0.14, 2.0), wood);
    shape("Straw bed", "cube", new Vector3(px, 0.2, pz), new Vector3(0.94, 0.08, 1.92), straws[1]!, undefined, Quaternion.euler(0, 1.5, 0));
    for (let i = 0; i < 34; i++) {
      const at = new Vector3(px + Random.range(-0.55, 0.55), 0.25 + Random.range(0, 0.04), pz + Random.range(-1.05, 1.05));
      const rot = Quaternion.euler(Random.range(-8, 8), Random.range(0, 180), Random.range(-6, 6));
      shape("Straw", "cube", at, new Vector3(0.012, 0.012, Random.range(0.25, 0.6)), Random.pick(straws)!, undefined, rot);
    }

    // A bucket near the light.
    const bucket = new Vector3(2.1, 0, 0.9);
    shape("Bucket", "cylinder", bucket.add(new Vector3(0, 0.21, 0)), new Vector3(0.34, 0.19, 0.34), wood);
    shape("Bucket water", "cylinder", bucket.add(new Vector3(0, 0.401, 0)), new Vector3(0.3, 0.002, 0.3), Material.create({ color: Color.hex("#101010"), smoothness: 0.9 }));
    for (const y of [0.1, 0.32]) shape("Hoop", "cylinder", bucket.add(new Vector3(0, y, 0)), new Vector3(0.352, 0.014, 0.352), iron);

    // Chains with shackles hanging from rings on the left wall.
    const wallX = -HX + 0.1;
    for (const [i, cz] of [-0.1, 0.9].entries()) {
      const top = 2.05;
      shape("Wall plate", "cube", new Vector3(wallX - 0.02, top + 0.05, cz), new Vector3(0.03, 0.16, 0.12), iron);
      ring("Wall ring", new Vector3(wallX + 0.03, top, cz), 0.04, 0.016, iron, Quaternion.euler(0, 0, 0));
      const links = i === 0 ? 9 : 12;
      let y = top - 0.07;
      for (let k = 0; k < links; k++) {
        // Swing the lower links out a little, as if one chain had been tugged.
        const sway = i === 1 ? k * 0.006 : 0;
        link(new Vector3(wallX + 0.03 + sway, y, cz), k % 2 ? 90 : 0, rust);
        y -= 0.058;
      }
      ring("Shackle", new Vector3(wallX + 0.03 + (i === 1 ? links * 0.006 : 0), y - 0.035, cz), 0.06, 0.02, iron, Quaternion.euler(0, links % 2 ? 90 : 0, 0), 10);
    }
  }

  /** Dust motes: tiny unlit specks, dim in the dark and bright where the sun catches them. */
  private buildDust() {
    this.dust = new GameObject("Dust");
    this.moteDim = Material.create({ color: Color.black, emission: new Color(0.012, 0.011, 0.01, 1), unlit: true });
    this.moteLit.warm = Material.create({ color: Color.black, emission: TINTS.warm.mote, unlit: true });
    this.moteLit.teal = Material.create({ color: Color.black, emission: TINTS.teal.mote, unlit: true });
    for (let i = 0; i < 120; i++) {
      const go = shape("Mote", "sphere", Vector3.zero, Vector3.one.scale(Random.range(0.008, 0.016)), this.moteDim, this.dust);
      go.getComponent(MeshRenderer)!.shadowCastingMode = "off";
      const m: Mote = { go, x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0, phase: Random.range(0, 100), lit: false };
      this.spawn(m, i % 5 < 3);
      this.motes.push(m);
    }
  }

  /** Puts a mote somewhere in the cell; `inBeam` puts it along the sunbeam. */
  private spawn(m: Mote, inBeam: boolean) {
    if (inBeam) {
      const s = Random.range(0.3, 5.6);
      const c = new Vector3(Random.range(WIN.x0, WIN.x1), Random.range(WIN.y0, WIN.y1), HZ).add(SUN_DIR.scale(s));
      m.x = c.x + Random.range(-0.25, 0.25);
      m.y = c.y + Random.range(-0.25, 0.25);
      m.z = c.z + Random.range(-0.25, 0.25);
    } else {
      m.x = Random.range(-HX + 0.2, HX - 0.2);
      m.y = Random.range(0.2, H - 0.2);
      m.z = Random.range(-HZ + 0.2, HZ - 0.2);
    }
    m.vx = Random.range(-0.03, 0.03);
    m.vy = Random.range(-0.015, 0.01);
    m.vz = Random.range(-0.03, 0.03);
  }

  applyTint(tint: (typeof TINTS)[Tint]) {
    this.sun.color = tint.sun;
    this.bounce.color = tint.bounce;
    this.camera.backgroundColor = tint.sky;
    for (const m of this.motes) if (m.lit) m.go.getComponent(MeshRenderer)!.material = this.moteLit[state.tint];
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyG")) toggle("godRays");
    if (input.getKeyDown("KeyF")) toggle("fog");
    if (input.getKeyDown("KeyD")) toggle("dust");
    if (input.getKeyDown("KeyT")) cycleTint();

    // A slow sway inside the cell, looking across the beams towards the window.
    const t = this.t;
    const cam = this.camera.transform;
    cam.position = new Vector3(-2 + Math.sin(t * 0.07) * 0.45, 1.6 + Math.sin(t * 0.11) * 0.12, -3.3 + Math.sin(t * 0.05) * 0.25);
    const target = new Vector3(0.4 + Math.sin(t * 0.06) * 0.6, 1.9 + Math.sin(t * 0.09) * 0.15, 2.5);
    cam.rotation = Quaternion.lookRotation(target.sub(cam.position));

    if (!state.dust) return;
    const lit = this.moteLit[state.tint];
    for (const m of this.motes) {
      // Drift on a slow wander, a little swirl on top.
      m.x += (m.vx + Math.sin(t * 0.31 + m.phase) * 0.02) * dt;
      m.y += (m.vy + Math.sin(t * 0.23 + m.phase * 1.7) * 0.012) * dt;
      m.z += (m.vz + Math.cos(t * 0.27 + m.phase * 0.6) * 0.02) * dt;
      if (Math.abs(m.x) > HX - 0.1 || m.y < 0.1 || m.y > H - 0.1 || Math.abs(m.z) > HZ - 0.1) this.spawn(m, Random.value < 0.6);
      m.go.transform.position = new Vector3(m.x, m.y, m.z);
      const sunny = inSun(m.x, m.y, m.z);
      if (sunny !== m.lit) {
        m.lit = sunny;
        m.go.getComponent(MeshRenderer)!.material = sunny ? lit : this.moteDim;
      }
    }
  }
}

function toggle(key: "godRays" | "fog" | "dust") {
  state[key] = !state[key];
  apply();
}
function cycleTint() {
  state.tint = state.tint === "warm" ? "teal" : "warm";
  apply();
}

register({ Cell });

function Button(props: { label: string; value: string; on?: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : "#4d8dff" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#6f7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the cell</text>
        <Button label="God rays" value={state.godRays ? "Sharp (per pixel)" : "Soft (volume only)"} on={state.godRays} keyName="G" onClick={() => toggle("godRays")} />
        <Button label="Fog" value={state.fog ? "On" : "Off"} on={state.fog} keyName="F" onClick={() => toggle("fog")} />
        <Button label="Dust" value={state.dust ? "On" : "Off"} on={state.dust} keyName="D" onClick={() => toggle("dust")} />
        <Button label="Sunlight" value={state.tint === "warm" ? "Warm" : "Teal"} keyName="T" onClick={cycleTint} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
