// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The startup video-settings prompt: a small UI-only window the game's process opens before its
// own window (`video_prompt.rs`). The host puts `GAME`, `SUBTITLE`, `RESIZABLE`, `MODE`, `WIDTH`, `HEIGHT`,
// `REFRESH`, `VSYNC` and `LEVEL` (constants: the game's current choice) and the resolution
// list's helpers (`video_prompt_sizes.ts`) in front of this text. Play saves the choice in this
// window's PlayerPrefs (`mode`, `width`, `height`, `refreshRate`, `vsync`, `level`) and exits
// with 10; Quit, Esc or closing the window exits with 0. Opened by a running game
// (`Application.showVideoSettings`, `STARTUP` false) the buttons are Apply and Cancel.

const PLAY = 10;
/** Rows a Page Up / Page Down moves in the resolution list. */
const PAGE = 5;

const MODES: readonly (readonly [string, string])[] = [
  ["Windowed", "Windowed"],
  ["FullScreenWindow", "Borderless"],
  ["ExclusiveFullScreen", "Fullscreen"],
];

const CSS = `
.choice { padding: 6px 12px; border-radius: 6px; cursor: pointer; background-color: rgba(255,255,255,0.1);
  border-width: 1px; border-color: rgba(255,255,255,0); }
.choice:hover { background-color: rgba(255,255,255,0.18); }
.choice.active { background-color: #4d8dff; }
.choice:focus { border-color: #ffffff; }
.list { flex-direction: column; width: 260px; height: 152px; overflow: auto; padding: 3px; border-radius: 6px; border-width: 1px;
  border-color: #2e3038; background-color: #14151a; }
.list:focus { border-color: #ffffff; }
.row { padding: 4px 10px; border-radius: 4px; cursor: pointer; flex-shrink: 0; }
.row:hover { background-color: rgba(255,255,255,0.08); }
.row.selected { background-color: #4d8dff; }
.button { height: 34px; padding-left: 24px; padding-right: 24px; margin-left: 5px; margin-right: 5px; border-radius: 8px;
  align-items: center; justify-content: center; cursor: pointer; flex-shrink: 0;
  background-color: #30333b; border-width: 1px; border-color: #41454f; }
.button.primary { background-color: #3d7bf0; border-color: #6a9bff; }
.button:focus { border-color: #ffffff; }
`;
addStyleSheet(CSS);

function activateOnKeys(e: PlatypusKeyboardEvent, f: () => void) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    f();
  }
}

function Choice(props: { label: string; active?: boolean; onClick: () => void }) {
  return (
    <box
      className={props.active ? "choice active" : "choice"}
      tabIndex={0}
      onClick={props.onClick}
      onKeyDown={(e: PlatypusKeyboardEvent) => activateOnKeys(e, props.onClick)}
    >
      <text style={{ color: "#f0f3f8", fontSize: 13, whiteSpace: "nowrap" }}>{props.label}</text>
    </box>
  );
}

function Row(props: { label: string; children: ReactNode }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "flex-start", marginTop: 12 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 100, flexShrink: 0, marginTop: 6 }}>{props.label}</text>
      <box style={{ flexDirection: "row", gap: 6, flexWrap: "wrap", alignItems: "center" }}>{props.children}</box>
    </box>
  );
}

/** A list box: several sizes at once, the chosen one highlighted and kept in view. The wheel
 * scrolls it; Up / Down, Page Up / Down, Home and End (and a gamepad's d-pad) move the choice;
 * Up at the top or Down at the bottom leaves the list. */
function SizeList(props: { sizes: Size[]; at: number; onPick: (i: number) => void }) {
  const selected = useRef<PlatypusElement>(null);
  useEffect(() => selected.current?.scrollIntoView({ block: "nearest" }), [props.at, props.sizes.length]);
  const onKey = (e: PlatypusKeyboardEvent) => {
    const last = props.sizes.length - 1;
    if (last < 0) return;
    const at = Math.max(0, props.at);
    let to = at;
    if (e.key === "ArrowUp") to = at - 1;
    else if (e.key === "ArrowDown") to = at + 1;
    else if (e.key === "PageUp") to = at - PAGE;
    else if (e.key === "PageDown") to = at + PAGE;
    else if (e.key === "Home") to = 0;
    else if (e.key === "End") to = last;
    else return;
    if (to < 0 && e.key === "ArrowUp") return;
    if (to > last && e.key === "ArrowDown") return;
    e.preventDefault();
    props.onPick(Math.min(last, Math.max(0, to)));
  };
  return (
    <box className="list" tabIndex={0} onKeyDown={onKey}>
      {props.sizes.length === 0 ? <text style={{ color: "#868b96", fontSize: 13, padding: 6 }}>…</text> : null}
      {props.sizes.map((s, i) => (
        <box
          key={`${s.w}x${s.h}@${s.hz}`}
          ref={i === props.at ? selected : undefined}
          className={i === props.at ? "row selected" : "row"}
          onClick={() => props.onPick(i)}
        >
          <text style={{ color: "#f0f3f8", fontSize: 13, whiteSpace: "nowrap" }}>{sizeLabel(s)}</text>
        </box>
      ))}
    </box>
  );
}

function Button(props: { label: string; primary: boolean; onPress: () => void; focusRef?: Ref<PlatypusElement | null> }) {
  return (
    <box
      ref={props.focusRef}
      className={props.primary ? "button primary" : "button"}
      tabIndex={0}
      onClick={() => props.onPress()}
      onKeyDown={(e: PlatypusKeyboardEvent) => activateOnKeys(e, props.onPress)}
    >
      <text style={{ color: "#ffffff", fontSize: 14, whiteSpace: "nowrap" }}>{props.label}</text>
    </box>
  );
}

