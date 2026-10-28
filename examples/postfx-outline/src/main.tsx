// SPDX-License-Identifier: Apache-2.0

// Edge detection: ink outlines drawn from the depth and normals of the picture, with cel bands
// for a comic look. Silhouettes and creases get a line; flat floor and sky do not.
//   O  outlines on / off     P  cel bands on / off     C  also ink brightness edges (shadows, texture)
//   Up / Down  line thickness
//
// `iron examples/postfx-outline -- color` starts with brightness edges on (for screenshots).

import { buildStage, sway } from "stage";

let outline: PostProcess;
let cel: PostProcess;
let thickness = 1.5;
let colorEdges = false;
let refresh = () => {};

class Outline extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    this.camera = buildStage().camera;
    cel = PostProcess.create("posterize");
    cel.order = 0;
    cel.setFloat("levels", 4);
    outline = PostProcess.create("outline");
    outline.order = 10;
    outline.setFloat("fade_distance", 60);
    outline.setColor("ink", new Color(0.02, 0.012, 0.008, 1));
    colorEdges = Application.args.indexOf("color") >= 0;
    apply();
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("KeyO")) outline.enabled = !outline.enabled;
    if (input.getKeyDown("KeyP")) cel.enabled = !cel.enabled;
    if (input.getKeyDown("KeyC")) colorEdges = !colorEdges;
    if (input.getKeyDown("ArrowUp")) thickness = Math.min(4, thickness + 0.5);
    if (input.getKeyDown("ArrowDown")) thickness = Math.max(0.5, thickness - 0.5);
    apply();
    sway(this.camera, this.t);
  }
}

function apply(): void {
  outline.setFloat("thickness", thickness);
  outline.setFloat("color_threshold", colorEdges ? 0.12 : 0);
  refresh();
}

register({ Outline });

function Hud() {
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  const on = (p?: PostProcess) => (p?.enabled === false ? "off" : "on");
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 5, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Ink outlines (edge detection)</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>Lines trace silhouettes and creases: balls, columns, rails. The flat floor stays clean.</text>
        <text style={{ color: "#7fb0ff", fontSize: 13 }}>{`O outlines ${on(outline)}   P cel bands ${on(cel)}   C brightness edges ${colorEdges ? "on" : "off"}`}</text>
        <text style={{ color: "#a9b3c3", fontSize: 12 }}>{`Up / Down: thickness ${thickness.toFixed(1)} px`}</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
