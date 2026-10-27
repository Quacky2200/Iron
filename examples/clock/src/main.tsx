// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Clocks: twelve faces, switched with the arrows at the sides, the dots at the bottom, the arrow
// keys or , and . —
//
// - Analog: rings that sweep and unwind, spline hands, a ticking second hand (`analog.ts`).
// - Dials: rings of numbers that turn the time under a window (`dials.ts`).
// - Meter: drums that roll like a water meter or a mileage counter (`meter.ts`).
// - Sundial: a garden sundial whose shadow tells the time under a moving sun (`sundial.ts`).
// - Sand timer: an hourglass that runs out each minute and turns over (`hourglass.ts`).
// - Lava lamp: glowing wax blobs that rise, sink, merge and shy from the mouse (`lava.ts`).
// - Underwater: a porthole clock on the sea bed with kelp, bubbles and fish (`underwater.ts`).
// - Chandelier: a classic clock before turning rosettes, candles and crystals (`chandelier.ts`).
// - Rainbow: lots of colour: chasing bands, hue-wheel ticks, confetti (`rainbow.ts`).
// - Morphing: Material-style digits that morph into each other (`morph.ts`).
// - Flip: cards that flip over like an old alarm clock, in Platypus's UI (`flip.tsx`).
// - Alarm clock: red seven-segment digits glowing through bloom (`segments.ts`).
//
// The 2D faces are GameObjects under one root each; switching turns roots on and off.
// V toggles vsync (the FPS readout is top right).

import { AnalogFace } from "./analog.ts";
import { ChandelierFace } from "./chandelier.ts";
import { BG, DIM, Face, label, two } from "./common.ts";
import { DialsFace } from "./dials.ts";
import { FlipClock } from "./flip.tsx";
import { HourglassFace } from "./hourglass.ts";
import { LavaFace } from "./lava.ts";
import { MeterFace } from "./meter.ts";
import { MorphFace } from "./morph.ts";
import { SegmentFace } from "./segments.ts";
import { RainbowFace } from "./rainbow.ts";
import { SundialFace } from "./sundial.ts";
import { UnderwaterFace } from "./underwater.ts";

/** The flip face has no objects: Platypus's UI draws it (see `Hud`). */
class FlipFace extends Face {
  readonly name = "Flip";
  readonly root: GameObject = new GameObject("Flip face");
  update(): void {}
}

class Clocks extends Component {
  faces: Face[] = [];
  current = 0;
  now: DateTime = DateTime.now;
  t = 0;
  private camera!: Camera;
  private fps = 60;
  private fpsLabel!: TextMesh;
  private fpsTimer = 0;

  override onAwake(): void {
    clocks = this;
    this.camera = new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: BG,
    });
    this.fpsLabel = label("FPS", { fontSize: 16, color: DIM }, new Vector3(7.5, 4.75, 0));
    this.faces = [new AnalogFace(), new DialsFace(), new MeterFace(), new SundialFace(), new HourglassFace(), new LavaFace(), new UnderwaterFace(), new ChandelierFace(), new RainbowFace(), new MorphFace(), new FlipFace(), new SegmentFace()];
    this.faces.forEach((f, i) => {
      f.root.enabled = i === this.current;
    });
    this.faces[this.current]!.show(this.camera);
    notify();
  }

  /** Shows face `i` (wrapping round the list). */
  select(i: number): void {
    const n = this.faces.length;
    const next = ((i % n) + n) % n;
    if (next === this.current) return;
    const old = this.faces[this.current]!;
    old.hide(this.camera);
    old.root.enabled = false;
    this.current = next;
    const face = this.faces[next]!;
    face.root.enabled = true;
    face.show(this.camera);
    notify();
  }

  override onUpdate(dt: number): void {
    this.t += dt;
    this.now = DateTime.now;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;
    this.fpsTimer += dt;
    if (this.fpsTimer > 0.25) {
      this.fpsTimer = 0;
      this.fpsLabel.text = `${Math.round(this.fps)} FPS · vsync ${QualitySettings.vSyncCount > 0 ? "on" : "off"} (V)`;
    }
    if (input.getKeyDown("KeyV")) QualitySettings.vSyncCount = QualitySettings.vSyncCount > 0 ? 0 : 1;
    if (input.getKeyDown("ArrowLeft") || input.getKeyDown("Comma")) this.select(this.current - 1);
    if (input.getKeyDown("ArrowRight") || input.getKeyDown("Period")) this.select(this.current + 1);
    this.faces[this.current]!.update(this.now, this.t, dt);
    // The flip face is React: it redraws every frame while it's up; the bar only on changes.
    if (this.faces[this.current] instanceof FlipFace) notify();
  }
}

