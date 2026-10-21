// God rays: the alien scan. A garden shed at night, dusty and dark but for the moon through
// the gaps between its planks. Outside, a craft hovers by the window and sweeps a scanning
// beam through the shed: a narrow cone of green (or red) light that slices into blades where
// it passes the planks' gaps, crawls over the workbench, the tools and the lawnmower, and
// lights the dust it goes through. Now and then it stops and locks on to something, pulsing,
// then moves on. The menu (or keys) switches the beam's colour (C), the dust in the air (F)
// and pauses the scan (Space).
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

// The shed: x from -W to W, z from -D (the window wall) to D, walls up to H, a pitched roof
// up to RIDGE.
const W = 2.2;
const D = 1.6;
const H = 2.1;
const RIDGE = 2.8;
/** The window in the -z wall. */
const WIN = { x0: -0.5, x1: 0.5, y0: 1.0, y1: 1.6 };
/** Where the craft hovers, outside the window. */
const CRAFT = new Vector3(0.6, 2.6, -D - 4.5);

const BEAMS = {
  green: new Color(0.25, 1, 0.35, 1),
  red: new Color(1, 0.12, 0.08, 1),
};
type Beam = keyof typeof BEAMS;

const state = { beam: "green" as Beam, dust: true, paused: false, haze: 0, beamHaze: 0 };

/**
 * The haze's quality (`RenderSettings.volumetricFog`): the finest volume, its noise cleaned
 * up more or less (the plank gaps' thin blades are the hard case).
 */
const HAZE = [
  { name: "Smooth (4 samples, 0.95)", denoising: "both", historyWeight: 0.95, samples: 4, motionSamples: 6 },
  { name: "Default (1 sample, 0.9)", denoising: "both", historyWeight: 0.9, samples: 1, motionSamples: 2 },
  { name: "Reprojection only", denoising: "reprojection", historyWeight: 0.9, samples: 1, motionSamples: 2 },
  { name: "None (steps)", denoising: "none", historyWeight: 0.9, samples: 1, motionSamples: 1 },
] as const;

/** How much haze the beam lights (`Light.volumetricIntensity`), not how bright it is. */
const BEAM_HAZE = [1, 0.6, 0.3];
let refreshMenu = () => {};

function apply(): void {
  // Dust in the air, or settled: thinner haze (with none, there'd be nothing for the beam to
  // light at all).
  RenderSettings.fogDensity = state.dust ? 0.12 : 0.04;
  const h = HAZE[state.haze]!;
  RenderSettings.volumetricFog = {
    intensity: state.dust ? 1.6 : 0.5,
    anisotropy: 0.3,
    steps: 6,
    distance: 14,
    resolution: "ultra",
    denoising: h.denoising,
    historyWeight: h.historyWeight,
    samples: h.samples,
    motionSamples: h.motionSamples,
  };
  if (scanner) scanner.volumetricIntensity = BEAM_HAZE[state.beamHaze]!;
  refreshMenu();
}

/** The scanning beam, for `apply`. */
let scanner: Light | null = null;
/** Colours the beam, the craft's glow and its lights by `state.beam`. */
let setBeam = () => {};

