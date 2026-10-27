// SPDX-License-Identifier: Apache-2.0

// Real-world light units (Unity 7's and HDRP's). A sunlit stone courtyard on a clear day (the sun
// at 100000 lux, the sky some 5000 nits) and, through a doorway, a hall lit by three lamps
// (1500 lumens each): about ten stops darker, as in life. The camera walks in from the sun
// and out again; auto exposure adapts across the gap (dazzled coming out, slow to see going
// in). The readout shows the exposure in use, how bright the view meters and the light
// falling on the camera (`Light.illuminanceAt`, in lux). P switches to a physical camera
// (`Camera.aperture` / `shutterSpeed` / `iso`): f/16, 1/200 s, ISO 200 suits the courtyard,
// and the hall goes black; Space pauses the walk.
//
// The project chooses its light units in its package.json: "platypus": { "lightUnits": "physical" }.

/** Where sunlight travels: high, from the south-west, behind the camera as it walks in. */
const SUN = new Vector3(0.35, -0.8, -0.45).normalized;
/** The sun's illuminance (lux) and the sky's luminance (nits): a clear midday. */
const SUN_LUX = 100000;
const SKY_NITS = 9000;
/** Each hall lamp's output (lumens), about a 100 W incandescent bulb. */
const LAMP_LUMENS = 1500;

// The courtyard: x and z within ±COURT; the hall north of it, z from -HALL_DEPTH to -COURT.
const COURT = 10;
const HALL_DEPTH = 26;
const WALL_H = 6;
const DOOR = { x0: -1.4, x1: 1.4, h: 3.2 };

/** The walk: the camera's z over time (s), eased between, looping. */
const WALK: [number, number][] = [
  [0, 7],
  [4, 4],
  [10, -14],
  [17, -18],
  [23, -6],
  [27, 6],
  [31, 7],
];
const WALK_TIME = 31;

const state = { physical: false, paused: false };
let refreshMenu = () => {};
let cameraRef: Camera | null = null;
/** The light meter's last reading at the camera, in lux. */
let lux = 0;

function apply(): void {
  if (state.physical && cameraRef) {
    // The physical camera's own exposure, fixed.
    RenderSettings.autoExposure = null;
    RenderSettings.fixedExposure = cameraRef.physicalExposure;
  } else {
    RenderSettings.fixedExposure = null;
    // Both ways in under a second: quicker than eyes, as games do, so there's no waiting
    // for the dark to come up.
    RenderSettings.autoExposure = { minEV: -2, maxEV: 18, compensation: 0.5, speedDarkToLight: 4, speedLightToDark: 3.5, metering: "centerWeighted", lowPercent: 30, highPercent: 95 };
  }
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material) {
  const go = new GameObject(name, { position: at, scale });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material);
}
/** A slab with a collider, so the light meter's rays (`Light.measureIlluminance`) stop at it. */
function solid(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material) {
  slab(name, x0, y0, z0, x1, y1, z1, material).addComponent(BoxCollider);
}
const mat = (hex: string, smoothness = 0.1) => Material.create({ color: Color.hex(hex), smoothness });

function zAt(t: number): number {
  for (let i = 0; i + 1 < WALK.length; i++) {
    const [t0, z0] = WALK[i]!;
    const [t1, z1] = WALK[i + 1]!;
    if (t <= t1) {
      const u = Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
      return z0 + (z1 - z0) * u * u * (3 - 2 * u);
    }
  }
  return WALK[WALK.length - 1]![1];
}

class Courtyard extends Component {
  private camera!: Camera;
  private t = 0;
  private readout = 0;

