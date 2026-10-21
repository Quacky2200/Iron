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

const BG = Color.hex("#11161f");
const INK = Color.hex("#e8edf5");
const DIM = Color.hex("#6f7f96");
const ACCENT = Color.hex("#e8a33d");
const HANDLE = Color.hex("#4fc3e8");

/** What a shape is drawn with. */
interface Look {
  points?: Vector2[];
  loop?: boolean;
  width?: number;
  color?: Color;
  fillColor?: Color;
}

/** A LineRenderer2D on its own object. */
function shape(name: string, look: Look, at: Vector3 = Vector3.zero): LineRenderer2D {
  const r = new GameObject(name, { position: at }).addComponent(LineRenderer2D);
  r.width = look.width !== undefined ? look.width : 0.06;
  r.color = look.color !== undefined ? look.color : INK;
  if (look.points !== undefined) r.points = look.points;
  if (look.loop !== undefined) r.loop = look.loop;
  if (look.fillColor !== undefined) r.fillColor = look.fillColor;
  return r;
}

/** What a label looks like. */
interface LabelLook {
  fontSize?: number;
  fontWeight?: number;
  color?: Color;
  alignment?: string;
}

function label(text: string, at: Vector3, look: LabelLook = {}): TextMesh {
  const t = new GameObject(`label: ${text}`, { position: at }).addComponent(TextMesh);
  t.text = text;
  t.fontSize = look.fontSize !== undefined ? look.fontSize : 24;
  t.color = look.color !== undefined ? look.color : DIM;
  if (look.fontWeight !== undefined) t.fontWeight = look.fontWeight;
  if (look.alignment !== undefined) t.alignment = look.alignment;
  return t;
}

interface Dot {
  line: LineRenderer2D;
  shown: boolean;
}

/** Round markers that follow positions; ones not needed are hidden. */
class Dots {
  private items: Dot[] = [];
  private readonly circle: Vector2[];
  private readonly name: string;
  private readonly color: Color;
  private readonly fill: Color;
  constructor(name: string, radius: number, color: Color, fill: Color) {
    this.name = name;
    this.color = color;
    this.fill = fill;
    this.circle = Shapes.circle(Vector2.zero, radius, 20);
  }

  set(positions: Vector3[], colours?: Color[]): void {
    while (this.items.length < positions.length) {
      const line = shape(`${this.name} ${this.items.length}`, { loop: true, width: 0.03, color: this.color, fillColor: this.fill });
      this.items.push({ line, shown: false });
    }
    for (let i = 0; i < this.items.length; i++) {
      const item = this.items[i]!;
      if (i < positions.length) {
        item.line.transform.position = positions[i]!;
        if (!item.shown) item.line.points = this.circle;
        item.shown = true;
        if (colours !== undefined && i < colours.length) item.line.fillColor = colours[i]!;
      } else if (item.shown) {
        item.line.points = [];
        item.shown = false;
      }
    }
  }
}

/** A knot or handle being dragged. */
class Drag {
  kind = "knot";
  index = 0;
}

