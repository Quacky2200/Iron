// Fog park: an old London park in a painted, storybook look, built from primitives. Gravel
// paths curve between lawns edged with low hoop-top railings; a pond with lily pads and
// reeds narrows under an arched green footbridge; big clumpy trees and two weeping willows,
// benches facing the water, a bandstand, gas lamps, flower beds and an entrance gate. Beyond
// the trees old London (terraced streets, chimney stacks, spires, a dome) sinks into linear
// distance fog.
// Two moods (N): a misty morning, with soft round mist lying on the pond (ellipsoid fog
// volumes) under an overcast sky, and night, when the gas lamps' spot lights make glowing
// cones in the mist (light shafts). The camera strolls the loop path at head height, over
// the bridge. The menu (or keys) switches the pond mist (M), the distance fog (D), the light
// shafts (L) and the mood (N).
import {
  Camera,
  Color,
  Component,
  Debug,
  FogVolume,
  GameObject,
  input,
  Light,
  Material,
  Mesh,
  MeshRenderer,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Vector3,
  register,
  type BloomSettings,
  type HeightFogSettings,
  type VolumetricFogSettings,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

const hex = Color.hex;

// ---------------------------------------------------------------------------------------
// Moods and toggles

type Mood = "morning" | "night";

interface MoodSettings {
  /** The sky: the camera's background and the fog's colour. */
  sky: Color;
  ambient: Color;
  sunDir: Vector3;
  sunColor: Color;
  sunIntensity: number;
  sunShafts: number;
  fogStart: number;
  fogEnd: number;
  height: HeightFogSettings;
  shafts: VolumetricFogSettings;
  mist: Color;
  bloom: BloomSettings | null;
  lamps: boolean;
}

const MOODS: Record<Mood, MoodSettings> = {
  // Overcast and pale, a gentle sun low in the south-east.
  morning: {
    sky: hex("#b9c6cc"),
    ambient: new Color(0.36, 0.4, 0.45, 1),
    sunDir: new Vector3(-0.45, -0.5, -0.74).normalized,
    sunColor: new Color(1, 0.93, 0.8, 1),
    sunIntensity: 1.35,
    sunShafts: 1,
    fogStart: 40,
    fogEnd: 155,
    height: { density: 0.035, baseHeight: 0, falloff: 0.45 },
    shafts: { intensity: 0.35, anisotropy: 0.3, albedo: Color.white, distance: 60 },
    mist: hex("#e6eaec"),
    bloom: null,
    lamps: false,
  },
  // A dim blue moon, the gas lamps lit.
  night: {
    sky: hex("#141c2c"),
    ambient: new Color(0.01, 0.013, 0.025, 1),
    sunDir: new Vector3(0.35, -0.7, 0.6).normalized,
    sunColor: new Color(0.5, 0.62, 1, 1),
    sunIntensity: 0.1,
    sunShafts: 0.5,
    fogStart: 22,
    fogEnd: 120,
    height: { density: 0.06, baseHeight: 0, falloff: 0.35 },
    shafts: { intensity: 1.4, anisotropy: 0.3, albedo: Color.white, distance: 60 },
    mist: new Color(0.03, 0.038, 0.06, 1),
    bloom: { threshold: 1, intensity: 0.6, scatter: 0.6 },
    lamps: true,
  },
};

/** What the menu shows and changes; `apply` sends it to `RenderSettings` and the scene. */
const state = { mist: true, distance: true, shafts: true, mood: "morning" as Mood };
let refreshMenu = () => {};

/** The scene's parts that change with the mood. */
const world = {
  camera: null as Camera | null,
  sun: null as { go: GameObject; light: Light } | null,
  lamps: [] as { light: Light; glass: MeshRenderer }[],
  mists: [] as { volume: FogVolume; density: number }[],
};

function apply(): void {
  const m = MOODS[state.mood];
  RenderSettings.ambientLight = m.ambient;
  RenderSettings.bloom = m.bloom;
  RenderSettings.fogColor = m.sky;
  RenderSettings.fogMode = "linear";
  RenderSettings.fogStartDistance = m.fogStart;
  RenderSettings.fogEndDistance = m.fogEnd;
  RenderSettings.heightFog = state.mist ? m.height : null;
  RenderSettings.volumetricFog = state.shafts ? m.shafts : null;
  // Height fog and shafts ride on the fog: with the distance fog off, keep the fog on with
  // no density so they still show.
  RenderSettings.fog = true;
  if (!state.distance) {
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogDensity = 0;
  }
  if (world.camera) world.camera.backgroundColor = m.sky;
  if (world.sun) {
    world.sun.go.transform.rotation = Quaternion.lookRotation(m.sunDir);
    world.sun.light.color = m.sunColor;
    world.sun.light.intensity = m.sunIntensity;
    world.sun.light.shadows = state.mood === "morning" ? "soft" : "hard";
    world.sun.light.volumetricIntensity = m.sunShafts;
  }
  for (const lamp of world.lamps) {
    lamp.light.intensity = m.lamps ? 28 : 0;
    lamp.light.volumetricIntensity = m.lamps ? 0.12 : 0;
    // A gas lantern glows all round, with a halo about its glass.
    lamp.light.haloSize = m.lamps ? 1.4 : 0;
    lamp.light.shadows = m.lamps ? "hard" : "none";
    lamp.glass.material = m.lamps ? GLOW : GLASS;
  }
  for (const mist of world.mists) {
    mist.volume.color = m.mist;
    mist.volume.density = state.mist ? mist.density : 0;
  }
  refreshMenu();
}

// ---------------------------------------------------------------------------------------
// Materials: flat, matte, painted colours (sRGB hex)

const mat = (color: string, smoothness = 0.08, metallic = 0) => Material.create({ color: hex(color), smoothness, metallic });

const GRASS = mat("#6f8c40", 0.04);
const GRAVEL = mat("#cfc4a6", 0.05);
const WATER = mat("#2d5650", 0.92, 0.35);
const KERB = mat("#9c9888", 0.1);
const SOIL = mat("#5a4532", 0.02);
const IRON = mat("#1e2b25", 0.35, 0.4);
const GREEN_PAINT = mat("#3e6a4d", 0.3);
const DECK = mat("#8a7458", 0.1);
const BENCH_WOOD = mat("#8b5a33", 0.15);
const CREAM = mat("#eee6d1", 0.15);
const STONE = mat("#c4bba5", 0.1);
const BRICK = mat("#8e5643", 0.05);
const ROOF = mat("#35574a", 0.2);
const BARK = [mat("#5e5548", 0.05), mat("#6b5e4c", 0.05)];
const LEAVES = [mat("#2f5527", 0.03), mat("#456f2f", 0.03), mat("#5f8738", 0.03), mat("#7c9c45", 0.03)];
const WILLOW = [mat("#7f9a4c", 0.03), mat("#93ad5a", 0.03), mat("#a9bf6a", 0.03)];
const SHRUB = [mat("#2f5228", 0.03), mat("#3d6331", 0.03)];
const LILY = [mat("#4f7a3a", 0.3), mat("#5d8a40", 0.3)];
const LILY_FLOWER = [mat("#ec9fb6"), mat("#f3efe4")];
const REED = [mat("#6b7f3a"), mat("#8a8f4a")];
const BULRUSH = mat("#5a3e26");
const FLOWERS = [mat("#f2c12e"), mat("#f2c12e"), mat("#e8a321"), mat("#c73a2c")];
const STALK = mat("#3f6d2f");
// Old London in the haze: soot-dark brick, stone and slate (the fog turns them blue-grey).
const CITY = [mat("#5e5550"), mat("#6b625b"), mat("#7a736b")];
const BRICK_ROW = [mat("#6e4f42"), mat("#5f463b"), mat("#7a5b4a"), mat("#8a7f70")];
const SLATE = mat("#4a4f57", 0.2);
const TERRACE = mat("#d8ccb0", 0.1);
const TERRACE_ROOF = mat("#7c8590", 0.1);
const GLASS = Material.create({ color: hex("#d9dccb"), smoothness: 0.9 });
const GLOW = Material.create({ color: Color.black, emission: new Color(5, 3.6, 1.8, 1), unlit: true });

// ---------------------------------------------------------------------------------------
// Building blocks

let meshes = 0;

function shape(
  name: string,
  mesh: "cube" | "sphere" | "cylinder" | "capsule" | "plane",
  at: Vector3,
  scale: Vector3,
  material: Material,
  rotation: Quaternion = Quaternion.identity,
  parent?: GameObject,
): MeshRenderer {
  const go = new GameObject(name, { position: at, rotation, scale, parent });
  meshes++;
  return go.addComponent(MeshRenderer, { mesh, material });
}

/** A box from `a` to `b`, `w` wide and `h` tall across it. */
function beam(name: string, a: Vector3, b: Vector3, w: number, h: number, material: Material, parent?: GameObject): MeshRenderer {
  const d = b.sub(a);
  const up = Math.abs(d.normalized.y) > 0.99 ? Vector3.forward : Vector3.up;
  return shape(name, "cube", a.add(b).scale(0.5), new Vector3(w, h, d.magnitude), material, Quaternion.lookRotation(d, up), parent);
}

/** A cylinder (or capsule) from `a` to `b`. */
function rod(name: string, a: Vector3, b: Vector3, diameter: number, material: Material, mesh: "cylinder" | "capsule" = "cylinder"): MeshRenderer {
  const d = b.sub(a);
  return shape(name, mesh, a.add(b).scale(0.5), new Vector3(diameter, d.magnitude / 2, diameter), material, Quaternion.fromToRotation(Vector3.up, d));
}

const v = (x: number, y: number, z: number) => new Vector3(x, y, z);
/** The horizontal direction at `deg` degrees about up (0: +z). */
const yawDir = (deg: number) => v(Math.sin((deg * Math.PI) / 180), 0, Math.cos((deg * Math.PI) / 180));
const pick = <T,>(list: T[]): T => list[Math.min(list.length - 1, Math.floor(Random.range(0, list.length)))]!;

/** Calls `fn` every `step` metres along a polyline (from `step / 2`), with the direction there. */
function walk(line: Vector3[], step: number, fn: (at: Vector3, dir: Vector3) => void): void {
  let next = step / 2;
  let run = 0;
  for (let i = 1; i < line.length; i++) {
    const a = line[i - 1]!;
    const seg = line[i]!.sub(a);
    const len = seg.magnitude;
    if (len === 0) continue;
    const dir = seg.scale(1 / len);
    while (next <= run + len) {
      fn(a.add(dir.scale(next - run)), dir);
      next += step;
    }
    run += len;
  }
}

/** A smooth curve through control points (Catmull-Rom), measured by distance along it. */
class Path {
  readonly pts: Vector3[] = [];
  readonly dist: number[] = [0];
  readonly length: number;
  private readonly per = 24;

  constructor(
    ctrl: [number, number][],
    readonly closed: boolean,
  ) {
    const n = ctrl.length;
    const c = (i: number) => {
      const k = closed ? ((i % n) + n) % n : Math.max(0, Math.min(n - 1, i));
      const p = ctrl[k]!;
      return v(p[0], 0, p[1]);
    };
    const segs = closed ? n : n - 1;
    for (let i = 0; i < segs; i++) {
      const [p0, p1, p2, p3] = [c(i - 1), c(i), c(i + 1), c(i + 2)];
      for (let k = 0; k < this.per; k++) {
        const t = k / this.per;
        const t2 = t * t;
        const t3 = t2 * t;
        const f = (a: number, b: number, cc: number, d: number) =>
          0.5 * (2 * b + (-a + cc) * t + (2 * a - 5 * b + 4 * cc - d) * t2 + (-a + 3 * b - 3 * cc + d) * t3);
        this.pts.push(v(f(p0.x, p1.x, p2.x, p3.x), 0, f(p0.z, p1.z, p2.z, p3.z)));
      }
    }
    this.pts.push(closed ? this.pts[0]! : c(n - 1));
    let total = 0;
    for (let i = 1; i < this.pts.length; i++) {
      total += this.pts[i]!.sub(this.pts[i - 1]!).magnitude;
      this.dist.push(total);
    }
    this.length = total;
  }

  /** How far along the curve control point `i` is. */
  distOf(i: number): number {
    return this.dist[i * this.per]!;
  }

  at(d: number): Vector3 {
    const L = this.length;
    d = this.closed ? ((d % L) + L) % L : Math.max(0, Math.min(L, d));
    let lo = 0;
    let hi = this.dist.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (this.dist[mid]! <= d) lo = mid;
      else hi = mid;
    }
    const a = this.dist[lo]!;
    const span = this.dist[hi]! - a;
    return this.pts[lo]!.lerp(this.pts[hi]!, span > 0 ? (d - a) / span : 0);
  }

  dir(d: number): Vector3 {
    return this.at(d + 0.4).sub(this.at(d - 0.4)).normalized;
  }

  /** Points every `step` metres from `from` to `to`, `offset` metres to the side given by `side`. */
  line(from: number, to: number, step: number, offset: number, side: (p: Vector3, n: Vector3) => Vector3): Vector3[] {
    const out: Vector3[] = [];
    for (let d = from; d <= to; d += step) {
      const p = this.at(d);
      const t = this.dir(d);
      out.push(p.add(side(p, v(-t.z, 0, t.x)).scale(offset)));
    }
    return out;
  }
}

