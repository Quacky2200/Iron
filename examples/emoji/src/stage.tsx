// The set: floor, spotlight beam and pool of light, drifting dust, the dark around the spot,
// and the red curtain. The curtain halves are WorldPanels (so they sit in front of the
// emoji); drawing them back squashes their folds towards the sides.
import { GATHER, GROUND, LIGHT, view } from "./config.ts";
import { Drape, Valance } from "./hud.tsx";

export class Mote extends Component {
  speed = 0.2 + Math.random() * 0.4;
  sway = Math.random() * 6.28;
  home = 0;
  override onUpdate(dt: number): void {
    const p = this.transform.position;
    let y = p.y + this.speed * dt;
    if (y > 5.5) y = GROUND + 0.2;
    this.sway += dt * 0.7;
    this.transform.position = new Vector3(this.home + Math.sin(this.sway) * 0.3, y, p.z);
  }
}

/** A rectangle with a round hole cut out, the hole's rim reached over a zero-width bridge. */
function framed(half: Vector2, hole: Vector2, radius: number): Vector2[] {
  const rect = [new Vector2(-half.x, -half.y), new Vector2(half.x, -half.y), new Vector2(half.x, half.y), new Vector2(-half.x, half.y)];
  const ring = Shapes.circle(hole, radius, 64);
  const corner = rect[0]!;
  const rim = ring[0]!;
  return [...rect, corner, ...ring, rim, corner];
}

function fill(go: GameObject, points: Vector2[], color: Color, order: number = 0): LineRenderer2D {
  const line = go.addComponent(LineRenderer2D);
  line.points = points;
  line.fillColor = color;
  line.width = 0;
  line.loop = true;
  if (order !== 0) line.sortingOrder = order;
  return line;
}

type Rgba = number[];
const DAY_SKY = Color.hex("#04050a");
const NIGHT_SKY = Color.hex("#0a1438");

/** A fill that fades with the spotlight, and its colour when fully on. */
interface Lit {
  line: LineRenderer2D;
  color: Rgba;
}

interface Twinkle {
  line: LineRenderer2D;
  alpha: number;
  phase: number;
}

/** Where the spotlight hangs from. */
export interface BeamFrom {
  x: number;
  y: number;
  top: number;
}

const mixn = (a: number, b: number, k: number) => a + (b - a) * k;

/** The set: floor, spotlight beam and pool of light, drifting dust, the dark around the spot, stars for the night. */
export class Stage {
  private lights: Lit[] = [];
  private camera: Camera;
  private beam: GameObject;
  private beamLine: LineRenderer2D;
  private pool: GameObject;
  private poolLine: LineRenderer2D;
  private stars: Twinkle[] = [];
  private shown: number = -1;

  constructor(camera: Camera) {
    this.camera = camera;
    const floor = new GameObject("Floor", { position: new Vector3(0, GROUND - 1.1, 0.2) });
    fill(floor, Shapes.rectangle(-20, -1.2, 40, 2.4), Color.hex("#131724"));
    this.beam = new GameObject("Beam", { position: new Vector3(LIGHT.x, 0, -1) });
    this.beamLine = this.light(fill(this.beam, [new Vector2(-0.7, 6), new Vector2(0.7, 6), new Vector2(5.4, GROUND), new Vector2(-5.4, GROUND)], new Color(1, 0.92, 0.68, 0.075)), [1, 0.92, 0.68, 0.075]);
    this.pool = new GameObject("Pool", { position: new Vector3(LIGHT.x, GROUND, -1) });
    this.poolLine = this.light(fill(this.pool, Shapes.ellipse(Vector2.zero, 5.6, 0.75, 56), new Color(1, 0.92, 0.68, 0.2)), [1, 0.92, 0.68, 0.2]);

    for (let i = 0; i < 28; i++) {
      const home = (Math.random() - 0.5) * 8;
      const mote = new GameObject(`Mote ${i}`, { position: new Vector3(home, GROUND + Math.random() * 9, -0.5) });
      this.light(fill(mote, Shapes.circle(Vector2.zero, 0.025 + Math.random() * 0.03, 8), new Color(1, 0.95, 0.8, 0.5)), [1, 0.95, 0.8, 0.5]);
      mote.addComponent(Mote).home = home;
    }

    // Rings of dark, each with a slightly wider hole, add up to a soft edge.
    const rings = 28;
    for (let i = 0; i < rings; i++) {
      const dark = new GameObject(`Dark ${i}`, { position: new Vector3(LIGHT.x, LIGHT.y, 3) });
      this.light(fill(dark, framed(new Vector2(30, 20), Vector2.zero, 3.2 + i * 0.22), new Color(0.01, 0.01, 0.03, 0.08), 10), [0.01, 0.01, 0.03, 0.08]);
    }

    // Stars for the night sky: small dots that twinkle.
    for (let i = 0; i < 70; i++) {
      const go = new GameObject(`Star ${i}`, { position: new Vector3((Math.random() - 0.5) * 2 * (view.half + 1), -3.2 + Math.random() * 8.2, -1.5) });
      go.enabled = false;
      this.stars.push({ line: fill(go, Shapes.circle(Vector2.zero, 0.03 + Math.random() * 0.05, 8), new Color(1, 1, 0.92, 0)), alpha: 0.5 + Math.random() * 0.5, phase: Math.random() * 6.28 });
    }
  }

