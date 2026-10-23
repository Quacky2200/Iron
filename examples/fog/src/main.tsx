// SPDX-License-Identifier: Apache-2.0

// Fog: a misty clearing at dusk. A low sun shines through a slatted pergola and between the
// trees, so light shafts cross the clearing; a lamp post's spot light makes a cone in the
// mist; height fog lies in the valley and swallows the horizon. The camera drifts slowly.
// The menu (or keys) switches the distance fog's mode (F), height fog (H), light shafts (L)
// and the shafts' samples per slice (Q).

const MIST = new Color(0.3, 0.33, 0.4, 1);
const MODES: (FogMode | "off")[] = ["exponentialSquared", "exponential", "linear", "off"];
const STEPS = [4, 8, 1];

/** What the menu shows and changes; `apply` sends it to `RenderSettings`. */
const state = { mode: MODES[0]!, height: true, shafts: true, steps: STEPS[0]! };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fogColor = MIST;
  RenderSettings.fogDensity = 0.028;
  RenderSettings.fogStartDistance = 4;
  RenderSettings.fogEndDistance = 70;
  if (state.mode !== "off") RenderSettings.fogMode = state.mode;
  RenderSettings.heightFog = state.height ? { density: 0.05, baseHeight: 0, falloff: 0.35 } : null;
  RenderSettings.volumetricFog = state.shafts ? { intensity: 0.2, anisotropy: 0.3, steps: state.steps, distance: 45 } : null;
  // Height fog and shafts ride on the fog: with the distance fog "off", keep the fog on
  // with no density so they still show.
  RenderSettings.fog = state.mode !== "off" || state.height || state.shafts;
  if (state.mode === "off") {
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogDensity = 0;
  }
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder" | "plane", at: Vector3, scale: Vector3, material: Material, parent?: GameObject) {
  const go = new GameObject(name, { position: at, scale, parent });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}

