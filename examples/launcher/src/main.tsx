// A game launcher mock-up, to see whether a store / launcher front end works in Iron's UI: the
// examples' screenshots (gen-games.ts) as a cover-flow carousel or a grid, browsed with the
// mouse (arrows, clicks, wheel), the keys (arrows, WASD) or a gamepad (d-pad, bumpers, left
// stick). Button prompts follow the last device used (prompts.tsx). A sidebar menu (Menu /
// Esc / the ☰ button) has the views, settings and Exit; a context menu (Y / C / right-click)
// has the selected game's actions. Both views show all games or just the favourites (the
// tabs, LT / RT, Tab). Opening a game shows its details and a Play button;
// playing shows a loading spinner, then says launching isn't implemented yet.
//
// `iron examples/launcher -- --select <example> --show details|loading|alert|menu|context
// --view grid --category favourites --favourites a,b --pad xbox|playstation|nintendo` starts on that game,
// screen, view, category and pad icons (for screenshot runs).
import "./launcher.css";

import { Application, input, PlayerPrefs, Screen } from "@iron/engine";
import { createRoot, Image, type IronElement, type IronWheelEvent, Text } from "@iron/react";
import { useEffect, useRef, useState } from "react";

import { type Game, GAMES } from "./games.gen.ts";
import {
  arg,
  backButton,
  type Device,
  deviceName,
  FaceButton,
  Glyphs,
  Hint,
  noteMouse,
  Trigger,
  okButton,
  padStyle,
  useDevice,
} from "./prompts.tsx";

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

type Mode = "browse" | "details" | "loading" | "alert" | "menu" | "context";
type View = "carousel" | "grid";
type Category = "all" | "favourites";
/** In tab order, left to right. */
const CATEGORIES: Category[] = ["all", "favourites"];
type Act = "left" | "right" | "up" | "down" | "ok" | "back" | "view" | "options" | "menu" | "escape" | "prevCategory" | "nextCategory";
interface Alert {
  title: string;
  body: string;
  code?: string;
  /** Where OK goes back to. */
  then: Mode;
}
interface MenuItem {
  label: string;
  /** An `ICONS` path. */
  icon: string;
  run: () => void;
  danger?: boolean;
}

const MODES: Mode[] = ["browse", "details", "loading", "alert", "menu", "context"];
const START_CATEGORY: Category =
  (arg("--category") ?? PlayerPrefs.getString("category", "all")) === "favourites" ? "favourites" : "all";
const START_NAME = arg("--select");
const START_MODE = MODES.find((m) => m === arg("--show")) ?? "browse";
const START_VIEW: View = (arg("--view") ?? PlayerPrefs.getString("view", "carousel")) === "grid" ? "grid" : "carousel";

/** `--show alert` starts on the alert Play leads to. */
const cantStart = (game: Game): Alert => ({
  title: `Can't start ${game.title}`,
  body: "Launching games from here isn't implemented yet. Run it with the iron binary instead.",
  code: `iron examples/${game.name}`,
  then: "details",
});

/** How many tiles fit across the window. */
const gridColumns = () => Math.max(1, Math.floor((Screen.width - 2 * MARGIN + GAP) / (TILE + GAP)));

/** Favourite games by folder name (`--favourites a,b` overrides the saved ones). */
const loadFavourites = () =>
  new Set((arg("--favourites") ?? PlayerPrefs.getString("favourites", "")).split(",").filter(Boolean));

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

// --- Small pieces ----------------------------------------------------------------------

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

function Star({ size = 14 }: { size?: number }) {
  return (
    <svg viewBox="0 0 20 20" style={{ width: size, height: size }}>
      <path d="M10 1.8 L12.5 7.2 L18.4 7.8 L14 11.8 L15.2 17.6 L10 14.7 L4.8 17.6 L6 11.8 L1.6 7.8 L7.5 7.2 Z" fill="currentColor" />
    </svg>
  );
}

