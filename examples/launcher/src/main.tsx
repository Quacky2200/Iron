// A game launcher mock-up, to see whether a store / launcher front end works in Iron's UI: the
// examples' screenshots (gen-games.ts) as a cover-flow carousel or a grid, browsed with the
// mouse (arrows, clicks, wheel), the keys (arrows, WASD) or a gamepad (d-pad, bumpers, left
// stick). Button prompts follow the last device used (prompts.tsx). Opening a game shows its
// details and a Play button; playing shows a loading spinner, then says launching isn't
// implemented yet.
//
// `iron examples/launcher -- --select <example> --show details|loading|alert --view grid
// --pad xbox|playstation|nintendo` starts on that game, screen, view and pad icons (for
// screenshot runs).
import "./launcher.css";

import { input, PlayerPrefs, Screen } from "@iron/engine";
import { createRoot, Image, type IronElement, type IronWheelEvent, Text } from "@iron/react";
import { useEffect, useRef, useState } from "react";

import { type Game, GAMES } from "./games.gen.ts";
import { arg, backButton, deviceName, type Device, FaceButton, Hint, noteMouse, okButton, padStyle, useDevice } from "./prompts.tsx";

/** How long "loading" spins before the alert. */
const LOADING_MS = 1100;
/** Left stick: past this it moves, repeating while held. */
const STICK = 0.55;
const REPEAT_FIRST_MS = 380;
const REPEAT_MS = 140;
/** Grid tiles: width and gap in UI pixels (launcher.css matches), and the side margins. */
const TILE = 210;
const GAP = 24;
const MARGIN = 48;

type Mode = "browse" | "details" | "loading" | "alert";
type View = "carousel" | "grid";
type Act = "left" | "right" | "up" | "down" | "ok" | "back" | "view";
const MODES: Mode[] = ["browse", "details", "loading", "alert"];

const START_INDEX = Math.max(0, GAMES.findIndex((g) => g.name === arg("--select")));
const START_MODE = MODES.find((m) => m === arg("--show")) ?? "browse";
const START_VIEW: View = (arg("--view") ?? PlayerPrefs.getString("view", "carousel")) === "grid" ? "grid" : "carousel";

/** How many tiles fit across the window. */
const gridColumns = () => Math.max(1, Math.floor((Screen.width - 2 * MARGIN + GAP) / (TILE + GAP)));

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

function CarouselIcon() {
  return (
    <svg viewBox="0 0 20 20" style={{ width: 18, height: 18 }}>
      <path d="M6 4 H14 V16 H6 Z" fill="currentColor" />
      <path d="M2 6 H4 V14 H2 Z M16 6 H18 V14 H16 Z" fill="currentColor" opacity={0.55} />
    </svg>
  );
}

function GridIcon() {
  return (
    <svg viewBox="0 0 20 20" style={{ width: 18, height: 18 }}>
      <path d="M3 3 H9 V9 H3 Z M11 3 H17 V9 H11 Z M3 11 H9 V17 H3 Z M11 11 H17 V17 H11 Z" fill="currentColor" />
    </svg>
  );
}

function Carousel({ index, select, open }: { index: number; select: (i: number) => void; open: () => void }) {
  return (
    <box className="track">
      {GAMES.map((g, i) => {
        const d = i - index;
        return (
          <box key={g.name} className={d === 0 ? "card selected" : "card"} style={cardStyle(d)} onClick={() => (d === 0 ? open() : select(i))}>
            {Math.abs(d) <= 5 ? <Shot game={g} className="card-shot" /> : null}
          </box>
        );
      })}
    </box>
  );
}

