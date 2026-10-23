// SPDX-License-Identifier: Apache-2.0

// Animation: clips, players and skinning.
// - A robot made of primitives with an `Animator`: a locomotion blend tree (idle, walk,
//   run by `speed`) and a jump on a trigger that returns by exit time. It speeds up and
//   slows down on its own and jumps every few seconds; arrow keys and space take over.
// - A worm from a glTF file: a skinned mesh whose bones play the file's animation.
// - A tentacle made in code: a skinned `Mesh` (bone weights, bind poses) bent by a looping
//   clip on its bone chain.

/** A looping curve: `amplitude` × sin over `period` seconds (smooth keys). */
function wave(amplitude: number, period: number, phase = 0, offset = 0): AnimationCurve {
  const points: [number, number][] = [];
  const steps = 16;
  for (let i = 0; i <= steps; i++) {
    const t = (i / steps) * period;
    points.push([t, offset + amplitude * Math.sin((i / steps) * Math.PI * 2 + phase)]);
  }
  const curve = AnimationCurve.fromPoints(points, "smooth");
  curve.postWrapMode = "loop";
  return curve;
}

/** A clip swinging the robot's limbs (degrees about x) and bobbing its body. */
function gait(name: string, period: number, legs: number, arms: number, bob: number): AnimationClip {
  const clip = new AnimationClip(name);
  clip.wrapMode = "loop";
  clip.setCurve("Body", "Transform", "localPosition.y", wave(bob, period / 2, 0, 1.1));
  clip.setCurve("Body/Left leg", "Transform", "localEulerAngles.x", wave(legs, period));
  clip.setCurve("Body/Right leg", "Transform", "localEulerAngles.x", wave(legs, period, Math.PI));
  clip.setCurve("Body/Left arm", "Transform", "localEulerAngles.x", wave(arms, period, Math.PI));
  clip.setCurve("Body/Right arm", "Transform", "localEulerAngles.x", wave(arms, period));
  // Every gait lasts one period so blending lines the steps up.
  return clip;
}

function jumpClip(): AnimationClip {
  const clip = new AnimationClip("jump");
  clip.setCurve("Body", "Transform", "localPosition.y", AnimationCurve.fromPoints([[0, 1.1], [0.15, 0.95], [0.45, 2.2], [0.8, 1.1]], "smooth"));
  clip.setCurve("Body/Left arm", "Transform", "localEulerAngles.x", AnimationCurve.fromPoints([[0, 0], [0.3, -150], [0.8, 0]], "smooth"));
  clip.setCurve("Body/Right arm", "Transform", "localEulerAngles.x", AnimationCurve.fromPoints([[0, 0], [0.3, -150], [0.8, 0]], "smooth"));
  return clip;
}

/** A box limb hanging from a pivot (its top), so rotating the pivot swings it. */
function limb(name: string, parent: GameObject, at: Vector3, length: number, material: Material): GameObject {
  const pivot = new GameObject(name, { parent, position: at });
  const part = new GameObject(`${name} mesh`, { parent: pivot, position: new Vector3(0, -length / 2, 0), scale: new Vector3(0.22, length, 0.22) });
  part.addComponent(MeshRenderer, { mesh: "cube", material });
  return pivot;
}

const state = { name: "", speed: 0 };
/** The HUD's re-render, called by the driver a few times a second. */
let refreshHud: () => void = () => {};

/** Drives the robot: automatic cycling until a key is pressed. */
class Driver extends Component {
  animator!: Animator;
  private t = 0;
  private manual = false;
  private speed = 0;
  private nextJump = 3;

