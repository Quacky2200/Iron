// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// God rays: the clouds. Late afternoon from high on a hillside, looking towards the sun over rolling
// country. The sun sits behind a broken layer of cumulus: it lights the clouds' edges and
// sends beams (crepuscular rays) fanning down through the gaps between them, across the hazy
// air to the fields and woods below. The clouds drift slowly, so the beams sweep and shift,
// opening and closing as gaps pass the sun. The menu (or keys) switches the haze and its
// beams (F), auto exposure (A) and pauses the clouds (Space).

/**
 * Where sunlight travels: the sun is ahead, a little to the right (15 degrees from the view)
 * and 30 degrees up, so it shows near the middle of the view, the hills catching it from the
 * side while its beams fan across.
 */
const SUN = new Vector3(-0.224, -0.5, 0.837).normalized;
/** The cloud layer: its height and thickness, metres. */
const CLOUD_BASE = 700;
/** How far the clouds sway to and fro (metres), and how long a sway takes (seconds). */
const SWAY = 220;
const SWAY_TIME = 50;
/** Where the line from the camera to the sun crosses the cloud layer, and the hole there. */
const SUN_GAP = new Vector3(293, 0, -766);
const GAP_RADIUS = 260;

const state = { haze: true, auto: true, paused: false };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fog = state.haze;
  RenderSettings.autoExposure = state.auto ? { minEV: 0, maxEV: 16, compensation: 0.9, speedDarkToLight: 0.6, speedLightToDark: 1 } : null;
  RenderSettings.postExposure = state.auto ? 0 : -4.5;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
const mat = (hex: string, smoothness = 0.05) => Material.create({ color: Color.hex(hex), smoothness });

