// SPDX-License-Identifier: Apache-2.0

// Layers and culling: a forest with a walker. The main camera skips the "Minimap" layer
// (bright map markers over everything) and draws the "Detail" layer (rocks and
// flowers) only out to 14 m; a top-down minimap camera draws just the ground, the
// walker and the markers. Trees have levels of detail: round crowns up close, blocky
// ones farther out, nothing when tiny. The menu turns layers on and off for the main
// camera and changes its field of view, render distance and the LOD bias.

let main: Camera | undefined;
/** Re-renders the menu once the camera exists. */
let refreshMenu = () => {};

function shape(name: string, mesh: "cube" | "sphere" | "capsule" | "cylinder", at: Vector3, scale: Vector3, material: Material, layer: string, parent?: GameObject) {
  const go = new GameObject(name, { position: at, scale, parent });
  go.addComponent(MeshRenderer, { mesh, material });
  go.layer = LayerMask.nameToLayer(layer);
  return go;
}

/** A flat disc high above its object, seen only by the minimap. */
function marker(parent: GameObject, color: Color, size: number) {
  shape("Marker", "cylinder", new Vector3(0, 8, 0), new Vector3(size, 0.05, size), Material.create({ color, unlit: true }), "Minimap", parent);
}

class Walker extends Component {
  private angle = 0;
  override onUpdate(dt: number) {
    this.angle += dt * 0.4;
    const at = new Vector3(Math.cos(this.angle) * 7, 0.9, Math.sin(this.angle) * 7);
    this.transform.position = at;
    this.transform.rotation = Quaternion.euler(0, -this.angle * (180 / Math.PI), 0);
  }
}

class LayersDemo extends Component {
  override onStart() {
    RenderSettings.skybox = { sunDirection: new Vector3(-0.4, 0.7, 0.3) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0.4, -0.7, -0.3)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.2, shadows: "soft" });

    const ground = new GameObject("Ground", { scale: Vector3.one.scale(6) });
    ground.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.32, 0.45, 0.25, 1), smoothness: 0.1 }) });

    // Trees (Default) with map markers; rocks and flowers on Detail.
    Random.initState(7);
    const bark = Material.create({ color: new Color(0.4, 0.28, 0.18, 1), smoothness: 0.2 });
    const leaves = Material.create({ color: new Color(0.18, 0.42, 0.2, 1), smoothness: 0.15 });
    const stone = Material.create({ color: new Color(0.55, 0.55, 0.52, 1), smoothness: 0.3 });
    const petals = [new Color(0.95, 0.85, 0.2, 1), new Color(0.9, 0.35, 0.6, 1), new Color(0.95, 0.95, 0.95, 1)].map((color) => Material.create({ color, smoothness: 0.2 }));
    for (let i = 0; i < 40; i++) {
      const a = Random.range(0, Math.PI * 2);
      const r = Random.range(10, 28);
      const at = new Vector3(Math.cos(a) * r, 0, Math.sin(a) * r);
      // Clear of the main camera.
      if (at.distance(new Vector3(0, 0, 17)) < 9) continue;
      const tree = new GameObject(`Tree ${i}`, { position: at });
      const trunk = shape("Trunk", "cylinder", new Vector3(0, 1, 0), new Vector3(0.35, 1, 0.35), bark, "Default", tree);
      const crown = shape("Crown", "sphere", new Vector3(0, 2.6, 0), Vector3.one.scale(2.2), leaves, "Default", tree);
      const block = shape("Crown (LOD1)", "cube", new Vector3(0, 2.6, 0), Vector3.one.scale(1.8), leaves, "Default", tree);
      const renderer = (go: GameObject) => go.getComponent(MeshRenderer)!;
      tree.addComponent(LODGroup, { size: 3.7 }).setLODs([
        { screenRelativeTransitionHeight: 0.25, renderers: [renderer(trunk), renderer(crown)] },
        { screenRelativeTransitionHeight: 0.06, renderers: [renderer(trunk), renderer(block)] },
      ]);
      marker(tree, new Color(0.2, 0.9, 0.3, 1), 2.6);
    }
    for (let i = 0; i < 160; i++) {
      const at = new Vector3(Random.range(-30, 30), 0, Random.range(-30, 30));
      if (Random.value < 0.35) {
        shape(`Rock ${i}`, "sphere", at.add(new Vector3(0, 0.15, 0)), new Vector3(0.6, 0.35, 0.5), stone, "Detail");
      } else {
        shape(`Flower ${i}`, "sphere", at.add(new Vector3(0, 0.25, 0)), Vector3.one.scale(0.3), petals[i % 3]!, "Detail");
      }
    }

    const walker = shape("Walker", "capsule", new Vector3(7, 0.9, 0), new Vector3(0.8, 0.9, 0.8), Material.create({ color: new Color(0.9, 0.45, 0.15, 1), smoothness: 0.5 }), "Player");
    walker.addComponent(Walker);
    marker(walker, new Color(1, 0.55, 0.1, 1), 4);

    const eye = new GameObject("Main Camera", { position: new Vector3(0, 4, 17), rotation: Quaternion.euler(-12, 0, 0) });
    main = eye.addComponent(Camera, { fieldOfView: 55, clearFlags: "skybox" });
    main.cullingMask = LayerMask.everything & ~LayerMask.getMask("Minimap");
    const far = main.layerCullDistances;
    far[LayerMask.nameToLayer("Detail")] = 14;
    main.layerCullDistances = far;

    const map = new GameObject("Minimap", { position: new Vector3(0, 30, 0), rotation: Quaternion.euler(-90, 0, 0) });
    map.addComponent(Camera, {
      orthographic: true,
      orthographicSize: 30,
      depth: 1,
      rect: new Rect(0.74, 0.03, 0.23, 0.31),
      backgroundColor: new Color(0.05, 0.07, 0.1, 1),
      cullingMask: LayerMask.getMask("Default", "Player", "Minimap"),
    });
    refreshMenu();
  }
}