// ---------------------------------------------------------------------------------------
// Layout: the pond, the bridge over its neck and the paths

/** The pond as overlapping discs (x, z, radius): a big west lobe, a narrow neck, an east lobe. */
const POND: [number, number, number][] = [
  [-10, -2, 6], [-15, -1, 4.5], [-6, -4, 4.5], [-11, 2, 4], [-5, 0, 3.5], [-17, -4, 3], [-9, -6, 3.5],
  [-1.5, -1, 2.8], [1.5, -1, 2.4], [4, -0.5, 2.6],
  [8, 0, 4.5], [12, 1, 4.5], [11, -2.5, 3.5], [14.5, -1, 3], [9, 3, 3],
];
/** Negative inside the water, else the distance to it. */
function pondDist(x: number, z: number): number {
  let d = Infinity;
  for (const [cx, cz, r] of POND) d = Math.min(d, Math.hypot(x - cx, z - cz) - r);
  return d;
}
/** Where the camera looks a little towards, the middle of the water. */
const POND_LOOK = v(-6, 0.3, -1);

/** The footbridge runs along z at x = BRIDGE.x, rising `rise` metres over the neck. */
const BRIDGE = { x: 1.5, south: 3.5, north: -5.5, rise: 1.1, width: 2.2 };
function onBridge(x: number, z: number): boolean {
  return Math.abs(x - BRIDGE.x) < 1.5 && z < BRIDGE.south && z > BRIDGE.north;
}
/** The deck's height at a point (0 off the bridge). */
function deck(x: number, z: number): number {
  if (!onBridge(x, z)) return 0;
  return BRIDGE.rise * Math.sin((Math.PI * (BRIDGE.south - z)) / (BRIDGE.south - BRIDGE.north));
}

