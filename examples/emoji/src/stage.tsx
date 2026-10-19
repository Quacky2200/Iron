// The set: floor, spotlight beam and pool of light, drifting dust, the dark around the spot,
// and the red curtain. The curtain halves are WorldPanels (so they sit in front of the
// emoji); drawing them back squashes their folds towards the sides.
import { Color, Component, GameObject, LineRenderer2D, Shapes, Vector2, Vector3, WorldPanel } from "@iron/engine";
import { createRoot } from "@iron/react";
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
  const [corner, rim] = [rect[0]!, ring[0]!];
  return [...rect, corner, ...ring, rim, corner];
}

const fill = (go: GameObject, points: Vector2[], color: Color, extra: Partial<LineRenderer2D> = {}) =>
  go.addComponent(LineRenderer2D, { points, fillColor: color, width: 0, loop: true, ...extra });

export function buildStage(): void {
  const floor = new GameObject("Floor", { position: new Vector3(0, GROUND - 1.1, 0.2) });
  fill(floor, Shapes.rectangle(-20, -1.2, 40, 2.4), Color.hex("#131724"));
  const beam = new GameObject("Beam", { position: new Vector3(LIGHT.x, 0, -1) });
  fill(beam, [new Vector2(-0.7, 6), new Vector2(0.7, 6), new Vector2(5.4, GROUND), new Vector2(-5.4, GROUND)], new Color(1, 0.92, 0.68, 0.075));
  const pool = new GameObject("Pool", { position: new Vector3(LIGHT.x, GROUND, -1) });
  fill(pool, Shapes.ellipse(Vector2.zero, 5.6, 0.75, 56), new Color(1, 0.92, 0.68, 0.2));

  for (let i = 0; i < 28; i++) {
    const home = (Math.random() - 0.5) * 8;
    const mote = new GameObject(`Mote ${i}`, { position: new Vector3(home, GROUND + Math.random() * 9, -0.5) });
    fill(mote, Shapes.circle(Vector2.zero, 0.025 + Math.random() * 0.03, 8), new Color(1, 0.95, 0.8, 0.5));
    mote.addComponent(Mote).home = home;
  }

  // Rings of dark, each with a slightly wider hole, add up to a soft edge.
  const rings = 28;
  for (let i = 0; i < rings; i++) {
    const dark = new GameObject(`Dark ${i}`, { position: new Vector3(LIGHT.x, LIGHT.y, 3) });
    fill(dark, framed(new Vector2(30, 20), Vector2.zero, 3.2 + i * 0.22), new Color(0.01, 0.01, 0.03, 0.08), { sortingOrder: 10 });
  }
}

const PPM = 36;
const Z = 1.5;

export class Curtain {
  /** 0 closed .. 1 drawn right back. */
  open = 0;
  private halves: [GameObject, number][] = [];
  private valance: GameObject;
  /** The half-width the panels were made for; other window shapes stretch them. */
  private built = view.half;

  constructor() {
    for (const side of [-1, 1] as const) {
      const root = new GameObject(side < 0 ? "Curtain left" : "Curtain right", { position: new Vector3(side * view.half, 0, Z) });
      const w = view.half + 0.3;
      // The art spans from the outer edge inwards (a little past the middle), so scaling the
      // root gathers it towards the edge.
      const art = new GameObject("Drape", { parent: root, position: new Vector3((-side * w) / 2, 0, 0) });
      const panel = art.addComponent(WorldPanel, { width: Math.round(w * PPM), height: 11 * PPM, pixelsPerMeter: PPM, resolution: 2, interactive: false });
      createRoot(panel).render(<Drape side={side < 0 ? "l" : "r"} />);
      this.halves.push([root, side]);
    }
    this.valance = new GameObject("Valance", { position: new Vector3(0, 5 - 100 / 48 / 2, Z + 0.2) });
    const width = Math.round(view.half * 2 * 48);
    const panel = this.valance.addComponent(WorldPanel, { width, height: 100, pixelsPerMeter: 48, resolution: 1.5, interactive: false });
    createRoot(panel).render(<Valance width={width} />);
    this.set(0);
  }

  set(o: number): void {
    this.open = o;
    const fit = view.half / this.built;
    const s = (1 - GATHER * o) * fit;
    for (const [h, side] of this.halves) {
      h.transform.position = new Vector3(side * view.half, 0, Z);
      h.transform.localScale = new Vector3(s, 1, 1);
    }
    this.valance.transform.localScale = new Vector3(fit, 1, 1);
  }

  /** Eases towards `target`, `rate` being per second. */
  approach(target: number, rate: number, dt: number): void {
    this.set(this.open + (target - this.open) * Math.min(1, rate * dt));
  }
}
