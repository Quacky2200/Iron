// SPDX-License-Identifier: Apache-2.0

// The flip clock, drawn by Platypus's UI: each digit is a card split across the middle. When it
// changes, the top flap (the old digit's top half) folds down on its hinge, then the new
// digit's bottom half folds down into place, in 3D (`perspective`, `rotateX`). Each half clips
// a full-height digit (`overflow: hidden`), so a flap shows exactly half of it.

const FLIP = 0.6;
const TOP = "#2c2d33";
const BOTTOM = "#24252a";
const TEXT = "#f2f2f2";

/** One half of a card showing `digit`: the top or bottom half of a full-height digit. */
function Half(props: { digit: string; half: "top" | "bottom"; w: number; h: number; origin?: string; transform?: string; shade?: number }) {
  const { w, h } = props;
  const top = props.half === "top";
  return (
    <box
      style={{
        position: "absolute",
        left: 0,
        top: top ? 0 : h / 2,
        width: w,
        height: h / 2,
        overflow: "hidden",
        backgroundColor: top ? TOP : BOTTOM,
        borderTopLeftRadius: top ? 10 : 0,
        borderTopRightRadius: top ? 10 : 0,
        borderBottomLeftRadius: top ? 0 : 10,
        borderBottomRightRadius: top ? 0 : 10,
        transformOrigin: props.origin,
        transform: props.transform,
      }}
    >
      <box style={{ position: "absolute", left: 0, top: top ? 0 : -h / 2, width: w, height: h, alignItems: "center", justifyContent: "center" }}>
        <text style={{ color: TEXT, fontSize: h * 0.78, fontWeight: 700 }}>{props.digit}</text>
      </box>
      {props.shade ? (
        <box style={{ position: "absolute", left: 0, top: 0, width: w, height: h / 2, backgroundColor: `rgba(0, 0, 0, ${props.shade})` }} />
      ) : null}
    </box>
  );
}

/** A card that flips from the digit it showed to `digit`; `now` is in seconds. */
function Card(props: { digit: string; now: number; w: number; h: number }) {
  const { w, h, now } = props;
  const state = useRef({ shown: props.digit, previous: props.digit, at: -Infinity });
  const s = state.current;
  if (props.digit !== s.shown) {
    s.previous = s.shown;
    s.shown = props.digit;
    s.at = now;
  }
  const p = Math.min(1, (now - s.at) / FLIP);
  const flipping = p < 1;
  const first = p < 0.5;
  // Each flap turns 90 degrees in half the time, easing like a falling card.
  const q = first ? p / 0.5 : (p - 0.5) / 0.5;
  const fall = q * q;
  return (
    <box style={{ width: w, height: h, perspective: "900px" }}>
      {/* Behind the flaps: the new top, and the old bottom until the second flap covers it. */}
      <Half digit={s.shown} half="top" w={w} h={h} />
      <Half digit={flipping ? s.previous : s.shown} half="bottom" w={w} h={h} />
      {flipping && first ? (
        <Half
          digit={s.previous}
          half="top"
          w={w}
          h={h}
          shade={fall * 0.45}
          origin="50% 100%"
          transform={`rotateX(${-fall * 90}deg)`}
        />
      ) : null}
      {flipping && !first ? (
        <Half
          digit={s.shown}
          half="bottom"
          w={w}
          h={h}
          shade={(1 - fall) * 0.45}
          origin="50% 0%"
          transform={`rotateX(${(1 - fall) * 90}deg)`}
        />
      ) : null}
      {/* The hinge. */}
      <box style={{ position: "absolute", left: 0, top: h / 2 - 1.5, width: w, height: 3, backgroundColor: "#0b0b0d" }} />
    </box>
  );
}

function Pair(props: { value: string; now: number; w: number; h: number }) {
  return (
    <box style={{ flexDirection: "row", gap: props.w * 0.08 }}>
      <Card digit={props.value.charAt(0)} now={props.now} w={props.w} h={props.h} />
      <Card digit={props.value.charAt(1)} now={props.now} w={props.w} h={props.h} />
    </box>
  );
}

export function FlipClock(props: { hours: string; minutes: string; seconds: string; date: string; now: number }) {
  const w = 130;
  const h = 190;
  return (
    <box style={{ position: "absolute", left: 0, right: 0, top: 0, bottom: 0, alignItems: "center", justifyContent: "center", pointerEvents: "none" }}>
      <box style={{ flexDirection: "column", alignItems: "center", gap: 26 }}>
        <box style={{ flexDirection: "row", alignItems: "flex-end", gap: 34 }}>
          <Pair value={props.hours} now={props.now} w={w} h={h} />
          <Pair value={props.minutes} now={props.now} w={w} h={h} />
          <Pair value={props.seconds} now={props.now} w={w * 0.5} h={h * 0.5} />
        </box>
        <text style={{ color: "#8c96a6", fontSize: 22 }}>{props.date}</text>
      </box>
    </box>
  );
}