  override onUpdate(dt: number) {
    this.t += dt;
    const up = input.getKey("ArrowUp");
    const down = input.getKey("ArrowDown");
    if (up || down || input.getKeyDown("Space")) this.manual = true;
    if (this.manual) {
      if (up) this.speed = Math.min(2, this.speed + dt);
      if (down) this.speed = Math.max(0, this.speed - dt);
      if (input.getKeyDown("Space")) this.animator.setTrigger("jump");
    } else {
      // 0 → 2 → 0 over 12 s, jumping now and then.
      this.speed = 1 - Math.cos((this.t / 12) * Math.PI * 2);
      if (this.t > this.nextJump) {
        this.animator.setTrigger("jump");
        this.nextJump = this.t + 5;
      }
    }
    this.animator.setFloat("speed", this.speed);
    state.name = this.animator.getCurrentAnimatorStateInfo().name;
    state.speed = this.speed;
    if (Math.floor(this.t * 10) !== Math.floor((this.t - dt) * 10)) refreshHud();
  }
}

class Stage extends Component {
  override onStart() {
    const cam = new GameObject("Main Camera");
    cam.addComponent(Camera, { fieldOfView: 45, clearFlags: "skybox" });
    cam.addComponent(CameraOrbit, { center: new Vector3(0, 1.2, 0), distance: 9, pitch: 12, yaw: 15, autoRotate: 0 });
    RenderSettings.skybox = { sunDirection: new Vector3(0.4, 0.6, 0.5) };
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(-0.4, -0.6, -0.5)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.2, shadows: "soft" });
    const floor = new GameObject("Floor");
    floor.addComponent(MeshRenderer, { mesh: "plane", material: Material.create({ color: new Color(0.3, 0.32, 0.36, 1), smoothness: 0.3 }) });

    // The robot.
    const steel = Material.create({ color: new Color(0.75, 0.78, 0.82, 1), metallic: 0.8, smoothness: 0.6 });
    const accent = Material.create({ color: new Color(0.95, 0.45, 0.15, 1), smoothness: 0.5 });
    const robot = new GameObject("Robot", { position: new Vector3(0, 0, 0) });
    const body = new GameObject("Body", { parent: robot, position: new Vector3(0, 1.1, 0) });
    const torso = new GameObject("Torso", { parent: body, position: new Vector3(0, 0.45, 0), scale: new Vector3(0.7, 0.9, 0.4) });
    torso.addComponent(MeshRenderer, { mesh: "cube", material: steel });
    const head = new GameObject("Head", { parent: body, position: new Vector3(0, 1.15, 0), scale: Vector3.one.scale(0.4) });
    head.addComponent(MeshRenderer, { mesh: "sphere", material: accent });
    limb("Left arm", body, new Vector3(-0.47, 0.85, 0), 0.75, accent);
    limb("Right arm", body, new Vector3(0.47, 0.85, 0), 0.75, accent);
    limb("Left leg", body, new Vector3(-0.2, 0, 0), 1.0, steel);
    limb("Right leg", body, new Vector3(0.2, 0, 0), 1.0, steel);
    const controller = new AnimatorController()
      .addParameter("speed", "float", 0)
      .addParameter("jump", "trigger")
      .addState("Locomotion", {
        blendParameter: "speed",
        children: [
          { motion: gait("idle", 1, 2, 4, 0.02), threshold: 0 },
          { motion: gait("walk", 1, 30, 25, 0.05), threshold: 1 },
          { motion: gait("run", 1, 55, 60, 0.12), threshold: 2 },
        ],
      }, { speed: 1, speedParameter: undefined })
      .addState("Jump", jumpClip())
      .addTransition({ from: "Locomotion", to: "Jump", conditions: [{ parameter: "jump", mode: "if" }], duration: 0.1 })
      .addTransition({ from: "Jump", to: "Locomotion", hasExitTime: true, exitTime: 0.95, duration: 0.15 });
    const animator = robot.addComponent(Animator, { controller });
    robot.addComponent(Driver, { animator });

    // The worm: a skinned glTF with its own animation (an `Animation` added by the model).
    Model.load("assets/worm.glb").instantiate({ position: new Vector3(-3, 0, -0.5) });

    // The tentacle: a skinned mesh made in code, four bones up a tapering tube.
    tentacle(new Vector3(3, 0, -0.5));
  }
}

