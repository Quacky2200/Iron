// M5.a showcase: GameObjects in 3D. A camera orbits a floor with standing coloured
// planes; a ring of panels spins as one object (children follow their parent). An
// orthographic camera draws a top-down minimap in the corner (camera depth and rect),
// and screen UI is drawn over the world. A menu stands in the world on a WorldPanel:
// React UI on a plane, with hover and clicks through the camera. Falling primitives thud
// in 3D (an AudioSource each, heard from the camera's AudioListener).
import {
  AudioListener,
  AudioSource,
  type Collision,
  Camera,
  CameraOrbit,
  Color,
  Component,
  GameObject,
  PlaneRenderer,
  Quaternion,
  Rect,
  register,
  Time,
  Vector2,
  Vector3,
  WorldPanel,
  addStyleSheet,
  BoxCollider,
  Light,
  Material,
  MeshRenderer,
  Random,
  Rigidbody,
  Rope,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

class Spin extends Component {
  degreesPerSecond = 40;
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.degreesPerSecond * dt, 0));
  }
}

/** Thuds from its object when it lands, louder for harder hits. */
class Thud extends Component {
  onCollisionEnter(c: Collision) {
    if (c.relativeSpeed < 0.8) return;
    this.getComponent(AudioSource)?.playOneShot("assets/thud.wav", Math.min(1, c.relativeSpeed / 6));
  }
}

class World extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 55, backgroundColor: Color.hex("#0b0e14") });
    cam.addComponent(AudioListener);
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 0.8, 0), distance: 7.5, pitch: 22, yaw: 20, autoRotate: 10 });

    // A minimap: straight down, orthographic, drawn over the main view (higher depth).
    const map = new GameObject("Minimap", { position: new Vector3(0, 20, 0), rotation: Quaternion.euler(-90, 0, 0) });
    map.addComponent(Camera, {
      orthographic: true,
      orthographicSize: 6.5,
      depth: 1,
      rect: new Rect(0.76, 0.03, 0.21, 0.35),
      backgroundColor: Color.hex("#05070b"),
    });

    const floor = new GameObject("Floor");
    floor.addComponent(PlaneRenderer, { size: new Vector2(12, 12), color: Color.hex("#4a5263") });
    floor.addComponent(BoxCollider, { size: new Vector3(12, 0.2, 12), center: new Vector3(0, -0.1, 0) });

    // A sun and a warm point light for the meshes.
    const sun = new GameObject("Sun", { rotation: Quaternion.euler(-50, 30, 0) });
    sun.addComponent(Light, { type: "directional", intensity: 1.1 });
    const lamp = new GameObject("Lamp", { position: new Vector3(0, 1.5, 0) });
    lamp.addComponent(Light, { type: "point", color: Color.hex("#ffb060"), intensity: 6, range: 5 });

    // Lit primitives with Rigidbodies, dropped into the ring.
    Random.initState(3);
    const kinds = ["cube", "sphere", "capsule", "cylinder"] as const;
    for (let i = 0; i < 10; i++) {
      const go = GameObject.createPrimitive(kinds[i % 4]!, {
        position: new Vector3(Random.range(-1.2, 1.2), 2 + i * 0.7, Random.range(-1.2, 1.2)),
        rotation: Random.rotation,
        scale: Vector3.one.scale(0.45),
      });
      go.addComponent(Rigidbody, { mass: 1 });
      go.addComponent(AudioSource, { spatialBlend: 1, maxDistance: 25, pitch: 0.9 + (i % 4) * 0.1 });
      go.addComponent(Thud);
      go.getComponent(MeshRenderer)!.material = new Material(Color.hex(["#e85d75", "#5dade8", "#7be07b", "#e8c25d"][i % 4]!), Color.black, 0.7);
    }

    // Six panels on a ring, children of one spinning object.
    const ring = new GameObject("Ring", { position: new Vector3(0, 0.9, 0) });
    ring.addComponent(Spin);
    for (let i = 0; i < 6; i++) {
      const angle = (i / 6) * 360;
      const dir = Quaternion.euler(0, angle, 0).rotate(Vector3.forward);
      const panel = new GameObject(`Panel ${i}`, {
        parent: ring,
        position: dir.scale(2.5),
        // Stand the plane up (it lies flat by default) and face it outwards.
        rotation: Quaternion.euler(0, angle, 0).mul(Quaternion.euler(90, 0, 0)),
      });
      const color = Color.hex(["#ff6b6b", "#ffd93d", "#6bcB77", "#4d96ff", "#b46bff", "#ff9f45"][i]!);
      panel.addComponent(PlaneRenderer, { size: new Vector2(1.4, 1.6), color });
      // A tile on the floor under each panel, so the top-down minimap shows the ring.
      const tile = new GameObject(`Tile ${i}`, { parent: ring, position: dir.scale(2.5).add(new Vector3(0, -0.88, 0)) });
      tile.addComponent(PlaneRenderer, { size: new Vector2(1.2, 1.2), color: color.withAlpha(0.6) });
    }
    // A menu in the world, behind the ring.
    const menu = new GameObject("Menu", { position: new Vector3(0, 3.1, -3.5), rotation: Quaternion.euler(-10, 0, 0) });
    const panel = menu.addComponent(WorldPanel, { width: 640, height: 400, pixelsPerMeter: 170 });
    createRoot(panel).render(<Menu />);

    // A ball on a rope, hung from a point beside the ring and swinging.
    const ball = GameObject.createPrimitive("sphere", { position: new Vector3(4.6, 1.3, 1), scale: Vector3.one.scale(0.6) });
    const ballBody = ball.addComponent(Rigidbody, { mass: 2 });
    ball.getComponent(MeshRenderer)!.material = new Material(Color.hex("#f2f2f2"), Color.black, 0.9);
    const hook = new GameObject("Hook", { position: new Vector3(3.2, 3.6, 1) });
    hook.addComponent(Rope, { length: 2.7, segments: 10, radius: 0.03, end: ballBody });

    // A see-through pane in the middle.
    const glass = new GameObject("Glass", { position: new Vector3(0, 1, 0), rotation: Quaternion.euler(90, 0, 0) });
    glass.addComponent(PlaneRenderer, { size: new Vector2(2, 2), color: new Color(0.6, 0.9, 1, 0.35) });
  }
}

