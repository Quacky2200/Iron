// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

// Post-processing "looks": several effects stacked into a finished picture, switched with the keys
// 1 to 6 (0 is the plain render). Each look is a handful of `PostProcess` passes with their orders
// and parameters; changing look turns the old ones off and the new ones on.
//
// `platypus-app examples/postfx-look -- 3` starts with look 3 (for screenshots).

import { buildStage, sway } from "stage";

interface Look {
  name: string;
  note: string;
  /** The passes it needs, by effect, with the setup that makes the look. */
  passes: { effect: PostEffect; order: number; setup: (p: PostProcess) => void }[];
}

const LOOKS: Look[] = [
  { name: "Plain", note: "no passes", passes: [] },
  {
    name: "Noir",
    note: "black and white, hard contrast, heavy vignette and grain",
    passes: [
      { effect: "colorGrading", order: 0, setup: (p) => { p.setFloat("saturation", 0); p.setFloat("contrast", 1.35); p.setFloat("exposure", -0.2); } },
      { effect: "vignette", order: 10, setup: (p) => { p.setFloat("intensity", 0.85); p.setFloat("smoothness", 0.6); } },
      { effect: "filmGrain", order: 20, setup: (p) => { p.setFloat("intensity", 0.28); p.setFloat("size", 1.8); p.setFloat("colored", 0); } },
    ],
  },
  {
    name: "Dream",
    note: "soft blur at the edges, warm grade, a little fringing",
    passes: [
      { effect: "tiltShift", order: 0, setup: (p) => { p.setFloat("focus", 0.5); p.setFloat("width", 0.16); p.setFloat("max_blur", 12); p.setFloat("highlight", 2); } },
      { effect: "colorGrading", order: 10, setup: (p) => { p.setFloat("saturation", 1.25); p.setFloat("exposure", 0.25); p.setVector("balance", 0.3, 0.1); p.setVector("lift", 0.05, 0.03, 0.07); } },
      { effect: "chromaticAberration", order: 20, setup: (p) => p.setFloat("amount", 0.01) },
      { effect: "vignette", order: 30, setup: (p) => { p.setFloat("intensity", 0.4); p.setColor("tint", new Color(0.35, 0.1, 0.2, 1)); } },
    ],
  },
  {
    name: "Comic",
    note: "ink lines over flat colour bands",
    passes: [
      { effect: "posterize", order: 0, setup: (p) => p.setFloat("levels", 4) },
      { effect: "outline", order: 10, setup: (p) => { p.setFloat("thickness", 1.5); p.setFloat("fade_distance", 60); p.setColor("ink", new Color(0.02, 0.012, 0.008, 1)); } },
      { effect: "colorGrading", order: 20, setup: (p) => { p.setFloat("saturation", 1.3); p.setFloat("contrast", 1.1); } },
    ],
  },
  {
    name: "Old TV",
    note: "curved glass, scanlines, grain, fringing",
    passes: [
      { effect: "chromaticAberration", order: 0, setup: (p) => p.setFloat("amount", 0.012) },
      { effect: "filmGrain", order: 10, setup: (p) => { p.setFloat("intensity", 0.14); p.setFloat("colored", 0.6); } },
      { effect: "crt", order: 20, setup: (p) => p.setFloat("curvature", 0.35) },
    ],
  },
  {
    name: "Arcade",
    note: "big pixels and a few colours per channel",
    passes: [
      { effect: "pixelate", order: 10, setup: (p) => { p.setFloat("size", 4); p.setFloat("levels", 8); } },
      { effect: "colorGrading", order: 0, setup: (p) => { p.setFloat("saturation", 1.2); p.setFloat("contrast", 1.1); } },
    ],
  },
];

let current = 0;
let live: PostProcess[] = [];
let refresh = () => {};

function choose(index: number): void {
  for (const p of live) p.destroy();
  live = [];
  current = index;
  for (const spec of LOOKS[index]!.passes) {
    const p = PostProcess.create(spec.effect);
    p.order = spec.order;
    spec.setup(p);
    live.push(p);
  }
  refresh();
}

class Looks extends Component {
  private camera!: Camera;
  private t = 0;

  override onStart() {
    this.camera = buildStage().camera;
    const first = Number(Application.args[0]);
    choose(first >= 0 && first < LOOKS.length ? first : 1);
  }

  override onUpdate(dt: number) {
    this.t += dt;
    for (let i = 0; i < LOOKS.length; i++) {
      if (input.getKeyDown(i === 0 ? "Digit0" : `Digit${i}`)) choose(i);
    }
    sway(this.camera, this.t);
  }
}

register({ Looks });

function Hud() {
  const [, setTick] = useState(0);
  refresh = () => setTick((t) => t + 1);
  const look = LOOKS[current]!;
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 5, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Looks: several effects stacked</text>
        {LOOKS.map((l, i) => (
          <text key={l.name} style={{ color: i === current ? "#7fb0ff" : "#a9b3c3", fontSize: 13 }}>{`${i === 0 ? 0 : i}  ${l.name}`}</text>
        ))}
        <text style={{ color: "#f0f3f8", fontSize: 12, marginTop: 4 }}>{`${look.name}: ${look.note}`}</text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