/** The camera's stroll: round the west lobe, over the bridge. */
const LOOP = new Path(
  [[1.5, 7], [1.5, 3.5], [1.5, -5.5], [1.5, -9], [-4, -11.8], [-12, -12.8], [-20, -10.5], [-24.5, -3], [-21.5, 5.5], [-14, 9.6], [-6, 9.6], [-1.2, 8.8]],
  true,
);
const GATE_PATH = new Path([[1.5, 7.2], [8, 9.8], [15, 10], [21, 9], [28.5, 8], [36, 8]], false);
const BAND_PATH = new Path([[-12, -12.6], [-12.3, -15.5], [-12.5, -18.2]], false);
const NORTH_PATH = new Path([[1.5, -9], [3, -17], [0.5, -27], [-3, -42]], false);
const WEST_PATH = new Path([[-24.5, -3], [-31, -4.5], [-39, -2.5], [-48, 1]], false);
const PATHS = [LOOP, GATE_PATH, BAND_PATH, NORTH_PATH, WEST_PATH];
function pathDist(x: number, z: number): number {
  let d = Infinity;
  for (const p of PATHS) for (const q of p.pts) d = Math.min(d, Math.hypot(x - q.x, z - q.z));
  return d;
}

const BANDSTAND = v(-12.5, 0, -21.6);
const GATE = v(28.5, 0, 8);
/** Flower beds: centre x, z and half-axes. */
const BEDS: [number, number, number, number][] = [
  [-14, 13.6, 3.4, 1.5],
  [-4.5, 13.2, 2.6, 1.3],
  [15, 14, 3, 1.4],
];
function inBed(x: number, z: number, margin: number): boolean {
  return BEDS.some(([cx, cz, ax, az]) => ((x - cx) / (ax + margin)) ** 2 + ((z - cz) / (az + margin)) ** 2 < 1);
}

// ---------------------------------------------------------------------------------------
// Pieces

function gravel(p: Path, width: number): void {
  const step = 1.1;
  for (let d = 0; d < p.length - (p.closed ? 0 : 0.2); d += step) {
    const a = p.at(d);
    const b = p.at(p.closed ? d + step : Math.min(d + step, p.length));
    const mid = a.add(b).scale(0.5);
    if (onBridge(mid.x, mid.z)) continue;
    const dir = b.sub(a);
    if (dir.sqrMagnitude < 1e-4) continue;
    shape("Gravel", "cube", v(mid.x, 0.012, mid.z), v(width, 0.024, dir.magnitude + 0.5), GRAVEL, Quaternion.lookRotation(dir));
  }
}

/**
 * A round tube swept along a path in the YZ plane (`sides` around): smooth iron rod for
 * railings, as one mesh.
 */
interface Arrays {
  positions: number[];
  normals: number[];
  triangles: number[];
}

function tube(path: Vector3[], radius: number, sides: number): Mesh {
  const a: Arrays = { positions: [], normals: [], triangles: [] };
  addTube(a, path, radius, sides);
  return Mesh.createFlat(a);
}

/** Adds a tube along `path` (not along x) to `a`. */
function addTube(a: Arrays, path: Vector3[], radius: number, sides: number): void {
  const { positions, normals, triangles } = a;
  const first = positions.length / 3;
  path.forEach((p, i) => {
    const t = path[Math.min(i + 1, path.length - 1)]!.sub(path[Math.max(i - 1, 0)]!).normalized;
    // Any direction not along the path will do for the ring's frame.
    const across = Math.abs(t.x) > 0.9 ? v(0, 0, 1) : v(1, 0, 0);
    const n = t.cross(across).normalized;
    const side = n.cross(t).normalized;
    for (let k = 0; k < sides; k++) {
      const a = (k / sides) * Math.PI * 2;
      const d = n.scale(Math.cos(a)).add(side.scale(Math.sin(a)));
      positions.push(p.x + d.x * radius, p.y + d.y * radius, p.z + d.z * radius);
      normals.push(d.x, d.y, d.z);
    }
    if (i === 0) return;
    const r0 = first + (i - 1) * sides;
    const r1 = first + i * sides;
    for (let k = 0; k < sides; k++) {
      const k1 = (k + 1) % sides;
      triangles.push(r0 + k, r1 + k, r1 + k1, r0 + k, r1 + k1, r0 + k1);
    }
  });
}

/** Spacing of the boundary's brick piers. */
const BAY = 12;
const PLINTH = 0.55;

/**
 * One bay of spear-top railings, as one mesh: round bars every 0.22 m with knobbed tops,
 * between a bottom and a top rail, running along local z.
 */
let railingPanel: Mesh | null = null;
function panel(): Mesh {
  if (railingPanel) return railingPanel;
  const a: Arrays = { positions: [], normals: [], triangles: [] };
  const L = BAY - 0.7;
  const H = 1.45;
  for (let z = -L / 2 + 0.11; z < L / 2; z += 0.22) {
    addTube(a, [v(0, 0, z), v(0, H, z)], 0.016, 6);
    // The spear's knob and point.
    addTube(a, [v(0, H, z), v(0, H + 0.05, z)], 0.032, 6);
    addTube(a, [v(0, H + 0.05, z), v(0, H + 0.13, z)], 0.012, 6);
  }
  for (const y of [0.1, H - 0.12]) addTube(a, [v(0, y, -L / 2), v(0, y, L / 2)], 0.022, 6);
  railingPanel = Mesh.createFlat(a);
  return railingPanel;
}

/** One hoop: two legs into the ground and a round arch, along local z. */
let hoopMesh: Mesh | null = null;
const HOOP_R = 0.35;
function hoop(): Mesh {
  if (hoopMesh) return hoopMesh;
  const LEG = 0.16;
  const path: Vector3[] = [v(0, -0.05, -HOOP_R), v(0, LEG * 0.5, -HOOP_R)];
  for (let k = 0; k <= 20; k++) {
    const a = Math.PI - (k * Math.PI) / 20;
    path.push(v(0, LEG + HOOP_R * Math.sin(a), HOOP_R * Math.cos(a)));
  }
  path.push(v(0, LEG * 0.5, HOOP_R), v(0, -0.05, HOOP_R));
  hoopMesh = tube(path, 0.016, 8);
  return hoopMesh;
}

/**
 * Low hoop-top railings along a polyline, the Victorian park kind: round iron rod bent into
 * arches that overlap, each starting at the middle of the one before. Every hoop draws the
 * same smooth tube mesh.
 */
function hoops(line: Vector3[]): void {
  const mesh = hoop();
  walk(line, HOOP_R, (c, t) => {
    if (pondDist(c.x, c.z) < 0.4) return;
    const go = new GameObject("Railing", { position: c, rotation: Quaternion.lookRotation(t) });
    go.addComponent(MeshRenderer, { mesh, material: IRON });
    meshes++;
  });
}

