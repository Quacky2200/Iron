// SPDX-License-Identifier: Apache-2.0

// Eye adaptation (`RenderSettings.autoExposure`): a bedroom on a bright day behind closed
// louvred shutters. The sun comes through the slats in crisp stripes across the floor: the
// room is dim, but the eyes have adapted and it's easy to see. Then the shutters swing open: blinding, the windows white,
// until the eyes settle into a bright room with sunlight across the floor and the bed. Later
// they close again: near black, and the room only slowly comes back as the eyes adapt to the
// dark. The camera looks round the window side all the while.
//
// Outside is a garden under a blue sky: a lawn, a hedge, trees and a shed, lit by the sun
// and the sky. Spot-meter the window (M) and the eyes adapt to the garden: it comes out of
// the glare while the room sinks into black around it. Clouds drift over the sun (K), so
// the light swells and fades; a bedside lamp (B) and a flickering television (V) light the
// room from inside, and give the metering, holding and speed settings something to do.
//
// The menu (two pages: exposure, 1, and the scene, 2) tries out every setting live, with the
// frame rate at the top; Q cycles the quality level, U switches vsync. A graph shows the last fifteen seconds:
// how bright the view meters (where the eyes are heading) against the exposure in use (where
// they are), both in EV100 (`Camera.sceneBrightness`, `Camera.exposure`), so you can watch
// them adapt. The metering zone is drawn over the view. Keys: auto exposure on or off (A)
// blending over a transition (T) to a manual exposure (- and =, a stop at a time, easing
// over the same transition); freezing it where it is (Z);
// holding it (L); how fast eyes adapt (Y: game-fast or eye-slow); the metering mode (M) and
// histogram band (H); the haze's noise settings (G); a soft floor (S) and a compensation
// curve (C), which stop the
// exposure short of the dark on purpose, so dark places still look dark (the graph shows
// the gap). Restart the cycle with R, pause with Space.

// The room: x from -W to W, z from -D (the window wall) to D, H tall.
const W = 4;
const D = 3.5;
const H = 2.8;
/** The windows in the -z wall: their centres along x, and sill and head heights. */
const WINDOWS = [-1.8, 1.4];
const WIN_W = 1.3;
const SILL = 0.9;
const HEAD = 2.3;

/** Where sunlight travels: in through the windows and down onto the floor. */
const SUN = new Vector3(0.35, -0.55, 0.76).normalized;
const SUN_INTENSITY = 40;

/**
 * The look round: where the camera looks (yaw in degrees, 0 = towards the windows) from
 * each time, eased between. It faces the windows as the shutters open and close.
 */
const LOOKS: [number, number][] = [
  [0, 30],
  [5, 60],
  [9, 0],
  [12, 17], // looking out of the right-hand window: spot-meter it to see the garden
  [17, 17],
  [21, 70],
  [24, 10],
  [28, -5],
  [33, 45],
  [37, 70],
  [40, 30],
];
const CYCLE = 40;
/** When the shutters open and close (seconds into the cycle), and how long a swing takes. */
const OPEN_AT = 10;
const CLOSE_AT = 25;
const SWING = 2;
/** How far the leaves swing open, degrees. */
const OPEN_ANGLE = 105;

/** The choices the menu cycles through. */
const METERING = ["centerWeighted", "average", "spot"] as const;
const METERING_NAMES = { centerWeighted: "Centre-weighted", average: "Average", spot: "Spot" };
const TRANSITIONS = [1, 3, 0];
/** The manual exposure's range (EV100), stepped a stop at a time. */
const MANUAL_MIN = -6;
const MANUAL_MAX = 10;
/** How fast the eyes adapt: like a game (under a second) or like real eyes (slow in the dark). */
const SPEEDS = [
  { name: "Game (under a second)", toLight: 4, toDark: 3.5 },
  { name: "Eyes (slow in the dark)", toLight: 1.2, toDark: 0.35 },
];
/** The graph: seconds shown and samples a second. */
const GRAPH_SECONDS = 15;
const GRAPH_RATE = 10;
const history: { exposure: number; scene: number }[] = [];

/**
 * The haze's quality (`RenderSettings.volumetricFog`): the finest volume, its noise cleaned
 * up more or less (the slats' fine stripes are the hard case).
 */
const HAZE = [
  { name: "Smooth (4 samples, 0.95)", denoising: "both", historyWeight: 0.95, samples: 4 },
  { name: "Default (1 sample, 0.9)", denoising: "both", historyWeight: 0.9, samples: 1 },
  { name: "Reprojection only", denoising: "reprojection", historyWeight: 0.9, samples: 1 },
  { name: "None (steps)", denoising: "none", historyWeight: 0.9, samples: 1 },
] as const;

function applyHaze(): void {
  const h = HAZE[state.haze]!;
  RenderSettings.volumetricFog = {
    intensity: 0.25,
    anisotropy: 0.5,
    steps: 4,
    distance: 12,
    albedo: new Color(1, 0.9, 0.78, 1),
    resolution: "ultra",
    denoising: h.denoising,
    historyWeight: h.historyWeight,
    samples: h.samples,
  };
  refreshMenu();
}