class Clouds extends Component {
  private camera!: Camera;
  private sky!: GameObject;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 2600;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    RenderSettings.ambientMode = "color";
    // The sky's light: about as bright as the sky looks, so shaded ground isn't black.
    RenderSettings.ambientLight = new Color(2.6, 3.0, 3.6, 1);
    RenderSettings.skybox = {
      zenith: new Color(1.6, 3.2, 7.5, 1),
      horizon: new Color(7, 7.4, 8, 1),
      ground: new Color(1.4, 1.3, 1.1, 1),
      sunDirection: SUN.scale(-1),
      sunColor: new Color(900, 830, 720, 1),
      sunSize: 0.9,
    };
    RenderSettings.tonemapping = "aces";
    // Only what's far brighter than white glows (the sun's disc, its rim on the clouds): a
    // burst of glare when it comes out, while the rest of the view stays crisp.
    RenderSettings.bloom = { threshold: 4, intensity: 0.7, scatter: 0.65 };
    // Haze, thicker low down: the beams show most in the air under the clouds.
    RenderSettings.fogMode = "exponential";
    // The haze's own colour is the sky's light it scatters: distance fades to a pale blue,
    // not grey; the sun's shafts add on top.
    RenderSettings.fogColor = new Color(2.4, 2.7, 3.2, 1);
    RenderSettings.fogDensity = 0.00018;
    RenderSettings.heightFog = { density: 0.0006, baseHeight: 0, falloff: 0.006 };
    RenderSettings.volumetricFog = { intensity: 0.9, anisotropy: 0.55, steps: 4, distance: 2400, albedo: new Color(1, 0.96, 0.9, 1) };

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.93, 0.8, 1), intensity: 12, shadows: "hard" });

    Random.initState(17);
    this.buildLand();
    this.buildClouds();

    // High on a hillside (out of view behind), looking out towards the sun.
    const eye = new GameObject("Main Camera", { position: new Vector3(0, 45, 330) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, nearClipPlane: 0.5, farClipPlane: 6000, clearFlags: "skybox" });
    apply();
  }

  /**
   * The country below: a patchwork of fields (grass, wheat, barley, ploughed earth) edged by
   * hedgerows with the odd tree, copses, and rolling hills towards the horizon.
   */
  private buildLand() {
    const crops = ["#7fa64a", "#94b858", "#6e9a40", "#c9b45a", "#d8c46a", "#a8a04e", "#8a6a48", "#9ab060"].map((h) => mat(h));
    const hedge = mat("#3e5a26");
    const tree = mat("#355226");
    const hill = mat("#6f9a44");
    shape("Ground", "cube", new Vector3(0, -1, -2000), new Vector3(9000, 2, 9000), crops[0]!);
    // Fields on one grid, turned a little as a whole: each sits inside its own cell, with a
    // gap round it where the hedgerows run, so no two overlap. Some hedges are missing, so
    // neighbouring fields of the same crop run together.
    const CELL = 150;
    const GAP = 6;
    const grid = Quaternion.euler(0, 12, 0);
    const place = (x: number, z: number, y: number) => grid.rotate(new Vector3(x, 0, z)).add(new Vector3(0, y, 0));
    for (let gx = -12; gx <= 12; gx++) {
      for (let gz = -26; gz <= 2; gz++) {
        const cx = gx * CELL;
        const cz = gz * CELL;
        const crop = crops[Math.floor(Random.value * crops.length)]!;
        shape("Field", "cube", place(cx, cz, 0.1), new Vector3(CELL - GAP, 0.4, CELL - GAP), crop, undefined, grid);
        // The hedges on this cell's -x and -z sides (the neighbours' give the others).
        if (Random.value < 0.8) shape("Hedge", "cube", place(cx, cz - CELL / 2, 1.6), new Vector3(CELL - GAP, 3.2, 2.5), hedge, undefined, grid);
        if (Random.value < 0.8) shape("Hedge", "cube", place(cx - CELL / 2, cz, 1.6), new Vector3(2.5, 3.2, CELL - GAP), hedge, undefined, grid);
        if (Random.value < 0.35) {
          const r = Random.range(5, 9);
          shape("Hedge tree", "sphere", place(cx + Random.range(-CELL / 2, CELL / 2), cz - CELL / 2, r * 1.3), new Vector3(r * 2, r * 2.2, r * 2), tree);
        }
      }
    }
    // Copses.
    for (let i = 0; i < 40; i++) {
      const c = new Vector3(Random.range(-1700, 1700), 0, Random.range(-4000, 100));
      for (let k = 0; k < 8; k++) {
        const r = Random.range(7, 13);
        shape("Copse tree", "sphere", c.add(new Vector3(Random.range(-30, 30), r * 1.2, Random.range(-30, 30))), new Vector3(r * 2, r * 2.3, r * 2), tree);
      }
    }
    // Rolling hills, out towards the horizon and to the sides, leaving the plain below open.
    for (let i = 0; i < 40; i++) {
      const x = Random.range(-3000, 3000);
      const z = Random.value < 0.7 ? Random.range(-6000, -3600) : Random.range(-3600, 400);
      const side = Math.abs(x) > 1400 || z < -3600;
      if (!side) continue;
      const r = Random.range(300, 700);
      const h = Random.range(60, 160);
      shape("Hill", "sphere", new Vector3(x, -h * 0.35, z), new Vector3(r * 2, h * 2, r * 2 * Random.range(0.7, 1.3)), hill);
    }
  }

  /**
   * A broken layer of cumulus between the camera and the sun: each cloud a heap of flattened
   * balls, lit white on top and grey underneath, casting the shadows the beams run between.
   */
  private buildClouds() {
    this.sky = new GameObject("Cloud layer");
    // Light scattering inside a cloud keeps its shaded side grey, not black: a glow of its own.
    const white = Material.create({ color: new Color(0.95, 0.95, 0.97, 1), smoothness: 0, emission: new Color(1.1, 1.15, 1.3, 1) });
    // High up, heaped round the line from the camera to the sun (where it crosses the layer,
    // `SUN_GAP`), so the sun peers through the gaps as they drift past.
    for (let i = 0; i < 60; i++) {
      const at = new Vector3(SUN_GAP.x + Random.range(-1700, 1700), CLOUD_BASE + Random.range(0, 120), SUN_GAP.z + Random.range(-1500, 900));
      // A ragged hole round the sun: it peers through, and the hill in front stays in the sun.
      if (Math.hypot(at.x - SUN_GAP.x, at.z - SUN_GAP.z) < GAP_RADIUS + Random.range(0, 120)) continue;
      const size = Random.range(90, 230);
      const puffs = 5 + Math.floor(Random.value * 7);
      for (let k = 0; k < puffs; k++) {
        const r = size * Random.range(0.45, 1);
        const p = at.add(new Vector3(Random.range(-size, size), Random.range(0, size * 0.5), Random.range(-size * 0.7, size * 0.7)));
        shape("Puff", "sphere", p, new Vector3(r * 2, r * 1.1, r * 2), white, this.sky);
      }
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyF")) toggle("haze");
    if (input.getKeyDown("KeyA")) toggle("auto");
    if (input.getKeyDown("Space")) toggle("paused");
    if (!state.paused) this.t += dt;
    // The layer sways slowly to and fro, so the gaps and their beams move while the hole
    // stays near the sun.
    const a = (this.t / SWAY_TIME) * Math.PI * 2;
    this.sky.transform.position = new Vector3(Math.sin(a) * SWAY, 0, Math.sin(a * 0.7) * SWAY * 0.4);
    // A slow look across the beams.
    const cam = this.camera.transform;
    const t = this.t;
    // Tilted up a little, so the sun (30 degrees up) sits inside the view among the clouds.
    cam.rotation = Quaternion.lookRotation(new Vector3(Math.sin(t * 0.03) * 0.2 + 0.12, 0.18 + Math.sin(t * 0.05) * 0.02, -1));
  }
}

function toggle(key: "haze" | "auto" | "paused") {
  if (key === "haze") state.haze = !state.haze;
  else if (key === "auto") state.auto = !state.auto;
  else state.paused = !state.paused;
  apply();
}

register({ Clouds });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#d0d8e2", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 140, backgroundColor: props.on ? "#3a7bd5" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f4f6f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#808a96", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 16, 22, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the clouds</text>
        <Button label="Haze" value={state.haze ? "On" : "Off"} on={state.haze} keyName="F" onClick={() => toggle("haze")} />
        <Button label="Auto exposure" value={state.auto ? "On" : "Off"} on={state.auto} keyName="A" onClick={() => toggle("auto")} />
        <Button label="Clouds" value={state.paused ? "Paused" : "Drifting"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
