// SPDX-License-Identifier: Apache-2.0

// God rays: a high-rise flat at sunset. Floor-to-ceiling windows on the 25th floor look west
// over a city; venetian blinds cover part of them. Over a minute the sun sinks from the
// afternoon to the horizon: its beams through the mullions and the blinds' slats swing round
// and stretch across the room, warming from gold to deep orange, while the haze over the
// city glows and the towers' windows light up. After dusk a lamp comes on and the city
// sparkles, then the afternoon comes round again. Auto exposure follows it all. The menu (or
// keys) switches the day passing (T: else held at golden hour), the blinds (B), the haze and
// its shafts (F) and auto exposure (A).

// The flat: its floor FLOOR metres up, x from -W to W, z from -D (the window wall, facing
// west, -z) to D, H tall.
const FLOOR = 80;
const W = 5;
const D = 4;
const H = 3;
/** Mullions along the window wall, every this many metres. */
const BAY = 1.25;
/** The blinds cover the window wall for x below this. */
const BLINDS_TO = -0.6;

/** Seconds from the afternoon round to the afternoon again. */
const DAY_LENGTH = 70;
/** Steps the sky takes over the cycle (each change is an op): fine enough not to show. */
const SKY_STEPS = 700;
/** How many levels the city's lights come on in. */
const LIGHT_LEVELS = 6;
/** Tower sizes (width, height) in floors and bays: each has its own tiled materials. */
const TOWER_KINDS = [
  [4, 7], [4, 10], [5, 13], [4, 18], [6, 22], [5, 32], [7, 42],
] as const;
/** A floor's height and a window bay's width, metres. */
const STOREY = 4.2;
const BAY_W = 6;
/** Storeys and bays in the facade's texture block (it repeats every this many). */
const BLOCK = 16;
/** Where in the day golden hour is held when the day isn't passing. */
const GOLDEN = 0.55;

/**
 * The sun `u` of the way (0..1) through the cycle: afternoon (0) to sunset (0.72), dusk and
 * night, then back. Where its light travels, its colour, strength, and how lit the city is.
 */
function sky(u: number) {
  const sunset = 0.72;
  const s = Math.min(u / sunset, 1);
  // From 38 degrees up down to 2 below the horizon, swinging a little north.
  const elevation = ((38 - 40 * s) * Math.PI) / 180;
  const azimuth = ((-14 + 20 * s) * Math.PI) / 180;
  const toSun = new Vector3(Math.cos(elevation) * Math.sin(azimuth), Math.sin(elevation), -Math.cos(elevation) * Math.cos(azimuth));
  const warm = s ** 1.6;
  const color = new Color(1, 0.9 - 0.5 * warm, 0.76 - 0.6 * warm, 1);
  const smooth = (a: number, b: number, x: number) => {
    const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
    return t * t * (3 - 2 * t);
  };
  // Fades in after night, out as it sets.
  const strength = smooth(0, 0.06, u) * (1 - smooth(0.62, 0.71, u));
  // The city's lights: on through dusk and night.
  const night = smooth(0.6, 0.78, u) * (1 - smooth(0.94, 1, u));
  return { dir: toSun.scale(-1).normalized, toSun, color, strength, night, warm };
}

const state = { day: true, blinds: true, haze: true, auto: true };
let refreshMenu = () => {};
let blindsRoot: GameObject | null = null;

function apply(): void {
  RenderSettings.fog = state.haze;
  RenderSettings.autoExposure = state.auto ? { minEV: -3, maxEV: 16, compensation: -0.8, speedDarkToLight: 1, speedLightToDark: 0.6 } : null;
  RenderSettings.postExposure = state.auto ? 0 : -3.5;
  if (blindsRoot) blindsRoot.enabled = state.blinds;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  const r = go.addComponent(MeshRenderer, { mesh, material });
  return { go, r };
}
/** A box between two corners (y measured from the flat's floor). */
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material, parent?: GameObject) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, FLOOR + (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material, parent);
}
const mat = (hex: string, smoothness = 0.1) => Material.create({ color: Color.hex(hex), smoothness });

