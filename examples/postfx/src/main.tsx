// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

// Post-processing: a sunlit colonnade seen down its avenue, with every built-in `PostProcess`
// effect switchable at run time (the menu, or the keys 1 to 9, 0, - and =), and one effect
// written in WGSL by the game itself (the ripple). Tab moves the depth of field's focus between
// the three balls; A turns everything off; T tours the effects one at a time.
//
// `platypus-app examples/postfx -- outline vignette` starts with those effects on (for screenshots).

type EffectName = PostEffect | "ripple";

interface Tweak {
  /** The parameter the - and + buttons change. */
  param: string;
  min: number;
  max: number;
  step: number;
}

interface Fx {
  key: string;
  code: string;
  name: EffectName;
  label: string;
  /** Where it runs among the passes (lower first): lens effects, then grading, then screen looks. */
  order: number;
  setup: (p: PostProcess) => void;
  tweak: Tweak;
}

/** A pass of the game's own: a shimmering wobble and a tint, written as a WGSL function. */
const RIPPLE = `
fn effect(uv: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let v = view_uv(uv);
    let wave = sin(v.y * 60.0 + post_time() * 4.0) * strength * 0.004;
    let wave2 = sin(v.x * 45.0 + post_time() * 3.0) * strength * 0.003;
    let c = scene(uv + vec2<f32>(wave, wave2) * fx.rect.zw);
    return vec4<f32>(mix(c.rgb, c.rgb * tint.rgb, clamp(strength, 0.0, 1.0)), c.a);
}`;

const EFFECTS: Fx[] = [
  { key: "1", code: "Digit1", name: "depthOfField", label: "Depth of field", order: 0, tweak: { param: "aperture", min: 0.5, max: 8, step: 0.5 },
    setup: (p) => { p.setFloat("focal_length", 85); p.setFloat("aperture", 1.6); p.setFloat("max_blur", 18); p.setFloat("blades", 6); } },
  { key: "2", code: "Digit2", name: "tiltShift", label: "Tilt-shift", order: 0, tweak: { param: "max_blur", min: 2, max: 30, step: 2 },
    setup: (p) => { p.setFloat("focus", 0.55); p.setFloat("width", 0.1); p.setFloat("max_blur", 16); } },
  { key: "3", code: "Digit3", name: "outline", label: "Ink outlines", order: 10, tweak: { param: "thickness", min: 0.5, max: 4, step: 0.5 },
    setup: (p) => { p.setFloat("thickness", 1.5); p.setFloat("fade_distance", 60); p.setColor("ink", new Color(0.02, 0.012, 0.008, 1)); } },
  { key: "4", code: "Digit4", name: "posterize", label: "Cel bands", order: 11, tweak: { param: "levels", min: 2, max: 8, step: 1 },
    setup: (p) => { p.setFloat("levels", 4); } },
  { key: "5", code: "Digit5", name: "colorGrading", label: "Colour grading", order: 20, tweak: { param: "saturation", min: 0, max: 2, step: 0.25 },
    setup: (p) => { p.setFloat("saturation", 1.25); p.setFloat("contrast", 1.12); p.setVector("balance", 0.25, 0.05); p.setVector("lift", 0.02, 0.0, 0.05); p.setVector("gain", 1.05, 1.0, 0.92); } },
  { key: "6", code: "Digit6", name: "chromaticAberration", label: "Chromatic aberration", order: 30, tweak: { param: "amount", min: 0, max: 0.06, step: 0.005 },
    setup: (p) => { p.setFloat("amount", 0.015); } },
  { key: "7", code: "Digit7", name: "vignette", label: "Vignette", order: 40, tweak: { param: "intensity", min: 0, max: 1, step: 0.1 },
    setup: (p) => { p.setFloat("intensity", 0.7); } },
  { key: "8", code: "Digit8", name: "filmGrain", label: "Film grain", order: 50, tweak: { param: "intensity", min: 0, max: 0.6, step: 0.05 },
    setup: (p) => { p.setFloat("intensity", 0.18); } },
  { key: "9", code: "Digit9", name: "blur", label: "Blur", order: 60, tweak: { param: "radius", min: 0, max: 24, step: 2 },
    setup: (p) => { p.setFloat("radius", 8); } },
  { key: "0", code: "Digit0", name: "pixelate", label: "Pixelate", order: 70, tweak: { param: "size", min: 2, max: 16, step: 2 },
    setup: (p) => { p.setFloat("size", 5); p.setFloat("levels", 6); } },
  { key: "-", code: "Minus", name: "crt", label: "CRT", order: 80, tweak: { param: "curvature", min: 0, max: 1, step: 0.1 },
    setup: (p) => { p.setFloat("curvature", 0.35); } },
  { key: "=", code: "Equal", name: "ripple", label: "Ripple (own WGSL)", order: 90, tweak: { param: "strength", min: 0, max: 3, step: 0.25 },
    setup: (p) => { p.setFloat("strength", 0.4); p.setColor("tint", new Color(0.8, 0.95, 1.1, 1)); } },
];

