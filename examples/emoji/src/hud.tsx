// The React side: what the emoji panel shows (a Noto SVG, or a face drawn in code whose eyes
// follow the mouse), the caption, the spinner wheel and the curtains. The director writes to
// small stores; components read them.
import { addStyleSheet } from "@iron/engine";
import { Image } from "@iron/react";
import { useEffect, useState } from "react";
import { BIG, PANEL } from "./config.ts";

function store<T>(init: T) {
  let value = init;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set(next: T) {
      value = next;
      listeners.forEach((f) => f());
    },
    use(): T {
      const [v, setV] = useState(value);
      useEffect(() => {
        const f = () => setV(value);
        listeners.add(f);
        f();
        return () => void listeners.delete(f);
      }, []);
      return v;
    },
  };
}

export const shown = store<{ code: string; look: "eyes" | "googly" | null; mirror: boolean; big: boolean }>({ code: "1f600", look: null, mirror: false, big: false });
export const caption = store<string | null>(null);
/** Whole-stage overlays drawn over the emoji: rain, storm clouds, a white flash (lightning), the room blushing red, a party. */
export const overlay = store<{ rain: boolean; storm: boolean; love: boolean; party: boolean; flash: number; countdown: string | null }>({ rain: false, storm: false, love: false, party: false, flash: 0, countdown: null });
/** Tomato splats on the emoji, in the panel's pixels. */
export interface Splat {
  x: number;
  y: number;
  size: number;
  rot: number;
}
export const splats = store<Splat[]>([]);
/** The rocket emoji is drawn in two pieces so it can turn upright and its flame can go out, flicker and roar. */
export const rocket = store({ turn: 0, flame: 0, power: 0, stars: 1 });
/** Pupil offsets (-1..1 of how far they can move) and how shut the eyelids are (0..1). */
export const eyes = store({ px: 0, py: 0, blink: 0 });

export interface WheelState {
  /** The Noto files on the wheel, clockwise from the top. */
  items: string[];
  angle: number;
  /** Degrees the pointer's flap is kicked back by a peg. */
  flap: number;
  /** 0..1: the winner swells and the rest dim. */
  glow: number;
  scale: number;
  fade: number;
  winner: number;
}
export const wheel = store<WheelState | null>(null);