function Icon({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 20 20" style={{ width: 18, height: 18 }}>
      <path d={d} fill="none" stroke="currentColor" strokeWidth={1.7} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
const ICONS = {
  library: "M3 4 H8 V16 H3 Z M10 4 H17 V9 H10 Z M10 11 H17 V16 H10 Z",
  carousel: "M6 4 H14 V16 H6 Z M2.5 6.5 V13.5 M17.5 6.5 V13.5",
  grid: "M3 3 H9 V9 H3 Z M11 3 H17 V9 H11 Z M3 11 H9 V17 H3 Z M11 11 H17 V17 H11 Z",
  settings: "M10 6.5 A3.5 3.5 0 1 0 10.01 6.5 Z M10 1.5 V4 M10 16 V18.5 M1.5 10 H4 M16 10 H18.5 M4 4 L5.8 5.8 M14.2 14.2 L16 16 M4 16 L5.8 14.2 M14.2 5.8 L16 4",
  exit: "M8 3 H4 V17 H8 M12 6 L16 10 L12 14 M16 10 H7",
  play: "M6 3.5 L16 10 L6 16.5 Z",
  info: "M10 18 A8 8 0 1 0 10 2 A8 8 0 1 0 10 18 Z M10 9 V14 M10 6 V6.2",
  star: "M10 2.5 L12.2 7.4 L17.5 7.9 L13.5 11.5 L14.6 16.7 L10 14.1 L5.4 16.7 L6.5 11.5 L2.5 7.9 L7.8 7.4 Z",
  folder: "M2.5 5 H8 L9.5 7 H17.5 V16 H2.5 Z",
};

function Spinner() {
  return (
    <svg className="spinner" viewBox="0 0 48 48">
      <circle cx={24} cy={24} r={20} fill="none" stroke="rgba(255,255,255,0.15)" strokeWidth={4} />
      <path d="M24 4 A20 20 0 0 1 44 24" fill="none" stroke="#7cc4ff" strokeWidth={4} strokeLinecap="round" />
    </svg>
  );
}

function Burger() {
  return (
    <box className="burger">
      <box />
      <box />
      <box />
    </box>
  );
}

/** A vertical menu: the pad / keys move `focus`, the mouse hovers it. */
function Menu({ items, focus, setFocus }: { items: MenuItem[]; focus: number; setFocus: (i: number) => void }) {
  return (
    <box className="menu-items">
      {items.map((item, i) => (
        <box
          key={item.label}
          className={`menu-item${i === focus ? " focused" : ""}${item.danger ? " danger" : ""}`}
          onPointerEnter={() => setFocus(i)}
          onClick={item.run}
        >
          <Icon d={item.icon} />
          <Text>{item.label}</Text>
        </box>
      ))}
    </box>
  );
}

// --- Views -----------------------------------------------------------------------------

interface ViewProps {
  games: Game[];
  index: number;
  favourites: Set<string>;
  select: (i: number) => void;
  open: () => void;
  context: (i: number) => void;
}

function Carousel({ games, index, favourites, select, open, context }: ViewProps) {
  return (
    <box className="track">
      {games.map((g, i) => {
        const d = i - index;
        return (
          <box
            key={g.name}
            className={d === 0 ? "card selected" : "card"}
            style={cardStyle(d)}
            onClick={() => (d === 0 ? open() : select(i))}
            onContextMenu={() => context(i)}
          >
            {Math.abs(d) <= 5 ? <Shot game={g} className="card-shot" /> : null}
            {favourites.has(g.name) ? (
              <box className="fav">
                <Star />
              </box>
            ) : null}
          </box>
        );
      })}
    </box>
  );
}

function Grid({ games, index, columns, favourites, select, open, context }: ViewProps & { columns: number }) {
  const selected = useRef<IronElement>(null);
  useEffect(() => {
    selected.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [index, columns]);
  return (
    <box className="grid-scroll">
      <box className="grid" style={{ width: columns * (TILE + GAP) - GAP }}>
        {games.map((g, i) => (
          // The cell's padding leaves room for the lifted tile when it scrolls into view.
          <box key={g.name} className="cell" ref={i === index ? selected : undefined}>
            <box
              className={i === index ? "tile selected" : "tile"}
              // Tiles rise in row by row when the grid appears.
              style={{ animationDelay: `${Math.min(Math.floor(i / columns), 6) * 60 + (i % columns) * 25}ms` }}
              onClick={() => {
                select(i);
                open();
              }}
              onContextMenu={() => context(i)}
            >
              <box className="tile-frame">
                <Shot game={g} className="tile-shot" />
                {favourites.has(g.name) ? (
                  <box className="fav">
                    <Star />
                  </box>
                ) : null}
              </box>
              <Text className="tile-title">{g.title}</Text>
            </box>
          </box>
        ))}
      </box>
    </box>
  );
}

/** The prompts along the bottom for the current screen. */
function Hints({ mode, view, device, empty }: { mode: Mode; view: View; device: Device; empty: boolean }) {
  const key = `${device.kind}-${device.kind === "gamepad" ? device.style : ""}-${mode}-${view}-${empty}`;
  return (
    <box key={key} className="hints">
      {mode === "browse" && empty ? (
        <>
          <Hint action="category" label="Category" device={device} />
          <Hint action="menu" label="Menu" device={device} />
        </>
      ) : null}
      {mode === "browse" && !empty ? (
        <>
          <Hint action={view === "grid" ? "move" : "browse"} label={view === "grid" ? "Move" : "Browse"} device={device} />
          <Hint action="ok" label="Open" device={device} />
          <Hint action="options" label="Options" device={device} />
          <Hint action="view" label={view === "grid" ? "Carousel" : "Grid"} device={device} />
          <Hint action="category" label="Category" device={device} />
          <Hint action="menu" label="Menu" device={device} />
        </>
      ) : null}
      {mode === "details" ? (
        <>
          <Hint action="ok" label="Play" device={device} />
          <Hint action="options" label="Options" device={device} />
          <Hint action="back" label="Back" device={device} />
        </>
      ) : null}
      {mode === "menu" || mode === "context" ? (
        <>
          <Hint action="move" label="Choose" device={device} />
          <Hint action="ok" label="Select" device={device} />
          <Hint action="back" label="Close" device={device} />
        </>
      ) : null}
      {mode === "alert" ? <Hint action="ok" label="OK" device={device} /> : null}
    </box>
  );
}

// --- The app ---------------------------------------------------------------------------

function App() {
  const [category, setCategoryState] = useState<Category>(START_CATEGORY);
  const [favourites, setFavourites] = useState(loadFavourites);
  const list = category === "favourites" ? GAMES.filter((g) => favourites.has(g.name)) : GAMES;
  const [index, setIndex] = useState(() => Math.max(0, list.findIndex((g) => g.name === START_NAME)));
  const [mode, setMode] = useState<Mode>(START_MODE);
  const [view, setViewState] = useState<View>(START_VIEW);
  const [columns, setColumns] = useState(gridColumns);
  const [focus, setFocus] = useState(0);
  /** Where the context menu was opened from, to go back there. */
  const [contextFrom, setContextFrom] = useState<Mode>("browse");
  const [alert, setAlert] = useState<Alert | null>(() => (START_MODE === "alert" && list[index] ? cantStart(list[index]!) : null));
  const device = useDevice();
  const pad = device.kind === "gamepad" ? device.style : null;
  // `null` when the category is empty (no favourites yet).
  const at = Math.max(0, Math.min(index, list.length - 1));
  const game: Game | null = list[at] ?? null;

  // The list shrank (a favourite removed): stay on the last one.
  useEffect(() => {
    if (index !== at) setIndex(at);
  }, [index, at]);
  // Nothing to show details of.
  useEffect(() => {
    if (!game && mode !== "browse" && mode !== "menu" && mode !== "alert") setMode("browse");
  }, [game, mode]);

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
  const clamp = (i: number) => Math.max(0, Math.min(list.length - 1, i));
  const setCategory = (c: Category) => {
    const next = c === "favourites" ? GAMES.filter((g) => favourites.has(g.name)) : GAMES;
    // Keep the selected game when it's in the other list too.
    setIndex(Math.max(0, next.findIndex((g) => g.name === game?.name)));
    setCategoryState(c);
    PlayerPrefs.setString("category", c);
  };
  const move = (by: number) => setIndex((i) => clamp(i + by));
  const show = (m: Mode) => {
    setFocus(0);
    setMode(m);
  };
  const openContext = (i = index) => {
    setIndex(i);
    setContextFrom(mode === "details" ? "details" : "browse");
    show("context");
  };
  const notYet = (title: string, then: Mode) => {
    setAlert({ title, body: "This isn't implemented yet: the launcher is a mock-up.", then });
    setMode("alert");
  };
  const toggleFavourite = () => {
    const next = new Set(favourites);
    if (!game) return;
    if (!next.delete(game.name)) next.add(game.name);
    setFavourites(next);
    PlayerPrefs.setString("favourites", [...next].join(","));
  };

  const sidebarItems: MenuItem[] = [
    { label: "Library", icon: ICONS.library, run: () => setMode("browse") },
    {
      label: view === "grid" ? "Carousel view" : "Grid view",
      icon: view === "grid" ? ICONS.carousel : ICONS.grid,
      run: () => {
        setView(view === "grid" ? "carousel" : "grid");
        setMode("browse");
      },
    },
    {
      label: category === "favourites" ? "All games" : "Favourites",
      icon: category === "favourites" ? ICONS.library : ICONS.star,
      run: () => {
        setCategory(category === "favourites" ? "all" : "favourites");
        setMode("browse");
      },
    },
    { label: "Settings", icon: ICONS.settings, run: () => notYet("Settings", "menu") },
    { label: "Exit", icon: ICONS.exit, run: () => Application.quit(), danger: true },
  ];
  const contextItems: MenuItem[] = [
    { label: "Play", icon: ICONS.play, run: () => show("loading") },
    { label: "Details", icon: ICONS.info, run: () => setMode("details") },
    {
      label: game && favourites.has(game.name) ? "Remove from favourites" : "Add to favourites",
      icon: ICONS.star,
      run: () => {
        toggleFavourite();
        setMode(contextFrom);
      },
    },
    { label: "Show in folder", icon: ICONS.folder, run: () => notYet("Show in folder", contextFrom) },
  ];
  const menuItems = mode === "menu" ? sidebarItems : mode === "context" ? contextItems : [];

  // Loading spins for a moment, then admits it can't start anything.
  useEffect(() => {
    if (mode !== "loading" || !game) return;
    const t = setTimeout(() => {
      setAlert(cantStart(game));
      setMode("alert");
    }, LOADING_MS);
    return () => clearTimeout(t);
  }, [mode, game]);

  // Keys, pad buttons and the stick all come here. Listeners are set up once, so they call
  // the latest render's handler.
  const handler = useRef<(a: Act) => void>(() => {});
  handler.current = (act: Act) => {
    const a = act === "escape" ? (mode === "browse" ? "menu" : "back") : act;
    if ((a === "prevCategory" || a === "nextCategory") && (mode === "browse" || mode === "menu")) {
      // The ends stop: LT on the first tab (or RT on the last) does nothing.
      const next = CATEGORIES[CATEGORIES.indexOf(category) + (a === "prevCategory" ? -1 : 1)];
      if (next) setCategory(next);
      setMode("browse");
    } else if (mode === "browse") {
      if (a === "menu") show("menu");
      else if (!game) return;
      else if (a === "left") move(-1);
      else if (a === "right") move(1);
      else if (view === "grid" && a === "up") setIndex((i) => (i - columns >= 0 ? i - columns : i));
      else if (view === "grid" && a === "down") {
        const lastRow = Math.floor((list.length - 1) / columns);
        setIndex((i) => (i + columns < list.length ? i + columns : Math.floor(i / columns) < lastRow ? list.length - 1 : i));
      } else if (a === "ok") setMode("details");
      else if (a === "options") openContext();
      else if (a === "view") setView(view === "grid" ? "carousel" : "grid");
    } else if (mode === "menu" || mode === "context") {
      const close = () => setMode(mode === "menu" ? "browse" : contextFrom);
      if (a === "up") setFocus((f) => (f + menuItems.length - 1) % menuItems.length);
      else if (a === "down") setFocus((f) => (f + 1) % menuItems.length);
      else if (a === "ok") menuItems[focus]?.run();
      else if (a === "back" || (a === "menu" && mode === "menu") || (a === "options" && mode === "context")) close();
    } else if (mode === "details") {
      if (a === "ok") setMode("loading");
      else if (a === "options") openContext();
      else if (a === "back") setMode("browse");
    } else if (mode === "alert") {
      if (a === "ok" || a === "back") setMode(alert?.then ?? "browse");
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
      // Esc opens the menu from the library and goes back everywhere else.
      Escape: "escape",
      Backspace: "back",
      KeyG: "view",
      KeyC: "options",
      ContextMenu: "options",
    };
    const PAD: Record<string, Act> = {
      DPadLeft: "left",
      LeftBumper: "left",
      DPadRight: "right",
      RightBumper: "right",
      DPadUp: "up",
      DPadDown: "down",
      North: "options",
      RightStick: "view",
      Start: "menu",
      LeftTrigger: "prevCategory",
      RightTrigger: "nextCategory",
    };
    const STEPS = new Set<Act>(["left", "right", "up", "down"]);
    const offKeys = input.on("keydown", (e) => {
      // Tab and Shift+Tab step through the categories, like the triggers.
      const shift = input.getKey("ShiftLeft") || input.getKey("ShiftRight");
      const a = e.code === "Tab" ? (shift ? "prevCategory" : "nextCategory") : KEYS[e.code];
      // Held arrows repeat; the rest act once per press.
      if (a && (!e.repeat || STEPS.has(a))) act(a);
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

  const showDetails = mode === "details" || mode === "loading" || (mode === "alert" && alert?.then === "details") || (mode === "context" && contextFrom === "details");

  return (
    <box className={pad ? "app pad" : "app"} onPointerMove={noteMouse} onPointerDown={noteMouse}>
      {/* The selected game's screenshot, large and dim, fading in on each change. */}
      {game ? (
        <box key={game.name} className="backdrop">
          <Shot game={game} className="backdrop-shot" />
        </box>
      ) : null}
      <box className="shade" />

      <box className="top">
        <box className="icon-button" onClick={() => show("menu")}>
          <Burger />
        </box>
        <Text className="logo">IRON</Text>
        <Text className="logo-sub">examples</Text>
        <box className="tabs">
          {pad ? <Trigger side={0} style={pad} /> : null}
          <box className={category === "all" ? "tab on" : "tab"} onClick={() => setCategory("all")}>
            <Text>All games</Text>
            <Text className="tab-count">{String(GAMES.length)}</Text>
          </box>
          <box className={category === "favourites" ? "tab on" : "tab"} onClick={() => setCategory("favourites")}>
            <Star size={13} />
            <Text>Favourites</Text>
            <Text className="tab-count">{String(favourites.size)}</Text>
          </box>
          {pad ? <Trigger side={1} style={pad} /> : null}
        </box>
        <box className="grow" />
        <Text key={deviceName(device)} className="device">
          {deviceName(device)}
        </Text>
        <box className="views">
          <box className={view === "carousel" ? "view-button on" : "view-button"} onClick={() => setView("carousel")}>
            <Icon d={ICONS.carousel} />
          </box>
          <box className={view === "grid" ? "view-button on" : "view-button"} onClick={() => setView("grid")}>
            <Icon d={ICONS.grid} />
          </box>
        </box>
        <Text className="count">{list.length ? `${at + 1} / ${list.length}` : "0 / 0"}</Text>
      </box>

      {!game ? (
        <box key={`empty-${category}`} className="view empty">
          <box className="empty-star">
            <Star size={44} />
          </box>
          <Text className="empty-title">No favourites yet</Text>
          <Text className="empty-body">Open a game's options and add it to your favourites to see it here.</Text>
          <box className="empty-hint">
            <Glyphs action="options" device={device} />
            <Text className="hint-label">Options on a game</Text>
          </box>
        </box>
      ) : view === "carousel" ? (
        <box key={`carousel-${category}`} className="view carousel-view">
          <box
            className="stage"
            onWheel={(e: IronWheelEvent) => {
              if (mode === "browse") move(Math.sign(e.deltaY || e.deltaX));
            }}
          >
            <Carousel games={list} index={at} favourites={favourites} select={setIndex} open={() => setMode("details")} context={openContext} />
            <box className="arrow left" disabled={at === 0} onClick={() => move(-1)}>
              <Text>{"‹"}</Text>
            </box>
            <box className="arrow right" disabled={at === list.length - 1} onClick={() => move(1)}>
              <Text>{"›"}</Text>
            </box>
          </box>
          <box key={`caption-${game.name}`} className="caption">
            <Text className="title">{game.title}</Text>
            <Text className="folder">{`examples/${game.name}`}</Text>
          </box>
        </box>
      ) : (
        <box key={`grid-${category}`} className="view grid-view">
          <Grid
            games={list}
            index={at}
            columns={columns}
            favourites={favourites}
            select={setIndex}
            open={() => setMode("details")}
            context={openContext}
          />
        </box>
      )}

      <Hints mode={mode} view={view} device={device} empty={!game} />

      {showDetails && game ? (
        <box className="scrim" onClick={() => mode === "details" && setMode("browse")}>
          <box className="details" onClick={(e) => e.stopPropagation()}>
            <Shot game={game} className="details-shot" />
            <box className="details-info">
              <box className="details-head">
                <Text className="details-title">{game.title}</Text>
                {favourites.has(game.name) ? (
                  <box className="fav inline">
                    <Star size={16} />
                  </box>
                ) : null}
              </box>
              <Text className="folder">{`examples/${game.name}`}</Text>
              <Text className={game.description ? "description" : "description none"}>
                {game.description || "No description yet."}
              </Text>
              <box className="grow" />
              <box className="buttons">
                <box className="button play" onClick={() => setMode("loading")}>
                  {pad ? <FaceButton button={okButton(pad)} style={pad} /> : <Icon d={ICONS.play} />}
                  <Text>Play</Text>
                </box>
                <box className="button" onClick={() => setMode("browse")}>
                  {pad ? <FaceButton button={backButton(pad)} style={pad} /> : null}
                  <Text>Back</Text>
                </box>
                <box className="button square" onClick={() => openContext()}>
                  {pad ? <FaceButton button="North" style={pad} /> : <Text>⋯</Text>}
                </box>
              </box>
            </box>
          </box>
        </box>
      ) : null}

      {mode === "menu" ? (
        <box className="side-scrim" onClick={() => setMode("browse")}>
          <box className="sidebar" onClick={(e) => e.stopPropagation()}>
            <box className="sidebar-head">
              <Text className="logo">IRON</Text>
              <Text className="logo-sub">launcher</Text>
            </box>
            <Menu items={sidebarItems} focus={focus} setFocus={setFocus} />
            <box className="grow" />
            <Text className="sidebar-foot">{`${GAMES.length} examples · ${deviceName(device)}`}</Text>
          </box>
        </box>
      ) : null}

      {mode === "context" && game ? (
        <box className="context-scrim" onClick={() => setMode(contextFrom)}>
          <box className="context" onClick={(e) => e.stopPropagation()}>
            <box className="context-head">
              <box className="context-thumb">
                <Shot game={game} className="tile-shot" />
              </box>
              <Text className="context-title">{game.title}</Text>
            </box>
            <Menu items={contextItems} focus={focus} setFocus={setFocus} />
          </box>
        </box>
      ) : null}

      {mode === "loading" && game ? (
        <box className="loading">
          <Spinner />
          <Text className="loading-text">{`Starting ${game.title}…`}</Text>
          <box className="bar">
            <box className="bar-fill" />
          </box>
        </box>
      ) : null}

      {mode === "alert" && alert ? (
        <box className="alert-scrim">
          <box className="alert">
            <Text className="alert-title">{alert.title}</Text>
            <Text className="alert-body">{alert.body}</Text>
            {alert.code ? <Text className="alert-code">{alert.code}</Text> : null}
            <box className="buttons end">
              <box className="button play" onClick={() => setMode(alert.then)}>
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
