// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Endless blocks in the fog: a Minecraft-like world that never ends. Terrain comes from
// seeded noise, built per 16×16-column chunk as one textured mesh holding only the faces
// you can see, so thousands of blocks are a few dozen draws. Chunks stream in and out in a
// ring around the camera; the fog is the render distance, as in Minecraft, so the world's
// edge never shows. A spare ring of chunks lies past the fog's end: fog culling skips
// drawing it (the HUD counts what it saved). The camera wanders on its own, forever.
// Keys: 1 to 4 pick a render preset (very small, medium, large, far: no fog and a big
// area), [ and ] fine-tune the distance, C toggles fog culling, F the fog, V vsync, Space
// pauses the flight. Every 300 frames the log gets the preset, what was drawn and culled
// and the average frame time, to compare them.

const CHUNK = 16;
const WATER = 11;
const SKY = Color.hex("#a9c8f5");
/** Milliseconds a frame may spend building chunks (nearest first; at least one). */
const BUILD_BUDGET_MS = 6;

/** Render presets: the fog's render distance in chunks, or no fog over a big area. */
const PRESETS = [
  { name: "Very small", distance: 2, fog: true },
  { name: "Medium", distance: 6, fog: true },
  { name: "Large", distance: 10, fog: true },
  { name: "Far (no fog)", distance: 14, fog: false },
];
/** The preset it starts with. */
const START_PRESET = 1;

// ---- Terrain: seeded value noise ----