// ---- React: the face switcher and the flip clock ----

let clocks: Clocks | null = null;
const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

function useClocks() {
  const [, setTick] = useState(0);
  useEffect(() => {
    const update = () => setTick((n) => n + 1);
    listeners.add(update);
    return () => {
      listeners.delete(update);
    };
  }, []);
  return clocks;
}

// Unpicked dots brighten under the pointer (the picked one is white inline). The arrows fade
// to white quickly (100 ms) under the pointer: a white copy over the faint one fades in.
addStyleSheet(`
  .dot { background-color: rgba(255, 255, 255, 0.28); }
  .dot:hover { background-color: rgba(255, 255, 255, 0.65); }
  .dot.on { background-color: #ffffff; }
  .arrow-lit { opacity: 0; transition: opacity 0.1s; }
  .arrow:hover .arrow-lit { opacity: 1; }
`);

/** A chevron at one side of the window, faint until the pointer is over it, then white. */
function Arrow(props: { side: "left" | "right"; onClick: () => void }) {
  const left = props.side === "left";
  const chevron = (stroke: string, lit: boolean) => (
    <svg
      className={lit ? "arrow-lit" : undefined}
      viewBox="0 0 24 48"
      style={lit ? { width: 26, height: 52, position: "absolute", left: 19, top: 44 } : { width: 26, height: 52 }}
    >
      <polyline points={left ? "18,4 6,24 18,44" : "6,4 18,24 6,44"} fill="none" stroke={stroke} strokeWidth={2.5} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
  const inner = (
    <box className="arrow" style={{ width: 64, height: 140, alignItems: "center", justifyContent: "center", pointerEvents: "auto", cursor: "pointer" }} onClick={props.onClick}>
      {chevron("rgba(255, 255, 255, 0.4)", false)}
      {/* The white copy on top, centred over the faint one. */}
      {chevron("#ffffff", true)}
    </box>
  );
  return left ? (
    <box style={{ position: "absolute", top: 0, bottom: 0, left: 8, width: 72, alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>{inner}</box>
  ) : (
    <box style={{ position: "absolute", top: 0, bottom: 0, right: 8, width: 72, alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>{inner}</box>
  );
}

function Hud() {
  const c = useClocks();
  if (!c || c.faces.length === 0) return null;
  const face = c.faces[c.current]!;
  const now = c.now;
  return (
    <box style={{ width: "100%", height: "100%", pointerEvents: "none" }}>
      {face instanceof FlipFace ? (
        <FlipClock hours={two(now.hour)} minutes={two(now.minute)} seconds={two(now.second)} date={now.toString("dddd d MMMM yyyy")} now={c.t} />
      ) : null}
      <Arrow side="left" onClick={() => c.select(c.current - 1)} />
      <Arrow side="right" onClick={() => c.select(c.current + 1)} />
      {/* The face's name, and a dot for each face (click one to go there). */}
      <box style={{ position: "absolute", left: 0, right: 0, bottom: 14, flexDirection: "row", justifyContent: "center", pointerEvents: "none" }}>
        <box style={{ flexDirection: "column", alignItems: "center", gap: 9 }}>
          <text style={{ color: "rgba(255, 255, 255, 0.75)", fontSize: 15, letterSpacing: 1 }}>{face.name}</text>
          <box style={{ flexDirection: "row", gap: 10 }}>
            {c.faces.map((f, i) => (
              <box
                key={f.name}
                className={i === c.current ? "dot on" : "dot"}
                style={{ width: 8, height: 8, borderRadius: 4, pointerEvents: "auto", cursor: "pointer" }}
                onClick={() => c.select(i)}
              />
            ))}
          </box>
        </box>
      </box>
    </box>
  );
}

register({ Clocks });
createRoot().render(<Hud />);
