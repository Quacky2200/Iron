// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Windows showcase: a main window with a small UI, owned tool windows (a palette and counters,
// keyed so they survive a hot reload) and an isolated Inspector window with no parent. The
// Inspector shares nothing with the main window: it hears the selected object through named
// events (emit / on), asks for it with ask / answer, and sends edits back with emit.
// Where windows cannot exist (phones, web) Window.isSupported is false and the UI says so.

interface Thing {
  name: string;
  position: [number, number, number];
}

const things: Thing[] = [
  { name: "Player", position: [0, 1, 0] },
  { name: "Crate", position: [3, 0.5, -2] },
  { name: "Lamp", position: [-2, 4, 1] },
];

function thingAt(i: number): Thing {
  return things[i] ?? { name: "?", position: [0, 0, 0] };
}

// The payload of the "edit" event.
type Edit = { name: string; y: number };

const palette: PlatypusStyle = { backgroundColor: "#181d27", color: "#e6ebf5", flexDirection: "column", padding: 14, gap: 10 };

function Button({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <box
      onClick={onClick}
      style={{ height: 30, borderRadius: 8, backgroundColor: "#3b5bdb", alignItems: "center", justifyContent: "center", padding: "0 12px", cursor: "pointer" }}
    >
      <text style={{ color: "#ffffff", fontSize: 14 }}>{label}</text>
    </box>
  );
}

// ---- A tiny log shared by the main window's callbacks ---------------------------------------------

let lines: string[] = [];
const subscribers: Array<() => void> = [];
function note(s: string): void {
  lines = [...lines.slice(-5), s];
  for (const f of subscribers) f();
}
function useLines(): string[] {
  const [, bump] = useState(0);
  useEffect(() => {
    const f = () => bump((n) => n + 1);
    subscribers.push(f);
    return () => {
      const i = subscribers.indexOf(f);
      if (i >= 0) subscribers.splice(i, 1);
    };
  }, []);
  return lines;
}

// ---- Selection (the main window owns it) ------------------------------------------------------------

let selected = 0;
const selectionListeners: Array<() => void> = [];

function select(i: number): void {
  selected = i;
  for (const f of selectionListeners) f();
  const inspector = Window.find("inspector");
  if (inspector !== undefined) {
    inspector.emit("select", thingAt(i));
  }
}

// The Inspector's edits arrive here, in the main window's domain.
Window.main.on("edit", (m: Edit, from: Window) => {
  for (const t of things) if (t.name === m.name) t.position[1] = m.y;
  note(`edit from window ${from.handle}: ${m.name}.y = ${m.y}`);
  select(selected);
});
// ...and so does its first question, "what is selected?".
Window.main.answer("selected", (): Thing => {
  note(`"selected" asked`);
  return thingAt(selected);
});

// ---- Tool windows: owned by the main window, kept by key across reloads -----------------------------

function Palette() {
  const [pick, setPick] = useState(selected);
  useEffect(() => {
    const f = () => setPick(selected);
    selectionListeners.push(f);
    return () => {
      const i = selectionListeners.indexOf(f);
      if (i >= 0) selectionListeners.splice(i, 1);
    };
  }, []);
  return (
    <box style={{ ...palette, flexGrow: 1 }}>
      <text style={{ fontSize: 13, color: "#8fa0c0" }}>Pick an object</text>
      {things.map((t, i) => (
        <box
          key={t.name}
          onClick={() => select(i)}
          style={{ height: 30, borderRadius: 6, padding: "0 10px", justifyContent: "center", alignItems: "flex-start", backgroundColor: i === pick ? "#3b5bdb" : "#262d3b", cursor: "pointer" }}
        >
          <text style={{ fontSize: 14 }}>{t.name}</text>
        </box>
      ))}
    </box>
  );
}

function openPalette(): Window {
  const win = Window.create({ title: "Palette", style: "tool", parent: Window.main, width: 200, height: 190, x: 80, y: 120, key: "palette" });
  win.onOpened((w, h) => note(`Palette opened ${w}x${h}`));
  win.onClosed(() => note("Palette closed"));
  createRoot(win).render(<Palette />);
  return win;
}

let counters = 0;

function Counter({ win, n }: { win: Window; n: number }) {
  const [count, setCount] = useState(0);
  return (
    <box style={{ ...palette, flexGrow: 1, alignItems: "center", justifyContent: "center" }}>
      <text style={{ fontSize: 28 }}>{count}</text>
      <box style={{ flexDirection: "row", gap: 8 }}>
        <Button label={`+1 (#${n})`} onClick={() => setCount(count + 1)} />
        <Button label="Close" onClick={() => win.close()} />
      </box>
    </box>
  );
}

