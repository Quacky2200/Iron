// SPDX-License-Identifier: Apache-2.0

// Auto exposure: a road tunnel through a hill. The camera drives in from a bright summer
// day: the tunnel mouth looks black at first and only slowly opens up as the eyes adapt to
// the sodium lamps (`RenderSettings.autoExposure`, slower from light to dark). Deep inside,
// the exit ahead is a white blaze you can't see out of, glowing through the haze; driving
// out into the sun dazzles for a moment, then the hills, trees and sky settle into view.
// Then round again. The menu (or keys) switches auto exposure (A: else a fixed exposure set
// for daylight), the haze and its light shafts (F) and pauses the drive (Space).

// The road runs along -z. The tunnel goes from its entrance at z = 0 to its exit at
// z = -TUNNEL_LENGTH, HALF_W either side of the middle and HEIGHT tall.
const TUNNEL_LENGTH = 140;
const HALF_W = 5;
const HEIGHT = 6.5;
/** Where the drive starts and ends (it starts over from the start). */
const START_Z = 45;
const END_Z = -TUNNEL_LENGTH - 70;
const SPEED = 11;

/**
 * Where sunlight travels: from behind the car and to its left, 55 degrees up, so the road in
 * is sunlit and the exit opens onto a front-lit landscape.
 */
const SUN = new Vector3(0.375, -0.819, -0.435).normalized;

const SODIUM = new Color(1, 0.55, 0.16, 1);

const state = { auto: true, haze: true, paused: false };
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.autoExposure = state.auto
    ? // minEV: in the tunnel the eyes stop adapting before it looks like daylight, so it
      // stays dim between the lamps.
      // Slow to adapt to the daylight on the way out: a few seconds of glare.
      // Quick to adapt to the dark on the way in, so the exit soon blazes.
      { minEV: -4, maxEV: 16, compensation: -0.7, speedDarkToLight: 0.7, speedLightToDark: 1.3 }
    : null;
  // Without adapting, an exposure that suits the daylight: the tunnel is near black.
  RenderSettings.postExposure = state.auto ? 0 : -5;
  RenderSettings.fog = state.haze;
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
const mat = (hex: string, smoothness = 0.05) => Material.create({ color: Color.hex(hex), smoothness });

/**
 * Baked ambient occlusion for the tunnel's inside: the sky's light barely gets in, so its
 * surfaces take only `share` of the ambient light (and of reflections) that open ground does.
 */
const shut = (share: number) => Texture.create(1, 1, [new Color(share, share, share, 1)], { linear: true });

/** Black tarmac: dark binder with grey and the odd pale stone in it, tiling. */
function tarmac(): Texture {
  const N = 128;
  const px: Color[] = [];
  for (let i = 0; i < N * N; i++) {
    const r = Random.value;
    const v = r < 0.65 ? Random.range(0.008, 0.014) : r < 0.94 ? Random.range(0.018, 0.035) : Random.range(0.05, 0.09);
    px.push(new Color(v, v, v, 1));
  }
  return Texture.create(N, N, px, { filterMode: "bilinear" });
}

class Drive extends Component {
  private camera!: Camera;
  private z = START_Z;
  private t = 0;

