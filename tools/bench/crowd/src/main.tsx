// SPDX-License-Identifier: Apache-2.0

// A bench scene (not an example): 50 mannequins jogging in a grid in front of the camera, each
// with its own Animator. Keys: 1 / 2 / 3 set every Animator's cullingMode (alwaysAnimate,
// cullUpdateTransforms, cullCompletely); L turns the camera to look away and back.
// To run it (docs/benchmarks.md, "Animator"): copy examples/fog-white-room/assets/mannequin.glb and
// its .meta into assets/, build with `platypus-build build`, then
// `platypus tools/bench/crowd --bench --bench-input tools/bench/crowd/bench.json --fixed-dt 0.0166667`.

const COUNT = 50;
const MODES: AnimatorCullingMode[] = ["alwaysAnimate", "cullUpdateTransforms", "cullCompletely"];

class Crowd extends Component {
  private camera!: Camera;
  private animators: Animator[] = [];
  private away = false;

  override onStart() {
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.4, 0.4, 0.45, 1);
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0.2, -1, 0.3)) });
    sun.addComponent(Light, { type: "directional", intensity: 1.2, shadows: "soft" });
    const floor = new GameObject("Floor", { position: new Vector3(0, -0.05, 0), scale: new Vector3(200, 0.1, 200) });
    floor.addComponent(MeshRenderer, { mesh: "cube", material: Material.create({ color: new Color(0.8, 0.8, 0.82, 1) }) });
    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 55, nearClipPlane: 0.1, farClipPlane: 200 });
    this.look();
    this.load();
  }

  private async load() {
    const model = await Model.load("assets/mannequin.glb").whenLoaded();
    const jog = model.animations.find((a) => a.name === "Jog_Fwd_Loop")!;
    jog.wrapMode = "loop";
    const controller = new AnimatorController().addState("Run", jog);
    for (let i = 0; i < COUNT; i++) {
      const at = new Vector3((i % 10) * 1.5 - 6.75, 0, 6 + Math.floor(i / 10) * 1.5);
      const figure = model.instantiate({ position: at, rotation: Quaternion.euler(0, 180, 0) });
      figure.getComponent(Animation)?.destroy();
      const a = figure.addComponent(Animator, { controller });
      a.play("Run", i / COUNT);
      this.animators.push(a);
    }
  }

  /** At the crowd from the origin, or the other way. */
  private look() {
    const cam = this.camera.transform;
    cam.position = new Vector3(0, 2, 0);
    const target = new Vector3(0, 0.5, this.away ? -20 : 10);
    cam.rotation = Quaternion.lookRotation(target.sub(cam.position));
  }

  override onUpdate() {
    for (let k = 0; k < MODES.length; k++) {
      if (input.getKeyDown(`Digit${k + 1}`)) for (const a of this.animators) a.cullingMode = MODES[k]!;
    }
    if (input.getKeyDown("KeyL")) {
      this.away = !this.away;
      this.look();
    }
  }
}

register({ Crowd });