function ellipse(cx: number, cz: number, ax: number, az: number, n = 48): Vector3[] {
  const out: Vector3[] = [];
  for (let i = 0; i <= n; i++) {
    const a = (i / n) * Math.PI * 2;
    out.push(v(cx + Math.cos(a) * ax, 0, cz + Math.sin(a) * az));
  }
  return out;
}

function pond(): void {
  for (const [x, z, r] of POND) {
    // A stone kerb peeks out round the water's edge.
    shape("Kerb", "cylinder", v(x, 0.0275, z), v((r + 0.35) * 2, 0.0275, (r + 0.35) * 2), KERB);
    shape("Water", "cylinder", v(x, 0.035, z), v(r * 2, 0.035, r * 2), WATER);
  }
  // Lily pads in clusters, a few in flower.
  let clusters = 0;
  for (let tries = 0; clusters < 9 && tries < 200; tries++) {
    const [cx, cz, r] = pick(POND);
    const a = Random.range(0, Math.PI * 2);
    const c = v(cx + Math.cos(a) * r * 0.6, 0, cz + Math.sin(a) * r * 0.6);
    if (pondDist(c.x, c.z) > -1.2 || Math.abs(c.x - BRIDGE.x) < 3) continue;
    clusters++;
    const n = 4 + Math.floor(Random.range(0, 3));
    for (let i = 0; i < n; i++) {
      const p = c.add(v(Random.range(-1, 1), 0, Random.range(-1, 1)));
      if (pondDist(p.x, p.z) > -0.4) continue;
      const d = Random.range(0.35, 0.8);
      shape("Lily pad", "cylinder", v(p.x, 0.074, p.z), v(d, 0.004, d), pick(LILY));
      if (Random.range(0, 1) < 0.25) shape("Lily", "sphere", v(p.x + 0.05, 0.11, p.z), v(0.15, 0.09, 0.15), pick(LILY_FLOWER));
    }
  }
  // Reeds in clumps at the edge.
  let clumps = 0;
  for (let tries = 0; clumps < 16 && tries < 400; tries++) {
    const [cx, cz, r] = pick(POND);
    const a = Random.range(0, Math.PI * 2);
    const out = v(Math.cos(a), 0, Math.sin(a));
    const p = v(cx, 0, cz).add(out.scale(r - 0.15));
    // On the shore line, not inside another disc, and clear of the bridge.
    if (pondDist(p.x + out.x * 0.3, p.z + out.z * 0.3) < 0.1 || Math.abs(p.x - BRIDGE.x) < 3.5) continue;
    clumps++;
    for (let i = 0; i < 5; i++) {
      const base = p.add(v(Random.range(-0.3, 0.3), 0.07, Random.range(-0.3, 0.3)));
      const h = Random.range(0.7, 1.35);
      const top = base.add(out.scale(Random.range(0.05, 0.25))).add(v(Random.range(-0.1, 0.1), h, Random.range(-0.1, 0.1)));
      rod("Reed", base, top, 0.045, pick(REED));
      if (i < 2) rod("Bulrush", top.sub(v(0, 0.28, 0)), top.sub(v(0, 0.06, 0)), 0.08, BULRUSH, "capsule");
    }
  }
}

function bridge(): void {
  const { x, south, north, width } = BRIDGE;
  const N = 12;
  const at = (i: number, y: number, side: number) => {
    const z = south + ((north - south) * i) / N;
    return v(x + side, deck(x, z) + y, z);
  };
  const half = width / 2;
  for (let i = 0; i < N; i++) {
    // The deck, its two green arched girders and the railings with an X lattice.
    beam("Deck", at(i, -0.05, 0), at(i + 1, -0.05, 0), width, 0.1, DECK);
    for (const s of [-1, 1]) {
      const side = s * (half + 0.05);
      beam("Girder", at(i, -0.22, side), at(i + 1, -0.22, side), 0.1, 0.36, GREEN_PAINT);
      beam("Handrail", at(i, 1.0, side), at(i + 1, 1.0, side), 0.08, 0.06, GREEN_PAINT);
      beam("Rail", at(i, 0.1, side), at(i + 1, 0.1, side), 0.05, 0.05, GREEN_PAINT);
      beam("Lattice", at(i, 0.1, side), at(i + 1, 0.97, side), 0.03, 0.03, GREEN_PAINT);
      beam("Lattice", at(i + 1, 0.1, side), at(i, 0.97, side), 0.03, 0.03, GREEN_PAINT);
    }
  }
  for (let i = 0; i <= N; i++) {
    for (const s of [-1, 1]) {
      const end = i === 0 || i === N;
      const p = at(i, 0, s * (half + 0.05));
      // Newel posts at the ends, taller with a ball; plain posts between.
      shape("Post", "cube", p.add(v(0, end ? 0.6 : 0.52, 0)), v(end ? 0.14 : 0.07, end ? 1.2 : 1.04, end ? 0.14 : 0.07), GREEN_PAINT);
      if (end) shape("Post ball", "sphere", p.add(v(0, 1.28, 0)), Vector3.one.scale(0.17), GREEN_PAINT);
    }
  }
  // Stone abutments and landings where it meets the paths.
  for (const z of [south, north]) {
    const out = Math.sign(z - (south + north) / 2);
    shape("Landing", "cube", v(x, 0.03, z + out * 0.4), v(width + 0.6, 0.06, 1.1), STONE);
    for (const s of [-1, 1]) shape("Abutment", "cube", v(x + s * (half + 0.35), 0.3, z + out * 0.2), v(0.5, 0.6, 1.2), STONE);
  }
}

function bench(at: Vector3, face: Vector3): void {
  // Local space: a sitter looks along -z, the back is at +z.
  const b = new GameObject("Bench", { position: at, rotation: Quaternion.lookRotation(v(face.x, 0, face.z)) });
  const id = Quaternion.identity;
  for (let i = 0; i < 4; i++) shape("Seat slat", "cube", v(0, 0.44, -0.2 + i * 0.11), v(1.7, 0.035, 0.085), BENCH_WOOD, id, b);
  for (let i = 0; i < 3; i++) {
    const y = 0.6 + i * 0.13;
    shape("Back slat", "cube", v(0, y, 0.15 + 0.168 * y), v(1.7, 0.09, 0.03), BENCH_WOOD, Quaternion.euler(9.5, 0, 0), b);
  }
  for (const x of [-0.8, 0.8]) {
    // Cast-iron ends: a front leg rising into the arm, a raked back leg, the seat rail.
    shape("Front leg", "cube", v(x, 0.33, -0.22), v(0.06, 0.66, 0.06), IRON, id, b);
    beam("Back leg", v(x, 0, 0.18), v(x, 0.95, 0.34), 0.06, 0.06, IRON, b);
    shape("Seat rail", "cube", v(x, 0.41, 0), v(0.05, 0.05, 0.52), IRON, id, b);
    beam("Arm", v(x, 0.66, -0.27), v(x, 0.69, 0.27), 0.06, 0.045, IRON, b);
    shape("Arm scroll", "sphere", v(x, 0.65, -0.29), Vector3.one.scale(0.1), IRON, id, b);
  }
}