function Grid({ index, columns, select, open }: { index: number; columns: number; select: (i: number) => void; open: () => void }) {
  const selected = useRef<IronElement>(null);
  useEffect(() => {
    selected.current?.scrollIntoView({ block: "nearest" });
  }, [index, columns]);
  return (
    <box className="grid-scroll">
      <box className="grid" style={{ width: columns * (TILE + GAP) - GAP }}>
        {GAMES.map((g, i) => (
          <box
            key={g.name}
            ref={i === index ? selected : undefined}
            className={i === index ? "tile selected" : "tile"}
            // Tiles rise in row by row when the grid appears.
            style={{ animationDelay: `${Math.min(Math.floor(i / columns), 6) * 60 + (i % columns) * 25}ms` }}
            onClick={() => {
              select(i);
              open();
            }}
          >
            <box className="tile-frame">
              <Shot game={g} className="tile-shot" />
            </box>
            <Text className="tile-title">{g.title}</Text>
          </box>
        ))}
      </box>
    </box>
  );
}

/** The prompts along the bottom for the current screen. */
function Hints({ mode, view, device }: { mode: Mode; view: View; device: Device }) {
  const key = `${device.kind}-${device.kind === "gamepad" ? device.style : ""}-${mode}-${view}`;
  return (
    <box key={key} className="hints">
      {mode === "browse" ? (
        <>
          <Hint action={view === "grid" ? "move" : "browse"} label={view === "grid" ? "Move" : "Browse"} device={device} />
          <Hint action="ok" label="Open" device={device} />
          <Hint action="view" label={view === "grid" ? "Carousel" : "Grid"} device={device} />
        </>
      ) : null}
      {mode === "details" ? (
        <>
          <Hint action="ok" label="Play" device={device} />
          <Hint action="back" label="Back" device={device} />
        </>
      ) : null}
      {mode === "alert" ? <Hint action="ok" label="OK" device={device} /> : null}
    </box>
  );
}

