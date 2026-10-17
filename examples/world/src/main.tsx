// M5.a showcase: GameObjects in 3D. A camera orbits a floor with standing coloured
// planes; a ring of panels spins as one object (children follow their parent). An
// orthographic camera draws a top-down minimap in the corner (camera depth and rect),
// and screen UI is drawn over the world.
import {
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
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

class Spin extends Component {
  degreesPerSecond = 40;
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.degreesPerSecond * dt, 0));
  }
}

class World extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 55, backgroundColor: Color.hex("#0b0e14") });
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 0.8, 0), distance: 7.5, pitch: 22, yaw: 20, autoRotate: 15 });

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
    // A see-through pane in the middle.
    const glass = new GameObject("Glass", { position: new Vector3(0, 1, 0), rotation: Quaternion.euler(90, 0, 0) });
    glass.addComponent(PlaneRenderer, { size: new Vector2(2, 2), color: new Color(0.6, 0.9, 1, 0.35) });
  }
}

register({ World });

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
    <box style={{ width: "100%", height: "100%", padding: 20 }}>
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