/** The layer the garden is on: only the sky's light (a fill light on this layer) reaches it. */
const OUTSIDE = 9;

interface State {
  haze: number;
  clouds: boolean;
  lamp: boolean;
  tv: boolean;
  auto: boolean;
  paused: boolean;
  hold: boolean;
  metering: number;
  band: boolean;
  soft: boolean;
  curve: boolean;
  transition: number;
  manual: number;
  speed: number;
  /** Set by freezing: the manual exposure is wherever the eyes were. */
  frozenAt: number | null;
}
const state: State = {
  haze: 0,
  clouds: true,
  lamp: true,
  tv: true,
  auto: true,
  paused: false,
  hold: false,
  metering: 0,
  band: false,
  soft: false,
  curve: false,
  transition: 0,
  manual: 0,
  speed: 0,
  frozenAt: null,
};
let refreshMenu = () => {};

function apply(): void {
  RenderSettings.exposureTransition = TRANSITIONS[state.transition]!;
  RenderSettings.fixedExposure = state.frozenAt ?? state.manual;
  const speed = SPEEDS[state.speed]!;
  RenderSettings.autoExposure = state.auto
    ? {
        minEV: -7,
        maxEV: 16,
        compensation: -0.6,
        speedDarkToLight: speed.toLight,
        speedLightToDark: speed.toDark,
        metering: METERING[state.metering],
        lowPercent: state.band ? 40 : 0,
        highPercent: state.band ? 90 : 100,
        darkAdaptation: state.soft ? 0.6 : 1,
        darkBelow: 3,
        curve: state.curve ? [[-4, -1.5], [4, 0]] : undefined,
        hold: state.hold,
      }
    : null;
  refreshMenu();
}

/** Turns auto exposure off right where it is: nothing changes on screen. */
function freeze(): void {
  if (!state.auto) return;
  RenderSettings.freezeExposure();
  state.auto = false;
  state.frozenAt = RenderSettings.fixedExposure;
  refreshMenu();
}

function shape(name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, material: Material, parent?: GameObject, rotation?: Quaternion) {
  const go = new GameObject(name, { position: at, scale, parent, rotation });
  go.addComponent(MeshRenderer, { mesh, material });
  return go;
}
/** A box between two corners. */
function slab(name: string, x0: number, y0: number, z0: number, x1: number, y1: number, z1: number, material: Material) {
  return shape(name, "cube", new Vector3((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2), new Vector3(Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)), material);
}
const mat = (hex: string, smoothness = 0.08) => Material.create({ color: Color.hex(hex), smoothness });

/** The yaw at time `t` of the look round, eased between the `LOOKS`. */
function yawAt(t: number): number {
  for (let i = 0; i + 1 < LOOKS.length; i++) {
    const [t0, y0] = LOOKS[i]!;
    const [t1, y1] = LOOKS[i + 1]!;
    if (t <= t1) {
      const u = Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
      return y0 + (y1 - y0) * u * u * (3 - 2 * u);
    }
  }
  return LOOKS[LOOKS.length - 1]![1];
}

class Bedroom extends Component {
  private camera!: Camera;
  private t = 0;
  private readout = 0;
  private sun!: Light;
  private lamp!: Light;
  private lampShade!: MeshRenderer;
  private shadeOn!: Material;
  private shadeOff!: Material;
  private tv!: Light;
  private screen!: MeshRenderer;
  private screens: Material[] = [];
  private screenOff!: Material;
  private flicker = 0;

