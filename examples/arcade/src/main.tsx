// Arcade: a small game with a React HUD. Steer the ball (WASD / arrow keys / left stick)
// to collect coins (1 point) and gems (5) before the round ends; quick pickups build a
// combo that multiplies points. Leave the controls alone and it plays itself.
//
// The game lives in components (`GameManager`, `Player`, `Pickup`); the HUD is React
// (`ScoreBoard`, `Timer`, `RoundOver`, `Settings`), fed by `useGame()`. Esc (or the
// gear) pauses and opens the settings: window mode, resolution, vsync, frame cap and
// quality, saved for next time (`PlayerPrefs`). Both reload while it runs:
// `bun run dev` and edit a HUD component (it keeps the score on screen) or a component's
// code (the round, the score and every coin stay where they are).
import {
  Application,
  type FullScreenMode,
  PlayerPrefs,
  QualitySettings,
  Screen,
  Color,
  Component,
  GameObject,
  input,
  Light,
  Material,
  MeshRenderer,
  property,
  Quaternion,
  Random,
  register,
  RenderSettings,
  Camera,
  Time,
  Vector3,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

const ARENA = 9;

// ---- Game ----

/** The round: score, combo, time left. The HUD listens to it. */
class GameManager extends Component {
  @property roundLength = 45;
  /** Pickups this close together in time keep the combo going. */
  @property comboWindow = 1;
  score = 0;
  best = 0;
  /** The settings menu is open: the game stands still. */
  paused = false;
  combo = 0;
  timeLeft = 45;
  lastPickup = -100;
  /** Recent pickups, for the HUD's "+N" popups. */
  recent: { id: number; points: number; at: number }[] = [];
  private nextPopup = 1;

  static get current(): GameManager | undefined {
    return GameObject.findObjectOfType(GameManager);
  }

  get over(): boolean {
    return this.timeLeft <= 0;
  }

  setPaused(on: boolean): void {
    this.paused = on;
    Time.timeScale = on ? 0 : 1;
    if (!on) PlayerPrefs.save();
    notify();
  }

  /** Points times the combo (x1, x2 ... up to x5). */
  collect(value: number): void {
    if (this.over) return;
    this.combo = Time.time - this.lastPickup <= this.comboWindow ? Math.min(this.combo + 1, 5) : 1;
    this.lastPickup = Time.time;
    const points = value * this.combo;
    this.score += points;
    this.best = Math.max(this.best, this.score);
    this.recent = [...this.recent.slice(-3), { id: this.nextPopup++, points, at: Time.time }];
    notify();
  }

  restart(): void {
    this.score = 0;
    this.combo = 0;
    this.timeLeft = this.roundLength;
    this.recent = [];
    notify();
  }

  override onStart() {
    this.timeLeft = this.roundLength;
  }

  /** Brought back by a reload (the HUD is new): show it the round. */
  override onReload() {
    notify();
  }

  override onUpdate(dt: number) {
    if (input.getButtonDown("Cancel")) this.setPaused(!this.paused);
    if (this.over || this.paused) return;
    const before = Math.ceil(this.timeLeft);
    this.timeLeft = Math.max(0, this.timeLeft - dt);
    if ((this.over || Time.time - this.lastPickup > this.comboWindow) && this.combo > 0) {
      this.combo = 0;
      notify();
    }
    // The HUD shows whole seconds.
    if (Math.ceil(this.timeLeft) !== before) notify();
  }
}

/** Rolls around the arena; plays itself when nobody's steering. */
class Player extends Component {
  @property speed = 6;
  velocity = Vector3.zero;
  idle = 0;

  override onUpdate(dt: number) {
    let steer = new Vector3(input.getAxis("Horizontal"), 0, -input.getAxis("Vertical"));
    this.idle = steer.magnitude > 0.1 ? 0 : this.idle + dt;
    if (this.idle > 1.5) {
      // Attract mode: head for the nearest pickup.
      const here = this.transform.position;
      const target = pickups()
        .map((p) => p.transform.position)
        .sort((a, b) => a.distance(here) - b.distance(here))[0];
      if (target) steer = target.sub(here).scale(1 / Math.max(1e-3, target.distance(here)));
    }
    if (steer.magnitude > 1) steer = steer.normalized;
    this.velocity = this.velocity.lerp(steer.scale(this.speed), Math.min(1, dt * 6));
    const p = this.transform.position.add(this.velocity.scale(dt));
    this.transform.position = new Vector3(Math.max(-ARENA, Math.min(ARENA, p.x)), 0.5, Math.max(-ARENA, Math.min(ARENA, p.z)));
    // Roll: turn about the axis across the motion.
    const along = this.velocity.magnitude * dt;
    if (along > 1e-5) {
      const axis = new Vector3(this.velocity.z, 0, -this.velocity.x).normalized;
      this.transform.rotation = Quaternion.angleAxis(-(along / 0.5) * (180 / Math.PI), axis).mul(this.transform.rotation);
    }
  }
}

/** A coin or a gem: spins, and moves somewhere else once collected. */
class Pickup extends Component {
  @property value = 1;
  @property spin = 120;
  bornAt = 0;

  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.spin * dt, 0), "world");
    const player = GameObject.findObjectOfType(Player);
    const game = GameManager.current;
    if (!player || !game || game.over) return;
    if (player.transform.position.distance(this.transform.position) < 0.9) {
      game.collect(this.value);
      this.transform.position = randomSpot(this.transform.position.y);
    }
  }
}