class Flat extends Component {
  private camera!: Camera;
  private sun!: Light;
  private lamp!: Light;
  private t = 0;
  private skyStep = -1;
  private lightLevel = -1;
  /** The towers: their renderers and kinds, and materials by kind and light level. */
  private towers: { r: MeshRenderer; kind: number; shift: number }[] = [];
  private towerMats: Material[][] = [];
  private lampShade!: MeshRenderer;
  private shadeMats: Material[] = [];

  override onStart() {
    QualitySettings.shadowDistance = 30;
    QualitySettings.shadowResolution = 4096;
    QualitySettings.shadowCascades = 4;
    RenderSettings.ambientMode = "color";
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1, intensity: 0.5, scatter: 0.65 };
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogDensity = 0.0009;
    RenderSettings.volumetricFog = { intensity: 1.1, anisotropy: 0.55, steps: 4, distance: 40 };

    const sunGo = new GameObject("Sun");
    this.sun = sunGo.addComponent(Light, { type: "directional", intensity: 10, shadows: "hard" });

    Random.initState(21);
    this.buildFlat();
    this.buildBlinds();
    this.buildFurniture();
    this.buildCity();

    const eye = new GameObject("Main Camera", { position: new Vector3(3.6, FLOOR + 1.5, 2.6) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 66, nearClipPlane: 0.1, farClipPlane: 2500, clearFlags: "skybox" });
    this.update(0);
    apply();
  }

  private buildFlat() {
    const wall = mat("#d9d3c8");
    const floor = mat("#8a6a4a", 0.45);
    const ceiling = mat("#eeeae2");
    const steel = mat("#2a2c30", 0.5);
    slab("Floor", -W - 0.3, -0.3, -D - 0.3, W + 0.3, 0, D + 0.3, floor);
    slab("Ceiling", -W - 0.3, H, -D - 0.3, W + 0.3, H + 0.3, D + 0.3, ceiling);
    slab("Wall", -W - 0.3, -0.3, D, W + 0.3, H + 0.3, D + 0.3, wall);
    slab("Wall", -W - 0.3, -0.3, -D - 0.3, -W, H + 0.3, D, wall);
    slab("Wall", W, -0.3, -D - 0.3, W + 0.3, H + 0.3, D, wall);
    // The window wall: steel mullions every BAY, a transom, a low sill and a head.
    slab("Sill", -W, -0.3, -D - 0.15, W, 0.35, -D, wall);
    slab("Head", -W, H - 0.25, -D - 0.15, W, H + 0.3, -D, wall);
    for (let x = -W; x <= W + 1e-3; x += BAY) slab("Mullion", x - 0.05, 0.35, -D - 0.12, x + 0.05, H - 0.25, -D - 0.02, steel);
    slab("Transom", -W, 2.1, -D - 0.12, W, 2.16, -D - 0.02, steel);
    // The building's face below and above the flat, so the city isn't seen through the floor.
    const facade = mat("#5a5e66", 0.3);
    slab("Facade", -W - 0.3, -FLOOR, -D - 0.6, W + 0.3, -0.3, -D - 0.15, facade);
    slab("Facade", -W - 0.3, H + 0.3, -D - 0.6, W + 0.3, H + 30, -D - 0.15, facade);
  }

  /** Venetian blinds over part of the window wall: slats tilted to let the sun through in stripes. */
  private buildBlinds() {
    blindsRoot = new GameObject("Blinds");
    const slat = mat("#f0ece4", 0.3);
    for (let y = 0.45; y < H - 0.3; y += 0.07) {
      shape("Slat", "cube", new Vector3((-W + BLINDS_TO) / 2, FLOOR + y, -D + 0.1), new Vector3(BLINDS_TO + W - 0.1, 0.006, 0.05), slat, blindsRoot, Quaternion.euler(35, 0, 0));
    }
  }

  private buildFurniture() {
    const fabric = mat("#4a5560");
    const oak = mat("#7a5a3a", 0.35);
    const black = mat("#1d1e20", 0.4);
    const rug = mat("#c9b9a0", 0.02);
    slab("Rug", -2.5, 0, -2.4, 1.5, 0.012, 1.0, rug);
    slab("Sofa seat", -2.2, 0.2, 0.6, 1.0, 0.45, 1.5, fabric);
    slab("Sofa back", -2.2, 0.2, 1.4, 1.0, 0.95, 1.7, fabric);
    for (const x of [-2.25, 0.95]) slab("Sofa arm", x, 0.2, 0.6, x + 0.3, 0.65, 1.7, fabric);
    slab("Coffee table", -1.4, 0.38, -1.0, 0.4, 0.42, -0.2, oak);
    for (const [x, z] of [[-1.35, -0.95], [0.35, -0.95], [-1.35, -0.25], [0.35, -0.25]] as const) slab("Table leg", x - 0.02, 0, z - 0.02, x + 0.02, 0.38, z + 0.02, black);
    shape("Pot", "cylinder", new Vector3(-4.3, FLOOR + 0.25, -3.2), new Vector3(0.45, 0.25, 0.45), black);
    const leaf = mat("#3e6a2e");
    for (let i = 0; i < 9; i++) {
      const a = (i / 9) * Math.PI * 2;
      shape("Leaf", "sphere", new Vector3(-4.3 + Math.cos(a) * 0.3, FLOOR + 0.9 + Random.range(0, 0.6), -3.2 + Math.sin(a) * 0.3), new Vector3(0.35, 0.6, 0.35), leaf);
    }
    slab("Shelves", W - 0.4, 0, -1.5, W, 2.0, 1.5, oak);
    // A floor lamp that comes on at dusk.
    shape("Lamp stand", "cylinder", new Vector3(1.6, FLOOR + 0.8, 1.6), new Vector3(0.04, 0.8, 0.04), black);
    this.lampShade = shape("Lamp shade", "cylinder", new Vector3(1.6, FLOOR + 1.65, 1.6), new Vector3(0.45, 0.2, 0.45), mat("#efe6d2")).r;
    this.shadeMats = [0, 0.25, 0.5, 0.75, 1].map((k) =>
      k === 0 ? mat("#efe6d2") : Material.create({ color: Color.hex("#efe6d2"), emission: new Color(3 * k, 2.2 * k, 1.2 * k, 1) }),
    );
    const bulb = new GameObject("Lamp", { position: new Vector3(1.6, FLOOR + 1.55, 1.6) });
    this.lamp = bulb.addComponent(Light, { type: "point", color: new Color(1, 0.72, 0.45, 1), intensity: 0, range: 7 });
  }

  /**
   * Towers across the city to the west, mostly well below the flat (it looks out over them)
   * with a taller skyline further off. Their windows are a texture (a grid of glass on the
   * facade) with an emission map of the ones lit at night: one window at a time comes on,
   * not whole faces, and nothing sits on top of the walls to flicker.
   */
  private buildCity() {
    // Both maps cover a block of BLOCK x BLOCK storeys and bays, 16 texels each: the facade
    // (a wide pane of glass in concrete per bay and storey) and which panes are lit. Emission
    // maps share the main texture's tiling, so the two line up.
    const N = 16;
    const M = BLOCK * N;
    const isGlass = (px: number, py: number) => px > 1 && px < 14 && py > 4 && py < 14;
    const facade: Color[] = [];
    for (let y = 0; y < M; y++) {
      for (let x = 0; x < M; x++) {
        // Dark glass in grey concrete: a city is darker than it looks.
        facade.push(isGlass(x % N, y % N) ? new Color(0.025, 0.032, 0.045, 1) : new Color(0.2, 0.2, 0.21, 1));
      }
    }
    const pane = Texture.create(M, M, facade, { filterMode: "bilinear" });
    // Flats are two bays wide. About a fifth are lit, in clusters: some floors are busy and
    // some asleep, and a lit flat makes its neighbour likelier. Most are warm, a few cool
    // (LED, a television), some dimmer behind curtains.
    const lights: (Color | null)[] = [];
    for (let i = 0; i < BLOCK * BLOCK; i++) lights.push(null);
    for (let floor = 0; floor < BLOCK; floor++) {
      const busy = Random.value;
      let last = false;
      for (let flat = 0; flat < BLOCK / 2; flat++) {
        const on: boolean = Random.value < 0.06 + 0.22 * busy + (last ? 0.2 : 0);
        last = on;
        if (!on) continue;
        const b = Random.range(0.55, 1);
        const c = Random.value < 0.2 ? new Color(0.8 * b, 0.88 * b, b, 1) : new Color(b, 0.85 * b, 0.62 * b, 1);
        // Mostly both windows, sometimes just one room.
        const both = Random.value < 0.7;
        const one = Random.value < 0.5 ? 0 : 1;
        for (const k of [0, 1]) {
          if (both || k === one) lights[floor * BLOCK + flat * 2 + k] = c;
        }
      }
    }
    const dark = new Color(0, 0, 0, 1);
    const litPx: Color[] = [];
    for (let y = 0; y < M; y++) {
      for (let x = 0; x < M; x++) {
        const c = lights[Math.floor(y / N) * BLOCK + Math.floor(x / N)];
        litPx.push(c && isGlass(x % N, y % N) ? c : dark);
      }
    }
    const lit = Texture.create(M, M, litPx, { filterMode: "bilinear" });
    const tints = ["#d8d2c8", "#bcc2cc", "#e0d4bc", "#aab2be"];
    this.towerMats = TOWER_KINDS.map(([bays, floors], k) =>
      Array.from({ length: LIGHT_LEVELS }, (_, level) => {
        const e = level / (LIGHT_LEVELS - 1);
        return Material.create({
          color: Color.hex(tints[k % tints.length]!),
          mainTexture: pane,
          mainTextureScale: new Vector2(bays / BLOCK, floors / BLOCK),
          emissionMap: lit,
          emission: new Color(2.6 * e, 2.1 * e, 1.6 * e, 1),
          smoothness: 0.4,
        });
      }),
    );
    slab("Ground", -3000, -FLOOR - 1, -3000, 3000, -FLOOR, 300, mat("#2e3136"));
    for (let i = 0; i < 300; i++) {
      // Mostly ahead (west), a few to the sides; near ones low, far ones taller.
      const d = Random.range(160, 1600);
      const a = Random.range(-1.1, 1.1);
      const x = Math.sin(a) * d;
      const z = -D - 20 - Math.cos(a) * d;
      const near = d < 600;
      const pool = near ? [0, 1, 2, 3] : [2, 3, 4, 5, 6];
      const kind = pool[Math.floor(Random.value * pool.length)]!;
      const [bays, floors] = TOWER_KINDS[kind]!;
      const wdt = bays * BAY_W;
      const h = floors * STOREY;
      const tower = shape("Tower", "cube", new Vector3(x, h / 2, z), new Vector3(wdt, h, wdt), this.towerMats[kind]![0]!);
      this.towers.push({ r: tower.r, kind, shift: i % 3 });
    }
  }

  /** Moves the sun, sky, lamp and city lights to time `u` of the cycle. */
  private update(u: number) {
    const s = sky(u);
    this.sun.transform.rotation = Quaternion.lookRotation(s.dir);
    this.sun.color = s.color;
    this.sun.intensity = 10 * s.strength;
    // Light from the sky inside: bright in the afternoon, dim at night.
    const day = 0.08 + 0.5 * s.strength;
    RenderSettings.ambientLight = new Color(day * 0.9, day * 0.85, day * (0.9 - 0.3 * s.warm), 1);
    this.lamp.intensity = 5 * s.night;
    const skyStep = Math.round(u * SKY_STEPS);
    if (skyStep !== this.skyStep) {
      this.skyStep = skyStep;
      const glow = s.strength;
      RenderSettings.skybox = {
        zenith: new Color(1.2 * glow + 0.02, 1.8 * glow + 0.03, 3.6 * glow + 0.08, 1),
        horizon: new Color(5 * glow * (1 + s.warm) + 0.15 * s.night, 4 * glow * (1 - 0.3 * s.warm) + 0.08 * s.night, 3 * glow * (1 - 0.7 * s.warm) + 0.12 * s.night, 1),
        ground: new Color(0.3 * glow + 0.02, 0.28 * glow + 0.02, 0.26 * glow + 0.03, 1),
        sunDirection: s.toSun,
        sunColor: new Color(80 * s.color.r * s.strength, 80 * s.color.g * s.strength, 80 * s.color.b * s.strength, 1),
        sunSize: 1.4,
      };
      RenderSettings.fogColor = new Color(0.04 + 0.1 * glow, 0.04 + 0.08 * glow, 0.05 + 0.08 * glow, 1);
      // Looking west into the sun, forward scattering lights the haze strongly: kept low, so
      // the city isn't washed out (the beams in the room still show).
      RenderSettings.volumetricFog = { intensity: 0.35, anisotropy: 0.4, steps: 4, distance: 40, albedo: new Color(1, 0.92, 0.85, 1) };
      this.lampShade.material = this.shadeMats[Math.round(s.night * (this.shadeMats.length - 1))]!;
    }
    const level = Math.round(s.night * (LIGHT_LEVELS - 1 + 2));
    if (level !== this.lightLevel) {
      this.lightLevel = level;
      // Not every tower lights up at once.
      for (const t of this.towers) t.r.material = this.towerMats[t.kind]![Math.min(LIGHT_LEVELS - 1, Math.max(0, level - t.shift))]!;
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyT")) toggle("day");
    if (input.getKeyDown("KeyB")) toggle("blinds");
    if (input.getKeyDown("KeyF")) toggle("haze");
    if (input.getKeyDown("KeyA")) toggle("auto");
    this.t += dt;
    this.update(state.day ? (this.t % DAY_LENGTH) / DAY_LENGTH : GOLDEN);
    // A slow look across the room from its far corner towards the windows.
    const t = this.t;
    const cam = this.camera.transform;
    cam.position = new Vector3(3.6 + Math.sin(t * 0.07) * 0.5, FLOOR + 1.5, 2.6 + Math.sin(t * 0.05) * 0.4);
    const target = new Vector3(-2.5 + Math.sin(t * 0.09) * 2.5, FLOOR + 1.0 + Math.sin(t * 0.06) * 0.3, -D);
    cam.rotation = Quaternion.lookRotation(target.sub(cam.position));
  }
}