function hash(x: number, z: number, seed: number): number {
  let h = (Math.imul(x, 374761393) + Math.imul(z, 668265263) + Math.imul(seed, 1442695041)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

function noise(x: number, z: number, seed: number): number {
  const xi = Math.floor(x);
  const zi = Math.floor(z);
  const fx = x - xi;
  const fz = z - zi;
  const u = fx * fx * (3 - 2 * fx);
  const v = fz * fz * (3 - 2 * fz);
  const a = hash(xi, zi, seed);
  const b = hash(xi + 1, zi, seed);
  const c = hash(xi, zi + 1, seed);
  const d = hash(xi + 1, zi + 1, seed);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}

function fbm(x: number, z: number, seed: number): number {
  let sum = 0;
  let amp = 0.5;
  let freq = 1;
  for (let o = 0; o < 4; o++) {
    sum += noise(x * freq, z * freq, seed + o * 17) * amp;
    amp *= 0.5;
    freq *= 2;
  }
  return sum / 0.9375;
}

/** The terrain's height at a column: rolling hills, with mountains in places. */
function height(x: number, z: number): number {
  const hills = fbm(x / 70, z / 70, 1);
  const mountains = Math.max(0, fbm(x / 260, z / 260, 2) - 0.55) * 110;
  return Math.max(2, Math.floor(8 + hills * 20 + mountains));
}

/** Whether a tree grows from this column (never near a chunk's edge, so it stays in it). */
function treeAt(x: number, z: number, h: number): boolean {
  const lx = ((x % CHUNK) + CHUNK) % CHUNK;
  const lz = ((z % CHUNK) + CHUNK) % CHUNK;
  return lx >= 2 && lx <= 13 && lz >= 2 && lz <= 13 && h > WATER + 1 && h < 34 && hash(x, z, 9) < 0.012;
}

// ---- The texture atlas: 16-pixel tiles, made in code ----

const enum Tile {
  GrassTop,
  GrassSide,
  Dirt,
  Stone,
  Sand,
  Water,
  Wood,
  Leaves,
}
const TILES = 8;
const PX = 16;

function atlas(): Texture {
  let seed = 7;
  const rnd = () => (seed = (Math.imul(seed, 1103515245) + 12345) & 0x7fffffff) / 0x7fffffff;
  const pixels: Color[] = new Array(TILES * PX * PX);
  const shade = (hex: string, spread: number) => {
    const c = Color.hex(hex);
    const f = 1 + (rnd() - 0.5) * spread;
    return new Color(c.r * f, c.g * f, c.b * f, 1);
  };
  for (let tile = 0; tile < TILES; tile++) {
    // Grass over the side's top edge, a few pixels deep with uneven drips.
    const grassDepth = Array.from({ length: PX }, () => 3 + Math.floor(rnd() * 3));
    for (let y = 0; y < PX; y++) {
      for (let x = 0; x < PX; x++) {
        let c: Color;
        switch (tile as Tile) {
          case Tile.GrassTop:
            c = shade("#6aaa3a", 0.35);
            break;
          case Tile.GrassSide:
            c = y < grassDepth[x]! ? shade("#6aaa3a", 0.3) : shade("#8a6142", 0.35);
            break;
          case Tile.Dirt:
            c = shade("#8a6142", 0.35);
            break;
          case Tile.Stone:
            c = shade(rnd() < 0.15 ? "#6f6f6f" : "#8d8d8d", 0.25);
            break;
          case Tile.Sand:
            c = shade("#dccf9a", 0.15);
            break;
          case Tile.Water:
            c = shade("#3d6fd6", 0.15);
            break;
          case Tile.Wood:
            c = shade(x % 4 === 0 ? "#5a4026" : "#6e5032", 0.2);
            break;
          case Tile.Leaves:
            c = shade(rnd() < 0.25 ? "#2d5a1c" : "#3f7a28", 0.3);
            break;
        }
        // `y` counts down from the tile's top; `Texture.create` takes the bottom row first.
        pixels[(PX - 1 - y) * TILES * PX + tile * PX + x] = c;
      }
    }
  }
  return Texture.create(TILES * PX, PX, pixels, { filterMode: "point", mipmaps: false, wrapMode: "clamp" });
}

// ---- Meshing ----

/** The six faces: outward normal and corners (wound so they face out), with each corner's
 * place across the face (u) and up it (v). */
interface Face {
  n: number[];
  corners: number[][];
  uv: number[][];
}

function faceCorner(axis: number, fixed: number, ua: number, va: number, u: number, v: number): number[] {
  const p = [0, 0, 0];
  p[axis] = fixed;
  p[ua] = u;
  p[va] = v;
  return p;
}

function makeFaces(): Face[] {
  const out: Face[] = [];
  const dirs: number[][] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
  for (const n of dirs) {
    let axis = 0;
    for (let k = 0; k < 3; k++) if (n[k] !== 0) axis = k;
    // Across and up: sides use y as up; tops and bottoms use x and z.
    const ua = axis === 1 ? 0 : axis === 0 ? 2 : 0;
    const va = axis === 1 ? 2 : 1;
    const fixed = n[axis]! > 0 ? 1 : 0;
    let quad: number[][] = [[0, 0], [1, 0], [1, 1], [0, 1]];
    // Wind counter-clockwise seen from outside: flip if the first triangle faces in.
    const a = faceCorner(axis, fixed, ua, va, quad[0]![0]!, quad[0]![1]!);
    const b = faceCorner(axis, fixed, ua, va, quad[1]![0]!, quad[1]![1]!);
    const c = faceCorner(axis, fixed, ua, va, quad[2]![0]!, quad[2]![1]!);
    const e1 = [b[0]! - a[0]!, b[1]! - a[1]!, b[2]! - a[2]!];
    const e2 = [c[0]! - a[0]!, c[1]! - a[1]!, c[2]! - a[2]!];
    const cross = [e1[1]! * e2[2]! - e1[2]! * e2[1]!, e1[2]! * e2[0]! - e1[0]! * e2[2]!, e1[0]! * e2[1]! - e1[1]! * e2[0]!];
    if (cross[0]! * n[0]! + cross[1]! * n[1]! + cross[2]! * n[2]! < 0) quad = [quad[0]!, quad[3]!, quad[2]!, quad[1]!];
    const corners: number[][] = [];
    for (const q of quad) corners.push(faceCorner(axis, fixed, ua, va, q[0]!, q[1]!));
    out.push({ n, corners, uv: quad });
  }
  return out;
}

const FACES: Face[] = makeFaces();

interface ChunkMesh {
  positions: Float32Array;
  normals: Float32Array;
  uv: Float32Array;
  triangles: Uint32Array;
  blocks: number;
}

/** Each face's corners (x, y, z four times), normal, and corners' places on the tile. */
const FACE_CORNERS: Float32Array[] = [];
const FACE_UV: Float32Array[] = [];
for (const f of FACES) {
  const c: number[] = [];
  for (const p of f.corners) c.push(p[0]!, p[1]!, p[2]!);
  FACE_CORNERS.push(new Float32Array(c));
  const t: number[] = [];
  for (const p of f.uv) t.push(p[0]!, p[1]!);
  FACE_UV.push(new Float32Array(t));
}
const SIDES = [0, 1, 4, 5];

/**
 * Growable buffers a chunk's faces are written into, reused from chunk to chunk (writing by
 * index into typed arrays is far quicker in QuickJS than pushing onto plain arrays).
 */
const buf: { pos: Float32Array; nrm: Float32Array; uv: Float32Array; tri: Uint32Array; faces: number } = {
  pos: new Float32Array(4096 * 12),
  nrm: new Float32Array(4096 * 12),
  uv: new Float32Array(4096 * 8),
  tri: new Uint32Array(4096 * 6),
  faces: 0,
};

function growFloats(a: Float32Array): Float32Array {
  const b = new Float32Array(a.length * 2);
  b.set(a);
  return b;
}
function growInts(a: Uint32Array): Uint32Array {
  const b = new Uint32Array(a.length * 2);
  b.set(a);
  return b;
}
function grow(): void {
  buf.pos = growFloats(buf.pos);
  buf.nrm = growFloats(buf.nrm);
  buf.uv = growFloats(buf.uv);
  buf.tri = growInts(buf.tri);
}

function face(f: number, x: number, y: number, z: number, tile: number): void {
  if ((buf.faces + 1) * 6 > buf.tri.length) grow();
  const c = FACE_CORNERS[f]!;
  const t = FACE_UV[f]!;
  const n = FACES[f]!.n;
  const v = buf.faces * 4;
  const p3 = v * 3;
  const p2 = v * 2;
  for (let i = 0; i < 4; i++) {
    const o = p3 + i * 3;
    buf.pos[o] = x + c[i * 3]!;
    buf.pos[o + 1] = y + c[i * 3 + 1]!;
    buf.pos[o + 2] = z + c[i * 3 + 2]!;
    buf.nrm[o] = n[0]!;
    buf.nrm[o + 1] = n[1]!;
    buf.nrm[o + 2] = n[2]!;
    // Inset a hair so neighbouring tiles never bleed in.
    buf.uv[p2 + i * 2] = (tile + 0.02 + t[i * 2]! * 0.96) / TILES;
    buf.uv[p2 + i * 2 + 1] = 0.01 + t[i * 2 + 1]! * 0.98;
  }
  const k = buf.faces * 6;
  buf.tri[k] = v;
  buf.tri[k + 1] = v + 1;
  buf.tri[k + 2] = v + 2;
  buf.tri[k + 3] = v;
  buf.tri[k + 4] = v + 2;
  buf.tri[k + 5] = v + 3;
  buf.faces++;
}

/** One chunk's visible faces, in its own space (columns 0..15 from its corner). */
function buildChunk(cx: number, cz: number): ChunkMesh {
  buf.faces = 0;
  let blocks = 0;
  const x0 = cx * CHUNK;
  const z0 = cz * CHUNK;
  // Heights with a one-column border, for the faces along the chunk's edges.
  const W = CHUNK + 2;
  const hs = new Int32Array(W * W);
  for (let z = -1; z <= CHUNK; z++) for (let x = -1; x <= CHUNK; x++) hs[(z + 1) * W + (x + 1)] = height(x0 + x, z0 + z);
  // Trees: their blocks, by local position.
  const tree = new Map<number, Tile>();
  const key = (x: number, y: number, z: number) => (y * 64 + (z + 8)) * 64 + (x + 8);
  for (let z = 0; z < CHUNK; z++) {
    for (let x = 0; x < CHUNK; x++) {
      const h = hs[(z + 1) * W + (x + 1)]!;
      if (!treeAt(x0 + x, z0 + z, h)) continue;
      const trunk = 4 + Math.floor(hash(x0 + x, z0 + z, 11) * 2);
      for (let y = h; y < h + trunk; y++) tree.set(key(x, y, z), Tile.Wood);
      for (let dy = -2; dy <= 1; dy++) {
        const r = dy < 0 ? 2 : 1;
        for (let dz = -r; dz <= r; dz++)
          for (let dx = -r; dx <= r; dx++) {
            if (Math.abs(dx) === r && Math.abs(dz) === r && dy !== -2) continue;
            const k = key(x + dx, h + trunk + dy, z + dz);
            if (!tree.has(k)) tree.set(k, Tile.Leaves);
          }
      }
    }
  }
  for (let z = 0; z < CHUNK; z++) {
    for (let x = 0; x < CHUNK; x++) {
      const h = hs[(z + 1) * W + (x + 1)]!;
      const sandy = h <= WATER + 1;
      const rocky = h >= 40;
      blocks += h;
      face(2, x, h - 1, z, sandy ? Tile.Sand : rocky ? Tile.Stone : Tile.GrassTop);
      // Sides down to each lower neighbour.
      for (let s = 0; s < 4; s++) {
        const f = SIDES[s]!;
        const n = FACES[f]!.n;
        const nh = hs[(z + n[2]! + 1) * W + (x + n[0]! + 1)]!;
        for (let y = nh > 0 ? nh : 0; y < h; y++) {
          const depth = h - 1 - y;
          face(f, x, y, z, sandy && depth < 3 ? Tile.Sand : rocky || depth >= 4 ? Tile.Stone : depth === 0 ? Tile.GrassSide : Tile.Dirt);
        }
      }
      // Water over low ground: a flat surface a little below the block top.
      if (h < WATER) face(2, x, WATER - 1.12, z, Tile.Water);
    }
  }
  const solid = (x: number, y: number, z: number) => {
    if (x >= -1 && x <= CHUNK && z >= -1 && z <= CHUNK && y < hs[(z + 1) * W + (x + 1)]!) return true;
    return tree.has(key(x, y, z));
  };
  for (const [k, tile] of tree) {
    const x = (k % 64) - 8;
    const z = (Math.floor(k / 64) % 64) - 8;
    const y = Math.floor(k / 4096);
    blocks++;
    for (let f = 0; f < 6; f++) {
      const n = FACES[f]!.n;
      if (!solid(x + n[0]!, y + n[1]!, z + n[2]!)) face(f, x, y, z, tile);
    }
  }
  const verts = buf.faces * 4;
  return {
    positions: buf.pos.slice(0, verts * 3),
    normals: buf.nrm.slice(0, verts * 3),
    uv: buf.uv.slice(0, verts * 2),
    triangles: buf.tri.slice(0, buf.faces * 6),
    blocks,
  };
}

// ---- Streaming ----

interface Chunk {
  cx: number;
  cz: number;
  go: GameObject;
  mesh: Mesh;
  blocks: number;
}

/** The settings the HUD shows and the keys change. */
const state = { preset: START_PRESET, distance: PRESETS[START_PRESET]!.distance, culling: true, fog: PRESETS[START_PRESET]!.fog, paused: false };
let refreshHud = () => {};

function applyFog(): void {
  RenderSettings.fog = state.fog;
  RenderSettings.fogMode = "linear";
  RenderSettings.fogColor = SKY;
  // The fog closes in over the last 40% of the render distance, as in Minecraft.
  RenderSettings.fogEndDistance = state.distance * CHUNK;
  RenderSettings.fogStartDistance = state.distance * CHUNK * 0.6;
  RenderSettings.fogCulling = state.culling;
  refreshHud();
}

class World extends Component {
  private camera!: Camera;
  private chunks = new Map<string, Chunk>();
  private pool: Chunk[] = [];
  private material!: Material;
  private pos = new Vector3(8, 40, 8);
  private heading = 0;
  private altitude = 40;
  private t = 0;
  fps = 60;
  blocks = 0;
  private sampleTime = 0;
  private sampleFrames = 0;
  /** Chunks built since the last log line (a sample with any isn't steady). */
  private built = 0;
  get loaded(): number {
    return this.chunks.size;
  }

  override onStart(): void {
    world = this;
    QualitySettings.shadowDistance = 60;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.55, 0.6, 0.7, 1);
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0.4, -0.75, 0.3).normalized) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.96, 0.88, 1), intensity: 1.3, shadows: "soft" });
    this.material = Material.create({ mainTexture: atlas(), smoothness: 0 });
    const eye = new GameObject("Main Camera", { position: this.pos });
    this.camera = eye.addComponent(Camera, { fieldOfView: 70, backgroundColor: SKY, farClipPlane: 400 });
    // Start above the ground, looking ahead (paused or not).
    this.altitude = this.groundAhead(new Vector3(1, 0, 0)) + 9;
    this.fly(0);
    applyFog();
    // Fill the nearest chunks at once, so the first frame isn't empty.
    this.stream(300);
  }

  /** The highest ground (or water) over the next 40 m along `dir`. */
  private groundAhead(dir: Vector3): number {
    let ground = WATER;
    for (let k = 0; k <= 40; k += 8) ground = Math.max(ground, height(Math.floor(this.pos.x + dir.x * k), Math.floor(this.pos.z + dir.z * k)));
    return ground;
  }

  /** Moves the camera `dt` seconds along its heading, keeping above the ground ahead. */
  private fly(dt: number): void {
    const dir = new Vector3(Math.cos(this.heading), 0, Math.sin(this.heading));
    const speed = 9;
    this.pos = this.pos.add(dir.scale(speed * dt));
    this.altitude += (this.groundAhead(dir) + 9 - this.altitude) * Math.min(1, dt * 0.8);
    this.pos = new Vector3(this.pos.x, this.altitude, this.pos.z);
    const cam = this.camera.transform;
    cam.position = this.pos;
    cam.rotation = Quaternion.lookRotation(dir.add(new Vector3(0, -0.22, 0)));
  }

  /** Chunks kept loaded: the render distance, plus a spare ring past the fog. */
  private radius(): number {
    return state.fog ? state.distance + 2 : state.distance;
  }

  /** Loads and drops chunks around the camera, building for at most `budget` ms. */
  private stream(budget: number): void {
    const ccx = Math.floor(this.pos.x / CHUNK);
    const ccz = Math.floor(this.pos.z / CHUNK);
    const r = this.radius();
    // Drop chunks that fell out of range (their objects and meshes are reused).
    for (const [k, c] of this.chunks) {
      if ((c.cx - ccx) ** 2 + (c.cz - ccz) ** 2 > (r + 1) ** 2) {
        c.go.enabled = false;
        this.chunks.delete(k);
        this.blocks -= c.blocks;
        this.pool.push(c);
      }
    }
    // Missing ones, nearest first.
    const missing: [number, number, number][] = [];
    for (let dz = -r; dz <= r; dz++)
      for (let dx = -r; dx <= r; dx++) {
        const d = dx * dx + dz * dz;
        if (d <= r * r && !this.chunks.has(`${ccx + dx},${ccz + dz}`)) missing.push([d, ccx + dx, ccz + dz]);
      }
    missing.sort((a, b) => a[0] - b[0]);
    const start = Date.now();
    for (const [, cx, cz] of missing) {
      if (Date.now() - start > budget) break;
      this.built++;
      const data = buildChunk(cx, cz);
      let c = this.pool.pop();
      if (c) {
        c.mesh.setFlat(data);
        c.go.transform.position = new Vector3(cx * CHUNK, 0, cz * CHUNK);
        c.go.enabled = true;
        c.cx = cx;
        c.cz = cz;
        c.blocks = data.blocks;
      } else {
        const mesh = Mesh.createFlat(data);
        const go = new GameObject(`Chunk`, { position: new Vector3(cx * CHUNK, 0, cz * CHUNK) });
        go.addComponent(MeshRenderer, { mesh, material: this.material });
        c = { cx, cz, go, mesh, blocks: data.blocks };
      }
      this.chunks.set(`${cx},${cz}`, c);
      this.blocks += data.blocks;
    }
  }

  override onUpdate(dt: number): void {
    this.t += dt;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.05;
    for (let i = 0; i < PRESETS.length; i++) if (input.getKeyDown(`Digit${i + 1}`)) setPreset(i);
    if (input.getKeyDown("Space")) {
      state.paused = !state.paused;
      refreshHud();
    }
    if (input.getKeyDown("BracketLeft")) setDistance(state.distance - 1);
    if (input.getKeyDown("BracketRight")) setDistance(state.distance + 1);
    if (input.getKeyDown("KeyC")) toggle("culling");
    if (input.getKeyDown("KeyF")) toggle("fog");
    if (input.getKeyDown("KeyV")) {
      QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
      refreshHud();
    }
    this.sampleTime += dt;
    if (++this.sampleFrames === 300) {
      const preset = PRESETS[state.preset]!;
      const ccx = Math.floor(this.pos.x / CHUNK);
      const ccz = Math.floor(this.pos.z / CHUNK);
      let gaps = 0;
      for (let dz = -state.distance; dz <= state.distance; dz++)
        for (let dx = -state.distance; dx <= state.distance; dx++)
          if (dx * dx + dz * dz <= state.distance * state.distance && !this.chunks.has(`${ccx + dx},${ccz + dz}`)) gaps++;
      Debug.log(
        `fog-world: ${preset.name}, ${state.distance} chunks, fog ${state.fog ? "on" : "off"}, culling ${state.culling ? "on" : "off"}: ` +
          `${this.chunks.size} chunks, ${(this.blocks / 1e6).toFixed(2)}M blocks, ${RenderStats.meshes} drawn, ${RenderStats.culledMeshes} culled ` +
          `(${RenderStats.fogCulled} by fog), ${gaps} chunks missing in view range, ${((this.sampleTime / 300) * 1000).toFixed(2)} ms a frame${this.built > 0 ? `, still loading (${this.built} built)` : ""}`,
      );
      this.sampleTime = 0;
      this.sampleFrames = 0;
      this.built = 0;
    }

    // The autopilot: a slow, wandering heading, flying above the ground ahead.
    if (state.paused) {
      this.stream(BUILD_BUDGET_MS);
      if (Math.floor(this.t * 4) !== Math.floor((this.t - dt) * 4)) refreshHud();
      return;
    }
    this.heading += (noise(this.t * 0.05, 0.5, 4) - 0.5) * dt * 1.2;
    this.fly(dt);

    this.stream(BUILD_BUDGET_MS);
    if (Math.floor(this.t * 4) !== Math.floor((this.t - dt) * 4)) refreshHud();
  }
}