function pickups(): Pickup[] {
  return GameObject.roots.flatMap((g) => g.getComponents(Pickup));
}

function randomSpot(y: number): Vector3 {
  return new Vector3(Random.range(-ARENA + 1, ARENA - 1), y, Random.range(-ARENA + 1, ARENA - 1));
}

/** Builds the arena once (a reload keeps what it made). */
class Arcade extends Component {
  override onStart() {
    // The frame cap the player chose last time (vsync and quality come back on their own).
    Application.targetFrameRate = PlayerPrefs.getInt("arcade.frameCap", -1);
    RenderSettings.skybox = { sunDirection: new Vector3(-0.4, 0.8, 0.3) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0.4, -0.8, -0.3)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.1, shadows: "soft" });
    const cam = new GameObject("Main Camera", { position: new Vector3(0, 17, 15.5), rotation: Quaternion.euler(-50, 0, 0) });
    cam.addComponent(Camera, { fieldOfView: 50, clearFlags: "skybox" });

    const floor = new GameObject("Floor", { scale: new Vector3(2, 1, 2) });
    floor.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.07, 0.09, 0.14, 1), smoothness: 0.5 }) });
    const wall = Material.create({ color: new Color(0.16, 0.2, 0.3, 1), smoothness: 0.3 });
    for (const [x, z, w, d] of [
      [0, -ARENA - 0.5, ARENA * 2 + 2, 1],
      [0, ARENA + 0.5, ARENA * 2 + 2, 1],
      [-ARENA - 0.5, 0, 1, ARENA * 2],
      [ARENA + 0.5, 0, 1, ARENA * 2],
    ] as const) {
      new GameObject("Wall", { position: new Vector3(x, 0.3, z), scale: new Vector3(w, 0.6, d) }).addComponent(MeshRenderer, { mesh: "cube", material: wall });
    }

    const ball = new GameObject("Player", { position: new Vector3(0, 0.5, 0) });
    ball.addComponent(MeshRenderer, { mesh: "sphere", material: Material.create({ color: new Color(0.95, 0.4, 0.2, 1), smoothness: 0.7 }) });
    ball.addComponent(Player);

    const gold = Material.create({ color: new Color(1, 0.75, 0.2, 1), emission: new Color(0.35, 0.22, 0.02, 1), metallic: 0.5, smoothness: 0.7 });
    for (let i = 0; i < 10; i++) {
      const coin = new GameObject(`Coin ${i}`, { position: randomSpot(0.6) });
      const disc = new GameObject("Disc", { parent: coin, rotation: Quaternion.euler(90, 0, 0), scale: new Vector3(0.7, 0.08, 0.7) });
      disc.addComponent(MeshRenderer, { mesh: "cylinder", material: gold });
      coin.addComponent(Pickup, { value: 1 });
    }
    const amethyst = Material.create({ color: new Color(0.6, 0.3, 1, 1), emission: new Color(0.25, 0.1, 0.5, 1), smoothness: 0.9 });
    for (let i = 0; i < 2; i++) {
      const gem = new GameObject(`Gem ${i}`, { position: randomSpot(0.7), scale: Vector3.one.scale(0.55) });
      gem.addComponent(MeshRenderer, { mesh: "sphere", material: amethyst });
      gem.addComponent(Pickup, { value: 5, spin: 200 });
    }

    new GameObject("Game").addComponent(GameManager);
  }
}

