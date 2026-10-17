// M5.a showcase: GameObjects in 3D. A camera orbits a floor with standing coloured
// planes; a ring of panels spins as one object (children follow their parent), and
// screen UI is drawn over the world.
import {
  Camera,
  Color,
  Component,
  GameObject,
  Mathf,
  PlaneRenderer,
  Quaternion,
  register,
  Time,
  Vector2,
  Vector3,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useEffect, useState } from "react";

/** Moves the camera round the origin, always looking at the centre. */
class Orbit extends Component {
  radius = 7;
  height = 3;
  speed = 15;
  override onUpdate() {
    const angle = (Time.time * this.speed + 20) * Mathf.Deg2Rad;
    this.transform.position = new Vector3(Math.sin(angle) * this.radius, this.height, Math.cos(angle) * this.radius);
    this.transform.lookAt(new Vector3(0, 0.8, 0));
  }
}

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
    cam.addComponent(Orbit);

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
      panel.addComponent(PlaneRenderer, {
        size: new Vector2(1.4, 1.6),
        color: Color.hex(["#ff6b6b", "#ffd93d", "#6bcB77", "#4d96ff", "#b46bff", "#ff9f45"][i]!),
      });
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
    const id = setTimeout(() => setT(Time.time), 100);
    return () => clearTimeout(id);
  }, [t]);
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