/** The passes by effect, made when first switched on and kept (so their settings stay). */
const passes = new Map<EffectName, PostProcess>();
const values = new Map<EffectName, number>();
let refreshMenu = () => {};
let tour = false;
let focusTarget = 1;

function pass(fx: Fx): PostProcess {
  let p = passes.get(fx.name);
  if (!p) {
    p = fx.name === "ripple" ? PostProcess.createCustom(RIPPLE, ["strength", "tint:color"]) : PostProcess.create(fx.name);
    p.order = fx.order;
    fx.setup(p);
    p.enabled = false;
    passes.set(fx.name, p);
    values.set(fx.name, p.getFloat(fx.tweak.param));
  }
  return p;
}

function isOn(fx: Fx): boolean {
  return passes.get(fx.name)?.enabled === true;
}

function toggle(fx: Fx): void {
  const p = pass(fx);
  p.enabled = !p.enabled;
  refreshMenu();
}

function change(fx: Fx, by: number): void {
  const p = pass(fx);
  const v = Math.min(fx.tweak.max, Math.max(fx.tweak.min, p.getFloat(fx.tweak.param) + by * fx.tweak.step));
  p.setFloat(fx.tweak.param, v);
  values.set(fx.name, v);
  refreshMenu();
}

function allOff(): void {
  for (const p of passes.values()) p.enabled = false;
  tour = false;
  refreshMenu();
}

/** Only the effect at `index` on (the tour). */
function solo(index: number): void {
  EFFECTS.forEach((fx, i) => (pass(fx).enabled = i === index));
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder" | "plane" | "capsule", at: Vector3, scale: Vector3, material: Material, parent?: GameObject) {
  const go = new GameObject(name, { position: at, scale, parent });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}

/** Sandstone flags: a 2 x 2 checker of two warm greys, so the floor has edges to blur and ink. */
function flags(): Texture {
  const n = 64;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const dark = (x < n / 2) !== (y < n / 2);
      const edge = x % (n / 2) < 2 || y % (n / 2) < 2;
      px.push(edge ? new Color(0.3, 0.27, 0.23, 1) : dark ? new Color(0.78, 0.7, 0.58, 1) : new Color(0.88, 0.8, 0.68, 1));
    }
  }
  return Texture.create(n, n, px, { filterMode: "trilinear", anisoLevel: 16 });
}

class Colonnade extends Component {
  private camera!: Camera;
  private balls: GameObject[] = [];
  private focus = 11;
  private t = 0;
  private toured = 0;

