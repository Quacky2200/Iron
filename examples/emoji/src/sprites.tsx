// Sprite particles and props: a pool of small world panels, each showing one Noto emoji, that
// the effects and signature acts fly around (hearts, tears, bubbles, notes, snowflakes, a swung
// pickaxe, a banana...). Unlike the line particles in fx.ts they have shapes and can be props.
import { Store } from "./hud.tsx";

export interface SpriteOpts {
  /** The Noto file (codepoint) to show. */
  code: string;
  x: number;
  y: number;
  vx?: number;
  vy?: number;
  /** Units per second squared, downwards (negative rises). */
  gravity?: number;
  /** Fraction of speed lost per second. */
  drag?: number;
  /** Degrees. */
  rot?: number;
  /** Degrees per second. */
  spin?: number;
  /** Size in world units (`scaleTo`: grows or shrinks to that over its life). */
  scale?: number;
  scaleTo?: number;
  life?: number;
  fadeIn?: number;
  fadeOut?: number;
  /** Sideways wobble, in units. */
  sway?: number;
  /** Depth: in front of the emoji by default; negative goes behind it. */
  z?: number;
  /** Takes over the motion: where it is at `t` seconds, or null when it's done. */
  anim?: (t: number) => SpriteAt | null;
}

/** Where a sprite is when `anim` takes over its motion. */
export interface SpriteAt {
  x: number;
  y: number;
  rot?: number;
  scale?: number;
  alpha?: number;
  sx?: number;
  sy?: number;
}

interface Slot {
  go: GameObject;
  panel: WorldPanel;
  code: Store<string>;
  on: boolean;
  o: SpriteOpts;
  t: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
  alpha: number;
  seed: number;
}

function SpritePic(props: { code: Store<string> }) {
  const code = props.code.use();
  return <Image src={`assets/emoji/${code}.svg`} style={{ width: 128, height: 128 }} />;
}

export class Sprites {
  private slots: Slot[] = [];
  private next = 0;

  constructor(count: number = 48) {
    for (let i = 0; i < count; i++) {
      const go = new GameObject(`Sprite ${i}`, { position: new Vector3(99, 0, 0.5) });
      go.enabled = false;
      const panel = go.addComponent(WorldPanel, { width: 128, height: 128, pixelsPerMeter: 128, resolution: 1, interactive: false });
      const code = new Store<string>("2728");
      createRoot(panel).render(<SpritePic code={code} />);
      this.slots.push({ go, panel, code, on: false, o: { code: "2728", x: 0, y: 0 }, t: 0, x: 0, y: 0, vx: 0, vy: 0, alpha: -1, seed: 0 });
    }
  }

  /** Starts a sprite, reusing the oldest slot when they're all busy. */
  emit(o: SpriteOpts): void {
    const s = this.slots[this.next]!;
    this.next = (this.next + 1) % this.slots.length;
    s.on = true;
    s.o = o;
    s.t = 0;
    s.x = o.x;
    s.y = o.y;
    s.vx = o.vx ?? 0;
    s.vy = o.vy ?? 0;
    s.alpha = -1;
    s.seed = Math.random() * 6.28;
    if (s.code.get() !== o.code) s.code.set(o.code);
    s.go.enabled = true;
    this.place(s, 0);
  }

  private place(s: Slot, dt: number): void {
    const o = s.o;
    let x = s.x;
    let y = s.y;
    let rot = (o.rot ?? 0) + (o.spin ?? 0) * s.t;
    let size = 1;
    if (o.scale !== undefined) {
      size = o.scale;
      if (o.scaleTo !== undefined) size = o.scale + (o.scaleTo - o.scale) * Math.min(1, s.t / (o.life ?? 1));
    }
    let alpha = 1;
    let sxm = 1;
    let sym = 1;
    if (o.anim) {
      const a = o.anim(s.t);
      if (!a) {
        s.on = false;
        s.go.enabled = false;
        return;
      }
      x = a.x;
      y = a.y;
      rot = a.rot ?? rot;
      size = a.scale ?? size;
      alpha = a.alpha ?? 1;
      sxm = a.sx ?? 1;
      sym = a.sy ?? 1;
    } else {
      const keep = Math.max(0, 1 - (o.drag ?? 0) * dt);
      s.vx *= keep;
      s.vy = s.vy * keep - (o.gravity ?? 0) * dt;
      s.x += s.vx * dt;
      s.y += s.vy * dt;
      x = s.x + (o.sway ?? 0) * Math.sin(s.t * 3 + s.seed);
      y = s.y;
      const life = o.life ?? 1;
      if (s.t >= life) {
        s.on = false;
        s.go.enabled = false;
        return;
      }
      if (o.fadeIn) alpha = Math.min(alpha, s.t / o.fadeIn);
      alpha = Math.min(alpha, (life - s.t) / (o.fadeOut ?? Math.min(0.4, life * 0.4)));
    }
    s.go.transform.position = new Vector3(x, y, o.z ?? 0.5);
    s.go.transform.localEulerAngles = new Vector3(0, 0, rot);
    s.go.transform.localScale = new Vector3(size * sxm, size * sym, 1);
    const a = Math.max(0, Math.min(1, alpha));
    if (Math.abs(a - s.alpha) > 0.02) {
      s.alpha = a;
      s.panel.tint = new Color(1, 1, 1, a);
    }
  }

  update(dt: number): void {
    for (const s of this.slots) {
      if (!s.on) continue;
      s.t += dt;
      this.place(s, dt);
    }
  }

  clear(): void {
    for (const s of this.slots) {
      s.on = false;
      s.go.enabled = false;
    }
  }
}