function basket(at: Vector3): void {
  shape("Basket liner", "cylinder", at.add(v(0, 0.36, 0)), v(0.36, 0.3, 0.36), mat("#14130f"));
  for (let i = 0; i < 8; i++) {
    const d = yawDir(i * 45).scale(0.2);
    shape("Basket bar", "cylinder", at.add(d).add(v(0, 0.36, 0)), v(0.03, 0.33, 0.03), IRON);
  }
  shape("Basket rim", "cylinder", at.add(v(0, 0.7, 0)), v(0.46, 0.02, 0.46), IRON);
  shape("Basket foot", "cylinder", at.add(v(0, 0.04, 0)), v(0.44, 0.04, 0.44), IRON);
}

/** A Victorian gas lamp: plinth, post with a ladder bar, a square lantern and its cap. */
function lamp(x: number, z: number): void {
  const parts = [
    shape("Lamp plinth", "cylinder", v(x, 0.22, z), v(0.36, 0.22, 0.36), GREEN_PAINT),
    shape("Lamp flare", "cylinder", v(x, 0.55, z), v(0.22, 0.12, 0.22), GREEN_PAINT),
    shape("Lamp post", "cylinder", v(x, 1.8, z), v(0.11, 1.3, 0.11), GREEN_PAINT),
    shape("Ladder bar", "cube", v(x, 2.6, z), v(0.7, 0.045, 0.045), GREEN_PAINT),
    shape("Lamp collar", "cylinder", v(x, 3.08, z), v(0.2, 0.04, 0.2), GREEN_PAINT),
    shape("Lantern cap", "cube", v(x, 3.6, z), v(0.48, 0.08, 0.48), GREEN_PAINT, Quaternion.euler(0, 45, 0)),
    shape("Lantern top", "cube", v(x, 3.69, z), v(0.28, 0.1, 0.28), GREEN_PAINT),
    shape("Finial", "sphere", v(x, 3.8, z), Vector3.one.scale(0.12), GREEN_PAINT),
  ];
  const glass = shape("Lantern", "cube", v(x, 3.33, z), v(0.32, 0.44, 0.32), GLASS);
  // Nothing of the lamp casts a shadow: its light sits inside the glass, above the post.
  for (const p of [...parts, glass]) p.shadowCastingMode = "off";
  // A gas lantern shines every way (only modern street lights cast a cone downwards).
  const go = new GameObject("Lamp light", { position: v(x, 3.33, z) });
  const light = go.addComponent(Light, { type: "point", color: new Color(1, 0.74, 0.42, 1), intensity: 0, range: 7, shadows: "none", volumetricIntensity: 0 });
  world.lamps.push({ light, glass });
}

function tree(x: number, z: number, size: number): void {
  const h = Random.range(2.6, 4) * size;
  const R = Random.range(2.6, 3.5) * size;
  const tr = Random.range(0.28, 0.42) * size;
  const bark = pick(BARK);
  const top = h + R * 0.45;
  shape("Trunk", "cylinder", v(x, top / 2, z), v(tr * 2, top / 2, tr * 2), bark);
  const cy = h + R * 0.75;
  // A fork or two reaching into the crown.
  if (Random.range(0, 1) < 0.65) {
    for (let k = 0; k < 2; k++) {
      const a = Random.range(0, Math.PI * 2);
      rod("Bough", v(x, h * 0.75, z), v(x + Math.cos(a) * R * 0.5, cy, z + Math.sin(a) * R * 0.5), tr * 1.1, bark);
    }
  }
  const g = Math.floor(Random.range(0, 3));
  const n = 6 + Math.floor(Random.range(0, 3));
  for (let k = 0; k < n; k++) {
    const ox = Random.range(-0.62, 0.62) * R;
    const oz = Random.range(-0.62, 0.62) * R;
    const oy = Random.range(-0.3, 0.45) * R;
    const s = R * Random.range(0.85, 1.25);
    // Clumps catch more light on top and fall into shade below.
    const shade = oy > 0.15 * R ? g + 1 : oy < -0.1 * R ? g - 1 : g;
    shape("Leaves", "sphere", v(x + ox, cy + oy, z + oz), v(s, s * 0.82, s), LEAVES[Math.max(0, Math.min(3, shade))]!);
  }
}

/** Points along a quadratic curve from `a` through control `c` to `b`. */
function curve(a: Vector3, c: Vector3, b: Vector3, n: number): Vector3[] {
  const out: Vector3[] = [];
  for (let i = 0; i <= n; i++) {
    const t = i / n;
    out.push(a.scale((1 - t) * (1 - t)).add(c.scale(2 * t * (1 - t))).add(b.scale(t * t)));
  }
  return out;
}

/**
 * A weeping willow leaning towards `lean`: a sturdy trunk flaring at the foot, boughs arching
 * out from a fork, a broad rounded crown, and curtains of thin leafy fronds hanging from its
 * edge almost to the ground (the fronds are two merged meshes, one per green).
 */
function willow(x: number, z: number, lean: Vector3): void {
  const base = v(x, 0, z);
  const fork = base.add(lean.scale(0.9)).add(v(0, 2.9, 0));
  const bark = BARK[0]!;
  shape("Willow foot", "cylinder", base.add(v(0, 0.3, 0)), v(1.25, 0.3, 1.25), bark);
  rod("Willow trunk", base, fork, 0.8, bark);
  const top = base.add(lean.scale(1.6)).add(v(0, 5.4, 0));
  // Boughs arching up and out from the fork.
  const boughs: Arrays = { positions: [], normals: [], triangles: [] };
  for (let k = 0; k < 6; k++) {
    const a = (k / 6) * Math.PI * 2 + Random.range(-0.3, 0.3);
    const out = v(Math.cos(a), 0, Math.sin(a));
    const end = top.add(out.scale(Random.range(2.2, 3.0))).add(v(0, Random.range(-0.4, 0.3), 0));
    const mid = fork.add(out.scale(1.0)).add(v(0, 2.4, 0));
    addTube(boughs, curve(fork, mid, end, 8), 0.13, 7);
  }
  const bm = Mesh.createFlat(boughs);
  const bgo = new GameObject("Willow boughs");
  bgo.addComponent(MeshRenderer, { mesh: bm, material: bark });
  meshes++;
  // The crown: a broad, rounded dome of overlapping clumps.
  for (let k = 0; k < 7; k++) {
    const a = (k / 7) * Math.PI * 2;
    const r = k === 0 ? 0 : Random.range(1.4, 2.2);
    const p = top.add(v(Math.cos(a) * r, Random.range(0.2, 0.9) - r * 0.25, Math.sin(a) * r));
    const sz = Random.range(2.4, 3.1);
    shape("Willow crown", "sphere", p, v(sz, sz * 0.62, sz), WILLOW[k % WILLOW.length]!);
  }
  // Fronds: thin, curving out from under the crown's edge, then hanging down.
  const fronds: Arrays[] = [
    { positions: [], normals: [], triangles: [] },
    { positions: [], normals: [], triangles: [] },
  ];
  for (let k = 0; k < 220; k++) {
    const a = Random.range(0, Math.PI * 2);
    const r = 1.6 + 2.0 * Math.sqrt(Random.range(0, 1));
    const out = v(Math.cos(a), 0, Math.sin(a));
    // Hang from the crown's underside: higher in the middle, lower at the rim.
    const y0 = top.y + 0.6 - 0.35 * r;
    const start = top.add(out.scale(r)).add(v(0, y0 - top.y, 0));
    const yEnd = Random.range(0.35, 1.6);
    const end = start.add(out.scale(Random.range(0.2, 0.6))).add(v(0, yEnd - y0, 0));
    const ctl = start.add(out.scale(Random.range(0.35, 0.7))).add(v(0, -0.25, 0));
    addTube(fronds[k % 2]!, curve(start, ctl, end, 6), Random.range(0.035, 0.06), 5);
  }
  fronds.forEach((f, i) => {
    const go = new GameObject("Willow fronds");
    go.addComponent(MeshRenderer, { mesh: Mesh.createFlat(f), material: WILLOW[i % WILLOW.length]! });
    meshes++;
  });
}