  override onStart() {
    QualitySettings.shadowDistance = 60;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.34, 0.36, 0.42, 1);
    RenderSettings.skybox = {};
    RenderSettings.tonemapping = "aces";
    RenderSettings.postExposure = 0.2;

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.45, -0.55, -0.7).normalized) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.9, 0.74, 1), intensity: 1.6, shadows: "soft" });

    const floor = new GameObject("Floor", { position: new Vector3(0, 0, -30), scale: new Vector3(2, 1, 12) });
    floor.addComponent(MeshRenderer, {
      mesh: "plane",
      material: Material.create({ color: Color.white, mainTexture: flags(), mainTextureScale: new Vector2(10, 60), smoothness: 0.25 }),
    });

    // Two rows of fluted columns with a slab across the top, the avenue between them.
    const stone = Material.create({ color: new Color(0.86, 0.8, 0.7, 1), smoothness: 0.2 });
    const dark = Material.create({ color: new Color(0.45, 0.4, 0.34, 1), smoothness: 0.15 });
    for (let i = 0; i < 9; i++) {
      const z = 3 - i * 5.5;
      for (const x of [-4.2, 4.2]) {
        shape("Base", "cube", new Vector3(x, 0.2, z), new Vector3(1.3, 0.4, 1.3), dark);
        shape("Column", "cylinder", new Vector3(x, 2.6, z), new Vector3(0.55, 2.2, 0.55), stone);
        shape("Capital", "cube", new Vector3(x, 4.95, z), new Vector3(1.2, 0.3, 1.2), dark);
      }
      shape("Lintel", "cube", new Vector3(0, 5.35, z), new Vector3(9.8, 0.4, 0.9), stone);
    }
    // Thin rails between the near columns: fine detail for outlines and blur to work on.
    const iron = Material.create({ color: new Color(0.08, 0.08, 0.09, 1), metallic: 0.8, smoothness: 0.5 });
    for (const x of [-3.2, 3.2]) {
      shape("Rail", "cube", new Vector3(x, 1.0, -8), new Vector3(0.06, 0.06, 16), iron);
      for (let k = 0; k < 14; k++) shape("Baluster", "cylinder", new Vector3(x, 0.5, 2 - k * 1.2), new Vector3(0.05, 0.5, 0.05), iron);
    }

    // Three balls to focus on, near, middle and far.
    const colors = [new Color(0.95, 0.2, 0.12, 1), new Color(0.15, 0.7, 0.3, 1), new Color(0.2, 0.4, 0.95, 1)];
    const at = [new Vector3(-1.0, 0.7, 4.0), new Vector3(0.6, 0.7, -2.5), new Vector3(-0.8, 0.7, -12.0)];
    for (let i = 0; i < 3; i++) {
      this.balls.push(shape(`Ball ${i}`, "sphere", at[i]!, Vector3.one.scale(1.4), Material.create({ color: colors[i]!, metallic: 0.2, smoothness: 0.85 })));
    }
    // Bright lamps far down the avenue: out of focus they become bokeh discs.
    const glow = Material.create({ color: Color.black, emission: new Color(6, 4.6, 2.8, 1), unlit: true });
    for (let i = 0; i < 6; i++) {
      for (const x of [-2.4, 2.4]) {
        const lamp = shape("Lamp", "sphere", new Vector3(x, 2.3 + (i % 2) * 0.6, -16 - i * 4), Vector3.one.scale(0.22), glow);
        lamp.getComponent(MeshRenderer)!.shadowCastingMode = "off";
      }
    }

    const eye = new GameObject("Main Camera", { position: new Vector3(0, 1.7, 9) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 50, farClipPlane: 200, clearFlags: "skybox" });

    // Every pass made (off), so the menu shows their values; those named on the command line start on.
    for (const fx of EFFECTS) pass(fx).enabled = Application.args.indexOf(fx.name) >= 0;
    refreshMenu();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    for (const fx of EFFECTS) if (input.getKeyDown(fx.code)) toggle(fx);
    if (input.getKeyDown("KeyA")) allOff();
    if (input.getKeyDown("KeyT")) {
      tour = !tour;
      this.toured = 0;
      if (!tour) allOff();
    }
    if (input.getKeyDown("Tab")) {
      focusTarget = (focusTarget + 1) % 3;
      refreshMenu();
    }
    if (tour) {
      const i = Math.floor(this.toured / 3) % EFFECTS.length;
      if (this.toured === 0 || Math.floor((this.toured - dt) / 3) !== Math.floor(this.toured / 3)) solo(i);
      this.toured += dt;
    }
    // A slow sway down the avenue.
    const cam = this.camera.transform;
    cam.position = new Vector3(Math.sin(this.t * 0.2) * 1.2, 1.7 + Math.sin(this.t * 0.13) * 0.15, 9);
    cam.rotation = Quaternion.lookRotation(new Vector3(0, 1.6, -20).sub(cam.position));
    // The lens pulls focus to the chosen ball, easing as a real one does.
    const target = this.balls[focusTarget]!.transform.position.sub(cam.position).magnitude;
    this.focus += (target - this.focus) * (1 - Math.exp(-dt * 4));
    passes.get("depthOfField")?.setFloat("focus_distance", this.focus);
  }
}

register({ Colonnade });

function Row(props: { fx: Fx }) {
  const { fx } = props;
  const on = isOn(fx);
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
      <text style={{ color: "#6f7a8c", fontSize: 12, width: 14 }}>{fx.key}</text>
      <box
        onClick={() => toggle(fx)}
        style={{ padding: "5px 10px", borderRadius: 6, cursor: "pointer", width: 176, backgroundColor: on ? "#4d8dff" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>{fx.label}</text>
      </box>
      <box onClick={() => change(fx, -1)} style={{ padding: "3px 8px", borderRadius: 5, cursor: "pointer", backgroundColor: "rgba(255,255,255,0.1)" }}>
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>-</text>
      </box>
      <text style={{ color: "#a9b3c3", fontSize: 12, width: 120 }}>{`${fx.tweak.param} ${(values.get(fx.name) ?? 0).toFixed(2)}`}</text>
      <box onClick={() => change(fx, 1)} style={{ padding: "3px 8px", borderRadius: 5, cursor: "pointer", backgroundColor: "rgba(255,255,255,0.1)" }}>
        <text style={{ color: "#f0f3f8", fontSize: 13 }}>+</text>
      </box>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  for (const fx of EFFECTS) if (!passes.has(fx.name)) values.set(fx.name, 0);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 6, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Post-processing</text>
        {EFFECTS.map((fx) => (
          <Row key={fx.name} fx={fx} />
        ))}
        <box style={{ flexDirection: "row", gap: 14, marginTop: 4 }}>
          <text style={{ color: "#a9b3c3", fontSize: 12 }}>{`Tab: focus on ball ${focusTarget + 1}`}</text>
          <text style={{ color: "#a9b3c3", fontSize: 12 }}>A: all off</text>
          <text style={{ color: "#a9b3c3", fontSize: 12 }}>{tour ? "T: stop the tour" : "T: tour"}</text>
        </box>
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
