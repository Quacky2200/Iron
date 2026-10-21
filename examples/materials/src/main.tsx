// Materials: physically based shading under a procedural sky.
// - The classic grid: spheres from rough to smooth (left to right) and from plastic to
//   metal (front to back).
// - A mirror wall (a `Mirror` on a quad) reflecting the scene exactly.
// - Models in typical materials: a gold bunny and a chrome teapot (both metallic).
// - A reflection probe over it all, so shiny things reflect each other, not just the sky.
// - A front row of maps and modes: a normal-mapped tile, a cut-out leaf (double-sided,
//   with holes in its shadow), glass (transparent: the highlight stays) next to a faded
//   sphere, and a specular-workflow sphere.

class Spin extends Component {
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, 25 * dt, 0));
  }
}

class Showroom extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 45, clearFlags: "skybox" });
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 0.9, 0), distance: 8.5, pitch: 14, yaw: 20, autoRotate: 8 });
    RenderSettings.skybox = { sunDirection: new Vector3(0.5, 0.45, 0.4) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.5, -0.45, -0.4)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.2, color: new Color(1, 0.95, 0.88, 1), shadows: "soft" });

    const floor = new GameObject("Floor", { scale: Vector3.one.scale(1.4) });
    floor.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.18, 0.18, 0.2, 1), smoothness: 0.6 }) });

    // The grid: smoothness 0..1 across, metallic 0 / 0.5 / 1 front to back.
    const tint = new Color(0.95, 0.6, 0.25, 1);
    for (let x = 0; x < 6; x++) {
      for (let z = 0; z < 3; z++) {
        const ball = new GameObject(`Ball ${x},${z}`, { position: new Vector3(-2.5 + x, 0.4, 1.4 - z), scale: Vector3.one.scale(0.75) });
        ball.addComponent(MeshRenderer, { mesh: "sphere", material: Material.create({ color: tint, smoothness: x / 5, metallic: z / 2 }) });
      }
    }

    // The front row: maps and rendering modes.
    const row = (x: number, name: string, mesh: "cube" | "sphere" | "quad", material: Material, scale = 0.75) => {
      const go = new GameObject(name, { position: new Vector3(x, 0.45, 2.7), scale: Vector3.one.scale(scale) });
      go.addComponent(MeshRenderer, { mesh, material });
      return go;
    };
    row(-2.5, "Bumpy tile", "cube", Material.create({ color: new Color(0.8, 0.78, 0.74, 1), smoothness: 0.55, normalMap: bumps(), normalScale: 1.2 }));
    const leaf = row(-1.25, "Leaf", "quad", Material.create({ mainTexture: leafTexture(), renderingMode: "cutout", alphaCutoff: 0.5, doubleSided: true }), 0.9);
    leaf.transform.rotation = Quaternion.euler(0, 30, 0);
    const glass = new Color(0.75, 0.88, 1, 0.25);
    row(0, "Glass", "sphere", Material.create({ color: glass, smoothness: 0.97, renderingMode: "transparent" }));
    row(1.25, "Faded", "sphere", Material.create({ color: glass, smoothness: 0.97, renderingMode: "fade" }));
    row(2.5, "Specular", "sphere", Material.create({ color: new Color(0.05, 0.05, 0.07, 1), smoothness: 0.85, specularColor: new Color(0.25, 0.85, 0.45, 1) }));

    // A mirror wall behind the grid.
    const mirror = new GameObject("Mirror", { position: new Vector3(0, 1.4, -2.2), scale: new Vector3(6, 2.8, 1) });
    mirror.addComponent(MeshRenderer, { mesh: "quad", material: Material.create({ color: new Color(0.95, 0.97, 1, 1), metallic: 1, smoothness: 1 }) });
    mirror.addComponent(Mirror);
    const frame = new GameObject("Mirror frame", { position: new Vector3(0, 1.4, -2.25), scale: new Vector3(6.2, 3, 0.06) });
    frame.addComponent(MeshRenderer, { mesh: "cube", material: Material.create({ color: new Color(0.1, 0.08, 0.06, 1), smoothness: 0.3 }) });

    // A probe over the showroom, kept live (the models turn).
    const probe = new GameObject("Probe", { position: new Vector3(0, 1, 0) });
    probe.addComponent(ReflectionProbe, { size: new Vector3(14, 8, 10), resolution: 128, refreshMode: "everyFrame" });

    // Models: gold and chrome.
    const gold = Material.create({ color: new Color(1, 0.77, 0.34, 1), metallic: 1, smoothness: 0.85 });
    const chrome = Material.create({ color: new Color(0.95, 0.95, 0.95, 1), metallic: 1, smoothness: 0.95 });
    const placed = [
      { file: "assets/bunny.glb", at: new Vector3(-3.6, 0, -0.4), material: gold },
      { file: "assets/teapot.glb", at: new Vector3(3.6, 0, -0.4), material: chrome },
    ];
    for (const p of placed) {
      const model = Model.load(p.file);
      const go = model.instantiate({ position: p.at });
      go.addComponent(Spin);
      model.whenLoaded().then(() => {
        for (const r of collect(go)) r.material = p.material;
      });
    }
  }
}

/** A tangent-space normal map of round bumps, made in code (data: `linear`). */
function bumps(): Texture {
  const n = 64;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const cx = (((x + 0.5) / n) * 4) % 1 - 0.5;
      const cy = (((y + 0.5) / n) * 4) % 1 - 0.5;
      const inside = cx * cx + cy * cy < 0.16;
      const [nx, ny] = inside ? [cx * 2, cy * 2] : [0, 0];
      const nz = Math.sqrt(Math.max(0, 1 - nx * nx - ny * ny));
      px.push(new Color(nx * 0.5 + 0.5, ny * 0.5 + 0.5, nz * 0.5 + 0.5, 1));
    }
  }
  return Texture.create(n, n, px, { linear: true, filterMode: "trilinear", anisoLevel: 8 });
}

/** A round green leaf with holes, as an alpha mask. */
function leafTexture(): Texture {
  const n = 64;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const u = (x + 0.5) / n;
      const v = (y + 0.5) / n;
      const inside = (u - 0.5) ** 2 + (v - 0.5) ** 2 < 0.2;
      const hole = Math.hypot(((u * 5) % 1) - 0.5, ((v * 5) % 1) - 0.5) < 0.18;
      px.push(new Color(0.15, 0.55, 0.12, inside && !hole ? 1 : 0));
    }
  }
  return Texture.create(n, n, px);
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

register({ Showroom });

function Caption() {
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, justifyContent: "flex-end", pointerEvents: "none" }}>
      <box style={{ padding: "8px 14px", borderRadius: 8, backgroundColor: "rgba(10,12,18,0.55)", flexDirection: "column", gap: 2, alignSelf: "flex-start" }}>
        <text style={{ color: "#eef0f4", fontSize: 15, fontWeight: 600 }}>Materials</text>
        <text style={{ color: "#b5bcc8", fontSize: 12 }}>Smoothness 0 → 1 across · metallic 0 / 0.5 / 1 front to back · mirror wall</text>
        <text style={{ color: "#b5bcc8", fontSize: 12 }}>Front: normal map · cut-out leaf · glass (transparent) · faded · specular workflow</text>
      </box>
    </box>
  );
}

createRoot().render(<Caption />);