  override onStart() {
    QualitySettings.shadowDistance = 20;
    QualitySettings.shadowResolution = 4096;
    // Hardly any light reaches the far corner: the sun's patches and the windows are the
    // room's light.
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.035, 0.032, 0.03, 1);
    RenderSettings.tonemapping = "aces";
    RenderSettings.bloom = { threshold: 1, intensity: 0.35, scatter: 0.55 };
    RenderSettings.fogMode = "exponential";
    RenderSettings.fogColor = new Color(0.01, 0.01, 0.01, 1);
    RenderSettings.fogDensity = 0.03;
    applyHaze();
    RenderSettings.fog = true;

    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(SUN) });
    this.sun = sun.addComponent(Light, { type: "directional", color: new Color(1, 0.93, 0.82, 1), intensity: SUN_INTENSITY, shadows: "soft" });
    // The sky's light, on the garden only (its layer): the room keeps its own dimness.
    const skyFill = new GameObject("Sky light", { rotation: Quaternion.lookRotation(new Vector3(0.1, -1, 0.2)) });
    const fill = skyFill.addComponent(Light, { type: "directional", color: new Color(0.6, 0.75, 1, 1), intensity: 9, volumetricIntensity: 0 });
    fill.cullingMask = 1 << OUTSIDE;
    // A clear blue sky with the sun in it, seen through the windows.
    RenderSettings.skybox = {
      zenith: new Color(12, 22, 48, 1),
      horizon: new Color(42, 46, 54, 1),
      ground: new Color(8, 8, 7, 1),
      sunDirection: SUN.scale(-1),
      sunColor: new Color(2500, 2300, 2000, 1),
      sunSize: 1.2,
    };
    // The bright sky mustn't show in the room's floor and furniture.
    RenderSettings.reflectionIntensity = 0.02;

    Random.initState(5);
    this.buildRoom();
    this.buildFurniture();
    this.buildGarden();
    this.buildLights();

    const eye = new GameObject("Main Camera", { position: new Vector3(0.2, 1.6, 0.4), rotation: Quaternion.lookRotation(new Vector3(0.17, -0.12, -1)) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 68, clearFlags: "skybox", farClipPlane: 200, nearClipPlane: 0.05 });
    apply();
  }

  /** The garden outside: a lawn, a hedge, trees, a shed and a path, on the outside layer. */
  private buildGarden() {
    const outside = (go: GameObject) => {
      go.layer = OUTSIDE;
      return go;
    };
    const grass = mat("#5f8a3a");
    const path = mat("#b8ab90");
    const hedge = mat("#3f6a2a");
    const bark = mat("#5a4030");
    const leaves = [mat("#4f7d34"), mat("#5d8b3c"), mat("#466f2c")];
    const shed = mat("#8a6a4a");
    const roof = mat("#5a3a2a");
    outside(slab("Lawn", -60, -0.6, -80, 60, -0.4, -D - 0.3, grass));
    outside(slab("Path", -0.7, -0.4, -30, 0.7, -0.38, -D - 0.3, path));
    outside(slab("Hedge", -40, -0.4, -16.6, 40, 1.8, -15, hedge));
    outside(slab("Shed", 5, -0.4, -12, 8.5, 2.2, -9, shed));
    outside(shape("Shed roof", "cube", new Vector3(6.75, 2.4, -10.5), new Vector3(3.9, 0.25, 3.4), roof, undefined, Quaternion.euler(0, 0, 8)));
    outside(slab("Shed door", 5.9, -0.4, -9.02, 6.9, 1.6, -8.98, roof));
    for (let i = 0; i < 18; i++) {
      const x = Random.range(-30, 30);
      const z = Random.range(-45, -18);
      const h = Random.range(3, 7);
      outside(shape("Trunk", "cylinder", new Vector3(x, h / 2 - 0.4, z), new Vector3(0.4, h / 2, 0.4), bark));
      const r = Random.range(1.8, 3.6);
      outside(shape("Crown", "sphere", new Vector3(x, h + r * 0.4, z), new Vector3(r * 2, r * 1.8, r * 2), leaves[i % 3]!));
    }
  }

  /** A bedside lamp by the bed and a television in the dark corner. */
  private buildLights() {
    const wood = mat("#5a3e28", 0.2);
    const black = mat("#151515", 0.6);
    // The bedside table at the head of the bed, and its lamp.
    slab("Bedside table", -W + 0.05, 0, -2.3, -W + 0.55, 0.55, -1.75, wood);
    shape("Lamp base", "cylinder", new Vector3(-W + 0.3, 0.7, -2.02), new Vector3(0.08, 0.15, 0.08), black);
    this.shadeOff = mat("#e8dcc0");
    this.shadeOn = Material.create({ color: Color.hex("#e8dcc0"), emission: new Color(3.5, 2.6, 1.6, 1) });
    this.lampShade = shape("Lamp shade", "cylinder", new Vector3(-W + 0.3, 0.98, -2.02), new Vector3(0.32, 0.14, 0.32), this.shadeOn).getComponent(MeshRenderer)!;
    const lamp = new GameObject("Bedside lamp", { position: new Vector3(-W + 0.3, 1.0, -2.02) });
    this.lamp = lamp.addComponent(Light, { type: "point", color: new Color(1, 0.72, 0.42, 1), intensity: 1.6, range: 5, volumetricIntensity: 0.5 });
    // The television on a low stand against the back wall, facing the room.
    slab("TV stand", -2.4, 0, D - 0.5, -0.8, 0.45, D - 0.05, wood);
    slab("TV", -2.2, 0.5, D - 0.2, -1.0, 1.2, D - 0.1, black);
    this.screenOff = Material.create({ color: Color.hex("#101418"), smoothness: 0.8 });
    // A few frames of a programme: cool blues, a warm scene, a bright cut.
    this.screens = [
      [0.4, 0.6, 1.2],
      [0.9, 1.1, 1.6],
      [1.4, 0.9, 0.5],
      [0.2, 0.3, 0.6],
      [2.0, 2.0, 2.2],
    ].map(([r, g, b]) => Material.create({ color: Color.black, emission: new Color(r!, g!, b!, 1), unlit: true }));
    this.screen = slab("Screen", -2.15, 0.55, D - 0.205, -1.05, 1.15, D - 0.2, this.screens[0]!).getComponent(MeshRenderer)!;
    const tv = new GameObject("TV light", { position: new Vector3(-1.6, 0.85, D - 0.6), rotation: Quaternion.lookRotation(new Vector3(0, -0.1, -1)) });
    this.tv = tv.addComponent(Light, { type: "spot", color: new Color(0.6, 0.75, 1, 1), intensity: 1.5, range: 6, spotAngle: 120, innerSpotAngle: 60, volumetricIntensity: 0 });
  }

  /** Clouds over the sun, the lamp and the television, each frame. */
  private updateLights(dt: number) {
    // Clouds: a slow mix of waves, now and then thick enough to dim the sun to a quarter.
    const t = this.t;
    const thick = Math.min(1, Math.max(0, 0.55 + 0.6 * Math.sin(t * 0.23) * Math.sin(t * 0.071 + 1) + 0.25 * Math.sin(t * 0.61)));
    this.sun.intensity = SUN_INTENSITY * (state.clouds ? 1 - 0.75 * thick * thick : 1);
    this.lamp.intensity = state.lamp ? 1.6 : 0;
    this.lampShade.material = state.lamp ? this.shadeOn : this.shadeOff;
    // The television changes scene every so often, flickering a little between.
    this.flicker -= dt;
    if (this.flicker <= 0) {
      this.flicker = Random.range(0.15, 1.2);
      const k = Math.floor(Random.value * this.screens.length);
      this.screen.material = state.tv ? this.screens[k]! : this.screenOff;
      const e = [0.8, 1.3, 1.2, 0.5, 2.0][k]!;
      this.tv.intensity = state.tv ? 1.4 * e : 0;
    }
    if (!state.tv) {
      this.screen.material = this.screenOff;
      this.tv.intensity = 0;
    }
  }


  private buildRoom() {
    const wall = mat("#c9bfae");
    const floor = mat("#7a5a3c", 0.3);
    const ceiling = mat("#e2ddd2");
    slab("Floor", -W - 0.2, -0.2, -D - 0.2, W + 0.2, 0, D + 0.2, floor);
    slab("Ceiling", -W - 0.2, H, -D - 0.2, W + 0.2, H + 0.2, D + 0.2, ceiling);
    slab("Wall", -W - 0.2, -0.2, D, W + 0.2, H + 0.2, D + 0.2, wall);
    slab("Wall", -W - 0.2, -0.2, -D, -W, H + 0.2, D, wall);
    slab("Wall", W, -0.2, -D, W + 0.2, H + 0.2, D, wall);
    // The window wall, cut round the windows.
    const T = 0.25;
    const edges = [-W - 0.2, ...WINDOWS.flatMap((x) => [x - WIN_W / 2, x + WIN_W / 2]), W + 0.2];
    for (let i = 0; i < edges.length; i += 2) slab("Wall", edges[i]!, -0.2, -D - T, edges[i + 1]!, H + 0.2, -D, wall);
    const frame = mat("#f2efe8", 0.3);
    for (const x of WINDOWS) {
      const x0 = x - WIN_W / 2;
      const x1 = x + WIN_W / 2;
      slab("Below window", x0, -0.2, -D - T, x1, SILL, -D, wall);
      slab("Above window", x0, HEAD, -D - T, x1, H + 0.2, -D, wall);
      slab("Sill", x0 - 0.08, SILL - 0.04, -D - 0.05, x1 + 0.08, SILL, -D + 0.12, frame);
      // Glazing bars: a cross.
      slab("Glazing bar", x - 0.025, SILL, -D - 0.14, x + 0.025, HEAD, -D - 0.1, frame);
      slab("Glazing bar", x0, (SILL + HEAD) / 2 - 0.025, -D - 0.14, x1, (SILL + HEAD) / 2 + 0.025, -D - 0.1, frame);
      this.buildShutters(x0, x1);
    }
    slab("Skirting", -W, 0, D - 0.02, W, 0.12, D, frame);
  }

  /** The leaves' hinges: left leaves turn one way, right ones the other. */
  private leaves: { hinge: GameObject; side: number }[] = [];

  /**
   * Louvred shutters inside a window from `x0` to `x1`: a leaf hinged at each side, its slats
   * tilted with the sun, so it comes through in clean stripes.
   */
  private buildShutters(x0: number, x1: number) {
    const wood = mat("#e9e3d6", 0.25);
    // Each leaf overlaps the opening by LAP on its outer side and the top, and sits on the
    // sill, flush against the wall: no gap for low sun to slip round.
    const LAP = 0.05;
    const w = (x1 - x0) / 2 + LAP;
    const top = HEAD + LAP;
    const z = -D + 0.002;
    for (const side of [-1, 1]) {
      const hinge = new GameObject("Shutter hinge", { position: new Vector3(side < 0 ? x0 - LAP : x1 + LAP, 0, z) });
      this.leaves.push({ hinge, side });
      // The leaf runs from the hinge towards the window's middle (local -side along x).
      const mid = (-side * w) / 2;
      const s = (name: string, cx: number, cy: number, sx: number, sy: number, sz: number, rot?: Quaternion) =>
        shape(name, "cube", new Vector3(cx, cy, 0.02), new Vector3(sx, sy, sz), wood, hinge, rot);
      s("Stile", -side * 0.04, (SILL + top) / 2, 0.08, top - SILL, 0.04);
      s("Stile", -side * (w - 0.03), (SILL + top) / 2, 0.06, top - SILL, 0.04);
      s("Rail", mid, SILL + 0.04, w, 0.08, 0.04);
      s("Rail", mid, top - 0.04, w, 0.08, 0.04);
      // Slats 5 cm apart, each 9 cm deep and tilted with the sun's fall, so it comes through
      // between them in clean stripes (its edges round the leaves are sealed).
      for (let y = SILL + 0.1; y < top - 0.06; y += 0.05) s("Slat", mid, y, w - 0.1, 0.008, 0.09, Quaternion.euler(50, 0, 0));
    }
  }

  /** Swings the leaves to `k` of the way open (0 closed, 1 open). */
  private setShutters(k: number) {
    for (const { hinge, side } of this.leaves) hinge.transform.localRotation = Quaternion.euler(0, side * k * OPEN_ANGLE, 0);
  }

  private buildFurniture() {
    const oak = mat("#6b4a2e", 0.2);
    const dark = mat("#3a2a1e", 0.2);
    const linen = mat("#e7e1d4");
    const blanket = mat("#2f4a6a");
    const fabric = mat("#6a3b2b");
    // The dark corner (+x, +z): a wardrobe, an armchair, a standard lamp (off), pictures.
    slab("Wardrobe", W - 0.65, 0, D - 2.4, W, 2.1, D - 1.2, dark);
    slab("Wardrobe door line", W - 0.66, 0.1, D - 1.81, W - 0.64, 2.0, D - 1.79, oak);
    slab("Wardrobe knob", W - 0.69, 1.05, D - 1.9, W - 0.65, 1.1, D - 1.85, oak);
    slab("Chair seat", W - 1.8, 0.35, D - 1.0, W - 1.0, 0.5, D - 0.2, fabric);
    slab("Chair back", W - 1.8, 0.35, D - 0.3, W - 1.0, 1.1, D - 0.1, fabric);
    for (const dx of [-0.38, 0.38]) slab("Chair arm", W - 1.4 + dx - 0.06, 0.35, D - 1.0, W - 1.4 + dx + 0.06, 0.7, D - 0.1, fabric);
    shape("Lamp stand", "cylinder", new Vector3(W - 0.4, 0.75, D - 0.4), new Vector3(0.04, 0.75, 0.04), dark);
    shape("Lamp shade", "cylinder", new Vector3(W - 0.4, 1.6, D - 0.4), new Vector3(0.45, 0.2, 0.45), linen);
    const frames = [mat("#b08a4a", 0.4), mat("#2a2a2a", 0.3)];
    const paintings = [mat("#2f5a4a"), mat("#7a4a2a"), mat("#3a4a7a")];
    for (const [x, y, w, h, k] of [[1.0, 1.7, 0.9, 0.65, 0], [2.3, 1.55, 0.5, 0.7, 1], [-0.2, 1.5, 0.6, 0.45, 2]] as const) {
      slab("Picture frame", x - w / 2, y - h / 2, D - 0.04, x + w / 2, y + h / 2, D, frames[k % 2]!);
      slab("Picture", x - w / 2 + 0.06, y - h / 2 + 0.06, D - 0.05, x + w / 2 - 0.06, y + h / 2 - 0.06, D - 0.04, paintings[k]!);
    }
    // The bed along the -x wall, where the sun falls.
    slab("Bed frame", -W, 0, -1.6, -W + 1.5, 0.35, 0.6, oak);
    slab("Mattress", -W + 0.05, 0.35, -1.55, -W + 1.45, 0.55, 0.55, linen);
    slab("Blanket", -W + 0.03, 0.5, -1.0, -W + 1.47, 0.6, 0.57, blanket);
    slab("Pillow", -W + 0.2, 0.55, -1.5, -W + 1.3, 0.7, -1.15, linen);
    slab("Headboard", -W, 0, -1.7, -W + 1.5, 1.1, -1.6, oak);
    // A desk and chair under the right window, a rug, a bookcase on the -x wall's far end.
    slab("Desk", 0.6, 0.72, -D + 0.15, 2.2, 0.76, -D + 0.85, oak);
    for (const [x, z] of [[0.65, -D + 0.2], [2.15, -D + 0.2], [0.65, -D + 0.8], [2.15, -D + 0.8]] as const) slab("Desk leg", x - 0.03, 0, z - 0.03, x + 0.03, 0.72, z + 0.03, oak);
    slab("Desk chair", 1.1, 0.45, -D + 1.0, 1.6, 0.5, -D + 1.5, dark);
    slab("Desk chair back", 1.1, 0.45, -D + 1.45, 1.6, 1.0, -D + 1.5, dark);
    slab("Rug", -1.6, 0, -1.2, 1.4, 0.015, 1.4, mat("#7a3a34", 0.02));
    slab("Bookcase", -W, 0, D - 1.3, -W + 0.4, 1.9, D - 0.1, dark);
    const spines = [mat("#8a2a2a"), mat("#2a4a7a"), mat("#c9a24a"), mat("#3a6a3a")];
    for (let shelf = 0; shelf < 4; shelf++) {
      for (let z = D - 1.25; z < D - 0.2; z += Random.range(0.05, 0.09)) {
        const h = Random.range(0.25, 0.38);
        slab("Book", -W + 0.05, 0.08 + shelf * 0.46, z, -W + 0.32, 0.08 + shelf * 0.46 + h, z + 0.04, spines[Math.floor(Random.value * 4)]!);
      }
    }
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyK")) toggle("clouds");
    if (input.getKeyDown("KeyG")) nextHaze();
    if (input.getKeyDown("KeyQ")) nextQuality();
    if (input.getKeyDown("KeyU")) toggleVSync();
    if (input.getKeyDown("Digit1") || input.getKeyDown("Digit2")) {
      tab = input.getKeyDown("Digit1") ? 0 : 1;
      refreshMenu();
    }
    // Frame rate, over half a second.
    timing.frames++;
    timing.time += dt;
    if (timing.time >= 0.5) {
      timing.fps = timing.frames / timing.time;
      timing.ms = (1000 * timing.time) / timing.frames;
      timing.frames = 0;
      timing.time = 0;
    }
    if (input.getKeyDown("KeyB")) toggle("lamp");
    if (input.getKeyDown("KeyV")) toggle("tv");
    if (input.getKeyDown("KeyA")) toggleAuto();
    if (input.getKeyDown("KeyZ")) freeze();
    if (input.getKeyDown("KeyT")) cycle("transition", TRANSITIONS.length);
    if (input.getKeyDown("Minus")) stepManual(-1);
    if (input.getKeyDown("Equal")) stepManual(1);
    if (input.getKeyDown("KeyL")) toggle("hold");
    if (input.getKeyDown("KeyM")) cycle("metering", METERING.length);
    if (input.getKeyDown("KeyY")) cycle("speed", SPEEDS.length);
    if (input.getKeyDown("KeyH")) toggle("band");
    if (input.getKeyDown("KeyS")) toggle("soft");
    if (input.getKeyDown("KeyC")) toggle("curve");
    if (input.getKeyDown("Space")) toggle("paused");
    // The graph's samples, and the readout.
    this.readout += dt;
    if (this.readout > 1 / GRAPH_RATE) {
      this.readout = 0;
      const cam = Camera.main;
      if (cam && Number.isFinite(cam.exposure)) {
        history.push({ exposure: cam.exposure, scene: cam.sceneBrightness });
        if (history.length > GRAPH_SECONDS * GRAPH_RATE) history.shift();
      }
      refreshMenu();
    }
    if (input.getKeyDown("KeyR")) this.t = 0;
    if (!state.paused) this.t = (this.t + dt) % CYCLE;
    this.updateLights(dt);
    const ease = (u: number) => {
      const c = Math.min(1, Math.max(0, u));
      return c * c * (3 - 2 * c);
    };
    this.setShutters(ease((this.t - OPEN_AT) / SWING) - ease((this.t - CLOSE_AT) / SWING));
    const yaw = (yawAt(this.t) * Math.PI) / 180;
    // A slight drift, as someone standing does.
    const cam = this.camera.transform;
    cam.position = new Vector3(0.2 + Math.sin(this.t * 0.4) * 0.04, 1.6 + Math.sin(this.t * 0.9) * 0.01, 0.4);
    cam.rotation = Quaternion.lookRotation(new Vector3(Math.sin(yaw), -0.12, -Math.cos(yaw)));
  }
}

