// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

// Vignette, film grain and colour grading: the cheap, subtle effects that make a render feel like film.
//   V  vignette (darker corners)    G  film grain (moving noise)    C  colour grading (warm, contrasty)
//   A  all three off, to compare    Up / Down  strength of all three
//
// `platypus-app examples/postfx-film -- 1.5` starts at that strength (for screenshots).

import { buildStage, sway } from "stage";

let vignette: PostProcess;
let grain: PostProcess;
let grade: PostProcess;
let strength = 1;
let refresh = () => {};

function apply(): void {
  vignette.setFloat("intensity", 0.6 * strength);
  grain.setFloat("intensity", 0.18 * strength);
  grade.setFloat("contrast", 1 + 0.15 * strength);
  grade.setFloat("saturation", 1 + 0.2 * strength);
  refresh();
}

class Film extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    this.camera = buildStage().camera;
    grade = PostProcess.create("colorGrading");
    grade.order = 0;
    grade.setVector("balance", 0.25, 0.05);
    grade.setVector("lift", 0.02, 0.0, 0.05);
    grade.setVector("gain", 1.05, 1.0, 0.92);
    vignette = PostProcess.create("vignette");
    vignette.order = 10;
    grain = PostProcess.create("filmGrain");
    grain.order = 20;
    const given = Number(Application.args[0]);
    if (given > 0) strength = given;
    apply();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyV")) vignette.enabled = !vignette.enabled;
    if (input.getKeyDown("KeyG")) grain.enabled = !grain.enabled;
    if (input.getKeyDown("KeyC")) grade.enabled = !grade.enabled;
    if (input.getKeyDown("KeyA")) {
      const any = vignette.enabled || grain.enabled || grade.enabled;
      vignette.enabled = grain.enabled = grade.enabled = !any;
    }
    if (input.getKeyDown("ArrowUp")) strength = Math.min(3, strength + 0.25);
    if (input.getKeyDown("ArrowDown")) strength = Math.max(0, strength - 0.25);
    apply();
    sway(this.camera, this.t);
  }
}

register({ Film });

function Hud() {
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  const on = (p?: PostProcess) => (p?.enabled === false ? "off" : "on");
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 5, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Vignette, grain and grading</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>Look for darker corners, shimmering noise in the midtones, and a warm tint.</text>
        <text style={{ color: "#7fb0ff", fontSize: 13 }}>{`V vignette ${on(vignette)}   G grain ${on(grain)}   C grading ${on(grade)}`}</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>{`A: all on / off    Up / Down: strength x${strength.toFixed(2)}`}</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
