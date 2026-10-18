// Splines: Iron's `Spline` in two halves.
//
// Left, splines that move: a closed ribbon whose knots drift, ridden by a car that goes by
// distance (`SplineAnimate`: a steady speed) and a ball that goes by each segment's own
// parameter (fast on long segments, slow on short ones); and a wave whose tension swings from
// straight lines (0) to loose curves (1).
//
// Right, one to edit: drag a knot or a handle; click empty space to add a knot at the end, or
// click near the curve to insert one there; right-click a knot (or Delete) to remove it.
// C opens or closes it, T cycles the selected knot's mode (auto, bezier, linear), [ and ]
// change the tension, F fills it. Dragging an auto knot's handle makes it a bezier knot with
// mirrored handles; hold Alt to move one handle on its own. V toggles vsync.
import {
  Camera,
  Color,
  Component,
  GameObject,
  input,
  LineRenderer2D,
  QualitySettings,
  register,
  Shapes,
  Spline,
  SplineAnimate,
  TextMesh,
  Vector2,
  Vector3,
} from "@iron/engine";

const BG = Color.hex("#11161f");
const INK = Color.hex("#e8edf5");
const DIM = Color.hex("#6f7f96");
const ACCENT = Color.hex("#e8a33d");
const HANDLE = Color.hex("#4fc3e8");

/** A LineRenderer2D on its own object. */
function shape(name: string, look: Partial<LineRenderer2D>, at = Vector3.zero): LineRenderer2D {
  return new GameObject(name, { position: at }).addComponent(LineRenderer2D, { width: 0.06, color: INK, ...look });
}

function label(text: string, at: Vector3, look: Partial<TextMesh> = {}): TextMesh {
  return new GameObject(`label: ${text}`, { position: at }).addComponent(TextMesh, { text, fontSize: 24, color: DIM, ...look });
}

/** Round markers that follow positions; ones not needed are hidden. */
class Dots {
  private items: { line: LineRenderer2D; shown: boolean }[] = [];
  private readonly circle: Vector2[];
  constructor(
    private readonly name: string,
    radius: number,
    private readonly look: Partial<LineRenderer2D>,
  ) {
    this.circle = Shapes.circle(Vector2.zero, radius, 20);
  }

  set(positions: readonly Vector3[], colours?: readonly Color[]): void {
    while (this.items.length < positions.length) {
      this.items.push({ line: shape(`${this.name} ${this.items.length}`, { loop: true, width: 0.03, ...this.look }), shown: false });
    }
    this.items.forEach((item, i) => {
      const p = positions[i];
      if (p) {
        item.line.transform.position = p;
        if (!item.shown) item.line.points = this.circle;
        item.shown = true;
        const c = colours?.[i];
        if (c) item.line.fillColor = c;
      } else if (item.shown) {
        item.line.points = [];
        item.shown = false;
      }
    });
  }
}

class SplineDemo extends Component {
  private camera!: Camera;
  private t = 0;
  private fps = 60;
  private fpsLabel!: TextMesh;
  private fpsTimer = 0;

  // Moving: the ribbon and its riders, and the wave.
  private ribbon!: Spline;
  private ribbonBase: Vector3[] = [];
  private ribbonLine!: LineRenderer2D;
  private car!: SplineAnimate;
  private ball!: GameObject;
  private wave!: Spline;
  private waveLine!: LineRenderer2D;
  private waveDots!: Dots;
  private tensionLabel!: TextMesh;
  private tensionTimer = 0;

  // Editing.
  private edit!: Spline;
  private editLine!: LineRenderer2D;
  private knotDots!: Dots;
  private handleDots!: Dots;
  private handleLines: LineRenderer2D[] = [];
  private status!: TextMesh;
  private selected = -1;
  private dragging: { kind: "knot" | "in" | "out"; index: number } | null = null;
  private filled = false;
  private drawn = -1;