function toggle(key: "paused" | "hold" | "band" | "soft" | "curve" | "clouds" | "lamp" | "tv") {
  if (key === "paused") state.paused = !state.paused;
  else if (key === "hold") state.hold = !state.hold;
  else if (key === "band") state.band = !state.band;
  else if (key === "soft") state.soft = !state.soft;
  else if (key === "curve") state.curve = !state.curve;
  else if (key === "clouds") state.clouds = !state.clouds;
  else if (key === "lamp") state.lamp = !state.lamp;
  else state.tv = !state.tv;
  apply();
}

/**
 * A stop darker or brighter for the manual exposure (eased over the transition, as turning
 * auto exposure on or off is). From a freeze it starts where the freeze held.
 */
function stepManual(stops: number) {
  const from = state.frozenAt ?? state.manual;
  state.frozenAt = null;
  state.manual = Math.min(MANUAL_MAX, Math.max(MANUAL_MIN, Math.round(from) + stops));
  apply();
}

function cycle(key: "transition" | "metering" | "speed", n: number) {
  if (key === "transition") state.transition = (state.transition + 1) % n;
  else if (key === "metering") state.metering = (state.metering + 1) % n;
  else state.speed = (state.speed + 1) % n;
  apply();
}

/** On, or off to the manual exposure (blending over the transition). */
function toggleAuto() {
  state.auto = !state.auto;
  state.frozenAt = null;
  apply();
}