function nearKnot(s: Spline, m: Vector3, r: number): number {
  for (let i = 0; i < s.count; i++) {
    if (s.knots[i]!.position.distance(m) < r) return i;
  }
  return -1;
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
  private dragging: Drag | null = null;
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
    for (let i = 0; i < 6; i++) {
      const a = (i / 6) * Math.PI * 2;
      this.ribbonBase.push(centre.add(new Vector3(Math.cos(a) * 3.1, Math.sin(a) * 1.7, 0)));
    }
    this.ribbon = new Spline(this.ribbonBase, { closed: true });
    this.ribbonLine = shape("ribbon", { loop: true, width: 0.08, color: Color.hex("#8f7af5"), fillColor: new Color(0.48, 0.35, 0.94, 0.18) });
    const car = shape("car", { points: Shapes.triangle(new Vector2(0.32, 0), new Vector2(-0.22, 0.18), new Vector2(-0.22, -0.18)), loop: true, width: 0.03, fillColor: ACCENT });
    this.car = car.gameObject.addComponent(SplineAnimate, { duration: 7 });
    this.car.spline = this.ribbon;
    this.ball = shape("ball", { points: Shapes.circle(Vector2.zero, 0.15, 20), loop: true, width: 0.03, fillColor: HANDLE }).gameObject;
    label("car: by distance (steady)   ball: by segment (uneven)", new Vector3(-4.5, -0.95, 0), { fontSize: 20 });

    // The wave: seven knots bobbing; its tension swings 0 to 1.
    const waveKnots: Vector3[] = [];
    for (let i = 0; i < 7; i++) waveKnots.push(new Vector3(-8 + i * 1.15, -2.7, 0));
    this.wave = new Spline(waveKnots);
    this.waveLine = shape("wave", { width: 0.07, color: Color.hex("#4fd18b") });
    this.waveDots = new Dots("wave knot", 0.07, INK, INK);
    this.tensionLabel = label("", new Vector3(-4.5, -4.2, 0), { fontSize: 20 });

    // The one to edit.
    this.edit = new Spline([new Vector3(1.6, -1.5, 0), new Vector3(3, 1.5, 0), new Vector3(4.6, -0.5, 0), new Vector3(6, 2, 0), new Vector3(7.4, 0, 0)]);
    this.editLine = shape("editable", { width: 0.07, color: Color.hex("#7fb2ff") });
    for (let i = 0; i < 2; i++) this.handleLines.push(shape(`handle line ${i}`, { width: 0.025, color: HANDLE }));
    this.handleDots = new Dots("handle", 0.09, HANDLE, BG);
    this.knotDots = new Dots("knot", 0.13, INK, BG);
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
    const centre = new Vector3(-4.5, 1.1, 0);
    for (let i = 0; i < this.ribbonBase.length; i++) {
      const base = this.ribbonBase[i]!;
      const out = base.sub(centre).normalized;
      const wobble = Math.sin(t * (0.9 + i * 0.23) + i * 1.7) * 0.45;
      this.ribbon.setPosition(i, base.add(out.scale(wobble)).add(new Vector3(0, Math.sin(t * 0.7 + i) * 0.15, 0)));
    }
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
    const waveAt: Vector3[] = [];
    for (let i = 0; i < this.wave.count; i++) waveAt.push(this.wave.knots[i]!.position);
    this.waveDots.set(waveAt);
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

    if (input.getMouseButtonDown(0) && m.x > 0.3) {
      const k = this.selected >= 0 ? s.knots[this.selected] : undefined;
      if (k && k.mode !== "linear" && k.position.add(s.handleOut(this.selected)).distance(m) < 0.2) {
        this.dragging = new Drag();
        this.dragging.kind = "out";
        this.dragging.index = this.selected;
      } else if (k && k.mode !== "linear" && k.position.add(s.handleIn(this.selected)).distance(m) < 0.2) {
        this.dragging = new Drag();
        this.dragging.kind = "in";
        this.dragging.index = this.selected;
      } else {
        const hit = nearKnot(s, m, 0.25);
        if (hit >= 0) {
          this.selected = hit;
          this.dragging = new Drag();
          this.dragging.index = hit;
        } else {
          const curve = s.nearest(m);
          const index = curve.distance < 0.25 && s.segmentCount > 0 ? s.locate(curve.t).segment + 1 : s.count;
          this.selected = s.insertKnot(index, m);
          this.dragging = new Drag();
          this.dragging.index = this.selected;
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
    if (input.getMouseButtonDown(1)) remove(nearKnot(s, m, 0.25));
    if (input.getKeyDown("Delete") || input.getKeyDown("Backspace")) remove(this.selected);
    if (input.getKeyDown("KeyC")) s.closed = !s.closed;
    if (input.getKeyDown("KeyF")) this.filled = !this.filled;
    if (input.getKeyDown("BracketLeft")) s.tension = Math.max(0, Math.round((s.tension - 0.1) * 10) / 10);
    if (input.getKeyDown("BracketRight")) s.tension = Math.min(1, Math.round((s.tension + 0.1) * 10) / 10);
    if (input.getKeyDown("KeyT") && this.selected >= 0) {
      const k = s.knots[this.selected];
      if (k) s.setMode(this.selected, k.mode === "auto" ? "bezier" : k.mode === "bezier" ? "linear" : "auto");
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
    if (this.filled) this.editLine.fillColor = new Color(0.5, 0.7, 1, 0.22);
    else this.editLine.fillColor = null;
    const knotAt: Vector3[] = [];
    const knotColours: Color[] = [];
    for (let i = 0; i < s.count; i++) {
      knotAt.push(s.knots[i]!.position);
      knotColours.push(i === this.selected ? ACCENT : BG);
    }
    this.knotDots.set(knotAt, knotColours);
    const k = this.selected >= 0 ? s.knots[this.selected] : undefined;
    const handles: Vector3[] = [];
    if (k && k.mode !== "linear") {
      handles.push(k.position.add(s.handleIn(this.selected)));
      handles.push(k.position.add(s.handleOut(this.selected)));
    }
    this.handleDots.set(handles);
    for (let i = 0; i < this.handleLines.length; i++) {
      const line = this.handleLines[i]!;
      line.points = k && i < handles.length ? [new Vector2(k.position.x, k.position.y), new Vector2(handles[i]!.x, handles[i]!.y)] : [];
    }
    const mode = k ? `#${this.selected} ${k.mode}` : "none";
    this.status.text = `${s.count} knots · ${s.closed ? "closed" : "open"} · tension ${s.tension.toFixed(1)} · length ${s.length.toFixed(2)} · selected ${mode}`;
  }
}

register({ SplineDemo });
