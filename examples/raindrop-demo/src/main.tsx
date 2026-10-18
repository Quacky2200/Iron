// Raindrops: ported from MiniGameEngine's RaindropDemo. Rain lands as rings of soft discs
// that spread out and dissolve (each disc is a GameObject with a SpriteRenderer, grown and
// faded by its `Droplet` component). The weather decides how often drops land: Drizzle
// (rarely), Rain, or Stormy (every check). Left click cycles the weather, 1-3 pick one,
// M mutes the music, F11 toggles fullscreen. The HUD is text only, as the original was.
//
// The original worked in form pixels (1008 x 729); the same numbers are kept and converted
// to world units, so the drops look the same at any window size.
import {
  audio,
  Camera,
  Color,
  Component,
  GameObject,
  input,
  register,
  Screen,
  Sprite,
  SpriteRenderer,
  Texture,
  Vector2,
  Vector3,
  type Playback,
  type Sound,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

// ---- Geometry: form pixels, as in the original ----

const FORM_W = 1008;
const FORM_H = 729;
/** The whole form height spans 10 world units (the camera's orthographic size is 5). */
const UNITS_PER_PX = 10 / FORM_H;
const MIN_RADIUS = 20;
const MAX_RADIUS = 125;
const DROPLETS = 8;
/** How often the weather is rolled, in seconds. */
const CHECK_INTERVAL = 0.025;

/** The original's chance masks: a drop lands when `roll & weather == weather`. */
enum Weather {
  Drizzle = 7,
  Rain = 3,
  Stormy = 0,
}

const WEATHER_ORDER = [Weather.Drizzle, Weather.Rain, Weather.Stormy];
const WEATHER_NAME: Record<Weather, string> = {
  [Weather.Drizzle]: "Drizzle",
  [Weather.Rain]: "Rain",
  [Weather.Stormy]: "Stormy",
};

/** #325f8c: the original's drop colour (50, 95, 140). */
const BASE_COLOUR = Color.hex("#325f8c");

const rand = (min: number, max: number) => min + Math.random() * (max - min);
const randInt = (min: number, maxExclusive: number) => Math.floor(rand(min, maxExclusive));

/** Scales HSL lightness, as the original's `ModifyBrightness` does. */
function modifyBrightness(c: Color, factor: number): Color {
  const max = Math.max(c.r, c.g, c.b);
  const min = Math.min(c.r, c.g, c.b);
  const l = (max + min) / 2;
  const d = max - min;
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  let h = 0;
  if (d !== 0) {
    if (max === c.r) h = ((c.g - c.b) / d) % 6;
    else if (max === c.g) h = (c.b - c.r) / d + 2;
    else h = (c.r - c.g) / d + 4;
    h = (h * 60 + 360) % 360;
  }
  return hslToColor(h, s, Math.min(1, l * factor), c.a);
}

function hslToColor(h: number, s: number, l: number, a: number): Color {
  const k = (n: number) => (n + h / 30) % 12;
  const f = (n: number) => l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return new Color(f(0), f(8), f(4), a);
}

/** A soft filled disc, the sprite every ripple uses. */
let discSprite: Sprite | null = null;
function getDiscSprite(): Sprite {
  if (discSprite) return discSprite;
  const n = 256;
  const px: Color[] = [];
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      const d = Math.hypot(x + 0.5 - n / 2, y + 0.5 - n / 2);
      px.push(new Color(1, 1, 1, Math.max(0, Math.min(1, n / 2 - 1 - d))));
    }
  }
  discSprite = Sprite.create(Texture.create(n, n, px), null, new Vector2(0.5, 0.5), n);
  return discSprite;
}

// ---- Ripples ----

/** One disc of a drop: it grows to `maxRadius` and fades out over `duration`, then goes. */
class Droplet extends Component {
  maxRadius = 50;
  duration = 0.8;
  colour = BASE_COLOUR;
  order = 0;
  private elapsed = 0;
  private renderer!: SpriteRenderer;

  override onAwake(): void {
    this.renderer = this.gameObject.addComponent(SpriteRenderer, {
      sprite: getDiscSprite(),
      color: this.colour,
      sortingOrder: this.order,
    });
  }

  override onUpdate(dt: number): void {
    this.elapsed += dt;
    const t = Math.min(1, this.elapsed / this.duration);
    // The disc's diameter is one unit at scale 1, so the scale is the diameter.
    const radius = this.maxRadius * t * UNITS_PER_PX;
    this.gameObject.transform.localScale = new Vector3(radius * 2, radius * 2, 1);
    this.renderer.color = new Color(this.colour.r, this.colour.g, this.colour.b, this.colour.a * (1 - t));
    if (t >= 1) this.gameObject.destroy();
  }
}

// ---- The demo ----

/** The scene: the camera, the rain, the weather and the music. */
class RaindropDemo extends Component {
  weather: Weather = Weather.Stormy;
  private music: Sound | null = null;
  private playback: Playback | null = null;
  private muted = false;
  private checkTimer = 0;
  private statsTimer = 0;
  private lastSpawn = { x: MIN_RADIUS, y: MIN_RADIUS };
  private textPhase = 0;
  private fps = 60;

