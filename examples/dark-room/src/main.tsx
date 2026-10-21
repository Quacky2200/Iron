// Eye adaptation (`RenderSettings.autoExposure`): a bedroom on a bright day behind closed
// louvred shutters. Slivers of sun get through the slats: the room is dim, but the eyes have
// adapted and it's easy to see. Then the shutters swing open: blinding, the windows white,
// until the eyes settle into a bright room with sunlight across the floor and the bed. Later
// they close again: near black, and the room only slowly comes back as the eyes adapt to the
// dark. The camera looks round the window side all the while. The menu (or keys) switches
// auto exposure (A: else a fixed exposure), restarts the cycle (R) and pauses it (Space).
import {
  Camera,
  Color,
  Component,
  GameObject,
  Light,
  Material,
  MeshRenderer,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Vector3,
  input,
  register,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

// The room: x from -W to W, z from -D (the window wall) to D, H tall.
const W = 4;
const D = 3.5;
const H = 2.8;
/** The windows in the -z wall: their centres along x, and sill and head heights. */
const WINDOWS = [-1.8, 1.4];
const WIN_W = 1.3;
const SILL = 0.9;
const HEAD = 2.3;

/** Where sunlight travels: in through the windows and down onto the floor. */
const SUN = new Vector3(0.35, -0.55, 0.76).normalized;

/**
 * The look round: where the camera looks (yaw in degrees, 0 = towards the windows) from
 * each time, eased between. It faces the windows as the shutters open and close.
 */
const LOOKS: [number, number][] = [
  [0, 30],
  [5, 60],
  [9, 0],
  [13, -10],
  [17, 35],
  [21, 70],
  [24, 10],
  [28, -5],
  [33, 45],
  [37, 70],
  [40, 30],
];
const CYCLE = 40;
/** When the shutters open and close (seconds into the cycle), and how long a swing takes. */
const OPEN_AT = 10;
const CLOSE_AT = 25;
const SWING = 2;
/** How far the leaves swing open, degrees. */
const OPEN_ANGLE = 105;

const state = { auto: true, paused: false };
let refreshMenu = () => {};

function apply(): void {
  // Slow to see in the dark, quick-ish to recover from the glare: as eyes do.
  RenderSettings.autoExposure = state.auto ? { minEV: -7, maxEV: 16, compensation: -0.6, speedDarkToLight: 0.6, speedLightToDark: 0.3 } : null;
  RenderSettings.postExposure = state.auto ? 0 : -3;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
/** A box between two corners. */
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material);
}
const mat = (hex: string, smoothness = 0.08) => Material.create({ color: Color.hex(hex), smoothness });

/** The yaw at time `t` of the look round, eased between the `LOOKS`. */
function yawAt(t: number): number {
  for (let i = 0; i + 1 < LOOKS.length; i++) {
    const [t0, y0] = LOOKS[i]!;
    const [t1, y1] = LOOKS[i + 1]!;
    if (t <= t1) {
      const u = Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
      return y0 + (y1 - y0) * u * u * (3 - 2 * u);
    }
  }
  return LOOKS[LOOKS.length - 1]![1];
}