function nextBeam() {
  state.beam = state.beam === "green" ? "red" : "green";
  setBeam();
  apply();
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

/**
 * Where the scan points at time `t`: a slow sweep across the shed and up and down it, with
 * pauses where it locks on to something (and `lock` 1 then).
 */
function scanAt(t: number): { target: Vector3; lock: number } {
  const stops = [
    new Vector3(-1.4, 0.95, 0.9), // the workbench
    new Vector3(1.2, 0.4, 0.6), // the lawnmower
    new Vector3(-0.2, 1.6, 1.5), // the tools on the back wall
    new Vector3(1.7, 1.2, -0.6), // the shelves
  ];
  const period = 6;
  const i = Math.floor(t / period) % stops.length;
  const u = (t % period) / period;
  const from = stops[i]!;
  const to = stops[(i + 1) % stops.length]!;
  // Hold for the first 40% (locked on), then sweep across with a wiggle, as if searching.
  if (u < 0.4) return { target: from, lock: Math.sin((u / 0.4) * Math.PI) };
  const s = (u - 0.4) / 0.6;
  const e = s * s * (3 - 2 * s);
  const p = from.add(to.sub(from).scale(e));
  return { target: p.add(new Vector3(Math.sin(t * 2.3) * 0.5, Math.sin(t * 3.1) * 0.35, 0)), lock: 0 };
}

class Shed extends Component {
  private camera!: Camera;
  private scanner!: Light;
  private glow!: Light;
  private t = 0;
  private craftLights: MeshRenderer[] = [];
  private craftOn: Material[] = [];
  private craftOff!: Material;

  override onStart() {
    QualitySettings.shadowDistance = 20;
    QualitySettings.shadowResolution = 4096;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.006, 0.007, 0.012, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 0.9, intensity: 0.7, scatter: 0.7 };
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogColor = new Color(0, 0, 0, 1);
    RenderSettings.fogDensity = 0.12;
    RenderSettings.autoExposure = { minEV: -6, maxEV: 10, compensation: -1, speedDarkToLight: 2, speedLightToDark: 0.8 };

    // Moonlight through the planks' gaps: faint, blue.
    const moon = new GameObject("Moon", { rotation: Quaternion.lookRotation(new Vector3(-0.4, -0.6, 0.7)) });
    moon.addComponent(Light, { type: "directional", color: new Color(0.55, 0.65, 1, 1), intensity: 0.3, shadows: "hard" });

    Random.initState(3);
    this.buildShed();
    this.buildContents();
    this.buildCraft();

    const eye = new GameObject("Main Camera", { position: new Vector3(0.8, 1.35, 1.3) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 72, backgroundColor: new Color(0.004, 0.006, 0.014, 1), nearClipPlane: 0.05, farClipPlane: 60 });
    apply();
  }

  /** Plank walls with gaps between the boards, a window, a pitched roof. */
  private buildShed() {
    const planks = [mat("#5a4430"), mat("#4e3b29"), mat("#634b34")];
    const floor = mat("#3d3025");
    slab("Floor", -W - 0.1, -0.1, -D - 0.1, W + 0.1, 0, D + 0.1, floor);
    const gap = 0.012;
    const board = 0.14;
    // Walls along x (the window wall at -z and the back at +z): vertical boards with gaps.
    for (const z of [-D, D]) {
      for (let x = -W; x < W - 0.01; x += board + gap) {
        const x1 = Math.min(x + board, W);
        const m = planks[Math.floor(Random.value * planks.length)]!;
        if (z < 0 && x1 > WIN.x0 && x < WIN.x1) {
          // Cut round the window.
          slab("Board", x, 0, z - 0.03, x1, WIN.y0, z, m);
          slab("Board", x, WIN.y1, z - 0.03, x1, H, z, m);
        } else slab("Board", x, 0, z - 0.03, x1, H, z, m);
      }
    }
    // Side walls along z.
    for (const x of [-W, W]) {
      for (let z = -D; z < D - 0.01; z += board + gap) {
        const m = planks[Math.floor(Random.value * planks.length)]!;
        slab("Board", x, 0, z, x + 0.03 * Math.sign(x), H, Math.min(z + board, D), m);
      }
    }
    // Gable ends and the roof: two slopes of boards meeting at the ridge.
    const rise = RIDGE - H;
    const run = W + 0.15;
    const slope = Math.atan2(rise, run);
    const len = Math.hypot(rise, run);
    for (const s of [-1, 1]) {
      shape("Roof", "cube", new Vector3((s * run) / 2, H + rise / 2 + 0.02, 0), new Vector3(len, 0.03, 2 * D + 0.3), planks[1]!, undefined, Quaternion.euler(0, 0, (-s * slope * 180) / Math.PI));
    }
    for (const z of [-D, D]) shape("Gable", "cube", new Vector3(0, H + rise / 2 - 0.1, z - 0.015), new Vector3(2 * W * 0.7, rise, 0.03), planks[2]!);
    // The window frame and its cross.
    const frame = mat("#2a221a");
    slab("Window frame", WIN.x0 - 0.04, WIN.y0 - 0.04, -D - 0.05, WIN.x1 + 0.04, WIN.y0, -D + 0.02, frame);
    slab("Window frame", WIN.x0 - 0.04, WIN.y1, -D - 0.05, WIN.x1 + 0.04, WIN.y1 + 0.04, -D + 0.02, frame);
    slab("Window bar", -0.015, WIN.y0, -D - 0.04, 0.015, WIN.y1, -D, frame);
    slab("Window bar", WIN.x0, (WIN.y0 + WIN.y1) / 2 - 0.015, -D - 0.04, WIN.x1, (WIN.y0 + WIN.y1) / 2 + 0.015, -D, frame);
    // Grass outside.
    slab("Lawn", -30, -0.2, -30, 30, -0.1, 30, mat("#16200f"));
  }

  /** A workbench with tools, a lawnmower, shelves of tins, a rake and spade, a sack. */
  private buildContents() {
    const wood = mat("#6b5238", 0.15);
    const metal = mat("#7a7f86", 0.55);
    const red = mat("#8a2a22", 0.35);
    const tin = [mat("#a89a5a", 0.5), mat("#5a7a8a", 0.5), mat("#8a5a3a", 0.5)];
    // Workbench along the -x wall.
    slab("Bench top", -W + 0.03, 0.85, -0.2, -W + 0.75, 0.9, 1.5, wood);
    for (const z of [-0.15, 1.45]) for (const x of [-W + 0.08, -W + 0.7]) slab("Bench leg", x - 0.03, 0, z - 0.03, x + 0.03, 0.85, z + 0.03, wood);
    slab("Vice", -W + 0.55, 0.9, 1.0, -W + 0.75, 1.02, 1.15, metal);
    for (let i = 0; i < 5; i++) shape("Jar", "cylinder", new Vector3(-W + 0.2 + Random.range(0, 0.3), 0.98, Random.range(0, 1.3)), new Vector3(0.08, 0.08, 0.08), tin[i % 3]!);
    // Tools on the back wall.
    for (let i = 0; i < 6; i++) {
      const x = -0.9 + i * 0.32;
      slab("Handle", x - 0.015, 1.0, D - 0.06, x + 0.015, 1.8, D - 0.03, wood);
      slab("Head", x - 0.07, 1.75, D - 0.07, x + 0.07, 1.85, D - 0.03, metal);
    }
    // Shelves on the +x wall with tins.
    for (const y of [0.6, 1.1, 1.6]) {
      slab("Shelf", W - 0.35, y, -1.3, W - 0.03, y + 0.03, 0.2, wood);
      for (let z = -1.2; z < 0.1; z += 0.18) shape("Tin", "cylinder", new Vector3(W - 0.2, y + 0.1, z), new Vector3(0.12, 0.08, 0.12), tin[Math.floor(Random.value * 3)]!);
    }
    // A push mower: deck, wheels, handle.
    slab("Mower deck", 0.8, 0.12, 0.3, 1.5, 0.32, 0.9, red);
    for (const [x, z] of [[0.85, 0.3], [1.45, 0.3], [0.85, 0.9], [1.45, 0.9]] as const) shape("Wheel", "cylinder", new Vector3(x, 0.1, z), new Vector3(0.18, 0.03, 0.18), mat("#1a1a1a"), undefined, Quaternion.euler(0, 0, 90));
    shape("Mower handle", "cube", new Vector3(1.15, 0.65, 1.15), new Vector3(0.5, 0.03, 0.03), metal);
    for (const x of [0.92, 1.38]) shape("Mower bar", "cube", new Vector3(x, 0.48, 1.02), new Vector3(0.03, 0.03, 0.6), metal, undefined, Quaternion.euler(-55, 0, 0));
    // A sack and a rake leaning in the corner.
    shape("Sack", "sphere", new Vector3(-1.5, 0.3, -1.1), new Vector3(0.6, 0.6, 0.5), mat("#8a7a5a"));
    shape("Rake", "cube", new Vector3(1.9, 0.8, 1.3), new Vector3(0.03, 1.6, 0.03), wood, undefined, Quaternion.euler(0, 0, 12));
  }

  /** The craft: a dark disc with a ring of lights, the scanner's eye and the scan itself. */
  private buildCraft() {
    const hull = mat("#2a2d33", 0.7);
    shape("Craft", "sphere", CRAFT, new Vector3(3.2, 0.55, 3.2), hull);
    shape("Dome", "sphere", CRAFT.add(new Vector3(0, 0.25, 0)), new Vector3(1.2, 0.7, 1.2), mat("#3a4a5a", 0.9));
    this.craftOff = Material.create({ color: Color.black, emission: new Color(0.05, 0.05, 0.05, 1), unlit: true });
    for (let i = 0; i < 12; i++) {
      const a = (i / 12) * Math.PI * 2;
      const go = shape("Craft light", "sphere", CRAFT.add(new Vector3(Math.cos(a) * 1.55, -0.05, Math.sin(a) * 1.55)), new Vector3(0.12, 0.12, 0.12), this.craftOff);
      this.craftLights.push(go.getComponent(MeshRenderer)!);
    }
    // The scan: a narrow cone from under the craft, aimed through the window.
    const eye = new GameObject("Scanner", { position: CRAFT.add(new Vector3(0, -0.3, 0.9)) });
    this.scanner = eye.addComponent(Light, { type: "spot", spotAngle: 9, innerSpotAngle: 5, range: 14, intensity: 60, shadows: "hard" });
    // A faint glow round the craft lighting the lawn and the shed's face.
    const glow = new GameObject("Craft glow", { position: CRAFT.add(new Vector3(0, -0.5, 0)) });
    this.glow = glow.addComponent(Light, { type: "point", range: 8, intensity: 1.5, volumetricIntensity: 0.3 });
    scanner = this.scanner;
    setBeam = () => this.setBeam();
    this.setBeam();
    apply();
  }

  private setBeam() {
    const c = BEAMS[state.beam];
    this.scanner.color = c;
    this.glow.color = c;
    this.craftOn = [0.4, 1, 3].map((k) => Material.create({ color: Color.black, emission: new Color(c.r * k * 3, c.g * k * 3, c.b * k * 3, 1), unlit: true }));
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyC")) nextBeam();
    if (input.getKeyDown("KeyF")) toggle("dust");
    if (input.getKeyDown("KeyG")) cycle("haze", HAZE.length);
    if (input.getKeyDown("KeyB")) cycle("beamHaze", BEAM_HAZE.length);
    if (input.getKeyDown("Space")) toggle("paused");
    if (!state.paused) this.t += dt;
    const { target, lock } = scanAt(this.t);
    const eye = this.scanner.transform;
    eye.rotation = Quaternion.lookRotation(target.sub(eye.position));
    // Locked on: brighter, pulsing.
    this.scanner.intensity = 60 * (1 + lock * (0.6 + 0.4 * Math.sin(this.t * 18)));
    // The ring of lights chases round.
    for (let i = 0; i < this.craftLights.length; i++) {
      const phase = (i - this.t * 8) % this.craftLights.length;
      const k = ((phase % 12) + 12) % 12;
      this.craftLights[i]!.material = k < 1 ? this.craftOn[2]! : k < 3 ? this.craftOn[1]! : k < 6 ? this.craftOn[0]! : this.craftOff;
    }
    // Crouched in the corner, looking across at the window and the beam.
    const cam = this.camera.transform;
    const t = this.t;
    cam.position = new Vector3(0.8 + Math.sin(t * 0.3) * 0.08, 1.35 + Math.sin(t * 0.5) * 0.03, 1.3);
    cam.rotation = Quaternion.lookRotation(new Vector3(-0.5 + Math.sin(t * 0.17) * 0.5, -0.2, -2));
  }
}

function toggle(key: "dust" | "paused") {
  state[key] = !state[key];
  apply();
}

function cycle(key: "haze" | "beamHaze", n: number) {
  state[key] = (state[key] + 1) % n;
  apply();
}

register({ Shed });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#c0d0c4", fontSize: 13, width: 90, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 140, backgroundColor: props.on ? "#2f8a4a" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#eef6f0", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#76887a", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(8, 12, 10, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the alien scan</text>
        <Button
          label="Beam"
          value={state.beam === "green" ? "Green" : "Red"}
          on={true}
          keyName="C"
          onClick={nextBeam}
        />
        <Button label="Dust" value={state.dust ? "In the air" : "Settled (thin)"} on={state.dust} keyName="F" onClick={() => toggle("dust")} />
        <Button label="Haze" value={HAZE[state.haze]!.name} on={state.haze === 0} keyName="G" onClick={() => cycle("haze", HAZE.length)} />
        <Button label="Beam haze" value={`${Math.round(BEAM_HAZE[state.beamHaze]! * 100)}%`} on={state.beamHaze === 0} keyName="B" onClick={() => cycle("beamHaze", BEAM_HAZE.length)} />
        <Button label="Scan" value={state.paused ? "Paused" : "Scanning"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
