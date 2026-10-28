// SPDX-License-Identifier: Apache-2.0

// Chromatic aberration alone: red and blue slide apart towards the edges, like a cheap lens.
// Look at the bright lamps and the rails near the screen edges for coloured fringes.
// Space turns it on and off, Up / Down change the amount.
//
// `platypus-game examples/postfx-aberration -- 0.03` starts with that amount (for screenshots).

import { buildStage, sway } from "stage";

let pass: PostProcess;
let amount = 0.02;
let refresh = () => {};

class Aberration extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    this.camera = buildStage().camera;
    const given = Number(Application.args[0]);
    if (given > 0) amount = given;
    pass = PostProcess.create("chromaticAberration");
    pass.setFloat("amount", amount);
    refresh();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("Space")) pass.enabled = !pass.enabled;
    if (input.getKeyDown("ArrowUp")) amount = Math.min(0.08, amount + 0.005);
    if (input.getKeyDown("ArrowDown")) amount = Math.max(0, amount - 0.005);
    pass.setFloat("amount", amount);
    sway(this.camera, this.t);
    refresh();
  }
}

register({ Aberration });

function Hud() {
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 5, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Chromatic aberration</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>Look for red and blue fringes on the lamps, rails and edges of the picture.</text>
        <text style={{ color: "#7fb0ff", fontSize: 13 }}>{`${pass?.enabled === false ? "OFF" : "ON"}   amount ${amount.toFixed(3)}`}</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>Space: on / off    Up / Down: amount</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
