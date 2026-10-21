// Quality settings: the same scene at Low, Medium, High and Ultra. Thin things show
// anti-aliasing (the fence, the fan's blades, the cables), their shadows show shadow
// quality, and Low's render scale softens everything but the UI. The menu switches
// levels or single settings; until it's used, the levels cycle every two seconds.
import {
  Camera,
  Color,
  Component,
  GameObject,
  Light,
  LineRenderer,
  Material,
  MeshRenderer,
  QualitySettings,
  Quaternion,
  RenderSettings,
  Time,
  Vector3,
  register,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

let demo = true;

class Fan extends Component {
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, 0, 20 * dt));
  }
}

class Demo extends Component {
  private next = 2;
  override onUpdate() {
    if (!demo || Time.time < this.next) return;
    this.next += 2;
    QualitySettings.setQualityLevel((QualitySettings.getQualityLevel() + 1) % QualitySettings.names.length);
  }
}

function box(name: string, at: Vector3, size: Vector3, material: Material, rotation = Quaternion.identity, parent?: GameObject) {
  const go = new GameObject(name, { position: at, scale: size, rotation, parent });
  go.addComponent(MeshRenderer, { mesh: "cube", material });
  return go;
}

class Scene extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera", { position: new Vector3(0, 1.7, 6.2), rotation: Quaternion.euler(-10, 0, 0) });
    cam.addComponent(Camera, { fieldOfView: 50, clearFlags: "skybox" });
    RenderSettings.skybox = { sunDirection: new Vector3(-0.5, 0.6, 0.4) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0.5, -0.6, -0.4)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.3, shadows: "soft" });

    const floor = new GameObject("Floor", { scale: Vector3.one.scale(1.2) });
    floor.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.55, 0.56, 0.52, 1), smoothness: 0.2 }) });

    // A picket fence across the back.
    const wood = Material.create({ color: new Color(0.62, 0.42, 0.26, 1), smoothness: 0.25 });
    for (let i = 0; i < 25; i++) box(`Picket ${i}`, new Vector3(-4.8 + i * 0.4, 0.6, -1.5), new Vector3(0.06, 1.2, 0.04), wood);
    box("Rail", new Vector3(0, 0.95, -1.53), new Vector3(10, 0.06, 0.04), wood);
    box("Rail", new Vector3(0, 0.3, -1.53), new Vector3(10, 0.06, 0.04), wood);

    // A fan of thin blades, turning: without anti-aliasing their edges crawl.
    const metal = Material.create({ color: new Color(0.85, 0.86, 0.9, 1), metallic: 1, smoothness: 0.75 });
    const hub = new GameObject("Fan", { position: new Vector3(-2, 1.5, 0.4) });
    hub.addComponent(Fan);
    for (let i = 0; i < 12; i++) {
      box(`Blade ${i}`, Vector3.zero, new Vector3(0.025, 1.6, 0.02), metal, Quaternion.euler(0, 0, i * 15), hub);
    }

    // Cables strung between two posts.
    box("Post", new Vector3(1.2, 1.1, 0.3), new Vector3(0.08, 2.2, 0.08), wood);
    box("Post", new Vector3(4, 1.1, 0.3), new Vector3(0.08, 2.2, 0.08), wood);
    for (let i = 0; i < 4; i++) {
      const y = 1.9 - i * 0.25;
      const points: Vector3[] = [];
      for (let k = 0; k <= 12; k++) {
        const t = k / 12;
        points.push(new Vector3(1.2 + t * 2.8, y - Math.sin(t * Math.PI) * (0.25 + i * 0.05), 0.3));
      }
      const cable = new GameObject(`Cable ${i}`);
      cable.addComponent(LineRenderer, {
        positions: points,
        width: 0.015,
        material: Material.create({ color: new Color(0.08, 0.08, 0.09, 1), smoothness: 0.6 }),
      });
    }

    // A chrome ball and a matte one.
    const ball = new GameObject("Chrome", { position: new Vector3(0.2, 0.55, 1.2), scale: Vector3.one.scale(1.1) });
    ball.addComponent(MeshRenderer, { mesh: "sphere", material: Material.create({ color: Color.white, metallic: 1, smoothness: 0.95 }) });
    const clay = new GameObject("Clay", { position: new Vector3(2.4, 0.4, 1.6), scale: Vector3.one.scale(0.8) });
    clay.addComponent(MeshRenderer, { mesh: "sphere", material: Material.create({ color: new Color(0.75, 0.3, 0.25, 1), smoothness: 0.1 }) });

    this.gameObject.addComponent(Demo);
  }
}