register({ LayersDemo });

function Toggle(props: { layer: string }) {
  const [, setTick] = useState(0);
  const layer = LayerMask.nameToLayer(props.layer);
  const on = main ? LayerMask.contains(main.cullingMask, layer) : true;
  return (
    <box
      onClick={() => {
        if (!main) return;
        main.cullingMask ^= 1 << layer;
        setTick((t) => t + 1);
      }}
      style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", backgroundColor: on ? "#4d8dff" : "rgba(255,255,255,0.1)" }}
    >
      <text style={{ color: "#f0f3f8", fontSize: 13 }}>{`${props.layer}: ${on ? "on" : "off"}`}</text>
    </box>
  );
}

/** A row whose button steps through `values`. */
/** A labelled row: every row's controls start at the same x. */
function Row(props: { label: string; children: React.ReactNode }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 110, flexShrink: 0 }}>{props.label}</text>
      <box style={{ flexDirection: "row", alignItems: "center", gap: 6 }}>{props.children}</box>
    </box>
  );
}

function Cycle(props: { label: string; values: number[]; get: () => number; set: (v: number) => void; show: (v: number) => string }) {
  const [, setTick] = useState(0);
  const value = props.get();
  return (
    <Row label={props.label}>
      <box
        onClick={() => {
          const i = props.values.indexOf(value);
          props.set(props.values[(i + 1) % props.values.length]!);
          setTick((t) => t + 1);
        }}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", backgroundColor: "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.show(value)}</text>
      </box>
    </Row>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.78)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Main camera layers</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>Detail is culled past 14 m</text>
        <Row label="Layers">
          {["Default", "Detail", "Player", "Minimap"].map((name) => (
            <Toggle key={name} layer={name} />
          ))}
        </Row>
        <Cycle
          label="Field of view"
          values={[55, 30, 75, 100]}
          get={() => main?.fieldOfView ?? 55}
          set={(v) => main && (main.fieldOfView = v)}
          show={(v) => `${v}° (${Math.round(main?.horizontalFieldOfView ?? 0)}° across)`}
        />
        <Cycle
          label="LOD bias"
          values={[1, 0.4, 2]}
          get={() => QualitySettings.lodBias}
          set={(v) => (QualitySettings.lodBias = v)}
          show={(v) => `${v}x`}
        />
        <Cycle
          label="Render distance"
          values={[1000, 30, 20]}
          get={() => main?.farClipPlane ?? 1000}
          set={(v) => main && (main.farClipPlane = v)}
          show={(v) => `${v} m`}
        />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
