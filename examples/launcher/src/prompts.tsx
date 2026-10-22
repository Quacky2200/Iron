// Button prompts that follow the last input device, as most PC games do: the first key or
// mouse move shows keyboard keys (and the cursor), the first pad button or stick push shows
// that pad's buttons (and hides the cursor). The pad's icons come from its name (Xbox,
// PlayStation, Nintendo), else from the platform.
import { Application, Cursor, input } from "@iron/engine";
import { Text } from "@iron/react";
import { type ReactNode, useSyncExternalStore } from "react";

export type PadStyle = "xbox" | "playstation" | "nintendo";
export type Device = { kind: "keyboard" } | { kind: "gamepad"; style: PadStyle };

/** A pad's icon set from its name; unknown pads get the platform's usual one. */
export function padStyle(name: string): PadStyle {
  const n = name.toLowerCase();
  if (/dualsense|dualshock|playstation|sony|ps[345]\b|wireless controller/.test(n)) return "playstation";
  if (/nintendo|switch|pro controller|joy-?con/.test(n)) return "nintendo";
  if (/xbox|xinput|microsoft/.test(n)) return "xbox";
  // Windows and Linux (Steam Deck included) use the Xbox layout as their generic pad.
  return "xbox";
}

/** The value after `--flag` in the script arguments. */
export function arg(flag: string): string | undefined {
  const i = Application.args.indexOf(flag);
  return i >= 0 ? Application.args[i + 1] : undefined;
}

// `--pad xbox|playstation|nintendo` starts in gamepad mode with those icons (for screenshots).
const forced = (["xbox", "playstation", "nintendo"] as const).find((s) => s === arg("--pad"));
let device: Device = forced ? { kind: "gamepad", style: forced } : { kind: "keyboard" };
const listeners = new Set<() => void>();

function use(next: Device) {
  if (next.kind === device.kind && (next.kind === "keyboard" || (device.kind === "gamepad" && next.style === device.style))) return;
  device = next;
  Cursor.visible = next.kind === "keyboard";
  for (const fn of listeners) fn();
}

/** The mouse moved or clicked: back to keyboard and mouse prompts. */
export function noteMouse() {
  use({ kind: "keyboard" });
}

input.on("keydown", () => use({ kind: "keyboard" }));
input.on("gamepadbuttondown", (e) => use({ kind: "gamepad", style: padStyle(e.gamepad.name) }));
input.on("gamepadaxis", (e) => {
  // Past the stick's drift, not every tiny reading.
  if (Math.abs(e.value) > 0.5) use({ kind: "gamepad", style: padStyle(e.gamepad.name) });
});
if (device.kind === "gamepad") Cursor.visible = false;

export function useDevice(): Device {
  return useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    () => device,
  );
}

// --- Icons -----------------------------------------------------------------------------

export type Action = "browse" | "move" | "ok" | "back" | "view";

const MAC = Application.platform === "MacOSX";

/** Face buttons by position, as each maker labels and colours them. */
const FACE: Record<PadStyle, Record<"South" | "East" | "West" | "North", { label: string; color: string }>> = {
  xbox: {
    South: { label: "A", color: "#6cc24a" },
    East: { label: "B", color: "#ef4f43" },
    West: { label: "X", color: "#3d8af7" },
    North: { label: "Y", color: "#f5c242" },
  },
  // PlayStation's symbols are drawn (`PsSymbol`); the colours are the classic ones.
  playstation: {
    South: { label: "cross", color: "#8ab4ff" },
    East: { label: "circle", color: "#ff6f6f" },
    West: { label: "square", color: "#f39ad6" },
    North: { label: "triangle", color: "#4fd9b0" },
  },
  // Nintendo swaps the letters: the bottom button is B, the right one A.
  nintendo: {
    South: { label: "B", color: "#e8e8e8" },
    East: { label: "A", color: "#e8e8e8" },
    West: { label: "Y", color: "#e8e8e8" },
    North: { label: "X", color: "#e8e8e8" },
  },
};
/** Confirm and back by position: Nintendo players confirm with A, the right-hand button. */
export function okButton(style: PadStyle): "South" | "East" {
  return style === "nintendo" ? "East" : "South";
}
export function backButton(style: PadStyle): "South" | "East" {
  return style === "nintendo" ? "South" : "East";
}