register({ Scene });

/** Re-renders when the level changes (or when asked to, after a single setting). */
function useQuality() {
  const [, setTick] = useState(0);
  useEffect(() => QualitySettings.onLevelChanged(() => setTick((t) => t + 1)), []);
  return () => setTick((t) => t + 1);
}

function Button(props: { label: string; active?: boolean; onClick: () => void }) {
  return (
    <box
      onClick={() => {
        demo = false;
        props.onClick();
      }}
      style={{
        padding: "6px 12px",
        borderRadius: 6,
        cursor: "pointer",
        backgroundColor: props.active ? "#4d8dff" : "rgba(255,255,255,0.1)",
      }}
    >
      <text style={{ color: "#f0f3f8", fontSize: 13 }}>{props.label}</text>
    </box>
  );
}

function next<T>(values: readonly T[], current: T): T {
  return values[(values.indexOf(current) + 1) % values.length]!;
}

function Menu() {
  const refresh = useQuality();
  const level = QualitySettings.getQualityLevel();
  const row = (label: string, value: string, onClick: () => void) => (
    <box style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 120 }}>{label}</text>
      <Button
        label={value}
        onClick={() => {
          onClick();
          refresh();
        }}
      />
    </box>
  );
  const shadowNames = { disable: "Off", hardOnly: "Hard only", all: "Soft" };
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box
        style={{
          pointerEvents: "auto",
          alignSelf: "flex-start",
          padding: 14,
          gap: 10,
          borderRadius: 10,
          backgroundColor: "rgba(12, 14, 20, 0.78)",
          flexDirection: "column",
        }}
      >
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>
          {`Quality: ${QualitySettings.names[level]}${demo ? " (cycling)" : ""}`}
        </text>
        <box style={{ flexDirection: "row", gap: 6 }}>
          {QualitySettings.names.map((name, i) => (
            <Button
              key={name}
              label={name}
              active={i === level}
              onClick={() => {
                QualitySettings.setQualityLevel(i);
                refresh();
              }}
            />
          ))}
        </box>
        {row("Anti-aliasing", QualitySettings.antiAliasing === 1 ? "Off" : `${QualitySettings.antiAliasing}x MSAA`, () => {
          QualitySettings.antiAliasing = next([1, 2, 4, 8], QualitySettings.antiAliasing);
        })}
        {row("Render scale", `${Math.round(QualitySettings.renderScale * 100)}%`, () => {
          QualitySettings.renderScale = next([0.5, 0.75, 1, 1.5], QualitySettings.renderScale);
        })}
        {row("Shadows", shadowNames[QualitySettings.shadows], () => {
          QualitySettings.shadows = next(["disable", "hardOnly", "all"] as const, QualitySettings.shadows);
        })}
        {row("Soft shadows", { low: "Low (3x3)", medium: "Medium (5x5)", high: "High (7x7)" }[QualitySettings.softShadowQuality], () => {
          QualitySettings.softShadowQuality = ({ low: "medium", medium: "high", high: "low" } as const)[QualitySettings.softShadowQuality];
        })}
        {row("Shadow map", `${QualitySettings.shadowResolution}`, () => {
          QualitySettings.shadowResolution = next([512, 1024, 2048, 4096], QualitySettings.shadowResolution);
        })}
      </box>
    </box>
  );
}

createRoot().render(<Menu />);