  override onAwake(): void {
    this.camera = new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: BG,
    });
    shape("divider", { points: [new Vector2(0, -4.6), new Vector2(0, 4.6)], width: 0.02, color: Color.hex("#2a3442") });
    label("Splines that move", new Vector3(-4.5, 4.3, 0), { fontSize: 34, fontWeight: 700, color: INK });
    label("A spline to edit", new Vector3(4.5, 4.3, 0), { fontSize: 34, fontWeight: 700, color: INK });
    this.fpsLabel = label("", new Vector3(7.5, 4.75, 0), { fontSize: 16 });

    // The ribbon: six knots round an ellipse; they drift in onUpdate.
    const centre = new Vector3(-4.5, 1.1, 0);
    this.ribbonBase = Array.from({ length: 6 }, (_, i) => {
      const a = (i / 6) * Math.PI * 2;
      return centre.add(new Vector3(Math.cos(a) * 3.1, Math.sin(a) * 1.7, 0));
    });
    this.ribbon = new Spline(this.ribbonBase, { closed: true });
    this.ribbonLine = shape("ribbon", { loop: true, width: 0.08, color: Color.hex("#8f7af5"), fillColor: new Color(0.48, 0.35, 0.94, 0.18) });
    const car = shape("car", { points: Shapes.triangle({ x: 0.32, y: 0 }, { x: -0.22, y: 0.18 }, { x: -0.22, y: -0.18 }), loop: true, width: 0.03, fillColor: ACCENT });
    this.car = car.gameObject.addComponent(SplineAnimate, { duration: 7 });
    this.car.spline = this.ribbon;
    this.ball = shape("ball", { points: Shapes.circle(Vector2.zero, 0.15, 20), loop: true, width: 0.03, fillColor: HANDLE }).gameObject;
    label("car: by distance (steady)   ball: by segment (uneven)", new Vector3(-4.5, -0.95, 0), { fontSize: 20 });

    // The wave: seven knots bobbing; its tension swings 0 to 1.
    this.wave = new Spline(Array.from({ length: 7 }, (_, i) => new Vector3(-8 + i * 1.15, -2.7, 0)));
    this.waveLine = shape("wave", { width: 0.07, color: Color.hex("#4fd18b") });
    this.waveDots = new Dots("wave knot", 0.07, { color: INK, fillColor: INK });
    this.tensionLabel = label("", new Vector3(-4.5, -4.2, 0), { fontSize: 20 });

    // The one to edit.
    this.edit = new Spline([new Vector3(1.6, -1.5, 0), new Vector3(3, 1.5, 0), new Vector3(4.6, -0.5, 0), new Vector3(6, 2, 0), new Vector3(7.4, 0, 0)]);
    this.editLine = shape("editable", { width: 0.07, color: Color.hex("#7fb2ff") });
    this.handleLines = [0, 1].map((i) => shape(`handle line ${i}`, { width: 0.025, color: HANDLE }));
    this.handleDots = new Dots("handle", 0.09, { color: HANDLE, fillColor: BG });
    this.knotDots = new Dots("knot", 0.13, { color: INK, fillColor: BG });
    this.status = label("", new Vector3(4.5, -3.55, 0), { fontSize: 20, color: INK });
    label(
      "Drag knots and handles. Click to add a knot; near the curve to insert one.\nRight-click or Delete removes. C close · T mode · [ ] tension · F fill · Alt: one handle",
      new Vector3(4.5, -4.3, 0),
      { fontSize: 17, alignment: "center" },
    );
  }

  override onUpdate(dt: number): void {
    this.t += dt;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;
    this.fpsTimer += dt;
    if (this.fpsTimer > 0.25) {
      this.fpsTimer = 0;
      this.fpsLabel.text = `${Math.round(this.fps)} FPS · vsync ${QualitySettings.vSyncCount > 0 ? "on" : "off"} (V)`;
    }
    if (input.getKeyDown("KeyV")) QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
    this.moveRibbon();
    this.moveWave(dt);
    this.editInput();
    this.drawEditable();
  }

  private moveRibbon(): void {
    const t = this.t;
    this.ribbonBase.forEach((base, i) => {
      const centre = new Vector3(-4.5, 1.1, 0);
      const out = base.sub(centre).normalized;
      const wobble = Math.sin(t * (0.9 + i * 0.23) + i * 1.7) * 0.45;
      this.ribbon.setPosition(i, base.add(out.scale(wobble)).add(new Vector3(0, Math.sin(t * 0.7 + i) * 0.15, 0)));
    });
    this.ribbonLine.points = this.ribbon.sample(20);
    this.car.place();
    // The ball goes by segment: the same time per segment, however long it is.
    const x = ((this.t / 7) * this.ribbon.segmentCount) % this.ribbon.segmentCount;
    this.ball.transform.position = this.ribbon.segmentPoint(Math.floor(x), x - Math.floor(x));
  }

  private moveWave(dt: number): void {
    const t = this.t;
    this.wave.tension = 0.5 + 0.5 * Math.sin(t * 0.6);
    for (let i = 0; i < this.wave.count; i++) {
      this.wave.setPosition(i, new Vector3(-8 + i * 1.15, -2.7 + Math.sin(t * 1.6 + i * 0.9) * 0.6, 0));
    }
    this.waveLine.points = this.wave.sample(16);
    this.waveDots.set(this.wave.knots.map((k) => k.position));
    this.tensionTimer += dt;
    if (this.tensionTimer > 0.1) {
      this.tensionTimer = 0;
      this.tensionLabel.text = `tension ${this.wave.tension.toFixed(2)}`;
    }
  }

  /** The mouse on the z = 0 plane. */
  private mouse(): Vector3 {
    const m = input.mousePosition;
    const p = this.camera.screenToWorldPoint({ x: m.x, y: m.y, z: 10 });
    return new Vector3(p.x, p.y, 0);
  }

  private editInput(): void {
    const s = this.edit;
    const m = this.mouse();
    const near = (p: Vector3, r: number) => p.distance(m) < r;

    if (input.getMouseButtonDown(0) && m.x > 0.3) {
      const k = this.selected >= 0 ? s.knots[this.selected] : undefined;
      if (k && k.mode !== "linear" && near(k.position.add(s.handleOut(this.selected)), 0.2)) {
        this.dragging = { kind: "out", index: this.selected };
      } else if (k && k.mode !== "linear" && near(k.position.add(s.handleIn(this.selected)), 0.2)) {
        this.dragging = { kind: "in", index: this.selected };
      } else {
        const hit = s.knots.findIndex((kk) => near(kk.position, 0.25));
        if (hit >= 0) {
          this.selected = hit;
          this.dragging = { kind: "knot", index: hit };
        } else {
          const curve = s.nearest(m);
          const index = curve.distance < 0.25 && s.segmentCount > 0 ? s.locate(curve.t).segment + 1 : s.count;
          this.selected = s.insertKnot(index, m);
          this.dragging = { kind: "knot", index: this.selected };
        }
      }
    }
    if (this.dragging && input.getMouseButton(0)) {
      const d = this.dragging;
      const k = s.knots[d.index];
      if (k && d.kind === "knot") s.setPosition(d.index, m);
      else if (k) {
        // Mirrored handles unless Alt is held.
        const free = input.getKey("AltLeft") || input.getKey("AltRight");
        const moved = m.sub(k.position);
        const other = d.kind === "out" ? s.handleIn(d.index) : s.handleOut(d.index);
        const mirror = free ? other : moved.scale(-1);
        if (d.kind === "out") s.setTangents(d.index, mirror, moved);
        else s.setTangents(d.index, moved, mirror);
      }
    }
    if (!input.getMouseButton(0)) this.dragging = null;

    const remove = (i: number) => {
      if (s.count <= 2 || i < 0) return;
      s.removeKnot(i);
      this.selected = Math.min(this.selected, s.count - 1);
    };
    if (input.getMouseButtonDown(1)) remove(s.knots.findIndex((k) => near(k.position, 0.25)));
    if (input.getKeyDown("Delete") || input.getKeyDown("Backspace")) remove(this.selected);
    if (input.getKeyDown("KeyC")) s.closed = !s.closed;
    if (input.getKeyDown("KeyF")) this.filled = !this.filled;
    if (input.getKeyDown("BracketLeft")) s.tension = Math.max(0, Math.round((s.tension - 0.1) * 10) / 10);
    if (input.getKeyDown("BracketRight")) s.tension = Math.min(1, Math.round((s.tension + 0.1) * 10) / 10);
    if (input.getKeyDown("KeyT") && this.selected >= 0) {
      const modes = ["auto", "bezier", "linear"] as const;
      const k = s.knots[this.selected];
      if (k) s.setMode(this.selected, modes[(modes.indexOf(k.mode) + 1) % modes.length]!);
    }
  }

  private drawEditable(): void {
    const s = this.edit;
    // Redraw only when the spline or the selection changed.
    const key = s.version * 1000 + this.selected + (this.filled ? 0.5 : 0);
    if (key === this.drawn) return;
    this.drawn = key;
    this.editLine.points = s.sample(24);
    this.editLine.loop = s.closed;
    this.editLine.fillColor = this.filled ? new Color(0.5, 0.7, 1, 0.22) : null;
    this.knotDots.set(
      s.knots.map((k) => k.position),
      s.knots.map((_, i) => (i === this.selected ? ACCENT : BG)),
    );
    const k = this.selected >= 0 ? s.knots[this.selected] : undefined;
    const handles = k && k.mode !== "linear" ? [k.position.add(s.handleIn(this.selected)), k.position.add(s.handleOut(this.selected))] : [];
    this.handleDots.set(handles);
    this.handleLines.forEach((line, i) => {
      line.points = handles[i] && k ? [k.position, handles[i]!] : [];
    });
    const mode = k ? `#${this.selected} ${k.mode}` : "none";
    this.status.text = `${s.count} knots · ${s.closed ? "closed" : "open"} · tension ${s.tension.toFixed(1)} · length ${s.length.toFixed(2)} · selected ${mode}`;
  }
}

register({ SplineDemo });