const BUMPERS: Record<PadStyle, [string, string]> = {
  xbox: ["LB", "RB"],
  playstation: ["L1", "R1"],
  nintendo: ["L", "R"],
};

function PsSymbol({ shape, color }: { shape: string; color: string }) {
  const stroke = { fill: "none", stroke: color, strokeWidth: 2.2, strokeLinejoin: "round" as const, strokeLinecap: "round" as const };
  return (
    <svg viewBox="0 0 20 20" style={{ width: 14, height: 14 }}>
      {shape === "cross" ? <path d="M4.5 4.5 L15.5 15.5 M15.5 4.5 L4.5 15.5" {...stroke} /> : null}
      {shape === "circle" ? <circle cx={10} cy={10} r={6.2} {...stroke} /> : null}
      {shape === "square" ? <path d="M4.5 4.5 H15.5 V15.5 H4.5 Z" {...stroke} /> : null}
      {shape === "triangle" ? <path d="M10 3.5 L16.5 15 H3.5 Z" {...stroke} /> : null}
    </svg>
  );
}

export function FaceButton({ button, style }: { button: "South" | "East" | "West" | "North"; style: PadStyle }) {
  const face = FACE[style][button];
  return (
    <box className="glyph face">
      {style === "playstation" ? (
        <PsSymbol shape={face.label} color={face.color} />
      ) : (
        <Text className="face-label" style={{ color: face.color }}>
          {face.label}
        </Text>
      )}
    </box>
  );
}

function Bumper({ label }: { label: string }) {
  return (
    <box className="glyph bumper">
      <Text>{label}</Text>
    </box>
  );
}

/** The d-pad, with the directions in use lit. */
function DPad({ lit }: { lit: "horizontal" | "all" }) {
  const on = "#ffffff";
  const off = "rgba(255,255,255,0.35)";
  const v = lit === "all" ? on : off;
  return (
    <box className="glyph dpad">
      <svg viewBox="0 0 24 24" style={{ width: 22, height: 22 }}>
        <path d="M9 2.5 H15 V9 H9 Z" fill={v} />
        <path d="M9 15 H15 V21.5 H9 Z" fill={v} />
        <path d="M2.5 9 H9 V15 H2.5 Z" fill={on} />
        <path d="M15 9 H21.5 V15 H15 Z" fill={on} />
        <path d="M9 9 H15 V15 H9 Z" fill="rgba(255,255,255,0.6)" />
      </svg>
    </box>
  );
}

export function Key({ children }: { children: ReactNode }) {
  return (
    <box className="glyph key">
      <Text>{children}</Text>
    </box>
  );
}

/** The glyphs for an action on the current device. */
export function Glyphs({ action, device }: { action: Action; device: Device }) {
  if (device.kind === "keyboard") {
    switch (action) {
      case "browse":
        return (
          <>
            <Key>←</Key>
            <Key>→</Key>
          </>
        );
      case "move":
        return (
          <>
            <Key>←</Key>
            <Key>↑</Key>
            <Key>↓</Key>
            <Key>→</Key>
          </>
        );
      case "ok":
        return <Key>{MAC ? "Return" : "Enter"}</Key>;
      case "back":
        return <Key>Esc</Key>;
      case "view":
        return <Key>G</Key>;
    }
  }
  const s = device.style;
  switch (action) {
    case "browse":
      return (
        <>
          <DPad lit="horizontal" />
          <Bumper label={BUMPERS[s][0]} />
          <Bumper label={BUMPERS[s][1]} />
        </>
      );
    case "move":
      return <DPad lit="all" />;
    case "ok":
      return <FaceButton button={okButton(s)} style={s} />;
    case "back":
      return <FaceButton button={backButton(s)} style={s} />;
    case "view":
      return <FaceButton button="North" style={s} />;
  }
}

/** One prompt: glyphs and what they do. */
export function Hint({ action, label, device }: { action: Action; label: string; device: Device }) {
  return (
    <box className="hint">
      <Glyphs action={action} device={device} />
      <Text className="hint-label">{label}</Text>
    </box>
  );
}

/** A short name for the device, shown in the top bar. */
export function deviceName(device: Device): string {
  if (device.kind === "keyboard") return "Keyboard & mouse";
  return { xbox: "Xbox controller", playstation: "PlayStation controller", nintendo: "Nintendo controller" }[device.style];
}