let world: World | null = null;

function setDistance(d: number) {
  state.distance = Math.max(2, Math.min(16, d));
  applyFog();
}
function setPreset(i: number) {
  state.preset = i;
  state.distance = PRESETS[i]!.distance;
  state.fog = PRESETS[i]!.fog;
  applyFog();
}
function toggle(key: "culling" | "fog") {
  if (key === "culling") state.culling = !state.culling;
  else state.fog = !state.fog;
  applyFog();
}

register({ World });

// ---- HUD ----

function Row(props: { label: string; value: string; keyName: string; onClick?: () => void; on?: boolean }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 120, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: props.onClick ? "pointer" : "default", minWidth: 150, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : props.onClick ? "#4d8dff" : "rgba(255,255,255,0.06)" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#6f7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Hud() {
  const [, setTick] = useState(0);
  refreshHud = () => setTick((t) => t + 1);
  const w = world;
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Endless blocks in the fog</text>
        <Row label="Preset" value={PRESETS[state.preset]!.name} keyName="1-4" onClick={() => setPreset((state.preset + 1) % PRESETS.length)} />
        <Row label="Render distance" value={`${state.distance} chunks (${state.distance * CHUNK} m)`} keyName="[ ]" onClick={() => setDistance(state.distance + 1)} />
        <Row label="Fog culling" value={state.culling ? "On" : "Off"} on={state.culling} keyName="C" onClick={() => toggle("culling")} />
        <Row label="Fog" value={state.fog ? "On" : "Off"} on={state.fog} keyName="F" onClick={() => toggle("fog")} />
        <Row label="Flight" value={state.paused ? "Paused" : "Flying"} on={!state.paused} keyName="Space" onClick={() => {
          state.paused = !state.paused;
          refreshHud();
        }} />
        <Row label="Chunks loaded" value={`${w?.loaded ?? 0} (${Math.round((w?.blocks ?? 0) / 1000)}k blocks)`} keyName="" />
        <Row label="Meshes drawn" value={`${RenderStats.meshes} (culled ${RenderStats.culledMeshes})`} keyName="" />
        <Row label="Hidden by fog" value={`${RenderStats.fogCulled}`} keyName="" />
        <Row label="Frame rate" value={`${Math.round(w?.fps ?? 0)} FPS (vsync ${QualitySettings.vSyncCount > 0 ? "on" : "off"})`} keyName="V" />
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