addStyleSheet(`
.caption { animation: caption-in 0.45s cubic-bezier(0.2, 1.7, 0.4, 1) both; }
@keyframes caption-in {
  from { opacity: 0; transform: translateY(28px) scale(0.6); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.drape {
  width: 100%; height: 100%;
  background-image:
    linear-gradient(180deg, rgba(0,0,0,0.6) 0%, rgba(0,0,0,0) 26%, rgba(0,0,0,0) 80%, rgba(0,0,0,0.45) 100%),
    repeating-linear-gradient(90deg, #4a0710 0px, #9c1222 30px, #d92c3c 46px, #9c1222 62px, #4a0710 92px);
  border-bottom: 12px solid #e0b04a;
}
.drape-l { box-shadow: inset -26px 0 34px rgba(0,0,0,0.5); }
.drape-r { box-shadow: inset 26px 0 34px rgba(0,0,0,0.5); }
.flame { transform-origin: 30% 70%; animation: flicker 0.16s ease-in-out infinite alternate; }
@keyframes flicker { from { transform: scale(0.94) rotate(-3deg); } to { transform: scale(1.1) rotate(3deg); } }
.flame-big { transform-origin: 30% 70%; animation: roar 0.11s ease-in-out infinite alternate; }
@keyframes roar { from { transform: scale(1.3) rotate(-6deg); } to { transform: scale(1.8) rotate(6deg); } }
.drop { position: absolute; top: -140px; background-image: linear-gradient(180deg, rgba(190,210,255,0), rgba(225,236,255,0.9)); }
@keyframes fall { from { transform: translateY(0px); } to { transform: translateY(940px); } }
.storm-dark { position: absolute; left: 0; top: 0; right: 0; bottom: 0; background-image: linear-gradient(180deg, rgba(4,6,18,0.85) 0%, rgba(6,8,24,0.55) 45%, rgba(6,8,24,0.5) 100%); animation: fadein 1.2s ease-out both; }
@keyframes fadein { from { opacity: 0; } to { opacity: 1; } }
.cloud-in { position: absolute; width: 420px; height: 200px; animation: fadein 1.6s ease-out both; }
.cloud { position: absolute; left: 0; top: 0; width: 420px; height: 200px; animation: drift 14s ease-in-out infinite alternate; }
@keyframes drift { from { transform: translateX(-70px); } to { transform: translateX(70px); } }
.puff { position: absolute; border-radius: 50%; background-image: radial-gradient(circle, rgba(78,84,110,0.97) 0%, rgba(48,52,76,0.95) 55%, rgba(30,33,52,0) 74%); }
.confetti { position: absolute; top: -30px; width: 11px; height: 17px; }
@keyframes confall { from { transform: translateY(0px) rotate(0deg); } to { transform: translateY(840px) rotate(900deg); } }
.balloon { position: absolute; bottom: -140px; }
@keyframes balloon {
  0% { transform: translate(0px, 0px) rotate(-4deg); }
  25% { transform: translate(26px, -240px) rotate(5deg); }
  50% { transform: translate(-22px, -480px) rotate(-5deg); }
  75% { transform: translate(28px, -720px) rotate(5deg); }
  100% { transform: translate(0px, -960px) rotate(-3deg); }
}
.flag { position: absolute; width: 38px; height: 48px; clip-path: polygon(0 0, 100% 0, 50% 100%); }
.love-glow { position: absolute; left: 0; top: 0; right: 0; bottom: 0; background-image: radial-gradient(circle, rgba(255,40,90,0) 30%, rgba(255,30,80,0.8) 100%); animation: lub 0.86s ease-out infinite; }
@keyframes lub { 0% { opacity: 0.15; } 8% { opacity: 1; } 24% { opacity: 0.35; } 36% { opacity: 0.85; } 60% { opacity: 0.2; } 100% { opacity: 0.15; } }
.heart-f { position: absolute; bottom: -90px; animation: floatup 4.5s ease-in infinite; }
@keyframes floatup { from { transform: translateY(0px) scale(0.6); opacity: 0; } 12% { opacity: 0.9; } to { transform: translateY(-820px) scale(1.15); opacity: 0; } }
.valance { width: 100%; height: 100%; flex-direction: column; }
.valance-bar {
  width: 100%; height: 50px; border-bottom: 5px solid #e0b04a;
  background-image: linear-gradient(180deg, #5a0911, #a61425 70%, #7c0e1a);
}
.valance-row { flex-direction: row; }
.scallop {
  width: 96px; height: 40px; border: 5px solid #e0b04a; border-top-width: 0;
  border-radius: 0 0 48px 48px;
  background-image: linear-gradient(180deg, #b01524, #8d1020);
}
`);

export const Face = () => {
  const s = shown.use();
  const marks = splats.use();
  const side = s.big ? BIG : PANEL;
  return (
    <box style={{ width: side, height: side, alignItems: "center", justifyContent: "center" }}>
     <box style={{ width: PANEL, height: PANEL }}>
      {s.look ? (
        <Eyed head={s.look === "googly"} />
      ) : s.code === "1f680" ? (
        <RocketArt />
      ) : (
        <Image src={`assets/emoji/${s.code}.svg`} style={{ width: PANEL, height: PANEL, transform: s.mirror ? "scaleX(-1)" : "none" }} />
      )}
      {marks.map((m, i) => (
        <box
          key={i}
          style={{
            position: "absolute",
            left: m.x - m.size / 2,
            top: m.y - m.size / 2,
            width: m.size,
            height: m.size,
            borderRadius: "50%",
            transform: `rotate(${m.rot}deg) scale(1, 0.8)`,
            backgroundImage: "radial-gradient(circle, rgba(240,50,35,1) 0%, rgba(215,35,22,0.95) 50%, rgba(190,25,15,0) 74%)",
          }}
        />
      ))}
     </box>
    </box>
  );
};

function RocketArt() {
  const r = rocket.use();
  const art = { position: "absolute" as const, left: 0, top: 0, width: PANEL, height: PANEL };
  return (
    <box style={{ width: PANEL, height: PANEL, transform: `translate(0px, ${-40 * r.turn}px) rotate(${-45 * r.turn}deg) scale(${1 - 0.1 * r.turn})` }}>
      {/* The flame is behind the rocket, as in the original: the fins cover its root. */}
      {r.flame > 0.01 && <Image src="assets/emoji/1f680-flame.svg" className={r.power > 0.5 ? "flame-big" : "flame"} style={{ ...art, opacity: r.flame }} />}
      <Image src="assets/emoji/1f680-body.svg" style={art} />
      {r.stars > 0.01 && <Image src="assets/emoji/1f680-stars.svg" style={{ ...art, opacity: r.stars }} />}
    </box>
  );
}

