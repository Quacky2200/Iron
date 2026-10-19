// Emoji parade: one random emoji at a time hops across the stage from right to left and
// lingers in a spotlight, where it takes a deep bow. Each emoji is a Noto SVG shown by a React
// WorldPanel on a pivot at its feet, so squash and bow are plain scale and rotation of that
// pivot, and the vector art stays sharp at 10em. The spotlight is soft dark rings with a hole,
// drawn over the stage; the caption is screen UI with a CSS pop-in.
import { addStyleSheet, Camera, Color, Component, GameObject, LineRenderer2D, register, Shapes, Vector2, Vector3, WorldPanel } from "@iron/engine";
import { createRoot, Image } from "@iron/react";
import { useEffect, useState } from "react";

// [Noto file (codepoint), name]
const EMOJI: [string, string][] = [
  ["1f600", "Grinning"], ["1f602", "Tears of joy"], ["1f923", "Rolling on the floor"], ["1f60a", "Blushing"], ["1f60d", "Heart eyes"],
  ["1f618", "Blowing a kiss"], ["1f60e", "Cool"], ["1f913", "Nerd"], ["1f633", "Embarrassed"], ["1f605", "Nervous sweat"],
  ["1f62d", "Sobbing"], ["1f621", "Angry"], ["1f92c", "Swearing"], ["1f631", "Screaming"], ["1f976", "Freezing"],
  ["1f975", "Overheated"], ["1f922", "Queasy"], ["1f92e", "Sick"], ["1f634", "Sleeping"], ["1f914", "Thinking"],
  ["1f644", "Eye roll"], ["1f60f", "Smirk"], ["1f973", "Party"], ["1f929", "Star-struck"], ["1f607", "Angel"],
  ["1f92a", "Zany"], ["1f92f", "Mind blown"], ["1f917", "Hugging"], ["1f643", "Upside down"], ["1f608", "Devil"],
  ["1f47b", "Ghost"], ["1f480", "Skull"], ["1f921", "Clown"], ["1f47d", "Alien"], ["1f916", "Robot"],
  ["1f4a9", "Poop"], ["1f435", "Monkey"], ["1f648", "See no evil"], ["1f436", "Dog"], ["1f431", "Cat"],
  ["1f984", "Unicorn"], ["1f438", "Frog"], ["1f437", "Pig"], ["1f52b", "Water pistol"], ["1f525", "Fire"],
  ["1f4af", "One hundred"], ["2764", "Heart"], ["1f44d", "Thumbs up"], ["1f383", "Pumpkin"], ["1f31e", "Sun"],
];

const GROUND = -3.7;
/** World units an emoji stands tall (about half of the screen's 10 units at this camera). */
const HEIGHT = 5.2;
const PANEL = 480;
const HOP = 0.9;
/** Seconds per hop and per trip across the stage. */
const BEAT = 0.8;
const CROSSING = 9;
const EDGE = 14;
const LIGHT = new Vector2(0, 0.2);

addStyleSheet(`
.caption { animation: caption-in 0.45s cubic-bezier(0.2, 1.7, 0.4, 1) both; }
@keyframes caption-in {
  from { opacity: 0; transform: translateY(28px) scale(0.6); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
`);

// The React side only needs to know which emoji is on stage.
const listeners = new Set<(i: number) => void>();
const showEmoji = (i: number) => listeners.forEach((f) => f(i));

function useEmoji(): number {
  const [i, setI] = useState(0);
  useEffect(() => {
    listeners.add(setI);
    return () => void listeners.delete(setI);
  }, []);
  return i;
}

function Face() {
  return <Image src={`assets/emoji/${EMOJI[useEmoji()]![0]}.svg`} style={{ width: PANEL, height: PANEL }} />;
}

function Hud() {
  const i = useEmoji();
  return (
    <box style={{ width: "100%", height: "100%", flexDirection: "column", alignItems: "center", justifyContent: "flex-end", paddingBottom: 26, pointerEvents: "none" }}>
      <text key={i} className="caption" style={{ fontSize: 46, fontWeight: 800, color: "#f6f1e2", letterSpacing: 4, textTransform: "uppercase", textShadow: "0 3px 12px rgba(0,0,0,0.8)" }}>
        {EMOJI[i]![1]}
      </text>
      <text style={{ marginTop: 4, fontSize: 15, color: "#8a93a8", letterSpacing: 6, textTransform: "uppercase" }}>{`Emoji parade · ${EMOJI.length} stars`}</text>
    </box>
  );
}

const smooth = (t: number) => t * t * (3 - 2 * t);

/** A rectangle with a round hole cut out, the hole's rim reached over a zero-width bridge. */
function framed(half: Vector2, hole: Vector2, radius: number): Vector2[] {
  const rect = [new Vector2(-half.x, -half.y), new Vector2(half.x, -half.y), new Vector2(half.x, half.y), new Vector2(-half.x, half.y)];
  const ring = Shapes.circle(hole, radius, 64);
  const [corner, rim] = [rect[0]!, ring[0]!];
  return [...rect, corner, ...ring, rim, corner];
}

class Mote extends Component {
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

class Parade extends Component {
  private actor!: GameObject;
  private shadow!: GameObject;
  private panel!: WorldPanel;
  private clock = 0;
  private last = -1;

  override onAwake(): void {
    new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: Color.hex("#04050a"),
    });
    this.stage();