const ev = (v: number | undefined) => (v === undefined || !Number.isFinite(v) ? "…" : `EV ${v.toFixed(1)}`);

register({ Bedroom });

/** A value with − and + buttons beside it (keys - and =). */
function Stepper(props: { label: string; value: string; on: boolean; onDown: () => void; onUp: () => void }) {
  const small = (text: string, onClick: () => void) => (
    <box onClick={onClick} style={{ padding: "6px 12px", borderRadius: 6, cursor: "pointer", backgroundColor: "rgba(255,255,255,0.14)" }}>
      <text style={{ color: "#f4f6f8", fontSize: 13 }}>{text}</text>
    </box>
  );
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#c8ccd2", fontSize: 13, width: 104, flexShrink: 0 }}>{props.label}</text>
      <box style={{ flexDirection: "row", gap: 6, minWidth: 150 }}>
        {small("−", props.onDown)}
        <box style={{ padding: "6px 12px", borderRadius: 6, flexGrow: 1, backgroundColor: props.on ? "#3a7bd5" : "rgba(255,255,255,0.1)" }}>
          <text style={{ color: "#f4f6f8", fontSize: 13 }}>{props.value}</text>
        </box>
        {small("+", props.onUp)}
      </box>
      <text style={{ color: "#7d838c", fontSize: 12 }}>- =</text>
    </box>
  );
}

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