/** A fragment of the exploding emoji: the same picture, small. */
export function Shard() {
  const s = shown.use();
  return <Image src={`assets/emoji/${s.look ? "1f440" : s.code}.svg`} style={{ width: 120, height: 120 }} />;
}

/** Eyes in code: whites, pupils with a glint, and lids that blink. With a head it's a yellow face. */
function Eyed({ head }: { head: boolean }) {
  const e = eyes.use();
  const w = head ? 120 : 190;
  const h = head ? 150 : 250;
  const pupil = head ? 54 : 84;
  const socket = (left: number, top: number) => (
    <box
      key={left}
      style={{
        position: "absolute",
        left,
        top,
        width: w,
        height: h,
        borderRadius: "50%",
        backgroundColor: "#ffffff",
        border: "5px solid #3a2a14",
        boxShadow: "inset 0 -10px 18px rgba(0,0,0,0.18)",
        transform: `scaleY(${1 - 0.93 * e.blink})`,
      }}
    >
      <box
        style={{
          position: "absolute",
          left: w / 2 - pupil / 2 - 5 + e.px * (w / 2 - pupil / 2 - 12),
          top: h / 2 - pupil / 2 - 5 + e.py * (h / 2 - pupil / 2 - 12),
          width: pupil,
          height: pupil,
          borderRadius: "50%",
          backgroundColor: "#171310",
        }}
      >
        <box style={{ position: "absolute", left: pupil * 0.22, top: pupil * 0.18, width: pupil * 0.28, height: pupil * 0.28, borderRadius: "50%", backgroundColor: "#ffffff" }} />
      </box>
    </box>
  );
  return (
    <box style={{ width: PANEL, height: PANEL }}>
      {head && (
        <box
          style={{
            position: "absolute",
            left: 20,
            top: 20,
            width: PANEL - 40,
            height: PANEL - 40,
            borderRadius: "50%",
            backgroundImage: "radial-gradient(circle, #ffe061 0%, #ffc61a 62%, #f0a000 100%)",
            boxShadow: "inset 0 -16px 34px rgba(160,80,0,0.4)",
          }}
        />
      )}
      {socket(head ? 105 : 40, head ? 120 : 110)}
      {socket(head ? 255 : 250, head ? 120 : 110)}
      {head && (
        <box style={{ position: "absolute", left: 130, top: 310, width: 220, height: 100, borderRadius: "0 0 110px 110px", backgroundColor: "#5a1f10" }}>
          <box style={{ position: "absolute", left: 60, top: 50, width: 100, height: 70, borderRadius: "50%", backgroundColor: "#e8587a" }} />
        </box>
      )}
    </box>
  );
}

const WHEEL = 215;
const CHIP = 84;
const COLOURS = ["#e63946", "#f4a261", "#2a9d8f", "#7b2cbf"];
// A gradient holds eight stops, so four colours (two stops each) repeat round the wheel.
const CONIC = `repeating-conic-gradient(from -22.5deg, ${COLOURS.slice(0, 4).map((c, i) => `${c} ${i * 45}deg ${(i + 1) * 45}deg`).join(", ")})`;