  /** What the spotlight is made of, so it can fade away. */
  private light(line: LineRenderer2D, color: Rgba): LineRenderer2D {
    this.lights.push({ line, color });
    return line;
  }

  /**
   * The spotlight: its colour (0 warm .. 1 a blue tractor beam), how open it is (1 a full cone,
   * near 0 a thin column), where it hangs from (the top of the stage by default, or the bottom
   * of a saucer) and how far down it reaches (1 to the floor, 0 absorbed into where it hangs).
   */
  setBeam(blue: number, width: number, from?: BeamFrom, reach: number = 1): void {
    const fx = from !== undefined ? from.x : 0;
    const fy = from !== undefined ? from.y : 6;
    const top = from !== undefined ? from.top : 0.7;
    const w = Math.max(0.0001, width);
    const r = Math.max(0.0001, reach);
    const bottom = fy + (GROUND - fy) * r;
    const half = 5.4 * w * r;
    this.beamLine.points = [new Vector2(fx - top, fy), new Vector2(fx + top, fy), new Vector2(fx + half, bottom), new Vector2(fx - half, bottom)];
    this.beamLine.fillColor = new Color(mixn(1, 0.35, blue), mixn(0.92, 0.7, blue), mixn(0.68, 1, blue), mixn(0.075, 0.2, blue));
    this.pool.transform.position = new Vector3(fx, GROUND, -1);
    this.pool.transform.localScale = new Vector3(w * r, 1, 1);
    this.poolLine.fillColor = new Color(mixn(1, 0.35, blue), mixn(0.92, 0.7, blue), mixn(0.68, 1, blue), mixn(0.2, 0.32, blue) * Math.min(1, Math.max(0, (r - 0.9) * 10)));
  }

  /** 0 is the spotlit stage, 1 a night sky with stars and no spotlight; `t` is the clock, for twinkling. */
  setNight(level: number, t: number): void {
    if (level <= 0.001 && this.shown === 0) return;
    this.shown = level <= 0.001 ? 0 : level;
    const day = 1 - level;
    for (const l of this.lights) l.line.fillColor = new Color(l.color[0]!, l.color[1]!, l.color[2]!, l.color[3]! * day);
    this.camera.backgroundColor = new Color(
      DAY_SKY.r + (NIGHT_SKY.r - DAY_SKY.r) * level,
      DAY_SKY.g + (NIGHT_SKY.g - DAY_SKY.g) * level,
      DAY_SKY.b + (NIGHT_SKY.b - DAY_SKY.b) * level,
      1,
    );
    for (const s of this.stars) {
      s.line.gameObject.enabled = level > 0.001;
      s.line.fillColor = new Color(1, 1, 0.92, level * s.alpha * (0.65 + 0.35 * Math.sin(t * 2.2 + s.phase)));
    }
  }
}

export function buildStage(camera: Camera): Stage {
  return new Stage(camera);
}

const PPM = 36;
const Z = 1.5;

/** One half of the curtain: its root and which side it is on. */
interface Half {
  root: GameObject;
  side: number;
}

export class Curtain {
  /** 0 closed .. 1 drawn right back. */
  open: number = 0;
  private halves: Half[] = [];
  private valance: GameObject;
  /** The half-width the panels were made for; other window shapes stretch them. */
  private built: number = view.half;

  constructor() {
    for (let k = 0; k < 2; k++) {
      const side = k === 0 ? -1 : 1;
      const root = new GameObject(side < 0 ? "Curtain left" : "Curtain right", { position: new Vector3(side * view.half, 0, Z) });
      const w = view.half + 0.3;
      // The art spans from the outer edge inwards (a little past the middle), so scaling the
      // root gathers it towards the edge.
      const art = new GameObject("Drape", { parent: root, position: new Vector3((-side * w) / 2, 0, 0) });
      const panel = art.addComponent(WorldPanel, { width: Math.round(w * PPM), height: 11 * PPM, pixelsPerMeter: PPM, resolution: 2, interactive: false });
      createRoot(panel).render(<Drape side={side < 0 ? "l" : "r"} />);
      this.halves.push({ root, side });
    }
    // The valance's scallops are fixed pixels, so a bigger pixels-per-metre makes the whole thing smaller.
    this.valance = new GameObject("Valance", { position: new Vector3(0, 5 - 100 / 72 / 2, Z + 0.2) });
    const width = Math.round(view.half * 2 * 72);
    const panel = this.valance.addComponent(WorldPanel, { width, height: 100, pixelsPerMeter: 72, resolution: 1.5, interactive: false });
    createRoot(panel).render(<Valance width={width} />);
    this.set(0);
  }

  set(o: number): void {
    this.open = o;
    const fit = view.half / this.built;
    const s = (1 - GATHER * o) * fit;
    for (const h of this.halves) {
      h.root.transform.position = new Vector3(h.side * view.half, 0, Z);
      h.root.transform.localScale = new Vector3(s, 1, 1);
    }
    this.valance.transform.localScale = new Vector3(fit, 1, 1);
  }

  /** Eases towards `target`, `rate` being per second. */
  approach(target: number, rate: number, dt: number): void {
    this.set(this.open + (target - this.open) * Math.min(1, rate * dt));
  }
}