// Registered so hot reload can bring them back with their state.
register({ GameManager, Player, Pickup }, { spawn: false });
register({ Arcade });

// ---- HUD (React) ----

const listeners = new Set<() => void>();
function notify() {
  for (const f of listeners) f();
}

interface GameView {
  paused: boolean;
  score: number;
  best: number;
  combo: number;
  timeLeft: number;
  over: boolean;
  recent: { id: number; points: number; at: number }[];
}

/** The round as React state: re-renders when the game says something changed. */
function useGame(): GameView | null {
  const read = (): GameView | null => {
    const g = GameManager.current;
    return g ? { paused: g.paused, score: g.score, best: g.best, combo: g.combo, timeLeft: g.timeLeft, over: g.over, recent: g.recent } : null;
  };
  const [view, setView] = useState(read);
  useEffect(() => {
    const update = () => setView(read());
    listeners.add(update);
    update();
    return () => {
      listeners.delete(update);
    };
  }, []);
  return view;
}

const panel = {
  padding: "10px 16px",
  borderRadius: 12,
  backgroundColor: "rgba(10, 12, 20, 0.72)",
  flexDirection: "column" as const,
};

/** The current score, the best and the combo, with "+N" popups for the last pickups. */
function ScoreBoard(props: { score: number; best: number; combo: number; recent: GameView["recent"] }) {
  return (
    <box style={{ ...panel, gap: 2, minWidth: 180 }}>
      <text style={{ color: "#9fb0c8", fontSize: 12, letterSpacing: 1 }}>SCORE</text>
      <box style={{ flexDirection: "row", alignItems: "baseline", gap: 10 }}>
        <text style={{ color: "#ffffff", fontSize: 40, fontWeight: 700 }}>{String(props.score)}</text>
        {props.combo > 1 && <text style={{ color: "#ffcf4a", fontSize: 18, fontWeight: 700 }}>{`x${props.combo}`}</text>}
      </box>
      <text style={{ color: "#9fb0c8", fontSize: 12 }}>{`Best ${props.best}`}</text>
      <box style={{ flexDirection: "row", gap: 6, height: 18 }}>
        {props.recent
          .filter((p) => Time.time - p.at < 1.2)
          .map((p) => (
            <text key={p.id} style={{ color: p.points >= 5 ? "#c89bff" : "#ffcf4a", fontSize: 14, fontWeight: 600 }}>
              {`+${p.points}`}
            </text>
          ))}
      </box>
    </box>
  );
}

/** Seconds left, red for the last ten. */
function Timer(props: { timeLeft: number }) {
  const s = Math.ceil(props.timeLeft);
  return (
    <box style={{ ...panel, alignItems: "center", minWidth: 90 }}>
      <text style={{ color: "#9fb0c8", fontSize: 12, letterSpacing: 1 }}>TIME</text>
      <text style={{ color: s <= 10 ? "#ff6b5e" : "#ffffff", fontSize: 32, fontWeight: 700 }}>{String(s)}</text>
    </box>
  );
}