/**
 * The last fifteen seconds: how bright the view meters (orange: where the eyes are heading)
 * and the exposure in use (white: where they are), in EV100. Where they part, the eyes are
 * still adapting, or (soft floor, curve) stopping short of the dark on purpose.
 */
function Graph() {
  const W = 300;
  const H = 96;
  let low = -4;
  let high = 4;
  for (const h of history) {
    low = Math.min(low, h.exposure);
    high = Math.max(high, h.exposure);
    if (Number.isFinite(h.scene)) {
      low = Math.min(low, h.scene);
      high = Math.max(high, h.scene);
    }
  }
  const lo = Math.floor(low - 0.5);
  const hi = Math.ceil(high + 0.5);
  const x = (i: number) => (i / (GRAPH_SECONDS * GRAPH_RATE - 1)) * W;
  const y = (v: number) => H - ((v - lo) / (hi - lo)) * H;
  const line = (pick: (h: { exposure: number; scene: number }) => number) => {
    let out = "";
    for (let i = 0; i < history.length; i++) {
      const v = pick(history[i]!);
      if (Number.isFinite(v)) out += `${out === "" ? "" : " "}${x(i).toFixed(1)},${y(v).toFixed(1)}`;
    }
    return out;
  };
  const scene = line((h) => h.scene);
  const exposure = line((h) => h.exposure);
  return (
    <box style={{ flexDirection: "column", gap: 4 }}>
      <svg viewBox={`0 0 ${W} ${H}`} style={{ width: W, height: H, backgroundColor: "rgba(255,255,255,0.05)", borderRadius: 4 }}>
        {/* A line needs two points: none until the first samples are in. */}
        {scene.includes(" ") && <polyline points={scene} fill="none" stroke="#f0a040" strokeWidth={2} />}
        {exposure.includes(" ") && <polyline points={exposure} fill="none" stroke="#ffffff" strokeWidth={2} />}
      </svg>
      <box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <text style={{ color: "#f0a040", fontSize: 12 }}>view meters {ev(Camera.main?.sceneBrightness)}</text>
        <text style={{ color: "#ffffff", fontSize: 12 }}>exposure {ev(Camera.main?.exposure)}</text>
        <text style={{ color: "#7d838c", fontSize: 12 }}>
          {lo}…{hi} EV
        </text>
      </box>
    </box>
  );
}

