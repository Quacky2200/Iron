// God rays: the forest. Morning sun slants through gaps in a dense canopy of tall trees, so
// long diagonal beams cross the soft lilac haze and land in bright patches on the mossy floor
// among ferns, rocks, fallen logs and small flowers. The canopy is built from many
// overlapping leaf clumps, so the sun's hard shadow map is full of holes the beams pass
// through; near the camera the beams are worked out per pixel (`sharpDistance`), so they keep
// crisp edges. Dust motes drift in the air. The camera walks slowly round a path at head
// height, always looking partly towards the sun, where forward scattering makes the beams
// brightest. The menu (or keys) switches the sharp god rays (G) and the fog (F); Space pauses
// the walk.
import {
  Camera,
  Color,
  Component,
  Debug,
  GameObject,
  input,
  Light,
  Material,
  MeshRenderer,
  ParticleSystem,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Vector3,
  register,
  type VolumetricFogSettings,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

const hex = Color.hex;
const v = (x: number, y: number, z: number) => new Vector3(x, y, z);

// ---------------------------------------------------------------------------------------
// Light and air

/** The way the sunlight travels: from behind-left of the far end, ~38° above the horizon. */
const SUN_DIR = v(0.5, -0.62, 0.6).normalized;
/** Horizontal, towards the sun. */
const TO_SUN = v(-SUN_DIR.x, 0, -SUN_DIR.z).normalized;

/** The haze in the canopy's shade: a soft lilac grey. Also the camera's background. */
const HAZE = hex("#57536a");
const SHARP_DISTANCE = 35;
const SHAFTS: VolumetricFogSettings = {
  intensity: 0.25,
  anisotropy: 0.45,
  albedo: hex("#ffe8c6"),
  steps: 4,
  distance: 60,
  sharpSamples: 48,
};

/** What the menu shows and changes; `apply` sends it to `RenderSettings`. */
const state = { sharp: true, fog: true, walking: true };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fogColor = HAZE;
  RenderSettings.fogMode = "exponential";
  RenderSettings.fogDensity = 0.012;
  RenderSettings.heightFog = { density: 0.025, baseHeight: 0, falloff: 0.4 };
  RenderSettings.volumetricFog = { ...SHAFTS, sharpDistance: state.sharp ? SHARP_DISTANCE : 0 };
  // The beams are the fog lit by the sun: with the fog off, they go too.
  RenderSettings.fog = state.fog;
  refreshMenu();
}

// ---------------------------------------------------------------------------------------
// Materials

const mat = (c: string, smoothness = 0.08) => Material.create({ color: hex(c), smoothness });
const BARK = ["#5c4f44", "#6b6159", "#4a4038", "#57504a"].map((c) => mat(c, 0.1));
const LEAVES = ["#26472a", "#2f5a2a", "#3d6e31", "#4f7f38", "#5e8c3e"].map((c) => mat(c));
const FERNS = ["#2e5e2c", "#3f7a34", "#5a8f3a", "#4a6e2c"].map((c) => mat(c));
const GROUND = mat("#4b5631", 0.04);
const PATH = mat("#6b5a44", 0.04);
const ROCK = mat("#7c7b74", 0.15);
const MOSS = mat("#4f6f30", 0.05);
const LOG = mat("#5b4a3a", 0.1);
const FLOWERS = ["#9a6fd0", "#b78ee0", "#7f58b8", "#f0ecf8", "#e6c94e"].map((c) => mat(c, 0.2));

const pick = <T,>(list: T[]): T => list[Math.min(list.length - 1, Math.floor(Random.range(0, list.length)))]!;

let meshes = 0;

function shape(name: string, mesh: "cube" | "sphere" | "cylinder" | "capsule" | "plane", at: Vector3, scale: Vector3, material: Material, rotation: Quaternion = Quaternion.identity): MeshRenderer {
  const go = new GameObject(name, { position: at, rotation, scale });
  meshes++;
  return go.addComponent(MeshRenderer, { mesh, material });
}

/** A cylinder (or capsule) from `a` to `b`. */
function rod(name: string, a: Vector3, b: Vector3, diameter: number, material: Material, mesh: "cylinder" | "capsule" = "cylinder"): MeshRenderer {
  const d = b.sub(a);
  return shape(name, mesh, a.add(b).scale(0.5), v(diameter, d.magnitude / 2, diameter), material, Quaternion.fromToRotation(Vector3.up, d));
}

