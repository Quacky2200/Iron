// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The classic computer graphics test models, as glTF: the Stanford Bunny and the Newell
// teaset (teapot, teacup, teaspoon), each turning on a pedestal under a key light, a
// rim light and spotlights while the camera orbits. Behind them, the classic material
// sweep: spheres from matte to glossy (`smoothness` 0 to 1).
//
// Models: the Stanford Bunny (Stanford Computer Graphics Laboratory, 1994) and the
// Newell teaset (Martin Newell, University of Utah, 1975), converted to .glb by
// tools/make_models.py.

class Turntable extends Component {
  degreesPerSecond = 20;
  override onUpdate(dt: number) {
    this.transform.rotate(new Vector3(0, this.degreesPerSecond * dt, 0));
  }
}

class Gallery extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 45, clearFlags: "skybox" });
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 0.8, 0), distance: 6.5, pitch: 18, yaw: 10, autoRotate: 6 });
    // A dusk sky: reflections (the silver spoon, the glazed teapot) and soft sky light.
    RenderSettings.skybox = {
      zenith: new Color(0.05, 0.07, 0.14, 1),
      horizon: new Color(0.45, 0.32, 0.28, 1),
      ground: new Color(0.06, 0.06, 0.07, 1),
      sunDirection: new Vector3(-0.6, 0.15, -0.8),
      sunColor: new Color(12, 7, 4, 1),
      sunSize: 3,
    };

    const floor = new GameObject("Floor", { scale: Vector3.one.scale(1.6) });
    floor.addComponent(MeshRenderer, { mesh: "plane", material: new Material(Color.hex("#3a3f4b"), Color.black, 0.5) });

    const key = new GameObject("Key light", { rotation: Quaternion.euler(-45, 35, 0) });
    key.addComponent(Light, { type: "directional", intensity: 0.9, color: Color.hex("#fff2e0"), shadows: "soft" });
    QualitySettings.shadowDistance = 14;
    const rim = new GameObject("Rim light", { rotation: Quaternion.euler(-20, 200, 0) });
    rim.addComponent(Light, { type: "directional", intensity: 0.35, color: Color.hex("#a8c4ff") });

    const pieces = [
      { file: "assets/bunny.glb", at: new Vector3(-2.4, 0, 0.3), scale: 1.1 },
      { file: "assets/teapot.glb", at: new Vector3(-0.6, 0, -0.4), scale: 1 },
      { file: "assets/teacup.glb", at: new Vector3(1.1, 0, 0.4), scale: 1 },
      // The spoon is small next to the pot: shown three times its size.
      { file: "assets/spoon.glb", at: new Vector3(2.5, 0, -0.2), scale: 3 },
    ];
    // The material sweep: one colour, smoothness from 0 to 1.
    for (let i = 0; i < 6; i++) {
      const ball = new GameObject(`Sphere ${i}`, { position: new Vector3(-2.5 + i, 0.45, -2.4), scale: Vector3.one.scale(0.8) });
      ball.addComponent(MeshRenderer, { mesh: "sphere", material: new Material(Color.hex("#c23a3a"), Color.black, i / 5) });
    }

    const pedestal = new Material(Color.hex("#d9d4cc"), Color.black, 0.3);
    pieces.forEach((p, i) => {
      const base = new GameObject(`Pedestal ${i}`, { position: p.at.add(new Vector3(0, 0.15, 0)), scale: new Vector3(1.3, 0.15, 1.3) });
      base.addComponent(MeshRenderer, { mesh: "cylinder", material: pedestal });
      const spot = new GameObject(`Spot ${i}`, { position: p.at.add(new Vector3(0, 4.5, 2)) });
      spot.transform.lookAt(p.at.add(new Vector3(0, 0.5, 0)));
      spot.addComponent(Light, { type: "spot", intensity: 18, range: 8, spotAngle: 32, color: Color.hex("#fff6e8") });
      const model = Model.load(p.file).instantiate({ position: p.at.add(new Vector3(0, 0.3, 0)), scale: Vector3.one.scale(p.scale) });
      model.addComponent(Turntable, { degreesPerSecond: 15 + i * 5 });
    });
  }
}

register({ Gallery });

function Credits() {
  return (
    <box style={{ width: "100%", height: "100%", padding: 20, justifyContent: "flex-end", pointerEvents: "none" }}>
      <box style={{ padding: "8px 14px", borderRadius: 8, backgroundColor: "rgba(10,12,18,0.6)", flexDirection: "column", gap: 2, alignSelf: "flex-start" }}>
        <text style={{ color: "#e8eaf0", fontSize: 15, fontWeight: 600 }}>Classic test models</text>
        <text style={{ color: "#a9b0c0", fontSize: 12 }}>Stanford Bunny — Stanford Computer Graphics Laboratory</text>
        <text style={{ color: "#a9b0c0", fontSize: 12 }}>Newell teaset — Martin Newell, University of Utah</text>
      </box>
    </box>
  );
}

createRoot().render(<Credits />);
