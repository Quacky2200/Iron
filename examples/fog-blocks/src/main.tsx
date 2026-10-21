// Fog blocks: a hill of grass blocks in pale sky-blue fog, after foggy Minecraft. Distance fog
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

const SKY = Color.hex("#c3d8fb");
const N = 44; // columns along each side
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
function block(at: Vector3, material: Material, scale?: Vector3): GameObject {
  const go = new GameObject("Block", { position: at, scale });
  go.addComponent(MeshRenderer, { mesh: "cube", material });
  cubes++;
  return go;
}

function shades(base: Color, count: number, spread: number): Material[] {
  const out: Material[] = [];
  for (let k = 0; k < count; k++) {
    const f = 1 + (k / Math.max(1, count - 1) - 0.5) * spread;
    out.push(Material.create({ color: new Color(base.r * f, base.g * f, base.b * f, 1), smoothness: 0.05 }));
  }
  return out;
}

class Blocks extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 60;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.42, 0.43, 0.46, 1);
    RenderSettings.tonemapping = "none";

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.45, -0.75, 0.5).normalized) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.95, 0.85, 1), intensity: 1.7, shadows: "soft" });

    Random.initState(7);
    const grass = shades(new Color(0.2, 0.42, 0.1, 1), 3, 0.25);
    const dirt = shades(new Color(0.33, 0.22, 0.13, 1), 3, 0.25);
    const pick = (list: Material[]) => list[Random.rangeInt(0, list.length)]!;

    // Each column: the top dirt block, a grass slab on it, and dirt down to the lowest
    // neighbour's top, so cliff faces are filled but buried blocks aren't made.
    for (let i = 0; i < N; i++) {
      for (let j = 0; j < N; j++) {
        const x = colX(i);
        const z = colX(j);
        const h = columnHeight(i, j);
        block(new Vector3(x, h - 0.5, z), pick(dirt));
        block(new Vector3(x, h + 0.1, z), pick(grass), new Vector3(1, 0.2, 1));
        const low = Math.min(columnHeight(i + 1, j), columnHeight(i - 1, j), columnHeight(i, j + 1), columnHeight(i, j - 1));
        for (let y = h - 1; y > low; y--) block(new Vector3(x, y - 0.5, z), pick(dirt));
      }
    }

    // Trees: one on the hilltop (it pokes out of the cloud), the rest scattered.
    const wood = Material.create({ color: new Color(0.26, 0.17, 0.09, 1), smoothness: 0.05 });
    const leaves = shades(new Color(0.1, 0.3, 0.07, 1), 2, 0.3);
    const spots: [number, number][] = [[Math.floor(HILL.x + HALF), Math.floor(HILL.z + HALF)]];
    while (spots.length < 8) {
      const s: [number, number] = [Random.rangeInt(3, N - 3), Random.rangeInt(3, N - 3)];
      if (spots.every(([a, b]) => Math.abs(a - s[0]) + Math.abs(b - s[1]) > 6)) spots.push(s);
    }
    for (const [i, j] of spots) {
      const x = colX(i);
      const z = colX(j);
      const ground = columnHeight(i, j) + 0.2;
      const trunk = Random.rangeInt(4, 6);
      for (let k = 0; k < trunk; k++) block(new Vector3(x, ground + k + 0.5, z), wood);
      const leaf = pick(leaves);
      for (let dy = -2; dy <= 1; dy++) {
        const r = dy < 0 ? 2 : 1;
        for (let dx = -r; dx <= r; dx++) {
          for (let dz = -r; dz <= r; dz++) {
            if (Math.abs(dx) === r && Math.abs(dz) === r && (r === 2 || dy === 1)) continue; // round the corners
            if (dx === 0 && dz === 0 && dy < 0) continue; // the trunk
            block(new Vector3(x + dx, ground + trunk + dy + 0.5, z + dz), leaf);
          }
        }
      }
    }

    // The cloud deck: a wide, flat box of fog at about the hilltop's height.
    const deck = new GameObject("Cloud deck", { position: new Vector3(0, CLOUD.y, 0) });
    cloud = deck.addComponent(FogVolume, { size: CLOUD.size, density: CLOUD.density, color: Color.hex("#e4edfd"), blendDistance: CLOUD.blend });

    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: SKY, farClipPlane: 250 });
    console.log(`fog-blocks: ${cubes} cubes`);
    apply();
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
    // Low down, stay a little above the blocks under and around the camera.
    const i = Math.floor(x + HALF);
    const j = Math.floor(z + HALF);
    let ground = 0;
    for (let di = -1; di <= 1; di++) for (let dj = -1; dj <= 1; dj++) ground = Math.max(ground, columnHeight(i + di, j + dj));
    const position = new Vector3(x, Math.max(ground + 2.4, Mathf.lerp(ground + 2.4, 30, up)), z);
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
