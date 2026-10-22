// A game launcher mock-up, to see whether a store / launcher front end works in Iron's UI: a
// cover-flow carousel of the examples' screenshots (gen-games.ts), browsed with the mouse
// (arrows, clicks, wheel), the keys (←/→, A/D) or a gamepad (d-pad, left stick). Opening one
// shows its details and a Play button; playing shows a loading spinner, then says launching
// isn't implemented yet.
//
// `iron examples/launcher -- --select <example> --show details|loading|alert` starts on that
// game and screen (for screenshot runs).
import "./launcher.css";

import { Application, input } from "@iron/engine";
import { createRoot, Image, type IronWheelEvent, Text } from "@iron/react";
import { useEffect, useRef, useState } from "react";

import { type Game, GAMES } from "./games.gen.ts";

/** How long "loading" spins before the alert. */
const LOADING_MS = 1100;
/** Left stick: past this it browses, repeating while held. */
const STICK = 0.55;
const REPEAT_FIRST_MS = 380;
const REPEAT_MS = 140;

type Mode = "browse" | "details" | "loading" | "alert";
const MODES: Mode[] = ["browse", "details", "loading", "alert"];

/** The value after `--flag` in the script arguments. */
function arg(flag: string): string | undefined {
  const i = Application.args.indexOf(flag);
  return i >= 0 ? Application.args[i + 1] : undefined;
}
const START_INDEX = Math.max(0, GAMES.findIndex((g) => g.name === arg("--select")));
const START_MODE = MODES.find((m) => m === arg("--show")) ?? "browse";

/** Where card `d` places from the selected one sits: the cover-flow shape. */
function cardStyle(d: number) {
  const side = Math.sign(d);
  const far = Math.abs(d);
  const x = far === 0 ? 0 : side * (250 + (far - 1) * 130);
  // Every state uses the same function list, so transitions blend between them.
  return {
    transform: `translateX(${x}px) rotateY(${-side * (far === 0 ? 0 : 50)}deg) scale(${far === 0 ? 1 : 0.82})`,
    opacity: far > 4 ? 0 : 1,
    zIndex: 100 - far,
  };
}

function Shot({ game, className }: { game: Game; className: string }) {
  if (game.shot) return <Image className={className} src={game.shot} />;
  // No screenshot yet: the title on a colour picked from the name.
  let hue = 0;
  for (const c of game.name) hue = (hue * 31 + c.charCodeAt(0)) % 360;
  return (
    <box className={`${className} placeholder`} style={{ background: `linear-gradient(135deg, hsl(${hue} 45% 32%), hsl(${(hue + 50) % 360} 50% 14%))` }}>
      <Text className="placeholder-title">{game.title}</Text>
    </box>
  );
}

function Spinner() {
  return (
    <svg className="spinner" viewBox="0 0 48 48">
      <circle cx={24} cy={24} r={20} fill="none" stroke="rgba(255,255,255,0.15)" strokeWidth={4} />
      <path d="M24 4 A20 20 0 0 1 44 24" fill="none" stroke="#7cc4ff" strokeWidth={4} strokeLinecap="round" />
    </svg>
  );
}