function shrub(x: number, z: number, size: number): void {
  const s = Random.range(1.1, 1.7) * size;
  shape("Shrub", "sphere", v(x, s * 0.32, z), v(s, s * 0.75, s), pick(SHRUB));
  shape("Shrub", "sphere", v(x + s * 0.35, s * 0.25, z + Random.range(-0.3, 0.3) * s), v(s * 0.7, s * 0.55, s * 0.7), pick(SHRUB));
}

function bandstand(C: Vector3): void {
  const T = Math.tan(Math.PI / 8);
  /** An octagonal slab: 8 boxes from the centre to each side. */
  const octa = (name: string, apothem: number, y: number, h: number, m: Material) => {
    for (let i = 0; i < 8; i++) {
      const d = yawDir(i * 45);
      shape(name, "cube", C.add(d.scale(apothem / 2)).add(v(0, y, 0)), v(2 * apothem * T + 0.02, h, apothem), m, Quaternion.euler(0, i * 45, 0));
    }
  };
  const corner = (i: number, r: number, y: number) => C.add(yawDir(i * 45 + 22.5).scale(r)).add(v(0, y, 0));
  octa("Plinth", 3.6, 0.45, 0.9, STONE);
  octa("Coping", 3.75, 0.92, 0.06, CREAM);
  octa("Floor", 3.5, 0.96, 0.04, DECK);
  // Steps up from the path (the +z side).
  for (let k = 0; k < 3; k++) {
    const top = 0.3 * (k + 1);
    shape("Step", "cube", C.add(v(0, top / 2, 3.6 + 0.4 * (3 - k) - 0.2)), v(2.2, top, 0.42), STONE);
  }
  const rc = 3.25 / Math.cos(Math.PI / 8);
  for (let i = 0; i < 8; i++) {
    shape("Column", "cylinder", corner(i, rc, 2.47), v(0.16, 1.47, 0.16), CREAM);
    shape("Column base", "cube", corner(i, rc, 1.06), v(0.26, 0.12, 0.26), CREAM);
    shape("Capital", "cube", corner(i, rc, 3.92), v(0.26, 0.1, 0.26), CREAM);
    // The frieze under the eaves, and a balustrade on every side but the steps'.
    beam("Frieze", corner(i - 1, rc, 4.05), corner(i, rc, 4.05), 0.12, 0.32, CREAM);
    if (i === 0) continue;
    const a = corner(i - 1, rc, 0);
    const b = corner(i, rc, 0);
    beam("Balustrade", a.add(v(0, 1.85, 0)), b.add(v(0, 1.85, 0)), 0.08, 0.07, CREAM);
    beam("Balustrade", a.add(v(0, 1.1, 0)), b.add(v(0, 1.1, 0)), 0.06, 0.05, CREAM);
    for (const t of [0.25, 0.5, 0.75]) {
      const p = a.lerp(b, t);
      rod("Baluster", p.add(v(0, 1.1, 0)), p.add(v(0, 1.85, 0)), 0.045, GREEN_PAINT);
    }
  }
  // The roof: eight facets rising to a point, each in four narrowing bands.
  const a0 = 4.15;
  const h = 2.6;
  const base = 4.2;
  const pitch = (Math.atan2(h, a0) * 180) / Math.PI;
  const slant = Math.hypot(a0, h);
  octa("Ceiling", 3.95, 4.17, 0.05, CREAM);
  for (let i = 0; i < 8; i++) {
    const d = yawDir(i * 45);
    for (let k = 0; k < 4; k++) {
      const t0 = k / 4;
      const tm = (k + 0.5) / 4;
      const at = C.add(d.scale(a0 * (1 - tm))).add(v(0, base + h * tm, 0));
      shape("Roof", "cube", at, v(2 * a0 * (1 - t0) * T + 0.06, 0.1, slant / 4 + 0.1), ROOF, Quaternion.euler(pitch, i * 45, 0));
    }
    const ra = a0 / Math.cos(Math.PI / 8);
    beam("Fascia", corner(i - 1, ra, base), corner(i, ra, base), 0.1, 0.16, CREAM);
  }
  shape("Finial", "cylinder", C.add(v(0, base + h + 0.3, 0)), v(0.14, 0.35, 0.14), ROOF);
  shape("Finial ball", "sphere", C.add(v(0, base + h + 0.72, 0)), Vector3.one.scale(0.3), CREAM);
  shape("Finial spike", "cylinder", C.add(v(0, base + h + 1.05, 0)), v(0.04, 0.25, 0.04), ROOF);
  // A clipped hedge behind it, and shrubs.
  for (let i = -3; i <= 3; i++) {
    const a = 180 + i * 22;
    const d = yawDir(a);
    shape("Hedge", "cube", C.add(d.scale(7)).add(v(0, 0.55, 0)), v(2.8, 1.1, 0.9), SHRUB[1]!, Quaternion.euler(0, a, 0));
  }
}

function gate(G: Vector3): void {
  for (const s of [-1, 1]) {
    // Brick pillars with stone caps and balls, either side of the path.
    const p = G.add(v(0, 0, s * 2.3));
    shape("Pillar plinth", "cube", p.add(v(0, 0.2, 0)), v(0.95, 0.4, 0.95), STONE);
    shape("Pillar", "cube", p.add(v(0, 1.5, 0)), v(0.78, 2.3, 0.78), BRICK);
    shape("Pillar cap", "cube", p.add(v(0, 2.73, 0)), v(0.98, 0.16, 0.98), STONE);
    shape("Pillar cap", "cube", p.add(v(0, 2.87, 0)), v(0.72, 0.14, 0.72), STONE);
    shape("Pillar ball", "sphere", p.add(v(0, 3.2, 0)), Vector3.one.scale(0.55), STONE);
    // A gate leaf hinged on the pillar, standing half open into the park; its top rises
    // towards the pillar.
    const leaf = new GameObject("Gate", { position: G.add(v(-0.05, 0, s * 1.9)), rotation: Quaternion.euler(0, 55 * s, 0) });
    const id = Quaternion.identity;
    for (let k = 0; k < 12; k++) {
      const hk = 2.0 + 0.4 * (1 - k / 11) ** 2;
      shape("Gate bar", "cylinder", v(0, hk / 2, -s * (0.06 + k * 0.155)), v(0.035, hk / 2, 0.035), IRON, id, leaf);
    }
    for (const y of [0.15, 1.0, 1.9]) shape("Gate rail", "cylinder", v(0, y, -s * 0.92), v(0.05, 0.93, 0.05), IRON, Quaternion.euler(90, 0, 0), leaf);
    // The boundary, right along that side of the park: a low brick wall with brick piers
    // every bay, spear-top railings on it, and a hedge behind.
    const mesh = panel();
    for (let k = 0; k < 6; k++) {
      const start = 2.75 + k * BAY;
      const mid = start + BAY / 2;
      shape("Boundary wall", "cube", G.add(v(0, PLINTH / 2, s * mid)), v(0.45, PLINTH, BAY), BRICK);
      shape("Wall coping", "cube", G.add(v(0, PLINTH + 0.04, s * mid)), v(0.55, 0.08, BAY), STONE);
      const rail = new GameObject("Railing", { position: G.add(v(0, PLINTH + 0.08, s * mid)) });
      rail.addComponent(MeshRenderer, { mesh, material: IRON });
      meshes++;
      const pier = G.add(v(0, 0, s * (start + BAY)));
      shape("Pier", "cube", pier.add(v(0, 1.0, 0)), v(0.62, 2.0, 0.62), BRICK);
      shape("Pier cap", "cube", pier.add(v(0, 2.06, 0)), v(0.76, 0.12, 0.76), STONE);
      shape("Pier ball", "sphere", pier.add(v(0, 2.3, 0)), Vector3.one.scale(0.34), STONE);
    }
    shape("Hedge", "cube", G.add(v(1.3, 0.95, s * (2.75 + 3 * BAY))), v(1.2, 1.9, 6 * BAY), SHRUB[0]!);
  }
}