// ---------------------------------------------------------------------------------------
// The path: an ellipse round the middle of the wood

const PATH_A = 12; // along x
const PATH_B = 20; // along z

const pathAt = (t: number) => v(PATH_A * Math.sin(t), 0, PATH_B * Math.cos(t));
const pathTangent = (t: number) => v(PATH_A * Math.cos(t), 0, -PATH_B * Math.sin(t)).normalized;

/** Points round the path, for keeping things off it. */
const PATH_POINTS: [number, number][] = [];
for (let i = 0; i < 128; i++) {
  const p = pathAt((i / 128) * Math.PI * 2);
  PATH_POINTS.push([p.x, p.z]);
}

function pathDistance(x: number, z: number): number {
  let best = Infinity;
  for (const [px, pz] of PATH_POINTS) best = Math.min(best, (px - x) ** 2 + (pz - z) ** 2);
  return Math.sqrt(best);
}

// ---------------------------------------------------------------------------------------
// The wood

const trunks: [number, number][] = [];

function farFromTrunks(x: number, z: number, gap: number): boolean {
  for (const [tx, tz] of trunks) if ((tx - x) ** 2 + (tz - z) ** 2 < gap * gap) return false;
  return true;
}

/** A tall tree: a trunk (maybe leaning), a bough or two, and a high crown of leaf clumps. */
function tree(x: number, z: number): void {
  const h = Random.range(13, 21);
  const lean = Random.range(0, 1) < 0.25 ? Random.range(5, 13) : Random.range(0, 4);
  const tilt = Quaternion.angleAxis(lean, v(Random.range(-1, 1), 0, Random.range(-1, 1)).normalized);
  const base = v(x, -0.3, z);
  const top = base.add(tilt.rotate(Vector3.up).scale(h));
  const width = Random.range(0.45, 0.8) * (h / 17);
  const bark = pick(BARK);
  rod("Trunk", base, top, width, bark);
  const R = Random.range(3.6, 5.4);
  for (let k = 0; k < 1 + Math.floor(Random.range(0, 2)); k++) {
    const a = Random.range(0, Math.PI * 2);
    rod("Bough", base.lerp(top, Random.range(0.6, 0.75)), top.add(v(Math.cos(a) * R * 0.7, Random.range(-1, 1), Math.sin(a) * R * 0.7)), width * 0.45, bark);
  }
  const g = Math.floor(Random.range(1, 4));
  const n = 6 + Math.floor(Random.range(0, 4));
  for (let k = 0; k < n; k++) {
    const a = Random.range(0, Math.PI * 2);
    const r = R * Math.sqrt(Random.range(0.05, 1));
    const oy = Random.range(-0.35, 0.5) * R;
    const s = R * Random.range(0.7, 1.15);
    // Clumps on top catch the sun; the lower ones sit in shade.
    const shade = oy > 0.2 * R ? g + 1 : oy < -0.1 * R ? g - 1 : g;
    const at = top.add(v(Math.cos(a) * r, oy, Math.sin(a) * r));
    shape("Leaves", "sphere", at, v(s, s * Random.range(0.55, 0.75), s), LEAVES[Math.max(0, Math.min(LEAVES.length - 1, shade))]!, Quaternion.euler(0, Random.range(0, 360), 0));
  }
}

function fern(x: number, z: number): void {
  const s = Random.range(0.6, 1.5);
  shape("Fern", "sphere", v(x, s * 0.12, z), v(s, s * 0.45, s), pick(FERNS));
}

function bush(x: number, z: number): void {
  const s = Random.range(1, 1.8);
  const m = pick(FERNS);
  shape("Bush", "sphere", v(x, s * 0.3, z), v(s, s * 0.75, s), m);
  shape("Bush", "sphere", v(x + Random.range(-0.5, 0.5) * s, s * 0.22, z + Random.range(-0.5, 0.5) * s), Vector3.one.scale(s * 0.7), m);
}

