// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Teapots in the fog: the classic distance-fog picture. A row of shiny orange teapots
// recedes into grey fog over a glossy checkerboard that reflects them (a `Mirror` on the
// floor), seen from low down; the far ones fade until the last are only ghosts.
// The menu (or keys) cycles the fog's mode (F) and moves the linear fog's end (Up / Down).
//
// Model: the Utah teapot from the Newell teaset (Martin Newell, University of Utah, 1975),
// converted to .glb by examples/classics/tools/make_models.py.

const GREY = new Color(0.47, 0.47, 0.47, 1);
const MODES: (FogMode | "off")[] = ["linear", "exponential", "exponentialSquared", "off"];
const COUNT = 8;
/** Teapots step back along this line, `SPACING` metres apart. */
const ROW = new Vector3(0.75, 0, -1).normalized;
const SPACING = 3.2;
const FIRST = new Vector3(0, 0, 0);
const FOG_START = 5;

/** What the menu shows and changes; `apply` sends it to `RenderSettings`. */
const state = { mode: MODES[0]!, end: 27 };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.fogColor = GREY;
  RenderSettings.fogDensity = 0.08;
  RenderSettings.fogStartDistance = FOG_START;
  RenderSettings.fogEndDistance = state.end;
  if (state.mode !== "off") RenderSettings.fogMode = state.mode;
  RenderSettings.fog = state.mode !== "off";
  refreshMenu();
}

class Teapots extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.32, 0.32, 0.32, 1);

    // A soft key light from the upper left, a little in front.
    const key = new GameObject("Key light", { rotation: Quaternion.lookRotation(new Vector3(0.6, -0.7, -0.4).normalized) });
    key.addComponent(Light, { type: "directional", color: new Color(1, 0.97, 0.92, 1), intensity: 1.1, shadows: "soft" });

    // The floor: 120 m across (the plane is 10 m), 1 m squares, glossy and partly a mirror.
    const floor = new GameObject("Floor", { position: new Vector3(0, 0, -30), scale: Vector3.one.scale(12) });
    floor.addComponent(MeshRenderer, {
      mesh: "plane",
      material: Material.create({ color: Color.white, mainTexture: checker(), mainTextureScale: new Vector2(60, 60), metallic: 0.4, smoothness: 0.92 }),
    });
    floor.addComponent(Mirror);

    // The row: one model, instantiated along the diagonal, all in glossy orange.
    const orange = Material.create({ color: new Color(1, 0.28, 0.02, 1), metallic: 0.6, smoothness: 0.8 });
    const model = Model.load("assets/teapot.glb");
    const pots: GameObject[] = [];
    for (let i = 0; i < COUNT; i++) {
      const at = FIRST.add(ROW.scale(i * SPACING));
      pots.push(model.instantiate({ position: at, rotation: Quaternion.euler(0, -25, 0) }));
    }
    model.whenLoaded().then(() => {
      for (const go of pots) for (const r of collect(go)) r.material = orange;
    });

    const eye = new GameObject("Main Camera", { position: new Vector3(-3.6, 0.6, 0.9) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 50, backgroundColor: GREY, farClipPlane: 150 });
    apply();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyF")) cycleMode();
    if (input.getKeyDown("ArrowUp")) moveEnd(2);
    if (input.getKeyDown("ArrowDown")) moveEnd(-2);
    // A very slow sway, low over the floor, always looking along the row.
    const cam = this.camera.transform;
    cam.position = new Vector3(-3.6 + Math.sin(this.t * 0.12) * 0.35, 0.6 + Math.sin(this.t * 0.09) * 0.06, 0.9);
    cam.rotation = Quaternion.lookRotation(FIRST.add(ROW.scale(2.2)).add(new Vector3(0, 0.45, 0)).sub(cam.position));
  }
}

/** A black-and-white checker, 2 × 2 squares, 64 texels each so the edges stay sharp up close. */
function checker(): Texture {
  const n = 128;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const white = (x < n / 2) !== (y < n / 2);
      px.push(white ? new Color(0.85, 0.85, 0.85, 1) : new Color(0.02, 0.02, 0.02, 1));
    }
  }
  // Trilinear and anisotropic rather than point: point sampling shimmers in the distance.
  return Texture.create(n, n, px, { filterMode: "trilinear", anisoLevel: 16 });
}

/** Every MeshRenderer under `go`. */
function collect(go: GameObject): MeshRenderer[] {
  const out: MeshRenderer[] = [];
  const visit = (o: GameObject) => {
    const r = o.getComponent(MeshRenderer);
    if (r) out.push(r);
    for (const c of o.children) visit(c);
  };
  visit(go);
  return out;
}

function cycleMode() {
  let i = 0;
  for (let k = 0; k < MODES.length; k++) if (MODES[k] === state.mode) i = k;
  state.mode = MODES[(i + 1) % MODES.length]!;
  apply();
}
function moveEnd(by: number) {
  state.end = Math.min(80, Math.max(FOG_START + 2, state.end + by));
  apply();
}

register({ Teapots });

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

const MODE_NAMES: Record<FogMode | "off", string> = { linear: "Linear", exponential: "Exponential", exponentialSquared: "Exponential squared", off: "Off" };

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Teapots in the fog</text>
        <Button label="Fog" value={MODE_NAMES[state.mode]!} on={state.mode !== "off"} keyName="F" onClick={cycleMode} />
        <Button label="Linear end" value={`${FOG_START}–${state.end} m`} on={state.mode === "linear"} keyName="Up / Down" onClick={() => moveEnd(2)} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