/** A tapering tube skinned to a chain of four bones, swaying on a looping clip. */
function tentacle(at: Vector3): void {
  const root = new GameObject("Tentacle", { position: at });
  const bones: GameObject[] = [];
  let parent = root;
  for (let i = 0; i < 4; i++) {
    const bone = new GameObject(`Bone${i}`, { parent, position: new Vector3(0, i === 0 ? 0 : 0.7, 0) });
    bones.push(bone);
    parent = bone;
  }
  // Rings every 0.175 m up to 2.8 m, weighted to the bones on either side.
  const vertices: Vector3[] = [];
  const normals: Vector3[] = [];
  const boneWeights: { indices: [number, number, number, number]; weights: [number, number, number, number] }[] = [];
  const triangles: number[] = [];
  const sides = 8;
  const rings = 17;
  for (let r = 0; r < rings; r++) {
    const y = (r / (rings - 1)) * 2.8;
    const radius = 0.28 * (1 - y / 3.2);
    const f = Math.min(3, y / 0.7);
    const a = Math.min(3, Math.floor(f));
    const b = Math.min(3, a + 1);
    const t = f - a;
    for (let s = 0; s < sides; s++) {
      const angle = (s / sides) * Math.PI * 2;
      const n = new Vector3(Math.cos(angle), 0, Math.sin(angle));
      vertices.push(new Vector3(n.x * radius, y, n.z * radius));
      normals.push(n);
      boneWeights.push({ indices: [a, b, 0, 0], weights: [1 - t, t, 0, 0] });
    }
  }
  for (let r = 0; r < rings - 1; r++) {
    for (let s = 0; s < sides; s++) {
      const i = r * sides + s;
      const j = r * sides + ((s + 1) % sides);
      triangles.push(i, i + sides, j, j, i + sides, j + sides);
    }
  }
  // Bind poses: each bone's inverse world matrix where the mesh was modelled.
  const bindposes = bones.map((_, i) => Matrix4x4.trs(new Vector3(0, -i * 0.7, 0), Quaternion.identity));
  const mesh = Mesh.create({ vertices, normals, triangles, boneWeights, bindposes });
  const skin = new GameObject("Tentacle skin", { parent: root });
  skin.addComponent(SkinnedMeshRenderer, {
    mesh,
    material: Material.create({ color: new Color(0.55, 0.2, 0.75, 1), smoothness: 0.7 }),
    bones,
    rootBone: bones[0],
  });
  const sway = new AnimationClip("sway");
  sway.wrapMode = "loop";
  let path = "";
  for (let i = 1; i < 4; i++) {
    path = path ? `${path}/Bone${i}` : `Bone0/Bone${i}`;
    sway.setCurve(path, "Transform", "localEulerAngles.z", wave(22, 2.4, i * 0.7));
    sway.setCurve(path, "Transform", "localEulerAngles.x", wave(12, 3.1, i * 1.1));
  }
  root.addComponent(Animation, { clip: sway });
}

register({ Stage });
register({ Driver }, { spawn: false });

function Hud() {
  const [, tick] = useState(0);
  useEffect(() => {
    refreshHud = () => tick((n) => n + 1);
    return () => {
      refreshHud = () => {};
    };
  }, []);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, justifyContent: "flex-end", pointerEvents: "none" }}>
      <box style={{ padding: "8px 14px", borderRadius: 8, backgroundColor: "rgba(10,12,18,0.6)", flexDirection: "column", gap: 2, alignSelf: "flex-start" }}>
        <text style={{ color: "#eef0f4", fontSize: 15, fontWeight: 600 }}>Animation</text>
        <text style={{ color: "#b5bcc8", fontSize: 12 }}>
          Robot: {state.name || "…"} · speed {state.speed.toFixed(2)} (↑ ↓ speed, space jumps)
        </text>
        <text style={{ color: "#b5bcc8", fontSize: 12 }}>Left: skinned glTF worm · right: skinned tentacle made in code</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