/** A squashed boulder with a cap of moss. */
function rock(x: number, z: number, size: number): void {
  const s = Random.range(0.8, 1.4) * size;
  const turn = Quaternion.euler(Random.range(-8, 8), Random.range(0, 360), Random.range(-8, 8));
  shape("Rock", "sphere", v(x, s * 0.12, z), v(s, s * 0.55, s * 0.8), ROCK, turn);
  shape("Moss", "sphere", v(x, s * 0.26, z), v(s * 0.86, s * 0.34, s * 0.66), MOSS, turn);
}

/** A fallen trunk lying across the floor, with moss along its back. */
function log(x: number, z: number): void {
  const a = Random.range(0, Math.PI);
  const len = Random.range(5, 9);
  const d = Random.range(0.55, 0.85);
  const half = v(Math.cos(a), 0, Math.sin(a)).scale(len / 2);
  const mid = v(x, d * 0.42, z);
  rod("Log", mid.sub(half), mid.add(half), d, LOG, "capsule");
  rod("Log moss", mid.sub(half.scale(0.7)).add(v(0, d * 0.3, 0)), mid.add(half.scale(0.6)).add(v(0, d * 0.3, 0)), d * 0.6, MOSS, "capsule");
}

/** A patch of small flowers. */
function flowers(x: number, z: number): void {
  const m = pick(FLOWERS);
  for (let k = 0; k < 9; k++) {
    const s = Random.range(0.08, 0.14);
    const a = Random.range(0, Math.PI * 2);
    const r = Random.range(0, 1.1);
    shape("Flower", "sphere", v(x + Math.cos(a) * r, Random.range(0.1, 0.28), z + Math.sin(a) * r), Vector3.one.scale(s), Random.range(0, 1) < 0.2 ? FLOWERS[3]! : m);
  }
}

function buildForest(): void {
  Random.initState(23);
  shape("Ground", "plane", Vector3.zero, Vector3.one.scale(14), GROUND);

  // The worn path, in soft overlapping discs.
  for (let i = 0; i < 72; i++) {
    const t = (i / 72) * Math.PI * 2;
    const p = pathAt(t);
    const s = Random.range(1.7, 2.2);
    shape("Path", "cylinder", v(p.x, 0.005, p.z), v(s, 0.01, s * 1.3), PATH, Quaternion.lookRotation(pathTangent(t)));
  }

  // Tall trees all through the wood, kept off the path.
  for (let tries = 0; trunks.length < 170 && tries < 4000; tries++) {
    const x = Random.range(-58, 58);
    const z = Random.range(-58, 58);
    if (pathDistance(x, z) < 3 || !farFromTrunks(x, z, 5)) continue;
    trunks.push([x, z]);
    tree(x, z);
  }

  // The undergrowth, thickest near the path where it shows.
  let ferns = 0;
  for (let tries = 0; ferns < 190 && tries < 3000; tries++) {
    const x = Random.range(-40, 40);
    const z = Random.range(-45, 45);
    const d = pathDistance(x, z);
    if (d < 1.4 || Random.range(0, 1) < d / 30) continue;
    fern(x, z);
    ferns++;
  }
  let bushes = 0;
  for (let tries = 0; bushes < 36 && tries < 1000; tries++) {
    const x = Random.range(-45, 45);
    const z = Random.range(-50, 50);
    if (pathDistance(x, z) < 2.6 || !farFromTrunks(x, z, 1.5)) continue;
    bush(x, z);
    bushes++;
  }
  let rocks = 0;
  for (let tries = 0; rocks < 30 && tries < 1000; tries++) {
    const x = Random.range(-40, 40);
    const z = Random.range(-45, 45);
    if (pathDistance(x, z) < 2 || !farFromTrunks(x, z, 1.5)) continue;
    rock(x, z, Random.range(0, 1) < 0.2 ? 2.2 : 1);
    rocks++;
  }
  // A mossy outcrop by the path, towards the sun.
  for (let k = 0; k < 5; k++) rock(-6 + Random.range(-2, 2), -22 + Random.range(-2, 2), Random.range(1.2, 2.4));

  let logs = 0;
  for (let tries = 0; logs < 6 && tries < 400; tries++) {
    const x = Random.range(-30, 30);
    const z = Random.range(-36, 36);
    if (pathDistance(x, z) < 5.5 || !farFromTrunks(x, z, 5)) continue;
    log(x, z);
    logs++;
  }

  // Flower patches beside the path.
  for (let k = 0; k < 18; k++) {
    const t = Random.range(0, Math.PI * 2);
    const p = pathAt(t);
    const out = p.normalized.scale(Random.range(1.8, 3.6) * (Random.range(0, 1) < 0.5 ? -1 : 1));
    flowers(p.x + out.x, p.z + out.z);
  }
  Debug.log(`godrays-forest: ${meshes} mesh renderers`);
}