function toggle(key: "day" | "blinds" | "haze" | "auto") {
  if (key === "day") state.day = !state.day;
  else if (key === "blinds") state.blinds = !state.blinds;
  else if (key === "haze") state.haze = !state.haze;
  else state.auto = !state.auto;
  apply();
}

register({ Flat });

function Button(props: { label: string; value: string; on: boolean; keyName: string; onClick: () => void }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#d6ccc0", fontSize: 13, width: 96, flexShrink: 0 }}>{props.label}</text>
      <box
        onClick={props.onClick}
        style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", minWidth: 150, backgroundColor: props.on ? "#d0763a" : "rgba(255,255,255,0.1)" }}
      >
        <text style={{ color: "#fbf3ea", fontSize: 13 }}>{props.value}</text>
      </box>
      <text style={{ color: "#8d8276", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(20, 14, 10, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>God rays: the sunset</text>
        <Button label="Time" value={state.day ? "Day passing" : "Golden hour"} on={state.day} keyName="T" onClick={() => toggle("day")} />
        <Button label="Blinds" value={state.blinds ? "Down" : "Up"} on={state.blinds} keyName="B" onClick={() => toggle("blinds")} />
        <Button label="Haze" value={state.haze ? "On" : "Off"} on={state.haze} keyName="F" onClick={() => toggle("haze")} />
        <Button label="Auto exposure" value={state.auto ? "On" : "Off"} on={state.auto} keyName="A" onClick={() => toggle("auto")} />
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