/** Re-renders the dialog: set by it, called when the monitor's modes arrive. */
let refresh: () => void = () => {};

/** Esc (or a gamepad's B) quits; the monitor's modes come with the first frames. */
class Keys extends Component {
  private seen = -1;
  override onUpdate() {
    if (input.getButtonDown("Cancel")) Application.quit(0);
    const n = Screen.resolutions.length;
    if (n !== this.seen) {
      this.seen = n;
      refresh();
    }
  }
}
register({ Keys });

function Dialog() {
  const startMode = MODES.some(([m]) => m === MODE) ? MODE : "Windowed";
  const [mode, setMode] = useState(startMode);
  // `null`: nothing picked yet (the monitor's modes may not be known): the current choice.
  const [picked, setPicked] = useState<Size | null>(null);
  const [vsync, setVsync] = useState(VSYNC);
  const names = QualitySettings.names;
  const [level, setLevel] = useState(names.indexOf(LEVEL) >= 0 ? LEVEL : names[names.length > 2 ? 2 : 0]!);
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  const play = useRef<PlatypusElement>(null);
  useEffect(() => play.current?.focus(), []);

  const monitor = Screen.currentResolution;
  // Plain objects for the helpers (they take shapes, not the engine's class).
  const modes = Screen.resolutions.map((r) => ({ width: r.width, height: r.height, refreshRate: r.refreshRate }));
  const sizes = sizeList(modes, { width: monitor.width, height: monitor.height }, mode);
  const want: Size =
    picked ??
    (WIDTH > 0 && HEIGHT > 0
      ? { w: WIDTH, h: HEIGHT, hz: Math.round(REFRESH) }
      : { w: monitor.width, h: monitor.height, hz: Math.round(monitor.refreshRate) });
  const at = nearestIndex(sizes, want);
  const size = at >= 0 ? sizes[at]! : null;

  const start = () => {
    PlayerPrefs.setString("mode", RESIZABLE ? mode : MODE);
    if (RESIZABLE && mode !== "FullScreenWindow" && size) {
      PlayerPrefs.setInt("width", size.w);
      PlayerPrefs.setInt("height", size.h);
      PlayerPrefs.setInt("refreshRate", size.hz);
    }
    PlayerPrefs.setInt("vsync", vsync ? 1 : 0);
    PlayerPrefs.setString("level", level);
    PlayerPrefs.save();
    Application.quit(PLAY);
  };

  return (
    <box style={{ width: "100%", height: "100%", flexDirection: "column", alignItems: "center", padding: 24, backgroundColor: "#1b1c21" }}>
      {/* The game's name (its icon is the window's, in the title bar). */}
      <box style={{ flexDirection: "column", alignItems: "center" }}>
        {headerLines(GAME, SUBTITLE).map((line, i) => (
          <text key={i} style={i === 0 ? { color: "#ffffff", fontSize: 22 } : { color: "#8d93a0", fontSize: 14, marginTop: 2 }}>
            {line}
          </text>
        ))}
      </box>
      <text style={{ color: "#b6bac4", fontSize: 14, marginTop: 10, textAlign: "center" }}>
        {STARTUP ? "Choose how the game looks. You can change it later in its settings." : "Choose how the game looks."}
      </text>
      <box style={{ flexDirection: "column", width: "100%", maxWidth: 400, marginTop: 6 }}>
      {/* A fixed-size game has no mode or resolution to choose: the rows are not displayed. */}
      <box style={{ flexDirection: "column", display: RESIZABLE ? "flex" : "none" }}>
        <Row label="Window">
          {MODES.map(([m, name]) => (
            <Choice key={m} label={name} active={m === mode} onClick={() => setMode(m)} />
          ))}
        </Row>
        <Row label="Resolution">
          {mode === "FullScreenWindow" ? (
            <text style={{ color: "#f0f3f8", fontSize: 13, marginTop: 6 }}>
              {monitor.width > 0 ? `The monitor's own (${monitor.width} × ${monitor.height})` : "The monitor's own"}
            </text>
          ) : (
            <SizeList sizes={sizes} at={at} onPick={(i) => setPicked(sizes[i]!)} />
          )}
        </Row>
      </box>
      <Row label="VSync">
        <Choice label="On" active={vsync} onClick={() => setVsync(true)} />
        <Choice label="Off" active={!vsync} onClick={() => setVsync(false)} />
      </Row>
      <Row label="Quality">
        {names.map((name) => (
          <Choice key={name} label={name} active={name === level} onClick={() => setLevel(name)} />
        ))}
      </Row>
      </box>
      <box style={{ flexGrow: 1 }} />
      <box style={{ flexDirection: "row", justifyContent: "center", marginTop: 16 }}>
        {/* In a running game the dialog never quits it: Cancel just closes the dialog. */}
        <Button label={STARTUP ? "Quit" : "Cancel"} primary={false} onPress={() => Application.quit(0)} />
        <Button label={STARTUP ? "Play" : "Apply"} primary={true} onPress={start} focusRef={play} />
      </box>
      <text style={{ color: "#6f7480", fontSize: 11, marginTop: 10, textAlign: "center" }}>VSync keeps frames in step with the display.</text>
    </box>
  );
}

createRoot().render(<Dialog />);