  override onStart() {
    QualitySettings.shadowDistance = 90;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    // Little light from the sky, so the tunnel is truly dark next to the sunlit day.
    RenderSettings.ambientMode = "color";
    // Daylight is far brighter than a tunnel (the sun some 7 stops over the tunnel's air):
    // the same ambient lights both, so it's low next to the sun.
    // Open ground gets plenty from the sky; the tunnel's surfaces shut most of it out
    // (`shut`), so it's dark inside next to the sunlit day.
    RenderSettings.ambientLight = new Color(0.9, 1.0, 1.2, 1);
    RenderSettings.skybox = {
      zenith: new Color(7, 12, 26, 1),
      horizon: new Color(24, 26, 28, 1),
      ground: new Color(5, 4.4, 3.6, 1),
      sunDirection: SUN.scale(-1),
      sunColor: new Color(480, 450, 400, 1),
      sunSize: 1.2,
    };
    RenderSettings.tonemapping = "aces";
    this.setBloom(1);
    RenderSettings.fogMode = "exponential";
    // The haze's own colour is dark: it only glows where light reaches it (the sun's shafts
    // outside, the lamps inside), so the tunnel's air stays dark.
    RenderSettings.fogColor = new Color(0.04, 0.045, 0.05, 1);
    RenderSettings.fogDensity = 0.004;
    RenderSettings.volumetricFog = { intensity: 0.9, anisotropy: 0.35, steps: 4, distance: 80, albedo: new Color(0.9, 0.95, 1, 1) };

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.96, 0.88, 1), intensity: 40, shadows: "hard" });

    Random.initState(11);
    this.buildRoad();
    this.buildTunnel();
    this.buildHill();
    this.buildCountry();

    const eye = new GameObject("Main Camera", { position: new Vector3(-2.2, 1.35, START_Z) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 62, farClipPlane: 600, clearFlags: "skybox" });
    apply();
  }

  private bloomStep = -1;

  /**
   * Bloom, scaled by `k` (0..1): full from deep inside, where the exit is a glow far ahead;
   * faded near it, where a full glow would swell into an orb filling the view and plain
   * overexposure looks right.
   */
  private setBloom(k: number) {
    const step = Math.round(k * 100);
    if (step === this.bloomStep) return;
    this.bloomStep = step;
    const f = step / 100;
    RenderSettings.bloom = { threshold: 1.0, intensity: 0.15 + 1.15 * f, scatter: 0.6 + 0.25 * f };
  }

  /** Black tarmac from end to end, a dashed middle line and solid edge lines. */
  private buildRoad() {
    const surface = tarmac();
    // Three unbroken stretches (in, through, out), the texture every 2 m. The same tarmac
    // throughout: a slight sheen in the open, damp (shinier) and shut off from the sky
    // through the tunnel.
    const road = (z0: number, z1: number, occlusion?: Texture) => {
      const length = z0 - z1;
      const m = Material.create({ color: Color.white, mainTexture: surface, mainTextureScale: new Vector2(HALF_W, length / 2), smoothness: occlusion ? 0.5 : 0.25, occlusionMap: occlusion });
      slab("Road", -HALF_W, -0.1, z1, HALF_W, 0, z0, m);
    };
    road(START_Z + 30, 0);
    road(0, -TUNNEL_LENGTH, shut(0.25));
    road(-TUNNEL_LENGTH, END_Z - 20);
    const paint = Material.create({ color: Color.hex("#e8e4d8"), smoothness: 0.2 });
    for (let z = START_Z + 28; z > END_Z - 18; z -= 9) slab("Dash", -0.08, 0, z - 3, 0.08, 0.012, z, paint);
    for (const x of [-HALF_W + 0.4, HALF_W - 0.4]) slab("Edge line", x - 0.06, 0, END_Z - 20, x + 0.06, 0.012, START_Z + 30, paint);
  }

  /** Concrete walls and ceiling, a kerb each side and sodium lamps along the roof. */
  private buildTunnel() {
    const dark = shut(0.08);
    const concrete = Material.create({ color: Color.hex("#8a8780"), smoothness: 0.05, occlusionMap: dark });
    const tiles = Material.create({ color: Color.hex("#b9b6ac"), smoothness: 0.25, occlusionMap: dark });
    const ceiling = Material.create({ color: Color.hex("#4a4844"), smoothness: 0.05, occlusionMap: dark });
    const z0 = 0;
    const z1 = -TUNNEL_LENGTH;
    for (const s of [-1, 1]) {
      // Lower walls tiled (lighter), upper bare concrete.
      slab("Wall tiles", s * HALF_W, 0, z1, s * (HALF_W + 0.3), 2.6, z0, tiles);
      slab("Wall", s * HALF_W, 2.6, z1, s * (HALF_W + 0.3), HEIGHT, z0, concrete);
      slab("Kerb", s * (HALF_W - 0.6), 0, z1, s * HALF_W, 0.22, z0, concrete);
    }
    slab("Ceiling", -HALF_W - 0.3, HEIGHT, z1, HALF_W + 0.3, HEIGHT + 0.4, z0, ceiling);
    // Portals: a concrete frame at each end.
    const portal = mat("#9a968c");
    for (const z of [z0, z1]) {
      // Standing proud of the hillside's face (at 0.6 m), not flush with it.
      const d = z === z0 ? 0.9 : -0.9;
      slab("Portal", -HALF_W - 3, HEIGHT, z, HALF_W + 3, HEIGHT + 2.2, z + d, portal);
      for (const s of [-1, 1]) slab("Portal", s * HALF_W, 0, z, s * (HALF_W + 3), HEIGHT, z + d, portal);
    }
    // Lamps: every 14 m a pair of glowing housings over a pool of sodium light, dim
    // between them.
    const glow = Material.create({ color: Color.black, emission: new Color(SODIUM.r * 8, SODIUM.g * 8, SODIUM.b * 8, 1), unlit: true });
    for (let z = -7; z > z1 + 4; z -= 14) {
      for (const s of [-1, 1]) slab("Lamp", s * 2.4 - 0.5, HEIGHT - 0.12, z - 0.18, s * 2.4 + 0.5, HEIGHT, z + 0.18, glow);
      const lamp = new GameObject("Sodium light", { position: new Vector3(0, HEIGHT - 0.5, z) });
      lamp.addComponent(Light, { type: "point", color: SODIUM, intensity: 20, range: 14 });
    }
    // A reflection probe through the tunnel: the road reflects the walls and lamps, not
    // the sky outside. It reaches below the road, so the road (its middle just under 0) is
    // inside it.
    const probe = new GameObject("Tunnel probe", { position: new Vector3(0, HEIGHT / 2 - 0.5, -TUNNEL_LENGTH / 2) });
    probe.addComponent(ReflectionProbe, { size: new Vector3(HALF_W * 2 + 0.6, HEIGHT + 1, TUNNEL_LENGTH), resolution: 256, boxProjection: true });
  }

  /** The hill the tunnel goes through: rock either side and above, grassy humps on top. */
  private buildHill() {
    const rock = mat("#6d665c");
    const grass = mat("#4f6b33");
    const z0 = 0.6;
    const z1 = -TUNNEL_LENGTH - 0.6;
    slab("Rock", -90, -1, z1, -HALF_W - 0.3, 26, z0, rock);
    slab("Rock", HALF_W + 0.3, -1, z1, 90, 26, z0, rock);
    slab("Rock", -HALF_W - 0.3, HEIGHT + 0.4, z1, HALF_W + 0.3, 26, z0, rock);
    // The hill's top, with grassy humps. Each hump's bottom stays well above the tunnel's
    // roof (HEIGHT + 0.4), so none shows through it.
    slab("Turf", -90, 26, z1, 90, 26.5, z0, grass);
    for (const [x, z, r, h] of [[-40, -30, 45, 18], [30, -60, 50, 22], [-10, -100, 55, 16], [55, -120, 40, 14], [-60, -125, 40, 12]] as const) {
      shape("Hill", "sphere", new Vector3(x, 30, z), new Vector3(r * 2, h * 2, r * 2), grass);
    }
    // Boulders heaped beside each portal, clear of the road.
    for (const z of [z0, z1]) {
      const d = z === z0 ? 1 : -1;
      for (const s of [-1, 1]) {
        for (let i = 0; i < 4; i++) {
          const r = Random.range(3, 6);
          shape("Boulder", "sphere", new Vector3(s * (HALF_W + 5 + r + i * 5), r * 0.5, z + d * Random.range(1, 6)), new Vector3(r * 2, r * 1.6, r * 2), rock);
        }
      }
    }
  }

  /** Fields, hedges and trees at both ends. */
  private buildCountry() {
    const field = mat("#5d7d3a");
    const hedge = mat("#35501f");
    const bark = mat("#4a3628");
    const leaves = [mat("#3f6b2a"), mat("#4d7a2e"), mat("#365e24")];
    // Up to the hillside at each end (they run on under the rock), so no gap by the portals.
    slab("Field", -400, -0.3, 0, 400, -0.1, 400, field);
    slab("Field", -400, -0.3, -TUNNEL_LENGTH, 400, -0.1, -800, field);
    for (const [z0, z1] of [[0, START_Z + 30], [-TUNNEL_LENGTH, END_Z - 20]] as const) {
      for (const s of [-1, 1]) slab("Verge", s * HALF_W, -0.1, Math.min(z0, z1), s * 40, -0.02, Math.max(z0, z1), field);
    }
    for (const sign of [1, -1]) {
      for (let i = 0; i < 70; i++) {
        const x = (Random.value < 0.5 ? -1 : 1) * Random.range(HALF_W + 4, 120);
        const z = sign > 0 ? Random.range(8, 260) : -TUNNEL_LENGTH - Random.range(8, 420);
        const h = Random.range(4, 9);
        shape("Trunk", "cylinder", new Vector3(x, h * 0.3, z), new Vector3(0.5, h * 0.3, 0.5), bark);
        const r = Random.range(2.2, 4.2);
        shape("Crown", "sphere", new Vector3(x, h * 0.6 + r * 0.6, z), new Vector3(r * 2, r * 1.8, r * 2), leaves[i % 3]!);
      }
      for (const s of [-1, 1]) {
        const z0 = sign > 0 ? 6 : -TUNNEL_LENGTH - 6;
        const z1 = sign > 0 ? START_Z + 30 : END_Z - 20;
        slab("Hedge", s * (HALF_W + 2), 0, Math.min(z0, z1), s * (HALF_W + 3.2), 1.6, Math.max(z0, z1), hedge);
      }
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyA")) toggle("auto");
    if (input.getKeyDown("KeyF")) toggle("haze");
    if (input.getKeyDown("Space")) toggle("paused");
    if (!state.paused) {
      this.t += dt;
      this.z -= SPEED * dt;
      if (this.z < END_Z) this.z = START_Z;
    }
    // Full bloom until 70 m from the exit, fading to a trace 12 m from it and outside.
    const toExit = this.z + TUNNEL_LENGTH;
    const k = Math.min(1, Math.max(0, (toExit - 12) / 58));
    this.setBloom(k * k * (3 - 2 * k));
    // A gentle sway in the lane and a slight bob, looking down the road.
    const cam = this.camera.transform;
    cam.position = new Vector3(-2.2 + Math.sin(this.t * 0.31) * 0.25, 1.35 + Math.sin(this.t * 2.1) * 0.015, this.z);
    cam.rotation = Quaternion.lookRotation(new Vector3(0.6 + Math.sin(this.t * 0.23) * 0.4, -0.25, -30));
  }
}

function toggle(key: "auto" | "haze" | "paused") {
  if (key === "auto") state.auto = !state.auto;
  else if (key === "haze") state.haze = !state.haze;
  else state.paused = !state.paused;
  apply();
}

register({ Drive });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#c8ccd2", fontSize: 13, width: 104, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on ? "#3a7bd5" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#f4f6f8", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#7d838c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 18, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Auto exposure: the tunnel</text>
        <Button label="Auto exposure" value={state.auto ? "On (eyes adapt)" : "Off (fixed)"} on={state.auto} keyName="A" onClick={() => toggle("auto")} />
        <Button label="Haze" value={state.haze ? "On" : "Off"} on={state.haze} keyName="F" onClick={() => toggle("haze")} />
        <Button label="Drive" value={state.paused ? "Paused" : "Driving"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