class Bedroom extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 20;
    QualitySettings.shadowResolution = 4096;
    // Hardly any light reaches the far corner: the sun's patches and the windows are the
    // room's light.
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.035, 0.032, 0.03, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1, intensity: 0.35, scatter: 0.55 };
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogColor = new Color(0.01, 0.01, 0.01, 1);
    RenderSettings.fogDensity = 0.03;
    RenderSettings.volumetricFog = { intensity: 0.25, anisotropy: 0.5, steps: 4, distance: 12, albedo: new Color(1, 0.9, 0.78, 1) };
    RenderSettings.fog = true;

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.93, 0.82, 1), intensity: 14, shadows: "hard" });

    Random.initState(5);
    this.buildRoom();
    this.buildFurniture();

    const eye = new GameObject("Main Camera", { position: new Vector3(0.2, 1.6, 0.4), rotation: Quaternion.lookRotation(new Vector3(0.17, -0.12, -1)) });
    // Bright daylight outside the windows.
    this.camera = eye.addComponent(Camera, { fieldOfView: 68, backgroundColor: new Color(12, 13, 15, 1), farClipPlane: 60, nearClipPlane: 0.05 });
    apply();
  }

  private buildRoom() {
    const wall = mat("#c9bfae");
    const floor = mat("#7a5a3c", 0.3);
    const ceiling = mat("#e2ddd2");
    slab("Floor", -W - 0.2, -0.2, -D - 0.2, W + 0.2, 0, D + 0.2, floor);
    slab("Ceiling", -W - 0.2, H, -D - 0.2, W + 0.2, H + 0.2, D + 0.2, ceiling);
    slab("Wall", -W - 0.2, -0.2, D, W + 0.2, H + 0.2, D + 0.2, wall);
    slab("Wall", -W - 0.2, -0.2, -D, -W, H + 0.2, D, wall);
    slab("Wall", W, -0.2, -D, W + 0.2, H + 0.2, D, wall);
    // The window wall, cut round the windows.
    const T = 0.25;
    const edges = [-W - 0.2, ...WINDOWS.flatMap((x) => [x - WIN_W / 2, x + WIN_W / 2]), W + 0.2];
    for (let i = 0; i < edges.length; i += 2) slab("Wall", edges[i]!, -0.2, -D - T, edges[i + 1]!, H + 0.2, -D, wall);
    const frame = mat("#f2efe8", 0.3);
    for (const x of WINDOWS) {
      const x0 = x - WIN_W / 2;
      const x1 = x + WIN_W / 2;
      slab("Below window", x0, -0.2, -D - T, x1, SILL, -D, wall);
      slab("Above window", x0, HEAD, -D - T, x1, H + 0.2, -D, wall);
      slab("Sill", x0 - 0.08, SILL - 0.04, -D - 0.05, x1 + 0.08, SILL, -D + 0.12, frame);
      // Glazing bars: a cross.
      slab("Glazing bar", x - 0.025, SILL, -D - 0.14, x + 0.025, HEAD, -D - 0.1, frame);
      slab("Glazing bar", x0, (SILL + HEAD) / 2 - 0.025, -D - 0.14, x1, (SILL + HEAD) / 2 + 0.025, -D - 0.1, frame);
      this.buildShutters(x0, x1);
    }
    slab("Skirting", -W, 0, D - 0.02, W, 0.12, D, frame);
  }

  /** The leaves' hinges: left leaves turn one way, right ones the other. */
  private leaves: { hinge: GameObject; side: number }[] = [];

  /**
   * Louvred shutters inside a window from `x0` to `x1`: a leaf hinged at each side, its slats
   * tilted so that, closed, only thin slivers of sun get between them.
   */
  private buildShutters(x0: number, x1: number) {
    const wood = mat("#e9e3d6", 0.25);
    const w = (x1 - x0) / 2;
    const z = -D + 0.03;
    for (const side of [-1, 1]) {
      const hinge = new GameObject("Shutter hinge", { position: new Vector3(side < 0 ? x0 : x1, 0, z) });
      this.leaves.push({ hinge, side });
      // The leaf runs from the hinge towards the window's middle (local -side along x).
      const mid = -side * w / 2;
      const s = (name: string, cx: number, cy: number, sx: number, sy: number, sz: number, rot?: Quaternion) =>
        shape(name, "cube", new Vector3(cx, cy, 0.02), new Vector3(sx, sy, sz), wood, hinge, rot);
      s("Stile", -side * 0.03, (SILL + HEAD) / 2, 0.06, HEAD - SILL, 0.04);
      s("Stile", -side * (w - 0.03), (SILL + HEAD) / 2, 0.06, HEAD - SILL, 0.04);
      s("Rail", mid, SILL + 0.04, w, 0.08, 0.04);
      s("Rail", mid, HEAD - 0.04, w, 0.08, 0.04);
      // Slats 5 cm apart, each 9 cm deep and tilted 50 degrees: they overlap, so only thin
      // slivers of sun get between them.
      for (let y = SILL + 0.1; y < HEAD - 0.06; y += 0.05) s("Slat", mid, y, w - 0.1, 0.008, 0.09, Quaternion.euler(-50, 0, 0));
    }
  }

  /** Swings the leaves to `k` of the way open (0 closed, 1 open). */
  private setShutters(k: number) {
    for (const { hinge, side } of this.leaves) hinge.transform.localRotation = Quaternion.euler(0, side * k * OPEN_ANGLE, 0);
  }

  private buildFurniture() {
    const oak = mat("#6b4a2e", 0.2);
    const dark = mat("#3a2a1e", 0.2);
    const linen = mat("#e7e1d4");
    const blanket = mat("#2f4a6a");
    const fabric = mat("#6a3b2b");
    // The dark corner (+x, +z): a wardrobe, an armchair, a standard lamp (off), pictures.
    slab("Wardrobe", W - 0.65, 0, D - 2.4, W, 2.1, D - 1.2, dark);
    slab("Wardrobe door line", W - 0.66, 0.1, D - 1.81, W - 0.64, 2.0, D - 1.79, oak);
    slab("Wardrobe knob", W - 0.69, 1.05, D - 1.9, W - 0.65, 1.1, D - 1.85, oak);
    slab("Chair seat", W - 1.8, 0.35, D - 1.0, W - 1.0, 0.5, D - 0.2, fabric);
    slab("Chair back", W - 1.8, 0.35, D - 0.3, W - 1.0, 1.1, D - 0.1, fabric);
    for (const dx of [-0.38, 0.38]) slab("Chair arm", W - 1.4 + dx - 0.06, 0.35, D - 1.0, W - 1.4 + dx + 0.06, 0.7, D - 0.1, fabric);
    shape("Lamp stand", "cylinder", new Vector3(W - 0.4, 0.75, D - 0.4), new Vector3(0.04, 0.75, 0.04), dark);
    shape("Lamp shade", "cylinder", new Vector3(W - 0.4, 1.6, D - 0.4), new Vector3(0.45, 0.2, 0.45), linen);
    const frames = [mat("#b08a4a", 0.4), mat("#2a2a2a", 0.3)];
    const paintings = [mat("#2f5a4a"), mat("#7a4a2a"), mat("#3a4a7a")];
    for (const [x, y, w, h, k] of [[1.0, 1.7, 0.9, 0.65, 0], [2.3, 1.55, 0.5, 0.7, 1], [-0.2, 1.5, 0.6, 0.45, 2]] as const) {
      slab("Picture frame", x - w / 2, y - h / 2, D - 0.04, x + w / 2, y + h / 2, D, frames[k % 2]!);
      slab("Picture", x - w / 2 + 0.06, y - h / 2 + 0.06, D - 0.05, x + w / 2 - 0.06, y + h / 2 - 0.06, D - 0.04, paintings[k]!);
    }
    // The bed along the -x wall, where the sun falls.
    slab("Bed frame", -W, 0, -1.6, -W + 1.5, 0.35, 0.6, oak);
    slab("Mattress", -W + 0.05, 0.35, -1.55, -W + 1.45, 0.55, 0.55, linen);
    slab("Blanket", -W + 0.03, 0.5, -1.0, -W + 1.47, 0.6, 0.57, blanket);
    slab("Pillow", -W + 0.2, 0.55, -1.5, -W + 1.3, 0.7, -1.15, linen);
    slab("Headboard", -W, 0, -1.7, -W + 1.5, 1.1, -1.6, oak);
    // A desk and chair under the right window, a rug, a bookcase on the -x wall's far end.
    slab("Desk", 0.6, 0.72, -D + 0.15, 2.2, 0.76, -D + 0.85, oak);
    for (const [x, z] of [[0.65, -D + 0.2], [2.15, -D + 0.2], [0.65, -D + 0.8], [2.15, -D + 0.8]] as const) slab("Desk leg", x - 0.03, 0, z - 0.03, x + 0.03, 0.72, z + 0.03, oak);
    slab("Desk chair", 1.1, 0.45, -D + 1.0, 1.6, 0.5, -D + 1.5, dark);
    slab("Desk chair back", 1.1, 0.45, -D + 1.45, 1.6, 1.0, -D + 1.5, dark);
    slab("Rug", -1.6, 0, -1.2, 1.4, 0.015, 1.4, mat("#7a3a34", 0.02));
    slab("Bookcase", -W, 0, D - 1.3, -W + 0.4, 1.9, D - 0.1, dark);
    const spines = [mat("#8a2a2a"), mat("#2a4a7a"), mat("#c9a24a"), mat("#3a6a3a")];
    for (let shelf = 0; shelf < 4; shelf++) {
      for (let z = D - 1.25; z < D - 0.2; z += Random.range(0.05, 0.09)) {
        const h = Random.range(0.25, 0.38);
        slab("Book", -W + 0.05, 0.08 + shelf * 0.46, z, -W + 0.32, 0.08 + shelf * 0.46 + h, z + 0.04, spines[Math.floor(Random.value * 4)]!);
      }
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyA")) toggle("auto");
    if (input.getKeyDown("Space")) toggle("paused");
    if (input.getKeyDown("KeyR")) this.t = 0;
    if (!state.paused) this.t = (this.t + dt) % CYCLE;
    const ease = (u: number) => {
      const c = Math.min(1, Math.max(0, u));
      return c * c * (3 - 2 * c);
    };
    this.setShutters(ease((this.t - OPEN_AT) / SWING) - ease((this.t - CLOSE_AT) / SWING));
    const yaw = (yawAt(this.t) * Math.PI) / 180;
    // A slight drift, as someone standing does.
    const cam = this.camera.transform;
    cam.position = new Vector3(0.2 + Math.sin(this.t * 0.4) * 0.04, 1.6 + Math.sin(this.t * 0.9) * 0.01, 0.4);
    cam.rotation = Quaternion.lookRotation(new Vector3(Math.sin(yaw), -0.12, -Math.cos(yaw)));
  }
}

function toggle(key: keyof typeof state) {
  state[key] = !state[key];
  apply();
}

register({ Bedroom });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#c8ccd2", fontSize: 13, width: 104, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on ? "#3a7bd5" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f4f6f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#7d838c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 18, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Eye adaptation: the bedroom</text>
        <Button label="Auto exposure" value={state.auto ? "On (eyes adapt)" : "Off (fixed)"} on={state.auto} keyName="A" onClick={() => toggle("auto")} />
        <Button label="Shutters" value={state.paused ? "Paused" : "Opening and closing"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