/** Frames per second, measured over each half second (real time, not game time). */
function FpsCounter() {
  const [fps, setFps] = useState(0);
  useEffect(() => {
    let frames = Time.frameCount;
    let since = Time.unscaledTime;
    let timer = 0;
    const sample = () => {
      const elapsed = Time.unscaledTime - since;
      if (elapsed > 0) setFps(Math.round((Time.frameCount - frames) / elapsed));
      frames = Time.frameCount;
      since = Time.unscaledTime;
      timer = setTimeout(sample, 500);
    };
    timer = setTimeout(sample, 500);
    return () => clearTimeout(timer);
  }, []);
  const color = fps >= 55 ? "#7ee08a" : fps >= 30 ? "#ffcf4a" : "#ff6b5e";
  return (
    <box style={{ ...panel, padding: "4px 10px", flexDirection: "row", gap: 6, alignItems: "baseline" }}>
      <text style={{ color, fontSize: 16, fontWeight: 700 }}>{String(fps)}</text>
      <text style={{ color: "#9fb0c8", fontSize: 11 }}>FPS</text>
    </box>
  );
}

const MODE_NAMES: Record<FullScreenMode, string> = {
  ExclusiveFullScreen: "Fullscreen",
  FullScreenWindow: "Borderless",
  MaximizedWindow: "Maximized",
  Windowed: "Windowed",
};
const WINDOW_SIZES = [
  [1280, 720],
  [1600, 900],
  [1920, 1080],
  [2560, 1440],
] as const;
const FRAME_CAPS = [30, 60, 120, 144, -1];

function Choice(props: { label: string; active?: boolean; disabled?: boolean; onClick: () => void }) {
  return (
    <box
      onClick={props.disabled ? undefined : props.onClick}
      style={{
        padding: "5px 11px",
        borderRadius: 6,
        cursor: props.disabled ? "default" : "pointer",
        opacity: props.disabled ? 0.4 : 1,
        backgroundColor: props.active ? "#4d8dff" : "rgba(255,255,255,0.1)",
      }}
    >
      <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.label}</text>
    </box>
  );
}

function Row(props: { label: string; children: React.ReactNode }) {
  return (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#9fb0c8", fontSize: 13, width: 100, flexShrink: 0 }}>{props.label}</text>
      <box style={{ flexDirection: "row", gap: 6, flexWrap: "wrap" }}>{props.children}</box>
    </box>
  );
}

/** Window mode, resolution, vsync, frame cap and quality; saved when it closes. */
function Settings() {
  // The engine applies window changes after this frame: look again shortly after.
  const [, setTick] = useState(0);
  const refresh = () => setTimeout(() => setTick((t) => t + 1), 100);
  const mode = Screen.fullScreenMode;
  const size = Screen.windowSize;
  const monitor = Screen.currentResolution;
  // Window sizes that fit the monitor; exclusive fullscreen uses the monitor's own modes.
  const sizes =
    mode === "ExclusiveFullScreen"
      ? Screen.resolutions.slice(0, 6).map((r) => [r.width, r.height, r.refreshRate] as const)
      : WINDOW_SIZES.filter(([w, h]) => monitor.width === 0 || (w <= monitor.width && h <= monitor.height)).map(([w, h]) => [w, h, 0] as const);
  const vsync = QualitySettings.vSyncCount > 0;
  const cap = Application.targetFrameRate;
  return (
    <box style={{ ...panel, padding: 22, gap: 12, pointerEvents: "auto", backgroundColor: "rgba(10, 12, 20, 0.94)", minWidth: 460 }}>
      <text style={{ color: "#ffffff", fontSize: 22, fontWeight: 700 }}>Settings</text>
      <Row label="Window">
        {Screen.supportedFullScreenModes.map((m) => (
          <Choice
            key={m}
            label={MODE_NAMES[m]}
            active={m === mode}
            onClick={() => {
              Screen.fullScreenMode = m;
              refresh();
            }}
          />
        ))}
      </Row>
      <Row label="Resolution">
        {sizes.map(([w, h, hz]) => (
          <Choice
            key={`${w}x${h}@${hz}`}
            label={hz ? `${w}×${h} ${Math.round(hz)}Hz` : `${w}×${h}`}
            active={w === size.width && h === size.height}
            disabled={mode === "FullScreenWindow" || mode === "MaximizedWindow"}
            onClick={() => {
              Screen.setResolution(w, h, mode, hz);
              refresh();
            }}
          />
        ))}
      </Row>
      <Row label="VSync">
        <Choice
          label="On"
          active={vsync}
          onClick={() => {
            QualitySettings.vSyncCount = 1;
            refresh();
          }}
        />
        <Choice
          label="Off"
          active={!vsync}
          onClick={() => {
            QualitySettings.vSyncCount = 0;
            refresh();
          }}
        />
      </Row>
      <Row label="Frame cap">
        {FRAME_CAPS.map((fps) => (
          <Choice
            key={fps}
            label={fps > 0 ? String(fps) : "None"}
            active={cap === fps}
            disabled={vsync}
            onClick={() => {
              Application.targetFrameRate = fps;
              PlayerPrefs.setInt("arcade.frameCap", fps);
              refresh();
            }}
          />
        ))}
      </Row>
      <Row label="Quality">
        {QualitySettings.names.map((name, i) => (
          <Choice
            key={name}
            label={name}
            active={i === QualitySettings.getQualityLevel()}
            onClick={() => {
              // A level sets vsync too: keep the player's choice.
              const v = QualitySettings.vSyncCount;
              QualitySettings.setQualityLevel(i);
              QualitySettings.vSyncCount = v;
              refresh();
            }}
          />
        ))}
      </Row>
      <text style={{ color: "#7d8aa0", fontSize: 12 }}>{vsync ? "With vsync on, frames follow the display; the cap applies with it off." : "Without vsync frames may tear; the cap keeps the GPU from running flat out."}</text>
      <box style={{ flexDirection: "row", justifyContent: "flex-end" }}>
        <Choice label="Back to the game (Esc)" active onClick={() => GameManager.current?.setPaused(false)} />
      </box>
    </box>
  );
}