register({ World });

addStyleSheet(`
  .menu-item { padding: 12px 22px; border-radius: 10px; background-color: rgba(255,255,255,0.08);
    transition: background-color 0.15s; cursor: pointer; }
  .menu-item:hover { background-color: #4d96ff; }
  .menu-item text { color: #e8eefc; font-size: 30px; }
`);

function Menu() {
  const [clicks, setClicks] = useState(0);
  return (
    <box
      style={{
        width: "100%",
        height: "100%",
        padding: 28,
        gap: 14,
        flexDirection: "column",
        borderRadius: 24,
        backgroundColor: "rgba(14, 18, 28, 0.88)",
        border: "2px solid rgba(255,255,255,0.18)",
      }}
    >
      <text style={{ color: "#ffd93d", fontSize: 38, fontWeight: 700 }}>World Panel</text>
      {["Play", "Options", `Clicked ${clicks}×`].map((label) => (
        <box key={label} className="menu-item" onClick={() => setClicks((n) => n + 1)}>
          <text>{label}</text>
        </box>
      ))}
    </box>
  );
}

function Hud() {
  const [t, setT] = useState(0);
  useEffect(() => {
    let id = 0;
    const loop = () => {
      setT(Time.time);
      id = setTimeout(loop, 100);
    };
    id = setTimeout(loop, 100);
    return () => clearTimeout(id);
  }, []);
  return (
    // The full-screen layout box lets the pointer through to the world panel.
    <box style={{ width: "100%", height: "100%", padding: 20, pointerEvents: "none" }}>
      <box
        style={{
          alignSelf: "flex-start",
          padding: "10px 16px",
          borderRadius: 10,
          backgroundColor: "rgba(10, 14, 22, 0.7)",
          border: "1px solid rgba(255,255,255,0.15)",
          flexDirection: "column",
          gap: 4,
        }}
      >
        <text style={{ color: "#e8eefc", fontSize: 18, fontWeight: 600 }}>Iron — GameObjects in 3D</text>
        <text style={{ color: "#9fb0cc", fontSize: 14 }}>{`Camera orbiting · ${t.toFixed(1)} s`}</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