function App() {
  const [index, setIndex] = useState(START_INDEX);
  const [mode, setMode] = useState<Mode>(START_MODE);
  const [view, setViewState] = useState<View>(START_VIEW);
  const [columns, setColumns] = useState(gridColumns);
  const device = useDevice();
  const pad = device.kind === "gamepad" ? device.style : null;
  const game = GAMES[index]!;

  // The grid follows the window's width.
  useEffect(() => {
    let t: ReturnType<typeof setTimeout>;
    const poll = () => {
      setColumns(gridColumns());
      t = setTimeout(poll, 250);
    };
    poll();
    return () => clearTimeout(t);
  }, []);

  const setView = (v: View) => {
    setViewState(v);
    PlayerPrefs.setString("view", v);
  };
  const clamp = (i: number) => Math.max(0, Math.min(GAMES.length - 1, i));
  const move = (by: number) => setIndex((i) => clamp(i + by));

  // Loading spins for a moment, then admits it can't start anything.
  useEffect(() => {
    if (mode !== "loading") return;
    const t = setTimeout(() => setMode("alert"), LOADING_MS);
    return () => clearTimeout(t);
  }, [mode]);

  // Keys, pad buttons and the stick all come here. Listeners are set up once, so they call
  // the latest render's handler.
  const handler = useRef<(a: Act) => void>(() => {});
  handler.current = (a: Act) => {
    if (mode === "browse") {
      if (a === "left") move(-1);
      else if (a === "right") move(1);
      else if (view === "grid" && a === "up") setIndex((i) => (i - columns >= 0 ? i - columns : i));
      else if (view === "grid" && a === "down") setIndex((i) => (i + columns < GAMES.length ? i + columns : Math.floor(i / columns) < Math.floor((GAMES.length - 1) / columns) ? GAMES.length - 1 : i));
      else if (a === "ok") setMode("details");
      else if (a === "view") setView(view === "grid" ? "carousel" : "grid");
    } else if (mode === "details") {
      if (a === "ok") setMode("loading");
      else if (a === "back") setMode("browse");
    } else if (mode === "alert") {
      if (a === "ok" || a === "back") setMode("details");
    }
  };
  useEffect(() => {
    const act = (a: Act) => handler.current(a);
    const KEYS: Record<string, Act> = {
      ArrowLeft: "left",
      KeyA: "left",
      ArrowRight: "right",
      KeyD: "right",
      ArrowUp: "up",
      KeyW: "up",
      ArrowDown: "down",
      KeyS: "down",
      Enter: "ok",
      Space: "ok",
      Escape: "back",
      Backspace: "back",
      KeyG: "view",
    };
    const PAD: Record<string, Act> = {
      DPadLeft: "left",
      LeftBumper: "left",
      DPadRight: "right",
      RightBumper: "right",
      DPadUp: "up",
      DPadDown: "down",
      Start: "ok",
      North: "view",
    };
    const offKeys = input.on("keydown", (e) => {
      const a = KEYS[e.code];
      // Held arrows repeat; the rest act once per press.
      if (a && (!e.repeat || a === "left" || a === "right" || a === "up" || a === "down")) act(a);
    });
    const offPad = input.on("gamepadbuttondown", (e) => {
      const style = padStyle(e.gamepad.name);
      const a = e.button === okButton(style) ? "ok" : e.button === backButton(style) ? "back" : PAD[e.button];
      if (a) act(a);
    });
    // The stick moves once when pushed, then repeats while held.
    const stick = { x: 0, y: 0 };
    let held: Act | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const repeat = (wait: number) => {
      timer = setTimeout(() => {
        if (held) act(held);
        repeat(REPEAT_MS);
      }, wait);
    };
    const offStick = input.on("gamepadaxis", (e) => {
      if (e.axis === "LeftStickX") stick.x = e.value;
      else if (e.axis === "LeftStickY") stick.y = e.value;
      else return;
      const { x, y } = stick;
      // Up is positive on the stick.
      const dir: Act | null =
        Math.abs(x) > STICK && Math.abs(x) >= Math.abs(y) ? (x < 0 ? "left" : "right") : Math.abs(y) > STICK ? (y > 0 ? "up" : "down") : null;
      if (dir === held) return;
      held = dir;
      clearTimeout(timer);
      if (dir) {
        act(dir);
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
    <box className={pad ? "app pad" : "app"} onPointerMove={noteMouse} onPointerDown={noteMouse}>
      {/* The selected game's screenshot, large and dim, fading in on each change. */}
      <box key={game.name} className="backdrop">
        <Shot game={game} className="backdrop-shot" />
      </box>
      <box className="shade" />

      <box className="top">
        <Text className="logo">IRON</Text>
        <Text className="logo-sub">examples</Text>
        <box className="grow" />
        <Text key={deviceName(device)} className="device">
          {deviceName(device)}
        </Text>
        <box className="views">
          <box className={view === "carousel" ? "view-button on" : "view-button"} onClick={() => setView("carousel")}>
            <CarouselIcon />
          </box>
          <box className={view === "grid" ? "view-button on" : "view-button"} onClick={() => setView("grid")}>
            <GridIcon />
          </box>
        </box>
        <Text className="count">{`${index + 1} / ${GAMES.length}`}</Text>
      </box>

      {view === "carousel" ? (
        <box key="carousel" className="view carousel-view">
          <box
            className="stage"
            onWheel={(e: IronWheelEvent) => {
              if (mode === "browse") move(Math.sign(e.deltaY || e.deltaX));
            }}
          >
            <Carousel index={index} select={setIndex} open={() => setMode("details")} />
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
        </box>
      ) : (
        <box key="grid" className="view grid-view">
          <Grid index={index} columns={columns} select={setIndex} open={() => setMode("details")} />
        </box>
      )}

      <Hints mode={mode} view={view} device={device} />

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
                  {pad ? (
                    <FaceButton button={okButton(pad)} style={pad} />
                  ) : (
                    <svg viewBox="0 0 16 16" style={{ width: 14, height: 14 }}>
                      <path d="M4 2.5 L13 8 L4 13.5 Z" fill="currentColor" />
                    </svg>
                  )}
                  <Text>Play</Text>
                </box>
                <box className="button" onClick={() => setMode("browse")}>
                  {pad ? <FaceButton button={backButton(pad)} style={pad} /> : null}
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
                {pad ? <FaceButton button={okButton(pad)} style={pad} /> : null}
                <Text>OK</Text>
              </box>
            </box>
          </box>
        </box>
      ) : null}
    </box>
  );
}

createRoot().render(<App />);
