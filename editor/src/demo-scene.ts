// A scene to look at in the Scene view until scenes open from the project: a few
// shapes on a floor, the three kinds of light, a camera, an audio source and a
// reflection probe (so every SceneIcon shows).

import {
  AudioSource,
  Camera,
  Color,
  createPrimitive,
  GameObject,
  Light,
  Material,
  MeshRenderer,
  Quaternion,
  Rect,
  ReflectionProbe,
  RenderSettings,
  Vector3,
} from "@iron/engine";

const paint = (go: GameObject, color: Color, o: { metallic?: number; smoothness?: number } = {}) => {
  go.getComponent(MeshRenderer)!.material = Material.create({ color, ...o });
};

export function buildDemoScene(): void {
  RenderSettings.skybox = {};
  const floor = createPrimitive("plane", { name: "Floor", scale: new Vector3(2, 1, 2) });
  paint(floor, new Color(0.2, 0.21, 0.22, 1), { smoothness: 0.1 });

  const shapes = new GameObject("Shapes");
  paint(createPrimitive("cube", { name: "Cube", parent: shapes, position: new Vector3(-2.5, 0.5, 0) }), Color.hex("#d9663a"));
  paint(createPrimitive("sphere", { name: "Sphere", parent: shapes, position: new Vector3(0, 0.75, -1), scale: new Vector3(1.5, 1.5, 1.5) }), Color.hex("#c8ccd2"), {
    metallic: 1,
    smoothness: 0.85,
  });
  paint(createPrimitive("cylinder", { name: "Cylinder", parent: shapes, position: new Vector3(2.5, 1, 0.5) }), Color.hex("#3d8ef0"));
  paint(
    createPrimitive("capsule", { name: "Capsule", parent: shapes, position: new Vector3(0.5, 1, 2.5), rotation: Quaternion.euler(0, 0, 90) }),
    Color.hex("#7ac74f"),
  );

  const lights = new GameObject("Lights");
  new GameObject("Directional Light", { parent: lights, position: new Vector3(-4, 6, -3), rotation: Quaternion.euler(-50, -30, 0) }).addComponent(Light, {
    type: "directional",
    intensity: 1.1,
    shadows: "soft",
  });
  new GameObject("Point Light", { parent: lights, position: new Vector3(3.5, 2.5, -2.5) }).addComponent(Light, {
    type: "point",
    color: Color.hex("#ffb36b"),
    intensity: 2,
    range: 6,
  });
  new GameObject("Spot Light", { parent: lights, position: new Vector3(-3, 4, 3), rotation: Quaternion.lookRotation(new Vector3(0.6, -1, -0.6)) }).addComponent(
    Light,
    { type: "spot", color: Color.hex("#9fc3ff"), intensity: 3, range: 9, spotAngle: 40 },
  );

  // The game's camera; the Game panel shows it (its rect follows the panel).
  const camera = new GameObject("Main Camera", { position: new Vector3(0, 3, 9), rotation: Quaternion.lookRotation(new Vector3(0, -0.3, -1)) });
  camera.addComponent(Camera, { clearFlags: "skybox", rect: new Rect(0, 0, 0, 0) });

  new GameObject("Music", { position: new Vector3(4, 0.5, 3.5) }).addComponent(AudioSource);
  new GameObject("Reflection Probe", { position: new Vector3(0, 1.5, 0) }).addComponent(ReflectionProbe, { size: new Vector3(12, 6, 12) });
}
