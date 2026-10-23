// SPDX-License-Identifier: Apache-2.0

// Cloth: three `Cloth` components on meshes made in code.
// - A flag pinned along its pole edge, flapping in gusty wind (`randomAcceleration`).
// - A sheet dropped onto a ball and a capsule (sphere and capsule colliders).
// - A curtain pinned at points along its top rail, swaying in a light breeze.

/** A sheet `w` × `h` metres in the xy plane (facing +z), with `cols` × `rows` cells. */
function sheet(w: number, h: number, cols: number, rows: number): Mesh {
  const vertices: Vector3[] = [];
  const uv: Vector2[] = [];
  const triangles: number[] = [];
  for (let r = 0; r <= rows; r++) {
    for (let c = 0; c <= cols; c++) {
      vertices.push(new Vector3((c / cols - 0.5) * w, (0.5 - r / rows) * h, 0));
      uv.push(new Vector2(c / cols, 1 - r / rows));
    }
  }
  const stride = cols + 1;
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const a = r * stride + c;
      triangles.push(a, a + stride, a + 1, a + 1, a + stride, a + stride + 1);
    }
  }
  return Mesh.create({ vertices, uv, triangles });
}

const fabric = (color: Color) => Material.create({ color, smoothness: 0.15, doubleSided: true });

class Stage extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 45, clearFlags: "skybox" });
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 1.4, 0), distance: 10, pitch: 12, yaw: 10, autoRotate: 0 });
    RenderSettings.skybox = { sunDirection: new Vector3(0.4, 0.6, 0.5) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.4, -0.6, -0.5)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.2, shadows: "soft" });
    new GameObject("Floor").addComponent(MeshRenderer, {
      mesh: "plane",
      material: Material.create({ color: new Color(0.32, 0.34, 0.38, 1), smoothness: 0.2 }),
    });
    const wood = Material.create({ color: new Color(0.45, 0.3, 0.18, 1), smoothness: 0.3 });

    // The flag: a pole, and a 2 × 1.2 m sheet pinned along its left edge.
    const pole = new GameObject("Pole", { position: new Vector3(-3.5, 1.75, 0), scale: new Vector3(0.08, 1.75, 0.08) });
    pole.addComponent(MeshRenderer, { mesh: "cylinder", material: wood });
    const flag = new GameObject("Flag", { position: new Vector3(-2.45, 2.8, 0) });
    flag.addComponent(MeshRenderer, { mesh: sheet(2, 1.2, 24, 14), material: fabric(new Color(0.85, 0.15, 0.12, 1)) });
    flag.addComponent(Cloth, {
      pinBox: { min: new Vector3(-1.05, -1, -1), max: new Vector3(-0.95, 1, 1) },
      externalAcceleration: new Vector3(6, 0, 1.5),
      randomAcceleration: new Vector3(8, 2, 6),
      bendingStiffness: 0.05,
      damping: 0.2,
    });

    // The ball and capsule, and a 2.5 m sheet falling onto them.
    const ball = new GameObject("Ball", { position: new Vector3(0, 0.6, 0), scale: Vector3.one.scale(1.2) });
    ball.addComponent(MeshRenderer, { mesh: "sphere", material: Material.create({ color: new Color(0.2, 0.5, 0.9, 1), smoothness: 0.7 }) });
    ball.addComponent(SphereCollider, { radius: 0.5 });
    const post = new GameObject("Post", { position: new Vector3(0.9, 0.7, 0.4), rotation: Quaternion.euler(0, 0, 70), scale: new Vector3(0.5, 0.7, 0.5) });
    post.addComponent(MeshRenderer, { mesh: "capsule", material: Material.create({ color: new Color(0.9, 0.7, 0.2, 1), smoothness: 0.5 }) });
    post.addComponent(CapsuleCollider, { radius: 0.5, height: 2 });
    const drop = new GameObject("Tablecloth", { position: new Vector3(0.3, 2.4, 0), rotation: Quaternion.euler(-90, 0, 0) });
    drop.addComponent(MeshRenderer, { mesh: sheet(2.6, 2.6, 26, 26), material: fabric(new Color(0.95, 0.92, 0.85, 1)) });
    drop.addComponent(Cloth, { sphereColliders: [ball], capsuleColliders: [post], bendingStiffness: 0.02, friction: 0.8 });

    // The curtain: a rail and a sheet pinned at five points along its top.
    const rail = new GameObject("Rail", { position: new Vector3(3.6, 2.9, -0.5), rotation: Quaternion.euler(0, 0, 90), scale: new Vector3(0.05, 1.2, 0.05) });
    rail.addComponent(MeshRenderer, { mesh: "cylinder", material: wood });
    const curtain = new GameObject("Curtain", { position: new Vector3(3.6, 1.8, -0.5) });
    curtain.addComponent(MeshRenderer, { mesh: sheet(2.2, 2.2, 22, 22), material: fabric(new Color(0.35, 0.2, 0.55, 1)) });
    curtain.addComponent(Cloth, {
      pins: [-1.1, -0.55, 0, 0.55, 1.1].map((x) => new Vector3(x, 1.1, 0)),
      pinRadius: 0.06,
      randomAcceleration: new Vector3(0.5, 0, 1.5),
      bendingStiffness: 0.1,
    });
  }
}

register({ Stage });

function Caption() {
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, justifyContent: "flex-end", pointerEvents: "none" }}>
      <box style={{ padding: "8px 14px", borderRadius: 8, backgroundColor: "rgba(10,12,18,0.6)", flexDirection: "column", gap: 2, alignSelf: "flex-start" }}>
        <text style={{ color: "#eef0f4", fontSize: 15, fontWeight: 600 }}>Cloth</text>
        <text style={{ color: "#b5bcc8", fontSize: 12 }}>A flag in gusty wind · a sheet over a ball and a capsule · a curtain pinned along its rail</text>
      </box>
    </box>
  );
}

createRoot().render(<Caption />);