function bed(cx: number, cz: number, ax: number, az: number): void {
  shape("Bed", "cylinder", v(cx, 0.025, cz), v(ax * 2, 0.025, az * 2), SOIL);
  const n = Math.round(ax * az * 4);
  for (let i = 0; i < n / 4; i++) {
    const a = Random.range(0, Math.PI * 2);
    const r = Math.sqrt(Random.range(0, 1)) * 0.8;
    shape("Leaves", "sphere", v(cx + Math.cos(a) * ax * r, 0.1, cz + Math.sin(a) * az * r), v(0.6, 0.3, 0.6), STALK);
  }
  for (let i = 0; i < n; i++) {
    const a = Random.range(0, Math.PI * 2);
    const r = Math.sqrt(Random.range(0, 1)) * 0.88;
    const p = v(cx + Math.cos(a) * ax * r, 0.04, cz + Math.sin(a) * az * r);
    const h = Random.range(0.25, 0.42);
    rod("Stalk", p, p.add(v(0, h, 0)), 0.025, STALK);
    shape("Flower", "sphere", p.add(v(0, h, 0)), v(0.14, 0.11, 0.14), pick(FLOWERS));
  }
  hoops(ellipse(cx, cz, ax + 0.35, az + 0.35));
}

/** A terraced row along `dir` from `at`: brick fronts, a pitched slate roof, chimney stacks. */
function terraceRow(at: Vector3, dir: number, length: number): void {
  const h = Random.range(9, 15);
  const depth = Random.range(8, 11);
  const yaw = Quaternion.euler(0, dir, 0);
  // The row's length runs along the box's z, which the yaw turns to `dir`.
  const along = yawDir(dir);
  const brick = pick(BRICK_ROW);
  shape("Terrace", "cube", at.add(v(0, h / 2, 0)), v(depth, h, length), brick, yaw);
  // The roof: a box turned 45° about the row's length, half of it showing as a gable.
  const r = depth / Math.SQRT2;
  shape("Roof", "cube", at.add(v(0, h, 0)), v(r, r, length), SLATE, Quaternion.euler(0, dir, 0).mul(Quaternion.euler(0, 0, 45)));
  // Chimney stacks along the ridge, each with a row of pots.
  for (let d = -length / 2 + 3; d < length / 2 - 2; d += Random.range(5, 8)) {
    // On the ridge, reaching well down into the roof so it never floats.
    const c = at.add(along.scale(d)).add(v(0, h + depth / 2 - 1.2, 0));
    shape("Chimney stack", "cube", c, v(1.1, 4.4, 2.2), brick, yaw);
    for (const o of [-0.6, 0, 0.6]) shape("Chimney pot", "cylinder", c.add(along.scale(o)).add(v(0, 2.5, 0)), v(0.35, 0.35, 0.35), SLATE);
  }
}

function skyline(): void {
  // Victorian London beyond the park: low terraced streets, mostly north, the landmarks
  // rising above them.
  for (let i = 0; i < 34; i++) {
    const a = Random.range(0, 1) < 0.8 ? -Math.PI / 2 + Random.range(-1.3, 1.3) : Random.range(0, Math.PI * 2);
    const r = Random.range(95, 140);
    const at = v(Math.cos(a) * r, 0, Math.sin(a) * r);
    // Rows run across the view (roughly square to the line from the park).
    const facing = (Math.atan2(at.x, at.z) * 180) / Math.PI;
    terraceRow(at, facing + Random.range(-20, 20), Random.range(22, 46));
  }
  const city = CITY[0]!;
  // A great dome on a drum, with a lantern and two towers in front.
  const D = v(-28, 0, -122);
  shape("Dome base", "cube", D.add(v(0, 10, 0)), v(36, 20, 26), city);
  shape("Drum", "cylinder", D.add(v(0, 25, 0)), v(17, 5, 17), city);
  shape("Dome", "sphere", D.add(v(0, 30, 0)), v(18, 20, 18), city);
  shape("Lantern", "cylinder", D.add(v(0, 41.5, 0)), v(3, 1.75, 3), city);
  shape("Lantern ball", "sphere", D.add(v(0, 44.5, 0)), Vector3.one.scale(2), city);
  for (const s of [-1, 1]) {
    shape("Tower", "cube", D.add(v(s * 14, 16, 15)), v(5, 32, 5), city);
    shape("Tower top", "cylinder", D.add(v(s * 14, 34, 15)), v(3.5, 2, 3.5), city);
  }
  // Church spires.
  for (const [x, z] of [[30, -112], [68, -126], [-70, -116], [5, -138], [-105, -60]] as const) {
    const m = pick(CITY);
    shape("Church tower", "cube", v(x, 13, z), v(6, 26, 6), m);
    shape("Belfry", "cylinder", v(x, 28, z), v(4, 2, 4), m);
    shape("Spire", "cylinder", v(x, 34, z), v(2.6, 4, 2.6), m);
    shape("Spire", "cylinder", v(x, 41, z), v(1.3, 3.5, 1.3), m);
    shape("Spire", "cylinder", v(x, 47, z), v(0.5, 3, 0.5), m);
  }
  // A clock tower.
  const K = v(56, 0, -104);
  shape("Clock tower", "cube", K.add(v(0, 24, 0)), v(7, 48, 7), city);
  shape("Belfry", "cube", K.add(v(0, 51, 0)), v(8.5, 6, 8.5), city);
  shape("Tower roof", "cube", K.add(v(0, 55, 0)), v(6, 2, 6), city);
  shape("Tower roof", "cube", K.add(v(0, 57, 0)), v(3.6, 2, 3.6), city);
  shape("Tower spire", "cylinder", K.add(v(0, 61, 0)), v(1, 3, 1), city);
  // Cream stucco terraces along the park's east side, past the gate.
  for (let i = 0; i < 5; i++) {
    const z = -24 + i * 13;
    shape("Terrace", "cube", v(92, 7, z), v(12, 14, 12.6), TERRACE);
    shape("Terrace roof", "cube", v(92, 14, z), v(8.5, 8.5, 12.6), TERRACE_ROOF, Quaternion.euler(0, 0, 45));
  }
}