  override onStart() {
    QualitySettings.shadowDistance = 60;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    RenderSettings.ambientMode = "color";
    // The sky's light, in nits; the hall's surfaces see little of it (occlusion below).
    RenderSettings.ambientLight = new Color(SKY_NITS * 0.75, SKY_NITS * 0.85, SKY_NITS, 1);
    RenderSettings.skybox = {
      zenith: new Color(SKY_NITS * 0.45, SKY_NITS * 0.7, SKY_NITS * 1.3, 1),
      horizon: new Color(SKY_NITS, SKY_NITS * 1.05, SKY_NITS * 1.1, 1),
      ground: new Color(SKY_NITS * 0.3, SKY_NITS * 0.28, SKY_NITS * 0.25, 1),
      sunDirection: SUN.scale(-1),
      // The sun's disc: far brighter than anything else (pre-exposure keeps it in range).
      sunColor: new Color(5e7, 4.8e7, 4.5e7, 1),
      sunSize: 0.6,
    };
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1.5, intensity: 0.4, scatter: 0.6 };

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.96, 0.9, 1), intensity: SUN_LUX, shadows: "hard" });

    this.build();
    const eye = new GameObject("Main Camera", { position: new Vector3(0, 1.65, 7) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 65, farClipPlane: 400, clearFlags: "skybox" });
    cameraRef = this.camera;
    apply();
  }

  private build() {
    const stone = mat("#b9ae98");
    const paving = mat("#a39a88", 0.2);
    const plaster = mat("#d8d0c0");
    // The hall's inside barely sees the sky: an occlusion map cuts its ambient light (and
    // reflections) to a few percent, as baked lighting would.
    const occlusion = Texture.create(1, 1, [new Color(0.03, 0.03, 0.03, 1)], { linear: true });
    const shut = Material.create({ color: Color.hex("#c8bca6"), smoothness: 0.15, occlusionMap: occlusion });
    const floorIn = Material.create({ color: Color.hex("#6a5440"), smoothness: 0.35, occlusionMap: occlusion });
    const columns = Material.create({ color: Color.hex("#d8d0c0"), smoothness: 0.1, occlusionMap: occlusion });
    slab("Paving", -COURT, -0.2, -COURT, COURT, 0, COURT, paving);
    for (const s of [-1, 1]) solid("Court wall", s * COURT, 0, -COURT, s * (COURT + 0.6), WALL_H, COURT, stone);
    solid("Court wall", -COURT, 0, COURT, COURT, WALL_H, COURT + 0.6, stone);
    // The hall's front wall, with the doorway.
    solid("Hall front", -COURT, 0, -COURT - 0.6, DOOR.x0, WALL_H, -COURT, plaster);
    solid("Hall front", DOOR.x1, 0, -COURT - 0.6, COURT, WALL_H, -COURT, plaster);
    solid("Hall front", DOOR.x0, DOOR.h, -COURT - 0.6, DOOR.x1, WALL_H, -COURT, plaster);
    // The hall: walls, roof, floor.
    const back = -COURT - HALL_DEPTH;
    slab("Hall floor", -COURT, -0.2, back, COURT, 0.001, -COURT - 0.6, floorIn);
    solid("Hall roof", -COURT - 0.6, WALL_H, back - 0.6, COURT + 0.6, WALL_H + 0.6, -COURT, shut);
    for (const s of [-1, 1]) solid("Hall wall", s * COURT, 0, back, s * (COURT + 0.6), WALL_H, -COURT - 0.6, shut);
    solid("Hall wall", -COURT, 0, back - 0.6, COURT, WALL_H, back, shut);
    // Columns and benches in the hall, a fountain basin in the courtyard.
    for (let z = -COURT - 5; z > back + 2; z -= 6) for (const x of [-5, 5]) shape("Column", "cylinder", new Vector3(x, WALL_H / 2, z), new Vector3(0.7, WALL_H / 2, 0.7), columns);
    shape("Basin", "cylinder", new Vector3(0, 0.35, 2), new Vector3(4, 0.35, 4), stone);
    shape("Water", "cylinder", new Vector3(0, 0.62, 2), new Vector3(3.5, 0.02, 3.5), Material.create({ color: Color.hex("#3a5a6a"), smoothness: 0.95 }));
    // Lamps: a glowing bulb (some 20000 nits) and a 1500 lumen point light each.
    const bulb = Material.create({ color: Color.black, emission: new Color(20000, 16000, 11000, 1), unlit: true });
    for (const z of [-COURT - 7, -COURT - 15, -COURT - 23]) {
      shape("Bulb", "sphere", new Vector3(0, WALL_H - 1.2, z), new Vector3(0.25, 0.25, 0.25), bulb);
      const lamp = new GameObject("Lamp", { position: new Vector3(0, WALL_H - 1.4, z) });
      lamp.addComponent(Light, { type: "point", color: new Color(1, 0.8, 0.55, 1), intensity: LAMP_LUMENS, range: 18, shadows: "hard" });
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyP")) {
      state.physical = !state.physical;
      apply();
    }
    if (input.getKeyDown("Space")) {
      state.paused = !state.paused;
      refreshMenu();
    }
    if (!state.paused) this.t = (this.t + dt) % WALK_TIME;
    const z = zAt(this.t);
    const going = zAt(this.t + 0.1) <= z;
    const cam = this.camera.transform;
    cam.position = new Vector3(Math.sin(this.t * 0.3) * 0.4, 1.65, z);
    cam.rotation = Quaternion.lookRotation(new Vector3(Math.sin(this.t * 0.2) * 0.3, -0.08, going ? -1 : 1));
    this.readout += dt;
    if (this.readout > 0.2) {
      this.readout = 0;
      // Light falling where the camera stands (an incident meter held facing up), shadows
      // counted: indoors the roof keeps the sun off it.
      void Light.measureIlluminance(cam.position, Vector3.up).then((v) => (lux = v));
      refreshMenu();
    }
  }
}

register({ Courtyard });

const ev = (v: number | undefined) => (v === undefined || !Number.isFinite(v) ? "…" : `EV ${v.toFixed(1)}`);

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  const cam = cameraRef;
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 6, borderRadius: 10, backgroundColor: "rgba(12, 14, 18, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Real-world light units</text>
        <text style={{ color: "#c8ccd2", fontSize: 13 }}>Light units: {GraphicsSettings.lightUnits}</text>
        <text style={{ color: "#c8ccd2", fontSize: 13 }}>
          Exposure {ev(cam?.exposure)} · view meters {ev(cam?.sceneBrightness)}
        </text>
        <text style={{ color: "#c8ccd2", fontSize: 13 }}>Light on the camera: {Math.round(lux)} lux</text>
        <text style={{ color: "#c8ccd2", fontSize: 13 }}>
          {state.physical ? `P: physical camera, f/${cam?.aperture} 1/${Math.round(1 / (cam?.shutterSpeed ?? 1))} s ISO ${cam?.iso} (${ev(cam?.physicalExposure)})` : "P: auto exposure (eyes adapt)"}
        </text>
        <text style={{ color: "#8d939c", fontSize: 12 }}>Space: {state.paused ? "paused" : "walking"}</text>
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