  override onAwake(): void {
    demo = this;
    new GameObject("Main Camera", { position: new Vector3(0, 0, 10) }).addComponent(Camera, {
      orthographic: true,
      orthographicSize: 5,
      backgroundColor: Color.black,
    });

    this.music = audio.load("assets/rains-falling-on-my-head-loop.mp3", { stream: true });
    this.playback = this.music.play({ loop: true });
    notify();
  }

  /** Left click: the next weather in the cycle. */
  cycleWeather(): void {
    const i = WEATHER_ORDER.indexOf(this.weather);
    this.setWeather(WEATHER_ORDER[(i + 1) % WEATHER_ORDER.length]!);
  }

  setWeather(weather: Weather): void {
    this.weather = weather;
    notify();
  }

  toggleMusic(): void {
    this.muted = !this.muted;
    if (this.muted) this.playback?.pause();
    else this.playback?.resume();
    notify();
  }

  override onUpdate(dt: number): void {
    this.textPhase = (this.textPhase + dt / 5) % 2;
    this.fps += (1 / Math.max(dt, 1e-3) - this.fps) * 0.1;

    WEATHER_ORDER.forEach((w, i) => {
      if (input.getKeyDown(`Digit${i + 1}`)) this.setWeather(w);
    });
    if (input.getKeyDown("KeyM")) this.toggleMusic();
    if (input.getKeyDown("F11")) Screen.fullScreen = !Screen.fullScreen;
    if (input.getMouseButtonDown(0) || input.getMouseButtonDown(1)) this.cycleWeather();

    // Roll the weather every 25 ms, as the original did.
    this.checkTimer += dt;
    if (this.checkTimer >= CHECK_INTERVAL) {
      this.checkTimer = 0;
      this.roll();
    }

    this.statsTimer += dt;
    if (this.statsTimer >= 0.05) {
      this.statsTimer = 0;
      notify();
    }
  }

  private roll(): void {
    const chance = randInt(0, 256);
    if ((chance & this.weather) !== this.weather) return;

    // The original's odds: 1 in 10 is a big, slow, darker drop that stays where it was.
    let radius: number;
    let movement: number;
    let colour: Color;
    if (randInt(1, 11) === 10) {
      radius = randInt(100, 200);
      movement = 1.6;
      colour = modifyBrightness(BASE_COLOUR, 0.25);
    } else {
      radius = randInt(20, 100);
      movement = 0.8;
      colour = BASE_COLOUR;
      this.lastSpawn = { x: randInt(MIN_RADIUS, FORM_W - MAX_RADIUS), y: randInt(MIN_RADIUS, FORM_H - MAX_RADIUS) };
    }
    this.spawn(this.lastSpawn, radius, movement, colour);
  }

  /** A ring of discs, each one dissolving a little later and dimmer than the one before. */
  private spawn(at: { x: number; y: number }, radius: number, movement: number, colour: Color): void {
    const rate = 0.12 * (Math.max(radius, 100) / 100);
    for (let i = 1; i <= DROPLETS; i++) {
      const duration = movement + rate * i;
      const shade = modifyBrightness(colour, (DROPLETS - i) / DROPLETS);
      const go = new GameObject("Droplet", {
        position: new Vector3((at.x - FORM_W / 2) * UNITS_PER_PX, (FORM_H / 2 - at.y) * UNITS_PER_PX, 0),
      });
      go.addComponent(Droplet, { maxRadius: radius, duration, colour: shade, order: i });
    }
  }

  /** What the HUD shows. */
  get stats(): { weather: string; muted: boolean; colour: Color; fps: number } {
    // The original's red-to-blue pulse for the weather text.
    const t = this.textPhase > 1 ? 2 - this.textPhase : this.textPhase;
    return { weather: WEATHER_NAME[this.weather], muted: this.muted, colour: new Color(1 - t, 0, t, 1), fps: this.fps };
  }
}

// ---- HUD (React text, as the original) ----

let demo: RaindropDemo | null = null;
const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

/** Re-renders the HUD whenever the demo says something changed. */
function useStats() {
  const read = () => {
    const d = demo;
    if (!d) return { weather: "", muted: false, colour: "#ff0000", fps: 0 };
    const s = d.stats;
    return { weather: s.weather, muted: s.muted, colour: `rgb(${Math.round(s.colour.r * 255)}, 0, ${Math.round(s.colour.b * 255)})`, fps: s.fps };
  };
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

function Hud() {
  const s = useStats();
  return (
    <box style={{ width: "100%", height: "100%", padding: 14, pointerEvents: "none" }}>
      <box style={{ position: "absolute", left: 0, right: 0, top: 0, bottom: 0, alignItems: "center", justifyContent: "center" }}>
        <text style={{ color: s.colour, fontSize: 40, fontFamily: "monospace" }}>{`Weather: ${s.weather}`}</text>
      </box>
      <text style={{ position: "absolute", right: 14, top: 14, color: "#808080", fontSize: 12, fontFamily: "monospace" }}>{`${Math.round(s.fps)} FPS`}</text>
      <text style={{ color: "#808080", fontSize: 13 }}>
        {`Click or 1-3: weather · M: ${s.muted ? "unmute" : "mute"} · F11: fullscreen`}
      </text>
    </box>
  );
}

register({ RaindropDemo, Droplet });
createRoot().render(<Hud />);