function Wheel() {
  const w = wheel.use();
  if (!w) return null;
  return (
    <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
      <box style={{ position: "relative", top: 28, width: WHEEL * 2, height: WHEEL * 2, opacity: w.fade, transform: `scale(${w.scale})` }}>
        <text style={{ position: "absolute", left: 0, right: 0, top: WHEEL * 2 + 22, textAlign: "center", fontSize: 40, fontWeight: 800, color: "#f6f1e2", letterSpacing: 6, textTransform: "uppercase", textShadow: "0 3px 12px rgba(0,0,0,0.8)" }}>
          Who's next?
        </text>
        <box style={{ position: "absolute", left: 0, top: 0, width: WHEEL * 2, height: WHEEL * 2, borderRadius: "50%", backgroundImage: CONIC, transform: `rotate(${w.angle}deg)` }}>
          {w.items.map((code, i) => {
            const lit = i === w.winner;
            return (
              <box key={i} style={{ position: "absolute", left: WHEEL, top: WHEEL, width: 0, height: 0, transform: `rotate(${i * 45}deg)` }}>
                <box
                  style={{
                    position: "absolute",
                    left: -CHIP / 2,
                    top: -(WHEEL - 28 - CHIP / 2) - CHIP / 2,
                    width: CHIP,
                    height: CHIP,
                    borderRadius: "50%",
                    backgroundColor: "rgba(255,255,255,0.88)",
                    boxShadow: "0 4px 10px rgba(0,0,0,0.35)",
                    opacity: lit ? 1 : 1 - 0.55 * w.glow,
                    transform: `scale(${lit ? 1 + 0.3 * w.glow : 1})`,
                  }}
                >
                  <Image src={`assets/emoji/${code}.svg`} style={{ position: "absolute", left: 8, top: 8, width: CHIP - 16, height: CHIP - 16 }} />
                </box>
              </box>
            );
          })}
        </box>
        <box style={{ position: "absolute", left: 0, top: 0, width: WHEEL * 2, height: WHEEL * 2, borderRadius: "50%", border: "14px solid #f3c969", boxShadow: "0 18px 60px rgba(0,0,0,0.65), inset 0 0 34px rgba(0,0,0,0.4)" }} />
        <box style={{ position: "absolute", left: WHEEL - 48, top: WHEEL - 48, width: 96, height: 96, borderRadius: "50%", backgroundImage: "radial-gradient(circle, #fff0b0, #e0b04a 75%)", boxShadow: "0 4px 14px rgba(0,0,0,0.5)", alignItems: "center", justifyContent: "center" }}>
          <text style={{ fontSize: 48, fontWeight: 800, color: "#7a2230" }}>?</text>
        </box>
        <box style={{ position: "absolute", left: WHEEL - 24, top: -34, width: 48, height: 58, backgroundColor: "#f6f1e2", clipPath: "polygon(0 0, 100% 0, 50% 100%)", transformOrigin: "50% 12%", transform: `rotate(${w.flap}deg)` }} />
      </box>
    </box>
  );
}

// Fixed random positions, so a re-render doesn't scatter them again.
const DROPS = Array.from({ length: 130 }, (_, i) => {
  const near = i % 2 === 0;
  return {
    left: `${Math.random() * 104 - 2}%`,
    width: near ? 3 : 2,
    height: near ? 70 + Math.random() * 60 : 40 + Math.random() * 30,
    dur: near ? 0.26 + Math.random() * 0.1 : 0.38 + Math.random() * 0.15,
    delay: -Math.random(),
    alpha: near ? 0.95 : 0.6,
  };
});
const HEARTS = Array.from({ length: 12 }, (_, i) => ({ left: `${4 + i * 8 + Math.random() * 5}%`, delay: -Math.random() * 4.5, size: 40 + Math.random() * 40 }));
const PARTY_COLOURS = ["#ff4d6d", "#ffd166", "#4cc9f0", "#80ed99", "#f72585", "#b5179e", "#ffffff", "#ff9f1c"];
const PIECES = Array.from({ length: 80 }, () => ({ left: `${Math.random() * 100}%`, dur: 2.4 + Math.random() * 1.6, delay: -Math.random() * 4, colour: pick(PARTY_COLOURS) }));
const BALLOONS = Array.from({ length: 10 }, (_, i) => ({ left: `${3 + i * 10 + Math.random() * 4}%`, size: 90 + Math.random() * 40, delay: -Math.random() * 6.5, dur: 5.5 + Math.random() * 2 }));
const FLAGS = Array.from({ length: 26 }, (_, i) => {
  const u = (i + 0.5) / 26;
  const swag = Math.sin(3 * Math.PI * u);
  return { left: `${u * 100}%`, top: 140 + 26 * Math.abs(swag), rot: 18 * Math.cos(3 * Math.PI * u) * Math.sign(swag), colour: PARTY_COLOURS[i % PARTY_COLOURS.length]! };
});
function pick<T>(list: T[]): T {
  return list[Math.floor(Math.random() * list.length)]!;
}
const CLOUDS = [
  { left: -60, top: -80, scale: 1.2 }, { left: 140, top: -40, scale: 1.0 }, { left: 320, top: -90, scale: 1.3 }, { left: 520, top: -50, scale: 1.1 },
  { left: 700, top: -90, scale: 1.25 }, { left: 900, top: -40, scale: 1.0 }, { left: 1090, top: -80, scale: 1.2 },
];
const PUFFS: [number, number, number][] = [[0, 70, 150], [80, 20, 190], [190, 0, 200], [280, 40, 170], [240, 95, 150], [110, 85, 170], [30, 100, 120]];

