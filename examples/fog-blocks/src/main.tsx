// Fog blocks: a hill of grass blocks in pale sky-blue fog, after foggy Minecraft, drawn like
// examples/fog-world: pixel-art tiles from a texture made in code, each 16×16-column chunk one
// mesh of the faces you can see. Distance fog
// in the sky's colour fades far blocks into it; height fog settles in the valleys and round
// the blocks' bases; a flat fog volume is a deck of low cloud with clear air above. The camera
// flies a loop: low among the blocks, up through the cloud to look down on the hilltop poking
// out of it, and back down. Keys: F distance fog, H height fog, C cloud deck, Space pauses.
import {
  Camera,
  Color,
  Component,
  FogVolume,
  GameObject,
  input,
  Light,
  Material,
  Mathf,
  Mesh,
  MeshRenderer,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Texture,
  Vector3,
  register,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

const SKY = Color.hex("#c3d8fb");
const N = 44; // columns along each side
const CHUNK = 16;
const HALF = N / 2;
const HILL = new Vector3(3, 0, -2); // the hilltop's column
const CLOUD = { y: 6, size: new Vector3(600, 5, 600), density: 0.5, blend: 2 };
const FLIGHT_SECONDS = 70; // one loop, low to high and back

/** What the menu shows and changes; `apply` sends it to `RenderSettings` and the cloud. */
const state = { fog: true, height: true, cloud: true, paused: false };
let cloud: FogVolume | null = null;
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fogColor = SKY;
  RenderSettings.fogMode = "exponentialSquared";
  // Height fog rides on the fog: with distance fog off, keep the fog on at no density.
  RenderSettings.fogDensity = state.fog ? 0.026 : 0;
  RenderSettings.heightFog = state.height ? { density: 0.12, baseHeight: 1, falloff: 0.32 } : null;
  RenderSettings.fog = state.fog || state.height;
  if (cloud) cloud.density = state.cloud ? CLOUD.density : 0;
  refreshMenu();
}

/** The top of the column at (x, z), in whole blocks: a big hill, a ridge and some swell. */
function height(x: number, z: number): number {
  const hill = 12 * Math.exp(-((x - HILL.x) ** 2 + (z - HILL.z) ** 2) / (2 * 8.5 * 8.5));
  const ridge = 4 * Math.exp(-((x + z * 0.6 + 10) ** 2) / 40);
  const waves = 1.4 * Math.sin(x * 0.31 + 1.3) * Math.cos(z * 0.27 - 0.4) + 0.8 * Math.sin((x + z) * 0.17);
  return Math.max(1, Math.round(2 + hill + ridge + waves));
}

/** The column's centre in world space; off the grid it's ground level (0). */
const colX = (i: number) => i - HALF + 0.5;
const columnHeight = (i: number, j: number) => (i < 0 || j < 0 || i >= N || j >= N ? 0 : height(colX(i), colX(j)));

let cubes = 0;

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

// ---- Meshing (as in examples/fog-world) ----

/** The six faces: outward normal and corners (wound so they face out), with each corner's
 * place across the face (u) and up it (v). */
interface Face {
  n: [number, number, number];
  corners: [number, number, number][];
  uv: [number, number][];
}

const FACES: Face[] = (() => {
  const out: Face[] = [];
  const dirs: [number, number, number][] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
  for (const n of dirs) {
    const axis = n.findIndex((c) => c !== 0);
    // Across and up: sides use y as up; tops and bottoms use x and z.
    const [ua, va] = axis === 1 ? [0, 2] : [axis === 0 ? 2 : 0, 1];
    const fixed = n[axis]! > 0 ? 1 : 0;
    let quad: [number, number][] = [[0, 0], [1, 0], [1, 1], [0, 1]];
    const corner = ([u, v]: [number, number]) => {
      const p: [number, number, number] = [0, 0, 0];
      p[axis] = fixed;
      p[ua] = u;
      p[va] = v;
      return p;
    };
    // Wind counter-clockwise seen from outside: flip if the first triangle faces in.
    const [a, b, c] = [corner(quad[0]!), corner(quad[1]!), corner(quad[2]!)];
    const e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    const cross = [e1[1]! * e2[2]! - e1[2]! * e2[1]!, e1[2]! * e2[0]! - e1[0]! * e2[2]!, e1[0]! * e2[1]! - e1[1]! * e2[0]!];
    if (cross[0]! * n[0] + cross[1]! * n[1] + cross[2]! * n[2] < 0) quad = [quad[0]!, quad[3]!, quad[2]!, quad[1]!];
    out.push({ n, corners: quad.map(corner), uv: quad });
  }
  return out;
})();

