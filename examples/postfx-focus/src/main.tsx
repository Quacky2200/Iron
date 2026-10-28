// SPDX-License-Identifier: Apache-2.0

// Blur and depth of field: three ways to throw things out of focus.
//   1  depth of field: a lens focused on one ball (Tab moves focus; the far lamps turn into bokeh discs)
//   2  tilt-shift: a sharp band across the picture, a "miniature" look
//   3  gaussian blur over everything
// Up / Down change the blur amount (aperture opening, blur size, radius).
//
// `iron examples/postfx-focus -- 2` starts in mode 2 (for screenshots).

import { buildStage, sway } from "stage";

type Mode = 0 | 1 | 2;
const NAMES = ["Depth of field", "Tilt-shift", "Gaussian blur"];
const NOTES = [
  "Only one ball is sharp; Tab pulls focus to the next. Bright lamps far away become discs.",
  "A sharp band across the middle; above and below it the world blurs, as in a model village.",
  "The same blur over the whole picture, spread by radius.",
];

let mode: Mode = 0;
let strength = 1;
let focusBall = 1;
let refresh = () => {};
let passes: PostProcess[] = [];

function select(m: Mode): void {
  mode = m;
  passes.forEach((p, i) => (p.enabled = i === m));
  refresh();
}

class Focus extends Component {
  private camera!: Camera;
  private balls: GameObject[] = [];
  private focus = 11;
  private t = 0;

  override onStart() {
    const stage = buildStage();
    this.camera = stage.camera;
    this.balls = stage.balls;
    const dof = PostProcess.create("depthOfField");
    dof.setFloat("focal_length", 85);
    dof.setFloat("max_blur", 18);
    dof.setFloat("blades", 6);
    const tilt = PostProcess.create("tiltShift");
    tilt.setFloat("focus", 0.55);
    tilt.setFloat("width", 0.1);
    const blur = PostProcess.create("blur");
    passes = [dof, tilt, blur];
    const given = Number(Application.args[0]);
    select(given >= 1 && given <= 3 ? ((given - 1) as Mode) : 0);
  }

  override onUpdate(dt: number) {
    this.t += dt;
    for (let i = 0; i < 3; i++) if (input.getKeyDown(`Digit${i + 1}`)) select(i as Mode);
    if (input.getKeyDown("Tab")) focusBall = (focusBall + 1) % 3;
    if (input.getKeyDown("ArrowUp")) strength = Math.min(3, strength + 0.25);
    if (input.getKeyDown("ArrowDown")) strength = Math.max(0.25, strength - 0.25);
    passes[0]!.setFloat("aperture", 1.6 / strength);
    passes[1]!.setFloat("max_blur", 16 * strength);
    passes[2]!.setFloat("radius", 8 * strength);
    sway(this.camera, this.t);
    // The lens pulls focus to the chosen ball, easing as a real one does.
    const target = this.balls[focusBall]!.transform.position.sub(this.camera.transform.position).magnitude;
    this.focus += (target - this.focus) * (1 - Math.exp(-dt * 4));
    passes[0]!.setFloat("focus_distance", this.focus);
    refresh();
  }
}

register({ Focus });

function Hud() {
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 5, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Blur and depth of field</text>
        {NAMES.map((n, i) => (
          <text key={n} style={{ color: i === mode ? "#7fb0ff" : "#a9b3c3", fontSize: 13 }}>{`${i + 1}  ${n}`}</text>
        ))}
        <text style={{ color: "#f0f3f8", fontSize: 12 }}>{NOTES[mode]}</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>{`Up / Down: amount x${strength.toFixed(2)}    Tab: focus on ball ${focusBall + 1}`}</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