class Clearing extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 45;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.12, 0.13, 0.17, 1);
    RenderSettings.tonemapping = "aces";

    // A low, warm sun behind the pergola and off to the side, slanting towards the camera.
    const sunDir = new Vector3(0.7, -0.5, 0.5).normalized;
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(sunDir) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.82, 0.6, 1), intensity: 1.5, shadows: "hard" });

    const ground = new GameObject("Ground", { scale: Vector3.one.scale(16) });
    ground.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.24, 0.3, 0.2, 1), smoothness: 0.05 }) });

    // The pergola: six pillars under a roof of slats.
    const stone = Material.create({ color: new Color(0.62, 0.6, 0.56, 1), smoothness: 0.2 });
    const wood = Material.create({ color: new Color(0.36, 0.25, 0.16, 1), smoothness: 0.15 });
    for (const x of [-4, 4]) {
      for (const z of [-4, 0, 4]) shape("Pillar", "cylinder", new Vector3(x, 2.6, z), new Vector3(0.6, 2.6, 0.6), stone);
      shape("Beam", "cube", new Vector3(x, 5.3, 0), new Vector3(0.5, 0.3, 9.4), wood);
    }
    for (let i = 0; i < 12; i++) shape("Slat", "cube", new Vector3(0, 5.6, -4.4 + i * 0.8), new Vector3(9.6, 0.22, 0.42), wood);

    // Trees all round, thinning out into the mist.
    Random.initState(11);
    const bark = Material.create({ color: new Color(0.3, 0.22, 0.15, 1), smoothness: 0.1 });
    const leaves = Material.create({ color: new Color(0.16, 0.3, 0.16, 1), smoothness: 0.1 });
    for (let i = 0; i < 90; i++) {
      const a = Random.range(0, Math.PI * 2);
      const r = Random.range(11, 70);
      const at = new Vector3(Math.cos(a) * r, 0, Math.sin(a) * r);
      // Keep the view down the clearing open.
      if (at.z > 6 && Math.abs(at.x) < 10) continue;
      const h = Random.range(4, 9);
      const tree = new GameObject(`Tree ${i}`, { position: at });
      shape("Trunk", "cylinder", new Vector3(0, h / 2, 0), new Vector3(0.5, h / 2, 0.5), bark, tree);
      shape("Crown", "sphere", new Vector3(0, h, 0), Vector3.one.scale(Random.range(2.5, 4)), leaves, tree);
    }

    // A lamp post by the path, its spot light a cone in the mist.
    const iron = Material.create({ color: new Color(0.1, 0.1, 0.11, 1), metallic: 0.8, smoothness: 0.5 });
    shape("Post", "cylinder", new Vector3(6.5, 2, 7), new Vector3(0.15, 2, 0.15), iron);
    // The head hangs off an arm, the light inside its globe (which casts no shadow, or it
    // would hide the light).
    shape("Arm", "cube", new Vector3(6.9, 4, 7), new Vector3(0.9, 0.08, 0.08), iron);
    const globe = shape("Lamp", "sphere", new Vector3(7.3, 3.85, 7), Vector3.one.scale(0.3), Material.create({ color: Color.black, emission: new Color(4, 3.2, 2, 1), unlit: true }));
    globe.getComponent(MeshRenderer)!.shadowCastingMode = "off";
    const lamp = new GameObject("Lamp light", { position: new Vector3(7.3, 3.85, 7), rotation: Quaternion.lookRotation(Vector3.down, Vector3.forward) });
    lamp.addComponent(Light, { type: "spot", color: new Color(1, 0.8, 0.55, 1), intensity: 80, range: 10, spotAngle: 50, shadows: "hard", volumetricIntensity: 3 });

    const eye = new GameObject("Main Camera", { position: new Vector3(0, 2.2, 20) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 55, backgroundColor: MIST, farClipPlane: 200 });
    apply();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyF")) cycleMode();
    if (input.getKeyDown("KeyH")) toggle("height");
    if (input.getKeyDown("KeyL")) toggle("shafts");
    if (input.getKeyDown("KeyQ")) cycleSteps();
    // A slow drift across the clearing, always looking at the pergola.
    const cam = this.camera.transform;
    cam.position = new Vector3(Math.sin(this.t * 0.08) * 7, 2.2 + Math.sin(this.t * 0.11) * 0.4, 20 + Math.sin(this.t * 0.05) * 2);
    cam.rotation = Quaternion.lookRotation(new Vector3(0, 3.2, 0).sub(cam.position));
  }
}

function cycleMode() {
  let i = 0;
  for (let k = 0; k < MODES.length; k++) if (MODES[k] === state.mode) i = k;
  state.mode = MODES[(i + 1) % MODES.length]!;
  apply();
}
function toggle(key: "height" | "shafts") {
  if (key === "height") state.height = !state.height;
  else state.shafts = !state.shafts;
  apply();
}
function cycleSteps() {
  state.steps = STEPS[(STEPS.indexOf(state.steps) + 1) % STEPS.length]!;
  apply();
}

register({ Clearing });

function Button(props: { label: string; value: string; on?: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on === false ? "rgba(255,255,255,0.1)" : "#4d8dff" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#6f7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

const MODE_NAMES: Record<FogMode | "off", string> = { exponentialSquared: "Exponential squared", exponential: "Exponential", linear: "Linear (4–70 m)", off: "Off" };

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Fog</text>
        <Button label="Distance fog" value={MODE_NAMES[state.mode]!} on={state.mode !== "off"} keyName="F" onClick={cycleMode} />
        <Button label="Height fog" value={state.height ? "On" : "Off"} on={state.height} keyName="H" onClick={() => toggle("height")} />
        <Button label="Light shafts" value={state.shafts ? "On" : "Off"} on={state.shafts} keyName="L" onClick={() => toggle("shafts")} />
        <Button label="Shaft samples" value={`${state.steps} per slice`} on={state.shafts} keyName="Q" onClick={cycleSteps} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