// ---------------------------------------------------------------------------------------
// The scene

/** Where the stroll starts: on the south shore, heading east towards the bridge. */
const START = LOOP.distOf(9) + 2;
const SPEED = 1.1;

class Park extends Component {
  private camera!: Camera;
  private drift!: GameObject;
  private rot: Quaternion | null = null;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 60;
    RenderSettings.ambientMode = "color";
    RenderSettings.tonemapping = "neutral";
    Random.initState(1898);

    const sun = new GameObject("Sun");
    world.sun = { go: sun, light: sun.addComponent(Light, { type: "directional", shadows: "soft" }) };

    shape("Ground", "plane", v(0, 0, 0), Vector3.one.scale(50), GRASS);
    pond();
    for (const p of PATHS) gravel(p, p === LOOP ? 2.4 : 2);
    bridge();

    // Hoop-top railings edging the lawns on the pond side of the north and south shores.
    const toPond = (p: Vector3, n: Vector3) => (pondDist(p.x + n.x * 2, p.z + n.z * 2) < pondDist(p.x - n.x * 2, p.z - n.z * 2) ? n : n.scale(-1));
    hoops(LOOP.line(LOOP.distOf(3) + 0.5, LOOP.distOf(5) - 2.5, 0.5, 1.6, toPond));
    hoops(LOOP.line(LOOP.distOf(8) - 2, LOOP.distOf(10) + 1, 0.5, 1.6, toPond));
    for (const [cx, cz, ax, az] of BEDS) bed(cx, cz, ax, az);

    // Benches facing the water, a waste basket by the first.
    bench(v(-8.5, 0, 11.6), v(0, 0, -1));
    basket(v(-7.2, 0, 11.75));
    bench(v(-7.5, 0, -14.3), v(0, 0, 1));
    bench(v(12.5, 0, 8.0), v(0, 0, -1));
    bench(v(-25.8, 0, 1.4), v(1, 0, -0.1));

    bandstand(BANDSTAND);
    gate(GATE);

    // Gas lamps: either end of the bridge, along the loop, by the gate and the bandstand.
    for (const [x, z] of [[-0.3, 4.8], [3.3, -6.8], [-4.5, -13.6], [-23.3, -10.2], [-23.6, 5.2], [-11.5, 11.4], [25.5, 10.4], [-10.4, -16.6]] as const) lamp(x, z);

    willow(8, -7.6, v(0, 0, 1));
    willow(17.3, 2.8, v(-1, 0, -0.2).normalized);

    // Big trees on the lawns and a belt of them round the park.
    const trees: [number, number, number][] = [];
    for (let tries = 0; trees.length < 56 && tries < 1500; tries++) {
      const x = Random.range(-62, 62);
      const z = Random.range(-58, 46);
      const size = Random.range(0.95, 1.4);
      if (pondDist(x, z) < 3.5 || pathDist(x, z) < 3.2) continue;
      if (BANDSTAND.sub(v(x, 0, z)).magnitude < 9 || inBed(x, z, 2) || (x > 23 && x < 34 && Math.abs(z - GATE.z) < 16)) continue;
      if (trees.some(([tx, tz, ts]) => Math.hypot(tx - x, tz - z) < 5.5 * Math.max(ts, size))) continue;
      trees.push([x, z, size]);
      tree(x, z, size);
    }
    // Round shrubs near the paths.
    for (let n = 0, tries = 0; n < 22 && tries < 600; tries++) {
      const x = Random.range(-30, 26);
      const z = Random.range(-28, 20);
      const pd = pathDist(x, z);
      if (pd < 3.2 || pd > 5.5 || pondDist(x, z) < 2 || inBed(x, z, 0.8) || BANDSTAND.sub(v(x, 0, z)).magnitude < 5) continue;
      if (trees.some(([tx, tz]) => Math.hypot(tx - x, tz - z) < 2)) continue;
      n++;
      shrub(x, z, Random.range(0.8, 1.2));
    }
    skyline();

    // Mist over the pond: a soft round layer on each lobe, spilling onto the banks, and a
    // thinner, wider one drifting over both.
    const mist = (name: string, at: Vector3, size: Vector3, density: number, blend: number) => {
      const go = new GameObject(name, { position: at });
      const volume = go.addComponent(FogVolume, { size, density, color: MOODS.morning.mist, blendDistance: blend, shape: "ellipsoid" });
      world.mists.push({ volume, density });
      return go;
    };
    mist("Pond mist (west)", v(-10.5, -0.1, -1.5), v(26, 2.4, 20), 1.2, 2.5);
    mist("Pond mist (east)", v(11, -0.1, 0), v(19, 2.2, 15), 1.1, 2.4);
    this.drift = mist("Drifting mist", v(-4, 0, -1), v(44, 3.2, 30), 0.35, 3);

    const eye = new GameObject("Main Camera", { position: v(0, 1.6, 0) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: MOODS.morning.sky, farClipPlane: 400 });
    world.camera = this.camera;
    apply();
    Debug.log(`fog-park: ${meshes} mesh renderers`);
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyM")) toggle("mist");
    if (input.getKeyDown("KeyD")) toggle("distance");
    if (input.getKeyDown("KeyL")) toggle("shafts");
    if (input.getKeyDown("KeyN")) toggleMood();
    // The thin mist wanders slowly over the water.
    this.drift.transform.position = v(-4 + Math.sin(this.t * 0.05) * 4, 0, -1 + Math.sin(this.t * 0.07 + 1) * 2);

    // A stroll along the loop at head height, looking ahead and a little towards the water.
    const s = START + this.t * SPEED;
    const p = LOOP.at(s);
    const pos = v(p.x, 1.6 + deck(p.x, p.z) + Math.sin(this.t * 0.9) * 0.015, p.z);
    const a = LOOP.at(s + 6);
    const ahead = v(a.x, 1.45 + deck(a.x, a.z), a.z);
    const look = ahead.sub(pos).normalized.scale(0.72).add(POND_LOOK.sub(pos).normalized.scale(0.28));
    const target = Quaternion.lookRotation(look);
    this.rot = this.rot ? this.rot.slerp(target, 1 - Math.exp(-dt * 1.8)) : target;
    const cam = this.camera.transform;
    cam.position = pos;
    cam.rotation = this.rot;
  }
}

function toggle(key: "mist" | "distance" | "shafts") {
  state[key] = !state[key];
  apply();
}
function toggleMood() {
  state.mood = state.mood === "morning" ? "night" : "morning";
  apply();
}

register({ Park });

// ---------------------------------------------------------------------------------------
// Menu

function Button(props: { label: string; value: string; on?: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 120, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : "#4d8dff" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#6f7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

const onOff = (on: boolean) => (on ? "On" : "Off");

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Fog park</text>
        <Button label="Mood" value={state.mood === "morning" ? "Misty morning" : "Night"} keyName="N" onClick={toggleMood} />
        <Button label="Pond mist" value={onOff(state.mist)} on={state.mist} keyName="M" onClick={() => toggle("mist")} />
        <Button label="Distance fog" value={onOff(state.distance)} on={state.distance} keyName="D" onClick={() => toggle("distance")} />
        <Button label="Light shafts" value={onOff(state.shafts)} on={state.shafts} keyName="L" onClick={() => toggle("shafts")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
