// The React side: what the emoji panel shows (a Noto SVG, or a face drawn in code whose eyes
// follow the mouse), the caption, the spinner wheel and the curtains. The director writes to
// small stores; components read them.
import { addStyleSheet } from "@iron/engine";
import { Image } from "@iron/react";
import { useEffect, useState } from "react";
import { PANEL } from "./config.ts";

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

export const shown = store<{ code: string; look: "eyes" | "googly" | null }>({ code: "1f600", look: null });
export const caption = store<string | null>(null);
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
  if (s.look) return <Eyed head={s.look === "googly"} />;
  return <Image src={`assets/emoji/${s.code}.svg`} style={{ width: PANEL, height: PANEL }} />;
};

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

export function Hud() {
  const name = caption.use();
  return (
    <box style={{ width: "100%", height: "100%", pointerEvents: "none" }}>
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