function App() {
  const [index, setIndex] = useState(START_INDEX);
  const [mode, setMode] = useState<Mode>(START_MODE);
  const game = GAMES[index]!;

  const move = (by: number) => setIndex((i) => Math.max(0, Math.min(GAMES.length - 1, i + by)));
  const back = () => setMode((m) => (m === "alert" || m === "details" ? (m === "alert" ? "details" : "browse") : m));
  const confirm = () => {
    if (mode === "browse") setMode("details");
    else if (mode === "details") setMode("loading");
    else if (mode === "alert") setMode("details");
  };

  // Loading spins for a moment, then admits it can't start anything.
  useEffect(() => {
    if (mode !== "loading") return;
    const t = setTimeout(() => setMode("alert"), LOADING_MS);
    return () => clearTimeout(t);
  }, [mode]);

  // Keys and pad buttons. The handlers change every render, so listeners call the latest.
  const actions = useRef({ move, confirm, back, mode });
  actions.current = { move, confirm, back, mode };
  useEffect(() => {
    const act = (what: "left" | "right" | "ok" | "back") => {
      const a = actions.current;
      if (what === "ok") a.confirm();
      else if (what === "back") a.back();
      else if (a.mode === "browse") a.move(what === "left" ? -1 : 1);
    };
    const offKeys = input.on("keydown", (e) => {
      if (e.code === "ArrowLeft" || e.code === "KeyA") act("left");
      else if (e.code === "ArrowRight" || e.code === "KeyD") act("right");
      else if (!e.repeat && (e.code === "Enter" || e.code === "Space")) act("ok");
      else if (!e.repeat && (e.code === "Escape" || e.code === "Backspace")) act("back");
    });
    const offPad = input.on("gamepadbuttondown", (e) => {
      if (e.button === "DPadLeft" || e.button === "LeftBumper") act("left");
      else if (e.button === "DPadRight" || e.button === "RightBumper") act("right");
      else if (e.button === "South" || e.button === "Start") act("ok");
      else if (e.button === "East") act("back");
    });
    // The stick browses once when pushed, then repeats while held.
    let held = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const repeat = (wait: number) => {
      timer = setTimeout(() => {
        act(held < 0 ? "left" : "right");
        repeat(REPEAT_MS);
      }, wait);
    };
    const offStick = input.on("gamepadaxis", (e) => {
      if (e.axis !== "LeftStickX") return;
      const dir = e.value > STICK ? 1 : e.value < -STICK ? -1 : 0;
      if (dir === held) return;
      held = dir;
      clearTimeout(timer);
      if (dir) {
        act(dir < 0 ? "left" : "right");
        repeat(REPEAT_FIRST_MS);
      }
    });
    return () => {
      offKeys();
      offPad();
      offStick();
      clearTimeout(timer);
    };
  }, []);

  return (
    <box className="app">
      {/* The selected game's screenshot, large and dim, fading in on each change. */}
      <box key={game.name} className="backdrop">
        <Shot game={game} className="backdrop-shot" />
      </box>
      <box className="shade" />

      <box className="top">
        <Text className="logo">IRON</Text>
        <Text className="logo-sub">examples</Text>
        <box className="grow" />
        <Text className="count">{`${index + 1} / ${GAMES.length}`}</Text>
      </box>

      <box
        className="stage"
        onWheel={(e: IronWheelEvent) => {
          if (mode === "browse") move(Math.sign(e.deltaY || e.deltaX));
        }}
      >
        <box className="track">
          {GAMES.map((g, i) => {
            const d = i - index;
            return (
              <box
                key={g.name}
                className={d === 0 ? "card selected" : "card"}
                style={cardStyle(d)}
                onClick={() => (d === 0 ? setMode("details") : setIndex(i))}
              >
                {Math.abs(d) <= 5 ? <Shot game={g} className="card-shot" /> : null}
              </box>
            );
          })}
        </box>
        <box className="arrow left" disabled={index === 0} onClick={() => move(-1)}>
          <Text>{"‹"}</Text>
        </box>
        <box className="arrow right" disabled={index === GAMES.length - 1} onClick={() => move(1)}>
          <Text>{"›"}</Text>
        </box>
      </box>

      <box key={`caption-${game.name}`} className="caption">
        <Text className="title">{game.title}</Text>
        <Text className="folder">{`examples/${game.name}`}</Text>
      </box>

      <box className="hints">
        <Hint keys="‹ ›" label="Browse" />
        <Hint keys="A / Enter" label="Open" />
        <Hint keys="B / Esc" label="Back" />
      </box>

      {mode !== "browse" ? (
        <box className="scrim" onClick={() => mode === "details" && setMode("browse")}>
          <box className="details" onClick={(e) => e.stopPropagation()}>
            <Shot game={game} className="details-shot" />
            <box className="details-info">
              <Text className="details-title">{game.title}</Text>
              <Text className="folder">{`examples/${game.name}`}</Text>
              <Text className={game.description ? "description" : "description none"}>
                {game.description || "No description yet."}
              </Text>
              <box className="grow" />
              <box className="buttons">
                <box className="button play" onClick={() => setMode("loading")}>
                  <svg viewBox="0 0 16 16" style={{ width: 14, height: 14 }}>
                    <path d="M4 2.5 L13 8 L4 13.5 Z" fill="currentColor" />
                  </svg>
                  <Text>Play</Text>
                </box>
                <box className="button" onClick={() => setMode("browse")}>
                  <Text>Back</Text>
                </box>
              </box>
            </box>
          </box>
        </box>
      ) : null}

      {mode === "loading" ? (
        <box className="loading">
          <Spinner />
          <Text className="loading-text">{`Starting ${game.title}…`}</Text>
          <box className="bar">
            <box className="bar-fill" />
          </box>
        </box>
      ) : null}

      {mode === "alert" ? (
        <box className="alert-scrim">
          <box className="alert">
            <Text className="alert-title">{`Can't start ${game.title}`}</Text>
            <Text className="alert-body">Launching games from here isn't implemented yet. Run it with the iron binary instead.</Text>
            <Text className="alert-code">{`iron examples/${game.name}`}</Text>
            <box className="buttons end">
              <box className="button play" onClick={() => setMode("details")}>
                <Text>OK</Text>
              </box>
            </box>
          </box>
        </box>
      ) : null}
    </box>
  );
}

function Hint({ keys, label }: { keys: string; label: string }) {
  return (
    <box className="hint">
      <Text className="key">{keys}</Text>
      <Text className="hint-label">{label}</Text>
    </box>
  );
}

createRoot().render(<App />);