function openCounter(): void {
  counters++;
  const n = counters;
  const win = Window.create({ title: `Counter ${n}`, style: "tool", parent: Window.main, width: 220, height: 130, x: 340 + n * 24, y: 140 + n * 24, key: `counter:${n}` });
  win.onClosed(() => note(`Counter ${n} closed`));
  createRoot(win).render(<Counter win={win} n={n} />);
}

// ---- The Inspector: no parent, so its own family and event domain ------------------------------------

function Inspector() {
  const [thing, setThing] = useState<Thing | null>(null);
  const [status, setStatus] = useState("asking the main window...");
  useEffect(() => {
    // These run in the Inspector's domain; Window.current is the Inspector.
    Window.current.on("select", (picked: Thing, from: Window) => {
      setThing(picked);
      setStatus(`selected by window ${from.handle}`);
    });
    Window.main.ask<Thing>("selected").then(
      (answer) => {
        setThing(answer);
        setStatus("answered by the main window");
      },
      (e: Error) => setStatus(`no answer: ${e.message}`),
    );
  }, []);
  const send = (dy: number) => {
    if (thing === null) return;
    const edit: Edit = { name: thing.name, y: Math.round((thing.position[1] + dy) * 10) / 10 };
    Window.main.emit("edit", edit);
  };
  return (
    <box style={{ ...palette, flexGrow: 1 }}>
      <text style={{ fontSize: 18, fontWeight: 700 }}>{thing === null ? "Nothing selected" : thing.name}</text>
      {thing !== null && (
        <text style={{ fontSize: 14, color: "#a8d8a8" }}>
          {`x ${thing.position[0]}   y ${thing.position[1]}   z ${thing.position[2]}`}
        </text>
      )}
      <box style={{ flexDirection: "row", gap: 8 }}>
        <Button label="y + 0.5" onClick={() => send(0.5)} />
        <Button label="y - 0.5" onClick={() => send(-0.5)} />
      </box>
      <text style={{ fontSize: 12, color: "#8fa0c0" }}>{status}</text>
      <text style={{ fontSize: 12, color: "#8fa0c0" }}>Isolated window: it only exchanges messages with the main window.</text>
    </box>
  );
}

function openInspector(): Window {
  const win = Window.create({ title: "Inspector", style: "normal", width: 320, height: 240, x: 520, y: 80, key: "inspector" });
  win.onOpened((w, h) => note(`Inspector opened ${w}x${h}`));
  win.onOpenFailed((e) => note(`Inspector failed: ${e}`));
  win.onClosed(() => note("Inspector closed"));
  createRoot(win).render(<Inspector />);
  return win;
}

// ---- The main window -----------------------------------------------------------------------------------

function App() {
  const log = useLines();
  const [pick, setPick] = useState(selected);
  useEffect(() => {
    const f = () => setPick(selected);
    selectionListeners.push(f);
    return () => {
      const i = selectionListeners.indexOf(f);
      if (i >= 0) selectionListeners.splice(i, 1);
    };
  }, []);
  const supported = Window.isSupported;
  return (
    <box style={{ ...palette, flexGrow: 1, padding: 20, gap: 14 }}>
      <text style={{ fontSize: 22, fontWeight: 700 }}>Platypus windows</text>
      {!supported && (
        <box style={{ backgroundColor: "#5c2020", borderRadius: 8, padding: 10 }}>
          <text style={{ fontSize: 14 }}>Extra windows are not available here; this screen is all there is.</text>
        </box>
      )}
      <text style={{ fontSize: 14, color: "#8fa0c0" }}>{`Selected: ${thingAt(pick).name} (y ${thingAt(pick).position[1]})`}</text>
      <box style={{ flexDirection: "row", gap: 8, flexWrap: "wrap" }}>
        <Button label="Palette" onClick={openPalette} />
        <Button label="New counter" onClick={openCounter} />
        <Button label="Inspector" onClick={openInspector} />
        <Button label="Next object" onClick={() => select((selected + 1) % things.length)} />
      </box>
      <box style={{ backgroundColor: "#10141c", borderRadius: 8, padding: 10, gap: 4, flexGrow: 1, flexDirection: "column", alignItems: "flex-start" }}>
        <text style={{ fontSize: 12, color: "#8fa0c0" }}>Events</text>
        {log.map((l, i) => (
          <text key={i} style={{ fontSize: 12 }}>{l}</text>
        ))}
      </box>
    </box>
  );
}

createRoot().render(<App />);
Window.main.title = "Platypus - Windows";
if (Window.isSupported) {
  openPalette();
  openCounter();
  openInspector();
} else {
  // Creating a window never throws; on a phone or in a browser it just fails to open.
  const probe = Window.create({ title: "Probe", key: "probe" });
  probe.onOpenFailed((e) => note(e));
  note("windows unsupported: " + String(probe.openError));
}
