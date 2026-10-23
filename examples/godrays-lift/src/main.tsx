// SPDX-License-Identifier: Apache-2.0

// God rays: the lift. A dark hotel corridor late at night: a carpet, doors to rooms, the
// green glow of an exit sign. At the end of it the lift is coming: the lamps over its doors
// count the floors down. It arrives and its doors slide open: a thin sliver of hard white
// light at their seam widens into a block of light that floods out along the dusty corridor
// and across the carpet, dazzling eyes used to the dark until they adjust. The doors stay
// open for a while, close, and the corridor sinks back into the dark. Then the lift comes
// again. The menu (or keys) switches the dust in the air (F), auto exposure (A) and pauses
// the lift (Space).

// The corridor runs along z from 0 (the lift at its end) to LENGTH, x from -HALF to HALF, H tall.
const LENGTH = 16;
const HALF = 1.4;
const H = 2.6;
/** The lift's doorway in the end wall: its half width and height; the car behind it. */
const DOOR_HALF = 0.55;
const DOOR_H = 2.15;
const CAR_DEPTH = 1.6;

/** The cycle, in seconds: floors counted down, the doors opening, open, closing, then dark. */
const ARRIVE = 4;
const OPENING = 1.8;
const OPEN = 6;
const CLOSING = 1.8;
const DARK = 4;
const CYCLE = ARRIVE + OPENING + OPEN + CLOSING + DARK;
const FLOOR_LAMPS = 5;
/** The car's light out through the doorway, fully open. */
const FLOOD = 2.5;

const state = { dust: true, auto: true, paused: false };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fog = state.dust;
  // Eyes used to the dark: slow to adapt down, so the light dazzles for a while.
  RenderSettings.autoExposure = state.auto ? { minEV: -6, maxEV: 12, compensation: -2, speedDarkToLight: 0.9, speedLightToDark: 0.4 } : null;
  RenderSettings.postExposure = state.auto ? 0 : 2;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
/** A box between two corners. */
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material, parent?: GameObject) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material, parent);
}
const mat = (hex: string, smoothness = 0.1) => Material.create({ color: Color.hex(hex), smoothness });
const ease = (u: number) => {
  const c = Math.min(1, Math.max(0, u));
  return c * c * (3 - 2 * c);
};

class Corridor extends Component {
  private camera!: Camera;
  private t = 0;
  private doors: { go: GameObject; side: number }[] = [];
  private lamps: MeshRenderer[] = [];
  private lampOn!: Material;
  private lampOff!: Material;
  private lit = -2;
  private flood!: Light;
  /** The car's fill: no shadows, so it too is off while the doors are shut. */
  private fill!: Light;

  override onStart() {
    QualitySettings.shadowDistance = 30;
    QualitySettings.shadowResolution = 4096;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.006, 0.006, 0.008, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1, intensity: 0.55, scatter: 0.65 };
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogColor = new Color(0, 0, 0, 1);
    RenderSettings.fogDensity = 0.05;
    RenderSettings.volumetricFog = { intensity: 0.6, anisotropy: 0.45, steps: 4, distance: 20 };

    this.buildCorridor();
    this.buildLift();