function RoundOver(props: { score: number; best: number }) {
  return (
    <box style={{ width: "100%", height: "100%", alignItems: "center", justifyContent: "center" }}>
      <box style={{ ...panel, padding: 28, gap: 10, alignItems: "center", pointerEvents: "auto", backgroundColor: "rgba(10, 12, 20, 0.94)" }}>
        <text style={{ color: "#ffffff", fontSize: 28, fontWeight: 700 }}>Round over</text>
        <text style={{ color: "#ffcf4a", fontSize: 20 }}>{`${props.score} points${props.score >= props.best && props.score > 0 ? " — a new best!" : ""}`}</text>
        <box
          onClick={() => GameManager.current?.restart()}
          style={{ marginTop: 6, padding: "8px 18px", borderRadius: 8, backgroundColor: "#4d8dff", cursor: "pointer" }}
        >
          <text style={{ color: "#ffffff", fontSize: 15, fontWeight: 600 }}>Play again</text>
        </box>
      </box>
    </box>
  );
}

function Hud() {
  const game = useGame();
  // The popups fade by time, so redraw while any are showing.
  const [, setFrame] = useState(0);
  useEffect(() => {
    if (!game?.recent.some((p) => Time.time - p.at < 1.2)) return;
    const t = setTimeout(() => setFrame((f) => f + 1), 100);
    return () => clearTimeout(t);
  });
  if (!game) return null;
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none", flexDirection: "column" }}>
      <box style={{ flexDirection: "row", justifyContent: "space-between", alignItems: "flex-start" }}>
        <ScoreBoard score={game.score} best={game.best} combo={game.combo} recent={game.recent} />
        <box style={{ flexDirection: "column", alignItems: "flex-end", gap: 8 }}>
          <Timer timeLeft={game.timeLeft} />
          <FpsCounter />
          <box style={{ pointerEvents: "auto" }}>
            <Choice label="Settings (Esc)" onClick={() => GameManager.current?.setPaused(true)} />
          </box>
        </box>
      </box>
      {game.paused && (
        <box style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%", alignItems: "center", justifyContent: "center" }}>
          <Settings />
        </box>
      )}
      {game.over && !game.paused && (
        <box style={{ position: "absolute", left: 0, top: 0, width: "100%", height: "100%" }}>
          <RoundOver score={game.score} best={game.best} />
        </box>
      )}
    </box>
  );
}

createRoot().render(<Hud />);