// ---------------------------------------------------------------------------------------
// The walk

/** Radians of the path per second: a lap of ~100 m in about three minutes. */
const WALK_SPEED = 0.036;
const EYE = 1.7;

class Forest extends Component {
  private camera!: Camera;
  private motes!: GameObject;
  private t = 0.4;
  private clock = 0;

  override onStart() {
    QualitySettings.shadowDistance = 55;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.05, 0.055, 0.065, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1.1, intensity: 0.35, scatter: 0.6 };

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN_DIR) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.86, 0.64, 1), intensity: 2.2, shadows: "hard", volumetricIntensity: 1 });

    buildForest();

    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: HAZE, nearClipPlane: 0.1, farClipPlane: 160 });

    // Dust and pollen hanging in the air round the walker, faint enough to show mostly in
    // the beams' glare.
    this.motes = new GameObject("Motes");
    this.motes.addComponent(ParticleSystem, {
      startLifetime: [10, 16],
      startSpeed: [0.02, 0.1],
      startSize: [0.025, 0.05],
      startColor: [new Color(1, 0.92, 0.75, 0.5), new Color(0.9, 0.9, 1, 0.3)],
      gravityModifier: 0.0006,
      rateOverTime: 30,
      maxParticles: 500,
      duration: 5,
      shape: { type: "box", size: { x: 16, y: 5, z: 16 } },
      simulationSpace: "world",
      blendMode: "additive",
      colorOverLifetime: [
        { time: 0, color: new Color(1, 1, 1, 0) },
        { time: 0.2, color: Color.white },
        { time: 0.8, color: Color.white },
        { time: 1, color: new Color(1, 1, 1, 0) },
      ],
    });

    apply();
    this.place();
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyG")) toggle("sharp");
    if (input.getKeyDown("KeyF")) toggle("fog");
    if (input.getKeyDown("Space")) toggle("walking");
    if (state.walking) {
      this.t += dt * WALK_SPEED;
      this.clock += dt;
      this.place();
    }
  }

  /** Puts the camera on the path, looking mostly towards the sun and a little up. */
  private place(): void {
    const p = pathAt(this.t);
    const bob = Math.sin(this.clock * 3.2) * 0.03;
    const at = v(p.x, EYE + bob, p.z);
    const ahead = pathTangent(this.t);
    const look = TO_SUN.add(ahead.scale(0.55)).normalized;
    const dir = v(look.x, 0.2 + Math.sin(this.clock * 0.13) * 0.05, look.z);
    const cam = this.camera.transform;
    cam.position = at;
    cam.rotation = Quaternion.lookRotation(dir);
    this.motes.transform.position = at.add(look.scale(4)).add(v(0, -1, 0));
  }
}

function toggle(key: keyof typeof state) {
  state[key] = !state[key];
  apply();
}

register({ Forest });

// ---------------------------------------------------------------------------------------
// Menu

function Button(props: { label: string; value: string; on?: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#b9b6c8", fontSize: 13, width: 80, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 120, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : "#7a9a4a" }}
      >
        <text style={{ color: "#f4f2ea", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#7d7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(18, 16, 24, 0.68)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the forest</text>
        <Button label="God rays" value={state.sharp ? "Sharp" : "Soft"} on={state.sharp && state.fog} keyName="G" onClick={() => toggle("sharp")} />
        <Button label="Fog" value={state.fog ? "On" : "Off (no beams)"} on={state.fog} keyName="F" onClick={() => toggle("fog")} />
        <Button label="Walk" value={state.walking ? "Walking" : "Paused"} on={state.walking} keyName="Space" onClick={() => toggle("walking")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