/** The metering zone over the view: a soft ring (centre-weighted), a small circle (spot). */
function MeteringZone() {
  if (!state.auto) return null;
  const mode = METERING[state.metering];
  if (mode === "average") return null;
  // Spot meters a circle of radius 0.12 (of the view's width and height); centre-weighted
  // counts half as much 0.34 out.
  const r = mode === "spot" ? 12 : 34;
  return (
    <box
      style={{
        position: "absolute",
        left: `${50 - r}%`,
        top: `${50 - r}%`,
        width: `${2 * r}%`,
        height: `${2 * r}%`,
        borderRadius: "50%",
        borderWidth: mode === "spot" ? 2 : 1,
        borderColor: mode === "spot" ? "rgba(240,160,64,0.8)" : "rgba(240,160,64,0.35)",
        pointerEvents: "none",
      }}
    />
  );
}

/** Which page of the menu shows: the exposure settings or the scene's. */
let tab = 0;

/** Frames a second and milliseconds a frame, measured over half a second. */
const timing = { fps: 0, ms: 0, frames: 0, time: 0 };

function Tab(props: { label: string; index: number; keyName: string }) {
  const on = tab === props.index;
  return (
    <box
      onClick={() => {
        tab = props.index;
        refreshMenu();
      }}
      style={{ padding: "5px 12px", borderRadius: 6, cursor: "pointer", backgroundColor: on ? "rgba(255,255,255,0.18)" : "transparent" }}
    >
      <text style={{ color: on ? "#ffffff" : "#9aa0a8", fontSize: 13 }}>
        {props.label} <span style={{ color: "#6d737c" }}>{props.keyName}</span>
      </text>
    </box>
  );
}