    // Down the corridor, off to one side, looking back at the lift.
    const eye = new GameObject("Main Camera", { position: new Vector3(0.75, 1.6, 9.5) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 64, backgroundColor: Color.black, nearClipPlane: 0.05, farClipPlane: 60 });
    apply();
  }

  private buildCorridor() {
    const carpet = mat("#5a2630", 0.02);
    const wall = mat("#b8a98e");
    const ceiling = mat("#d8d2c4");
    const wood = mat("#4a3020", 0.35);
    const trim = mat("#c9b27a", 0.5);
    slab("Floor", -HALF - 0.2, -0.2, -CAR_DEPTH - 0.4, HALF + 0.2, 0, LENGTH + 0.2, carpet);
    slab("Ceiling", -HALF - 0.2, H, 0, HALF + 0.2, H + 0.2, LENGTH + 0.2, ceiling);
    for (const s of [-1, 1]) {
      slab("Wall", s * HALF, -0.2, 0, s * (HALF + 0.2), H + 0.2, LENGTH, wall);
      slab("Skirting", s * (HALF - 0.02), 0, 0, s * HALF, 0.12, LENGTH, wood);
      slab("Dado rail", s * (HALF - 0.025), 0.95, 0, s * HALF, 1.0, LENGTH, trim);
    }
    slab("Wall", -HALF - 0.2, -0.2, LENGTH, HALF + 0.2, H + 0.2, LENGTH + 0.2, wall);
    // Room doors along both sides, with handles and numbers.
    for (let z = 3; z < LENGTH - 1; z += 3.2) {
      for (const s of [-1, 1]) {
        const zz = z + (s > 0 ? 1.4 : 0);
        slab("Room door", s * (HALF - 0.04), 0, zz - 0.45, s * HALF, 2.05, zz + 0.45, wood);
        slab("Door handle", s * (HALF - 0.07), 1.0, zz + 0.3, s * (HALF - 0.04), 1.04, zz + 0.38, trim);
        slab("Room number", s * (HALF - 0.05), 1.55, zz - 0.1, s * (HALF - 0.04), 1.65, zz + 0.1, trim);
      }
    }
    // The exit sign over the far end: a faint green glow, the corridor's only other light.
    const sign = Material.create({ color: Color.black, emission: new Color(0.1, 1.4, 0.3, 1), unlit: true });
    slab("Exit sign", -0.3, H - 0.32, LENGTH - 0.06, 0.3, H - 0.1, LENGTH - 0.02, sign);
    const exit = new GameObject("Exit light", { position: new Vector3(0, H - 0.4, LENGTH - 0.3) });
    exit.addComponent(Light, { type: "point", color: new Color(0.2, 1, 0.35, 1), intensity: 0.25, range: 4, volumetricIntensity: 0.4 });
  }

  /** The end wall with the doorway, the car behind it (lit), its doors and the floor lamps. */
  private buildLift() {
    const wall = mat("#b8a98e");
    const steel = mat("#9aa0a8", 0.75);
    const brass = mat("#b08a3a", 0.6);
    slab("End wall", -HALF - 0.2, -0.2, -0.2, -DOOR_HALF - 0.12, H + 0.2, 0, wall);
    slab("End wall", DOOR_HALF + 0.12, -0.2, -0.2, HALF + 0.2, H + 0.2, 0, wall);
    slab("End wall", -DOOR_HALF - 0.12, DOOR_H + 0.1, -0.2, DOOR_HALF + 0.12, H + 0.2, 0, wall);
    // A brass surround, standing proud of the wall.
    slab("Surround", -DOOR_HALF - 0.12, 0, 0, -DOOR_HALF, DOOR_H + 0.1, 0.03, brass);
    slab("Surround", DOOR_HALF, 0, 0, DOOR_HALF + 0.12, DOOR_H + 0.1, 0.03, brass);
    slab("Surround", -DOOR_HALF - 0.12, DOOR_H, 0, DOOR_HALF + 0.12, DOOR_H + 0.1, 0.03, brass);
    // The car: brushed steel inside, a bright ceiling panel.
    const back = -0.2 - CAR_DEPTH;
    slab("Car back", -DOOR_HALF - 0.4, 0, back - 0.05, DOOR_HALF + 0.4, DOOR_H + 0.2, back, steel);
    for (const s of [-1, 1]) slab("Car side", s * (DOOR_HALF + 0.4), 0, back, s * (DOOR_HALF + 0.45), DOOR_H + 0.2, -0.2, steel);
    slab("Car ceiling", -DOOR_HALF - 0.4, DOOR_H + 0.2, back, DOOR_HALF + 0.4, DOOR_H + 0.25, -0.2, steel);
    const panel = Material.create({ color: Color.black, emission: new Color(18, 18, 17, 1), unlit: true });
    slab("Ceiling light", -0.5, DOOR_H + 0.17, back + 0.2, 0.5, DOOR_H + 0.2, -0.4, panel);
    slab("Handrail", -DOOR_HALF - 0.3, 0.95, back + 0.05, DOOR_HALF + 0.3, 1.0, back + 0.1, brass);
    // The car's light: a wide cone out through the doorway (it's what floods the corridor,
    // its edges cut by the doors) and a fill inside.
    const flood = new GameObject("Car light", { position: new Vector3(0, DOOR_H + 0.1, back + 0.25), rotation: Quaternion.lookRotation(new Vector3(0, -0.32, 1)) });
    // Off while the doors are shut (its shadow through thin steel would leak), growing as they
    // part: the gap is what lets the light out; the doors' shadow shapes its edges.
    this.flood = flood.addComponent(Light, { type: "spot", color: new Color(0.95, 0.97, 1, 1), intensity: 0, range: 20, spotAngle: 120, innerSpotAngle: 90, shadows: "hard" });
    const fill = new GameObject("Car fill", { position: new Vector3(0, DOOR_H - 0.2, back + CAR_DEPTH / 2) });
    this.fill = fill.addComponent(Light, { type: "point", color: new Color(0.95, 0.97, 1, 1), intensity: 0, range: 2.5, volumetricIntensity: 0 });
    // The doors: two steel leaves meeting in the middle, sliding apart behind the wall. They
    // overlap the doorway's sides and top, so no light gets round them shut.
    for (const side of [-1, 1]) {
      const go = new GameObject("Lift door");
      slab("Door leaf", 0, 0, -0.14, side * (DOOR_HALF + 0.2), DOOR_H + 0.18, -0.06, steel, go);
      this.doors.push({ go, side });
    }
    // Floor lamps over the doorway, counting the car down.
    this.lampOff = Material.create({ color: Color.hex("#2a2418"), smoothness: 0.5 });
    this.lampOn = Material.create({ color: Color.black, emission: new Color(6, 3.6, 1.2, 1), unlit: true });
    for (let i = 0; i < FLOOR_LAMPS; i++) {
      const x = (i - (FLOOR_LAMPS - 1) / 2) * 0.16;
      const lamp = shape("Floor lamp", "sphere", new Vector3(x, DOOR_H + 0.27, 0.04), new Vector3(0.07, 0.07, 0.04), this.lampOff);
      this.lamps.push(lamp.getComponent(MeshRenderer)!);
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyF")) toggle("dust");
    if (input.getKeyDown("KeyA")) toggle("auto");
    if (input.getKeyDown("Space")) toggle("paused");
    if (!state.paused) this.t = (this.t + dt) % CYCLE;
    const t = this.t;
    // The lamps count down while it arrives; the last stays lit while it's here.
    const lit = t < ARRIVE ? FLOOR_LAMPS - 1 - Math.floor((t / ARRIVE) * FLOOR_LAMPS) : t < CYCLE - DARK ? 0 : -1;
    if (lit !== this.lit) {
      this.lit = lit;
      for (let i = 0; i < FLOOR_LAMPS; i++) this.lamps[i]!.material = i === FLOOR_LAMPS - 1 - lit ? this.lampOn : this.lampOff;
    }
    // Open: 0 shut, 1 fully apart.
    const open = ease((t - ARRIVE) / OPENING) - ease((t - ARRIVE - OPENING - OPEN) / CLOSING);
    this.flood.intensity = FLOOD * Math.sqrt(open);
    this.fill.intensity = 3 * open;
    for (const { go, side } of this.doors) go.transform.position = new Vector3(side * open * (DOOR_HALF + 0.05), 0, 0);
    // Standing still, breathing a little.
    const cam = this.camera.transform;
    cam.position = new Vector3(0.75 + Math.sin(t * 0.4) * 0.03, 1.6 + Math.sin(t * 0.9) * 0.01, 9.5);
    cam.rotation = Quaternion.lookRotation(new Vector3(-0.75, -0.25, -9.5));
  }
}

function toggle(key: "dust" | "auto" | "paused") {
  if (key === "dust") state.dust = !state.dust;
  else if (key === "auto") state.auto = !state.auto;
  else state.paused = !state.paused;
  apply();
}

register({ Corridor });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#d4c8b6", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 140, backgroundColor: props.on ? "#b07a3a" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#fbf3e8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#8a7f70", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(16, 12, 8, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the lift</text>
        <Button label="Dust" value={state.dust ? "In the air" : "Off"} on={state.dust} keyName="F" onClick={() => toggle("dust")} />
        <Button label="Auto exposure" value={state.auto ? "On" : "Off"} on={state.auto} keyName="A" onClick={() => toggle("auto")} />
        <Button label="Lift" value={state.paused ? "Paused" : "Running"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