function Rain() {
  return (
    <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, overflow: "hidden", pointerEvents: "none" }}>
      {DROPS.map((d, i) => (
        <box key={i} className="drop" style={{ left: d.left, width: d.width, height: d.height, opacity: d.alpha, animation: `fall ${d.dur}s linear ${d.delay}s infinite` }} />
      ))}
    </box>
  );
}

/** Dark, heavy clouds drift across the top and the room goes dim. */
function Storm() {
  return (
    <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, overflow: "hidden", pointerEvents: "none" }}>
      <box className="storm-dark" />
      {CLOUDS.map((c, i) => (
        <box key={i} className="cloud-in" style={{ left: c.left, top: c.top, transform: `scale(${c.scale})` }}>
          <box className="cloud" style={{ animationDelay: `${-i * 2.3}s` }}>
            {PUFFS.map(([x, y, size], k) => (
              <box key={k} className="puff" style={{ left: x, top: y, width: size, height: size * 0.8 }} />
            ))}
          </box>
        </box>
      ))}
    </box>
  );
}

/** Confetti raining down, balloons rising and bunting across the top. */
function Party() {
  return (
    <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, overflow: "hidden", pointerEvents: "none" }}>
      {BALLOONS.map((b, i) => (
        <Image key={i} src="assets/emoji/1f388.svg" className="balloon" style={{ left: b.left, width: b.size, height: b.size, animation: `balloon ${b.dur}s linear ${b.delay}s infinite` }} />
      ))}
      {FLAGS.map((f, i) => (
        <box key={i} className="flag" style={{ left: f.left, top: f.top, backgroundColor: f.colour, transform: `rotate(${f.rot}deg)` }} />
      ))}
      {PIECES.map((p, i) => (
        <box key={i} className="confetti" style={{ left: p.left, backgroundColor: p.colour, animation: `confall ${p.dur}s linear ${p.delay}s infinite` }} />
      ))}
    </box>
  );
}

function Love() {
  return (
    <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, overflow: "hidden", pointerEvents: "none" }}>
      <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, backgroundColor: "rgba(255,40,80,0.16)" }} />
      <box className="love-glow" />
      {HEARTS.map((h, i) => (
        <Image key={i} src="assets/emoji/2764.svg" className="heart-f" style={{ left: h.left, width: h.size, height: h.size, animation: `floatup 4.5s ease-in ${h.delay}s infinite` }} />
      ))}
    </box>
  );
}

export function Hud() {
  const name = caption.use();
  const ov = overlay.use();
  return (
    <box style={{ width: "100%", height: "100%", pointerEvents: "none" }}>
      {ov.storm && <Storm />}
      {ov.rain && <Rain />}
      {ov.love && <Love />}
      {ov.party && <Party />}
      {ov.countdown && (
        <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
          <text key={ov.countdown} className="caption" style={{ fontSize: ov.countdown.length > 1 ? 150 : 260, fontWeight: 900, color: "#fff7e0", letterSpacing: 6, textShadow: "0 6px 30px rgba(0,0,0,0.9)" }}>
            {ov.countdown}
          </text>
        </box>
      )}
      {ov.flash > 0.01 && <box style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, backgroundColor: "#ffffff", opacity: ov.flash }} />}
      <text style={{ position: "absolute", left: 28, top: 22, fontSize: 18, fontWeight: 800, color: "#e0b04a", letterSpacing: 7, textTransform: "uppercase", textShadow: "0 2px 8px rgba(0,0,0,0.9)" }}>
        ★ Emoji celebrity
      </text>
      <Wheel />
      {name && (
        <box style={{ position: "absolute", left: 0, right: 0, bottom: 34, alignItems: "center", flexDirection: "column" }}>
          <text key={name} className="caption" style={{ fontSize: 46, fontWeight: 800, color: "#f6f1e2", letterSpacing: 4, textTransform: "uppercase", textShadow: "0 3px 12px rgba(0,0,0,0.85)" }}>
            {name}
          </text>
        </box>
      )}
    </box>
  );
}

/** One half of the curtain: folds (a repeating gradient) that squash up as it's drawn back. */
export const Drape = ({ side }: { side: "l" | "r" }) => <box className={`drape drape-${side}`} />;

/** The gold-edged swag along the top. */
export function Valance({ width }: { width: number }) {
  const n = Math.ceil(width / 96) + 1;
  return (
    <box className="valance">
      <box className="valance-bar" />
      <box className="valance-row">
        {Array.from({ length: n }, (_, i) => (
          <box key={i} className="scallop" />
        ))}
      </box>
    </box>
  );
}