function ExposurePage() {
  return (
    <>
      <Button
        label="Auto exposure"
        value={state.auto ? "On (eyes adapt)" : state.frozenAt !== null ? `Frozen (${ev(state.frozenAt)})` : `Off (manual ${ev(state.manual)})`}
        on={state.auto}
        keyName="A"
        onClick={toggleAuto}
      />
      <Button label="Freeze here" value="Off, unseen" on={false} keyName="Z" onClick={freeze} />
      <Button label="Transition" value={TRANSITIONS[state.transition] ? `${TRANSITIONS[state.transition]} s blend` : "Instant"} on={TRANSITIONS[state.transition]! > 0} keyName="T" onClick={() => cycle("transition", TRANSITIONS.length)} />
      <Stepper label="Manual EV" value={ev(state.frozenAt ?? state.manual)} on={!state.auto} onDown={() => stepManual(-1)} onUp={() => stepManual(1)} />
      <Button label="Hold" value={state.hold ? "Held where it is" : "Adapting"} on={state.hold} keyName="L" onClick={() => toggle("hold")} />
      <Button label="Adapts like" value={SPEEDS[state.speed]!.name} on={state.speed === 0} keyName="Y" onClick={() => cycle("speed", SPEEDS.length)} />
      <Button label="Metering" value={METERING_NAMES[METERING[state.metering] ?? "centerWeighted"]} on={true} keyName="M" onClick={() => cycle("metering", METERING.length)} />
      <Button label="Histogram" value={state.band ? "40–90% (no extremes)" : "Everything"} on={state.band} keyName="H" onClick={() => toggle("band")} />
      <Button label="Soft floor" value={state.soft ? "Below EV 3: 60% of the way" : "Off: all the way"} on={state.soft} keyName="S" onClick={() => toggle("soft")} />
      <Button label="Curve" value={state.curve ? "-1.5 stops at EV -4" : "Off"} on={state.curve} keyName="C" onClick={() => toggle("curve")} />
    </>
  );
}

function ScenePage() {
  return (
    <>
      <Button label="Quality" value={QualitySettings.names[QualitySettings.getQualityLevel()] ?? "?"} on={true} keyName="Q" onClick={nextQuality} />
      <Button label="VSync" value={QualitySettings.vSyncCount > 0 ? "On (capped)" : "Off (as fast as it can)"} on={QualitySettings.vSyncCount > 0} keyName="U" onClick={toggleVSync} />
      <Button label="Haze" value={HAZE[state.haze]!.name} on={state.haze === 0} keyName="G" onClick={nextHaze} />
      <Button label="Shutters" value={state.paused ? "Paused" : "Opening and closing"} on={!state.paused} keyName="Space" onClick={() => toggle("paused")} />
      <Button label="Clouds" value={state.clouds ? "Drifting over the sun" : "Clear sky"} on={state.clouds} keyName="K" onClick={() => toggle("clouds")} />
      <Button label="Bedside lamp" value={state.lamp ? "On" : "Off"} on={state.lamp} keyName="B" onClick={() => toggle("lamp")} />
      <Button label="Television" value={state.tv ? "On" : "Off"} on={state.tv} keyName="V" onClick={() => toggle("tv")} />
    </>
  );
}

function nextHaze() {
  state.haze = (state.haze + 1) % HAZE.length;
  applyHaze();
}

/** The next quality level (Low, Medium, High, Ultra), then the haze again (Low turns it off). */
function nextQuality() {
  QualitySettings.setQualityLevel((QualitySettings.getQualityLevel() + 1) % QualitySettings.names.length);
  applyHaze();
}

function toggleVSync() {
  QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
  refreshMenu();
}

function Menu() {
  const [, setTick] = useState(0);
  refreshMenu = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <MeteringZone />
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 14, gap: 8, borderRadius: 10, backgroundColor: "rgba(12, 14, 18, 0.72)", flexDirection: "column" }}>
        <box style={{ flexDirection: "row", justifyContent: "space-between", alignItems: "baseline", gap: 16 }}>
          <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Eye adaptation: the bedroom</text>
          <text style={{ color: timing.fps >= 200 ? "#7ad08a" : timing.fps >= 60 ? "#e0c060" : "#e07060", fontSize: 13 }}>
            {timing.fps.toFixed(0)} fps · {timing.ms.toFixed(2)} ms
          </text>
        </box>
        <Graph />
        <box style={{ flexDirection: "row", gap: 4 }}>
          <Tab label="Exposure" index={0} keyName="1" />
          <Tab label="Scene" index={1} keyName="2" />
        </box>
        {tab === 0 ? <ExposurePage /> : <ScenePage />}
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