interface ChunkMesh {
  positions: Float32Array;
  normals: Float32Array;
  uv: Float32Array;
  triangles: Uint32Array;
  blocks: number;
}

/** Each face's corners (x, y, z four times), normal, and corners' places on the tile. */
const FACE_CORNERS = FACES.map((f) => new Float32Array(f.corners.flat()));
const FACE_UV = FACES.map((f) => new Float32Array(f.uv.flat()));
const SIDES = [0, 1, 4, 5];

/**
 * Growable buffers a chunk's faces are written into, reused from chunk to chunk (writing by
 * index into typed arrays is far quicker in QuickJS than pushing onto plain arrays).
 */
const buf = {
  pos: new Float32Array(4096 * 12),
  nrm: new Float32Array(4096 * 12),
  uv: new Float32Array(4096 * 8),
  tri: new Uint32Array(4096 * 6),
  faces: 0,
};

function grow(): void {
  const more = <T extends Float32Array | Uint32Array>(a: T): T => {
    const b = new (a.constructor as new (n: number) => T)(a.length * 2);
    b.set(a);
    return b;
  };
  buf.pos = more(buf.pos);
  buf.nrm = more(buf.nrm);
  buf.uv = more(buf.uv);
  buf.tri = more(buf.tri);
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
    buf.nrm[o] = n[0];
    buf.nrm[o + 1] = n[1];
    buf.nrm[o + 2] = n[2];
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

class Blocks extends Component {
  private camera!: Camera;
  private t = 0;
  /** The low pass's height: level, clearing the tallest ground anywhere on its circle. */
  private low = 0;

  override onStart() {
    QualitySettings.shadowDistance = 60;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.42, 0.43, 0.46, 1);
    RenderSettings.tonemapping = "none";

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.45, -0.75, 0.5).normalized) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.95, 0.85, 1), intensity: 1.7, shadows: "soft" });

    Random.initState(7);
    // Trees: one on the hilltop (it pokes out of the cloud), the rest scattered.
    const tree = new Map<number, Tile>();
    const key = (i: number, y: number, j: number) => (y * 128 + (j + 8)) * 128 + (i + 8);
    const spots: [number, number][] = [[Math.floor(HILL.x + HALF), Math.floor(HILL.z + HALF)]];
    while (spots.length < 8) {
      const sp: [number, number] = [Random.rangeInt(3, N - 3), Random.rangeInt(3, N - 3)];
      if (spots.every(([a, b]) => Math.abs(a - sp[0]) + Math.abs(b - sp[1]) > 6)) spots.push(sp);
    }
    for (const [i, j] of spots) {
      const ground = columnHeight(i, j);
      const trunk = Random.rangeInt(4, 6);
      for (let k = 0; k < trunk; k++) tree.set(key(i, ground + k, j), Tile.Wood);
      for (let dy = -2; dy <= 1; dy++) {
        const r = dy < 0 ? 2 : 1;
        for (let dx = -r; dx <= r; dx++) {
          for (let dz = -r; dz <= r; dz++) {
            if (Math.abs(dx) === r && Math.abs(dz) === r && (r === 2 || dy === 1)) continue; // round the corners
            if (dx === 0 && dz === 0 && dy < 0) continue; // the trunk
            tree.set(key(i + dx, ground + trunk + dy, j + dz), Tile.Leaves);
          }
        }
      }
    }
    const solid = (i: number, y: number, j: number) => y < columnHeight(i, j) || tree.has(key(i, y, j));

    // The hill in 16×16-column chunks, each one mesh: grass tops, dirt sides with a grass rim down to
    // the lowest neighbour, stone deeper down; then the trees' blocks.
    const material = Material.create({ mainTexture: atlas(), smoothness: 0 });
    for (let ci = 0; ci < N; ci += CHUNK) {
      for (let cj = 0; cj < N; cj += CHUNK) {
        buf.faces = 0;
        for (let i = ci; i < Math.min(N, ci + CHUNK); i++) {
          for (let j = cj; j < Math.min(N, cj + CHUNK); j++) {
            const h = columnHeight(i, j);
            const x = i - HALF;
            const z = j - HALF;
            cubes += h;
            face(2, x, h - 1, z, Tile.GrassTop);
            for (let k = 0; k < 4; k++) {
              const f = SIDES[k]!;
              const n = FACES[f]!.n;
              for (let y = Math.max(columnHeight(i + n[0], j + n[2]), 0); y < h; y++) {
                const depth = h - 1 - y;
                face(f, x, y, z, depth === 0 ? Tile.GrassSide : depth >= 4 ? Tile.Stone : Tile.Dirt);
              }
            }
          }
        }
        for (const [k, tile] of tree) {
          const i = (k % 128) - 8;
          const j = (Math.floor(k / 128) % 128) - 8;
          const y = Math.floor(k / 16384);
          if (i < ci || i >= ci + CHUNK || j < cj || j >= cj + CHUNK) continue;
          cubes++;
          for (let f = 0; f < 6; f++) {
            const n = FACES[f]!.n;
            if (!solid(i + n[0], y + n[1], j + n[2])) face(f, i - HALF, y, j - HALF, tile);
          }
        }
        const verts = buf.faces * 4;
        const mesh = Mesh.createFlat({
          positions: buf.pos.slice(0, verts * 3),
          normals: buf.nrm.slice(0, verts * 3),
          uv: buf.uv.slice(0, verts * 2),
          triangles: buf.tri.slice(0, buf.faces * 6),
        });
        new GameObject("Chunk").addComponent(MeshRenderer, { mesh, material });
      }
    }

    // The cloud deck: a wide, flat box of fog at about the hilltop's height.
    const deck = new GameObject("Cloud deck", { position: new Vector3(0, CLOUD.y, 0) });
    cloud = deck.addComponent(FogVolume, { size: CLOUD.size, density: CLOUD.density, color: Color.hex("#e4edfd"), blendDistance: CLOUD.blend });

    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: SKY, farClipPlane: 250 });
    console.log(`fog-blocks: ${cubes} cubes`);
    apply();
    // The low circle (radius 16 round the hill) at one steady height, clear of every
    // column within two of it, so the camera glides level instead of bobbing over blocks.
    for (let k = 0; k < 360; k++) {
      const a = (k / 360) * Math.PI * 2;
      const i = Math.floor(HILL.x + Math.cos(a) * 16 + HALF);
      const j = Math.floor(HILL.z + Math.sin(a) * 16 + HALF);
      for (let di = -2; di <= 2; di++) for (let dj = -2; dj <= 2; dj++) this.low = Math.max(this.low, columnHeight(i + di, j + dj) + 2.4);
    }
    this.fly();
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyF")) toggle("fog");
    if (input.getKeyDown("KeyH")) toggle("height");
    if (input.getKeyDown("KeyC")) toggle("cloud");
    if (input.getKeyDown("Space")) toggle("paused");
    if (!state.paused) this.t += dt;
    this.fly();
  }

  /** Circle the hill, rising from among the blocks to above the cloud and back down. */
  private fly() {
    const phase = (this.t / FLIGHT_SECONDS) * Math.PI * 2;
    const up = Mathf.smoothStep(0, 1, (1 - Math.cos(phase)) / 2); // 0 low, 1 high
    const angle = this.t * 0.07 + 2.2;
    const radius = Mathf.lerp(16, 34, up);
    const x = HILL.x + Math.cos(angle) * radius;
    const z = HILL.z + Math.sin(angle) * radius;
    // Level on the low pass, rising smoothly to the high one.
    const position = new Vector3(x, Mathf.lerp(this.low, 30, up), z);
    const target = new Vector3(HILL.x, Mathf.lerp(9, 11, up), HILL.z);
    const cam = this.camera.transform;
    cam.position = position;
    cam.rotation = Quaternion.lookRotation(target.sub(position));
  }
}

function toggle(key: keyof typeof state) {
  state[key] = !state[key];
  apply();
}

register({ Blocks });

function Button(props: { label: string; value: string; on?: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 110, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : "#4d8dff" }}
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
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Blocks in the clouds</text>
        <Button label="Distance fog" value={state.fog ? "On" : "Off"} on={state.fog} keyName="F" onClick={() => toggle("fog")} />
        <Button label="Height fog" value={state.height ? "On" : "Off"} on={state.height} keyName="H" onClick={() => toggle("height")} />
        <Button label="Cloud deck" value={state.cloud ? "On" : "Off"} on={state.cloud} keyName="C" onClick={() => toggle("cloud")} />
        <Button label="Flight" value={state.paused ? "Paused" : "Flying"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
