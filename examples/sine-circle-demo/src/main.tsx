// Sine Circle: a closed curve whose radius wobbles like a sine wave around a circle, ported
// from MiniGameEngine's SineDemo. The engine draws it with a LineRenderer2D (thick strokes drawn by Rust) and moves,
// sizes, rotates and recolours it with transitions (`tween.ts`). Keys 1-4 pick a sine formula
// (left click cycles them), right click flips the background, Space pauses, F5 shows the
// centre point, F11 toggles fullscreen. The HUD is text only, as the original was. The music
// ("Oldskool" by Of Far Different Nature, CC0; see assets/CREDITS.md) loops and plays while
// it runs.
//
// The original worked in form pixels (1008 x 729); the same numbers are kept here and
// converted to world units, so the curve looks the same at any window size.
import {
  audio,
  Camera,
  Color,
  Component,
  GameObject,
  LineRenderer2D,
  input,
  property,
  QualitySettings,
  Screen,
  Time,
  Vector3,
  register,
  type Playback,
  type Sound,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

import { Easings, lerpNumber, Tween } from "./tween.ts";

// ---- Geometry: form pixels, as in the original ----

const FORM_W = 1008;
const FORM_H = 729;
const CENTER = { x: FORM_W / 2, y: FORM_H / 2 };
/** The whole form height spans 10 world units (the camera's orthographic size is 5). */
const UNITS_PER_PX = 10 / FORM_H;

/** The pen, in form pixels (the original's was 1; a little more reads better on big screens). */
const PEN_WIDTH = 2;

/** Spline samples between each pair of the 361 points (about 1,100 segments in all). */
const SPLINE_STEPS = 3;

type SineType = "pi" | "normal" | "doubleSine" | "doubleCos";

/** The four formulas, in the order left click cycles them; fast or slow transitions. */
const SINE_TYPES: { type: SineType; fast: boolean }[] = [
  { type: "pi", fast: true },
  { type: "normal", fast: true },
  { type: "doubleSine", fast: false },
  { type: "doubleCos", fast: false },
];

const SINE_TYPE_ORDER: SineType[] = SINE_TYPES.map((t) => t.type);

/** Labels for the picker; the two double-wave curves are the original's, bugs and all. */
const SINE_NAMES: Record<SineType, string> = {
  pi: "Pi sine",
  normal: "Normal sine",
  doubleSine: "Double sine",
  doubleCos: "Double cos",
};

const hsl = (h: number, s: number, l: number): Color => {
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const channel = (t: number) => {
    const u = ((t % 1) + 1) % 1;
    if (u < 1 / 6) return p + (q - p) * 6 * u;
    if (u < 1 / 2) return q;
    if (u < 2 / 3) return p + (q - p) * (2 / 3 - u) * 6;
    return p;
  };
  return new Color(channel(h + 1 / 3), channel(h), channel(h - 1 / 3), 1);
};

const lerpColor = (a: Color, b: Color, t: number) =>
  new Color(lerpNumber(a.r, b.r, t), lerpNumber(a.g, b.g, t), lerpNumber(a.b, b.b, t), 1);

const rand = (min: number, max: number) => min + Math.random() * (max - min);

// ---- The curve ----

/** A sine circle in form pixels: its radius wobbles by `depth` at `frequency` around `center`. */
class SineCircle {
  radius = FORM_H / 3;
  frequency = 40;
  depth = 40;
  /** Degrees. */
  rotation = 0;
  center = { x: CENTER.x, y: CENTER.y };
  type: SineType = "normal";
  maxDegrees = 360;
  color = Color.white;

  /** The point for one degree, in form pixels (y down, as the original's WinForms space). */
  private at(d: number): { x: number; y: number } {
    const rad = (d * Math.PI) / 180;
    const rot = (this.rotation * Math.PI) / 180;
    const R = this.radius;
    const f = this.frequency;
    const k = this.depth;
    let tr: number;
    switch (this.type) {
      case "normal": {
        const r = R + k * Math.sin(f * (rad + rot));
        return { x: r * Math.cos(rad) + this.center.x, y: r * Math.sin(rad) + this.center.y };
      }
      case "pi":
        tr = R + f * Math.cos(Math.PI * rad) * Math.cos(Math.PI + k * rad);
        break;
      case "doubleSine":
        tr = R + f * Math.sin(k * d + d) * Math.sin(k * d - d);
        break;
      case "doubleCos":
        tr = R + f * Math.cos(k * d + d) * Math.sin(k * d - d);
        break;
    }
    return { x: this.center.x + tr * Math.cos(rad + rot), y: this.center.y + tr * Math.sin(rad + rot) };
  }

  /**
   * The closed curve in world units, built as the original did: a point every degree (0 to
   * 360), joined by a smooth closed curve. GDI+'s `DrawClosedCurve` is a cardinal spline at
   * tension 0.5, which is a Catmull-Rom spline; each segment is sampled `SPLINE_STEPS` times.
   */
  points(): { x: number; y: number }[] {
    const raw: { x: number; y: number }[] = [];
    for (let d = 0; d <= this.maxDegrees; d++) raw.push(this.at(d));
    const n = raw.length;
    const out: { x: number; y: number }[] = [];
    for (let i = 0; i < n; i++) {
      const p0 = raw[(i - 1 + n) % n]!;
      const p1 = raw[i]!;
      const p2 = raw[(i + 1) % n]!;
      const p3 = raw[(i + 2) % n]!;
      for (let k = 0; k < SPLINE_STEPS; k++) {
        const t = k / SPLINE_STEPS;
        const t2 = t * t;
        const t3 = t2 * t;
        const x = 0.5 * (2 * p1.x + (p2.x - p0.x) * t + (2 * p0.x - 5 * p1.x + 4 * p2.x - p3.x) * t2 + (3 * p1.x - p0.x - 3 * p2.x + p3.x) * t3);
        const y = 0.5 * (2 * p1.y + (p2.y - p0.y) * t + (2 * p0.y - 5 * p1.y + 4 * p2.y - p3.y) * t2 + (3 * p1.y - p0.y - 3 * p2.y + p3.y) * t3);
        out.push({ x: (x - CENTER.x) * UNITS_PER_PX, y: (CENTER.y - y) * UNITS_PER_PX });
      }
    }
    return out;
  }
}

// ---- The demo ----

/** The scene: the camera, the curve and its transitions, the music and the pause state. */
class SineDemo extends Component {
  private camera!: Camera;
  private sine!: SineCircle;
  private showCentre = false;
  private curve!: LineRenderer2D;
  private centre!: LineRenderer2D;
  private music: Sound | null = null;
  private playback: Playback | null = null;
  private typeIndex = -1;
  private light = true;

  private radius!: Tween<number>;
  private frequency!: Tween<number>;
  private depth!: Tween<number>;
  private position!: Tween<{ x: number; y: number }>;
  private rotation!: Tween<number>;
  private colour!: Tween<Color>;
  private blink = new Tween<number>({ start: 0, lerp: lerpNumber, pick: () => 1, duration: 0.1, loop: true, pingPong: true, enabled: true });

  private paused = true;
  private statsTimer = 0;
  private fps = 60;

  override onAwake(): void {
    demo = this;
    const cam = new GameObject("Main Camera", { position: new Vector3(0, 0, 10) });
    this.camera = cam.addComponent(Camera, { orthographic: true, orthographicSize: 5, backgroundColor: Color.white });

    this.sine = new SineCircle();
    this.curve = new GameObject("Sine Circle").addComponent(LineRenderer2D, { width: PEN_WIDTH * UNITS_PER_PX, loop: true });
    this.centre = new GameObject("Centre").addComponent(LineRenderer2D, {
      width: PEN_WIDTH * UNITS_PER_PX,
      color: Color.hex("#ff0000"),
      loop: true,
    });
    this.music = audio.load("assets/oldskool.mp3", { stream: true });

    this.radius = new Tween<number>({
      start: 50,
      lerp: lerpNumber,
      pick: () => rand(20, FORM_H / 3),
      easing: Easings.easeIn,
      loop: true,
      pingPong: true,
      enabled: true,
    });
    this.frequency = new Tween<number>({ start: 24, lerp: lerpNumber, pick: () => rand(0, 85), easing: Easings.easeIn, loop: true, pingPong: true, enabled: true });
    this.depth = new Tween<number>({ start: 25, lerp: lerpNumber, pick: () => rand(0, 80), easing: Easings.easeIn, loop: true, pingPong: true, enabled: true });
    this.position = new Tween<{ x: number; y: number }>({
      start: { ...CENTER },
      lerp: (a, b, t) => ({ x: lerpNumber(a.x, b.x, t), y: lerpNumber(a.y, b.y, t) }),
      pick: () => ({ x: rand(FORM_W / 4, (FORM_W / 4) * 3), y: rand(FORM_H / 4, (FORM_H / 4) * 3) }),
      easing: Easings.easeIn,
      loop: true,
      enabled: true,
    });
    this.rotation = new Tween<number>({
      start: 0,
      lerp: lerpNumber,
      pick: () => rand(0, 360),
      easing: Easings.easeIn,
      loop: true,
      pingPong: true,
      halfDuration: true,
      enabled: true,
    });
    this.colour = new Tween<Color>({
      start: hsl(0, 1, 0.3),
      lerp: lerpColor,
      pick: () => hsl(Math.random(), 1, this.light ? 0.3 : 0.5),
      duration: 5,
      easing: Easings.easeInOut,
      loop: true,
      pingPong: true,
      enabled: true,
    });

    this.advanceType();
    this.setPaused(true);
    this.applyFrame();
    notify();
  }

  /** Left click: the next sine formula. */
  advanceType(): void {
    this.setType((this.typeIndex + 1) % SINE_TYPES.length);
  }

  /** Switches to one of the four formulas, with its transition speeds. */
  setType(index: number): void {
    this.typeIndex = index;
    const { type, fast } = SINE_TYPES[index]!;
    this.sine.type = type;
    if (fast) {
      this.radius.duration = 1;
      this.frequency.duration = 1;
      this.depth.duration = 2;
      this.position.duration = 0.8;
      this.rotation.duration = 3;
    } else {
      this.radius.duration = 10;
      this.frequency.duration = 10;
      this.depth.duration = 10;
      this.rotation.duration = 6;
      this.position.duration = 4;
    }
    notify();
  }

  /** Right click (and the menu): flips the background and the curve's colour range. */
  toggleBackground(): void {
    this.light = !this.light;
    this.camera.backgroundColor = this.light ? Color.white : Color.black;
    notify();
  }

  setPaused(on: boolean): void {
    this.paused = on;
    if (on) {
      this.playback?.pause();
    } else if (this.playback) {
      this.playback.resume();
    } else if (this.music) {
      this.playback = this.music.play({ loop: true });
    }
    notify();
  }

  togglePaused(): void {
    this.setPaused(!this.paused);
  }

  override onUpdate(dt: number): void {
    if (!this.paused) {
      for (const t of [this.radius, this.frequency, this.depth, this.position, this.rotation, this.colour]) t.update(dt);
      this.applyFrame();
    }
    this.blink.update(dt);

    if (input.getKeyDown("Space")) this.togglePaused();
    if (input.getKeyDown("F5")) {
      this.showCentre = !this.showCentre;
      this.draw();
    }
    if (input.getKeyDown("F11")) Screen.fullScreen = !Screen.fullScreen;
    if (input.getKeyDown("KeyV")) QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
    for (let i = 0; i < SINE_TYPES.length; i++) {
      if (input.getKeyDown(`Digit${i + 1}`)) this.setType(i);
    }
    if (input.getMouseButtonDown(0)) this.advanceType();
    if (input.getMouseButtonDown(1)) this.toggleBackground();

    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;
    this.statsTimer += dt;
    if (this.statsTimer >= 0.05) {
      this.statsTimer = 0;
      notify();
    }
  }

  private applyFrame(): void {
    const s = this.sine;
    s.radius = this.radius.value;
    s.frequency = this.frequency.value;
    s.depth = this.depth.value;
    s.rotation = this.rotation.value % 360;
    s.center = { x: this.position.value.x, y: this.position.value.y };
    s.color = this.colour.value;
    this.draw();
  }

  /** Hands the curve (and the centre ring, F5) to their line renderers. */
  private draw(): void {
    this.curve.points = this.sine.points();
    this.curve.color = this.sine.color;
    const c = this.sine.center;
    const r = 6;
    this.centre.points = this.showCentre
      ? Array.from({ length: 24 }, (_, i) => ({
          x: (c.x + Math.cos((i / 24) * Math.PI * 2) * r - CENTER.x) * UNITS_PER_PX,
          y: (CENTER.y - c.y - Math.sin((i / 24) * Math.PI * 2) * r) * UNITS_PER_PX,
        }))
      : [];
  }

  /** What the HUD shows. */
  get stats(): { debug: string; paused: boolean; textColor: string; blink: string; typeIndex: number; fps: number; vsync: boolean } {
    const s = this.sine;
    const c = this.colour.value;
    const css = (x: Color) => `rgb(${Math.round(x.r * 255)}, ${Math.round(x.g * 255)}, ${Math.round(x.b * 255)})`;
    const blinkColour = lerpColor(Color.black, Color.white, this.blink.value);
    return {
      debug: `Sine Generator [Position:${Math.round(s.center.x)},${Math.round(s.center.y)}, Rotation:${s.rotation.toFixed(2)}, Radius:${s.radius.toFixed(2)}, Frequency:${Math.round(s.frequency)}, Depth:${Math.round(s.depth)}, Type:${SINE_TYPES[this.typeIndex]?.type ?? "none"}]`,
      paused: this.paused,
      textColor: css(c),
      blink: css(blinkColour),
      typeIndex: this.typeIndex,
      fps: this.fps,
      vsync: QualitySettings.vSyncCount > 0,
    };
  }
}

// ---- HUD and menu (React) ----

let demo: SineDemo | null = null;
const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

/** Re-renders the HUD whenever the demo says something changed. */
function useStats() {
  const read = () => demo?.stats ?? { debug: "", paused: true, textColor: "#000", blink: "#000", typeIndex: -1, fps: 0, vsync: true };
  const [view, setView] = useState(read);
  useEffect(() => {
    const update = () => setView(read());
    listeners.add(update);
    update();
    return () => {
      listeners.delete(update);
    };
  }, []);
  return view;
}

// Text only, as the original: its stats line, the keys, and a blinking [paused].
const HUD_KEYS = "1-4: sine type · Space: pause · F5: centre · F11: fullscreen · V: vsync";
const HUD_MOUSE = "Left-click: next sine circle · Right-click: switch background";

function Hud() {
  const s = useStats();
  return (
    <box style={{ width: "100%", height: "100%", padding: 14, flexDirection: "column", pointerEvents: "none" }}>
      <text style={{ color: s.textColor, fontSize: 12, fontFamily: "monospace" }}>{s.debug}</text>
      <text style={{ color: "#808080", fontSize: 12, fontFamily: "monospace" }}>{`${SINE_TYPE_ORDER.map((t, i) => `${i + 1} ${SINE_NAMES[t]}${i === s.typeIndex ? "*" : ""}`).join(" · ")}`}</text>

      <text style={{ position: "absolute", right: 14, top: 14, color: "#808080", fontSize: 12, fontFamily: "monospace" }}>{`${Math.round(s.fps)} FPS · vsync ${s.vsync ? "on" : "off"} (V)`}</text>
      <box style={{ position: "absolute", left: 0, right: 0, bottom: 14, alignItems: "center", flexDirection: "column" }}>
        <text style={{ color: "#808080", fontSize: 13 }}>{`${HUD_MOUSE}`}</text>
        <text style={{ color: "#808080", fontSize: 13 }}>{HUD_KEYS}</text>
      </box>

      {s.paused ? (
        <box style={{ position: "absolute", left: 0, right: 0, top: 0, bottom: 0, alignItems: "center", justifyContent: "center" }}>
          <text style={{ color: s.blink, fontSize: 20 }}>[paused] Space to start</text>
        </box>
      ) : null}
    </box>
  );
}

register({ SineDemo });
createRoot().render(<Hud />);