    this.shadow = new GameObject("Shadow", { position: new Vector3(0, GROUND, 0.1) });
    this.shadow.addComponent(LineRenderer2D, {
      points: Shapes.ellipse(Vector2.zero, 2.4, 0.35, 40),
      fillColor: new Color(0, 0, 0, 0.45),
      width: 0,
      loop: true,
    });

    this.actor = new GameObject("Emoji", { position: new Vector3(EDGE, GROUND, 0) });
    const art = new GameObject("Art", { position: new Vector3(0, HEIGHT / 2, 0), parent: this.actor });
    const panel = this.panel = art.addComponent(WorldPanel, { width: PANEL, height: PANEL, pixelsPerMeter: PANEL / HEIGHT, resolution: 1.5, interactive: false });
    createRoot(panel).render(<Face />);
    createRoot().render(<Hud />);
    this.next();
  }

  /** The beam, the pool of light on the floor, drifting dust and the dark around the spot. */
  private stage(): void {
    const floor = new GameObject("Floor", { position: new Vector3(0, GROUND - 1.1, 0.2) });
    floor.addComponent(LineRenderer2D, {
      points: Shapes.rectangle(-20, -1.2, 40, 2.4),
      fillColor: Color.hex("#131724"),
      width: 0,
      loop: true,
    });
    const beam = new GameObject("Beam", { position: new Vector3(LIGHT.x, 0, -1) });
    beam.addComponent(LineRenderer2D, {
      points: [new Vector2(-0.7, 6), new Vector2(0.7, 6), new Vector2(5.4, GROUND), new Vector2(-5.4, GROUND)],
      fillColor: new Color(1, 0.92, 0.68, 0.075),
      width: 0,
      loop: true,
    });
    const pool = new GameObject("Pool", { position: new Vector3(LIGHT.x, GROUND, -1) });
    pool.addComponent(LineRenderer2D, {
      points: Shapes.ellipse(Vector2.zero, 5.6, 0.75, 56),
      fillColor: new Color(1, 0.92, 0.68, 0.2),
      width: 0,
      loop: true,
    });

    for (let i = 0; i < 28; i++) {
      const home = (Math.random() - 0.5) * 8;
      const mote = new GameObject(`Mote ${i}`, { position: new Vector3(home, GROUND + Math.random() * 9, -0.5) });
      mote.addComponent(LineRenderer2D, {
        points: Shapes.circle(Vector2.zero, 0.025 + Math.random() * 0.03, 8),
        fillColor: new Color(1, 0.95, 0.8, 0.5),
        width: 0,
        loop: true,
      });
      mote.addComponent(Mote).home = home;
    }

    // Rings of dark, each with a slightly wider hole, add up to a soft edge.
    const rings = 28;
    for (let i = 0; i < rings; i++) {
      const dark = new GameObject(`Dark ${i}`, { position: new Vector3(LIGHT.x, LIGHT.y, 3) });
      dark.addComponent(LineRenderer2D, {
        points: framed(new Vector2(30, 20), Vector2.zero, 3.2 + i * 0.22),
        fillColor: new Color(0.01, 0.01, 0.03, 0.08),
        width: 0,
        loop: true,
        sortingOrder: 10,
      });
    }
  }

  /** A different emoji from the last, just off the right edge. */
  private next(): void {
    let i = this.last;
    while (i === this.last) i = Math.floor(Math.random() * EMOJI.length);
    this.last = i;
    showEmoji(i);
    this.clock = 0;
  }

  override onUpdate(dt: number): void {
    this.clock += dt;
    const u = this.clock / CROSSING;
    if (u >= 1) {
      this.next();
      return;
    }
    // Slow down through the middle of the stage, where the light is.
    const along = u + (0.7 * Math.sin(u * Math.PI * 2)) / (Math.PI * 2);
    const x = EDGE - along * EDGE * 2;
    const spot = Math.exp(-((x - LIGHT.x) * (x - LIGHT.x)) / 9);
    // The dark is drawn behind world panels, so the emoji dims itself outside the light.
    const lit = 0.3 + 0.7 * spot;
    this.panel.tint = new Color(lit, lit, lit, 1);

    // One hop: 0..1 in the air, then a short landing.
    const beat = (this.clock / BEAT) % 1;
    const air = 0.72;
    let lift = 0;
    let squash = 0;
    if (beat < air) {
      lift = Math.sin((beat / air) * Math.PI);
    } else {
      // Squash in fast on touchdown, then spring back.
      const land = (beat - air) / (1 - air);
      squash = Math.sin(land * Math.PI) * (1 - land * 0.25);
    }
    // In the light the landings get deeper and become bows.
    const deep = 1 + spot * 0.8;
    const sy = 1 + 0.09 * lift - 0.32 * squash * deep;
    const sx = 1 - 0.05 * lift + 0.2 * squash * deep;
    // Bow towards the way it's going (left) on the floor, lean back in the air.
    const tilt = (14 + 24 * spot) * smooth(squash) - 5 * lift;

    const t = this.actor.transform;
    t.position = new Vector3(x, GROUND + lift * HOP, 0);
    t.localScale = new Vector3(sx, sy, 1);
    t.localEulerAngles = new Vector3(0, 0, tilt);

    // The shadow shrinks and fades as it leaves the floor and spreads when it lands.
    const spread = (1 - 0.35 * lift) * (1 + 0.25 * squash);
    this.shadow.transform.position = new Vector3(x, GROUND, 0.1);
    this.shadow.transform.localScale = new Vector3(spread, spread, 1);
  }
}

register({ Parade, Mote });
