// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// PS1 fog: a blocky tomb in the style of the first Tomb Raider games. A long corridor of
// 1 m sandstone blocks opens into a hall with pillars, a pool and steps up to a dais, lit by
// a few orange torches. Black linear fog sits very close to the camera, as on the PS1: it
// begins `fogStartDistance` metres out and hides everything past `fogEndDistance`. A small
// explorer walks a loop on her own; the camera follows behind. The render scale is low
// and anti-aliasing off for the chunky low-res look.
// Keys: Up/Down (or +/-) move the fog's end by 1 m (the start follows at 20%), P toggles
// the low-res look, F toggles the fog.

const DARK = new Color(0.008, 0.005, 0.003, 1);
const START_RATIO = 0.2;
const LOW_RES = 0.4;

/** What the HUD shows and the keys change; `apply` sends it to the engine. */
const state = { end: 12, fog: true, lowRes: true };
let refreshHud = () => {};

function apply(): void {
  RenderSettings.fogColor = DARK;
  RenderSettings.fogMode = "linear";
  RenderSettings.fogStartDistance = state.end * START_RATIO;
  RenderSettings.fogEndDistance = state.end;
  RenderSettings.fog = state.fog;
  QualitySettings.renderScale = state.lowRes ? LOW_RES : 1;
  QualitySettings.antiAliasing = state.lowRes ? 1 : 4;
  refreshHud();
}

function setEnd(end: number) {
  state.end = Math.min(40, Math.max(3, end));
  apply();
}
function toggle(key: "fog" | "lowRes") {
  if (key === "fog") state.fog = !state.fog;
  else state.lowRes = !state.lowRes;
  apply();
}

/** A coarse grey speckle, point filtered: tinted per block by its material colour. */
function stoneTexture(): Texture {
  const px: Color[] = [];
  for (let y = 0; y < 16; y++) {
    for (let x = 0; x < 16; x++) {
      const v = Random.range(0.78, 1) * (x === 0 || y === 0 ? 0.7 : 1);
      px.push(new Color(v, v, v, 1));
    }
  }
  return Texture.create(16, 16, px, { filterMode: "point" });
}

/** A handful of slightly mismatched materials around one base colour. */
function palette(base: Color, texture: Texture, count = 6): Material[] {
  const out: Material[] = [];
  for (let i = 0; i < count; i++) {
    const k = Random.range(0.8, 1.15);
    const warm = Random.range(-0.04, 0.04);
    const color = new Color(base.r * k * (1 + warm), base.g * k, base.b * k * (1 - warm), 1);
    out.push(Material.create({ color, smoothness: 0.08, mainTexture: texture }));
  }
  return out;
}

const pick = (list: Material[]) => list[Math.floor(Random.range(0, list.length)) % list.length]!;

function block(at: Vector3, size: Vector3, material: Material, parent?: GameObject): GameObject {
  const go = new GameObject("Block", { position: at, scale: size, parent });
  go.addComponent(MeshRenderer, { mesh: "cube", material });
  return go;
}

// The layout, in metres: the corridor runs along +z from z = 0 to 16 (x -1.5..1.5, 3 m
// high); the hall spans x -7..7, z 16..30 (5 m high). The pool fills x -3..3, z 20..24;
// steps at z 25..26 lead up to a 1 m dais from z 26 to the back wall.
const CORRIDOR_END = 16;
const HALL_END = 30;

/** The floor's height under a point, for the walker. */
function groundAt(x: number, z: number): number {
  if (z >= 26) return 1;
  if (z >= 25 && Math.abs(x) <= 3) return 0.5;
  return 0;
}

/** Where the camera may go: inside the rooms, short of the walls. */
function keepInside(p: Vector3): Vector3 {
  const inCorridor = p.z < CORRIDOR_END - 0.3;
  const half = inCorridor ? 1.2 : 6.6;
  const top = inCorridor ? 2.7 : 4.6;
  return new Vector3(Math.min(half, Math.max(-half, p.x)), Math.min(top, p.y), Math.min(HALL_END - 0.4, Math.max(0.3, p.z)));
}

function buildTomb(): void {
  const tex = stoneTexture();
  const sand = palette(new Color(0.42, 0.3, 0.17, 1), tex);
  const earth = palette(new Color(0.26, 0.18, 0.11, 1), tex);
  const pale = palette(new Color(0.55, 0.44, 0.28, 1), tex);
  const tomb = new GameObject("Tomb");

  // Corridor: 1 m floor tiles, wall columns and ceiling slabs. The last metre's walls and
  // ceiling are the hall's front wall (its lintel and the blocks beside the mouth): building
  // them twice made coplanar faces that flickered.
  for (let z = 0; z < CORRIDOR_END; z++) {
    for (let x = -1; x <= 1; x++) block(new Vector3(x, -0.5, z + 0.5), Vector3.one, pick(earth), tomb);
    if (z === CORRIDOR_END - 1) continue;
    for (const x of [-2, 2]) block(new Vector3(x, 1.5, z + 0.5), new Vector3(1, 3, 1), pick(sand), tomb);
    block(new Vector3(0, 3.5, z + 0.5), new Vector3(3, 1, 1), pick(earth), tomb);
  }
  // The mouth is 4 m wide under the lintel; narrow it to the corridor's 3 m.
  for (const x of [-1.75, 1.75]) block(new Vector3(x, 1.5, CORRIDOR_END - 0.5), new Vector3(0.5, 3, 1), pick(sand), tomb);
  for (let x = -1; x <= 1; x++) block(new Vector3(x, 1.5, -0.5), new Vector3(1, 3, 1), pick(sand), tomb);

  // Hall: 2 m floor and ceiling slabs, with a hole for the pool.
  for (let x = -6; x <= 6; x += 2) {
    for (let z = CORRIDOR_END + 1; z < HALL_END; z += 2) {
      const pool = Math.abs(x) <= 2 && (z === 21 || z === 23);
      if (!pool) block(new Vector3(x, -0.5, z), new Vector3(2, 1, 2), pick(earth), tomb);
      block(new Vector3(x, 5.5, z), new Vector3(2, 1, 2), pick(earth), tomb);
    }
  }
  // Hall walls, 1 m columns: the sides, the back, and the front around the corridor's
  // mouth (a lintel over it).
  for (let z = CORRIDOR_END; z < HALL_END; z++) {
    for (const x of [-7.5, 7.5]) block(new Vector3(x, 2.5, z + 0.5), new Vector3(1, 5, 1), pick(sand), tomb);
  }
  for (let x = -7; x < 7; x++) {
    block(new Vector3(x + 0.5, 2.5, HALL_END + 0.5), new Vector3(1, 5, 1), pick(sand), tomb);
    if (Math.abs(x + 0.5) > 1.5) block(new Vector3(x + 0.5, 2.5, CORRIDOR_END - 0.5), new Vector3(1, 5, 1), pick(sand), tomb);
    else block(new Vector3(x + 0.5, 4, CORRIDOR_END - 0.5), new Vector3(1, 2, 1), pick(pale), tomb);
  }

  // The pool: a pit with a floor and four sides, and still water.
  block(new Vector3(0, -2.5, 22), new Vector3(6, 1, 4), pick(earth), tomb);
  // (The floor tiles round the edge already cover the top metre.)
  for (const x of [-3.5, 3.5]) block(new Vector3(x, -1.5, 22), new Vector3(1, 1, 4), pick(sand), tomb);
  for (const z of [19.5, 24.5]) block(new Vector3(0, -1.5, z), new Vector3(8, 1, 1), pick(sand), tomb);
  const water = block(new Vector3(0, -0.45, 22), new Vector3(6, 0.1, 4), Material.create({ color: new Color(0.02, 0.12, 0.14, 1), smoothness: 0.85, emission: new Color(0, 0.015, 0.02, 1) }), tomb);
  water.name = "Water";

  // Pillars, stacked from rough 1 m drums with a wider cap.
  for (const x of [-5, 5]) {
    for (const z of [18.5, 23.5]) {
      for (let y = 0; y < 4; y++) block(new Vector3(x, y + 0.5, z), new Vector3(1, 1, 1).scale(Random.range(0.92, 1.02)), pick(pale), tomb);
      block(new Vector3(x, 4.5, z), new Vector3(1.4, 1, 1.4), pick(sand), tomb);
    }
  }

  // Steps (0.5 m) up to the dais (1 m), and an altar block on it.
  for (let x = -2; x <= 2; x += 2) block(new Vector3(x, 0.25, 25.5), new Vector3(2, 0.5, 1), pick(pale), tomb);
  for (let x = -6; x <= 6; x += 2) {
    for (const z of [27, 29]) block(new Vector3(x, 0.5, z), new Vector3(2, 1, 2), pick(sand), tomb);
  }
  block(new Vector3(0, 1.5, 29), new Vector3(2, 1, 1), pick(pale), tomb);

  // Torches: a bracket, an emissive flame and an orange point light, low on the walls.
  const flame = Material.create({ color: Color.black, emission: new Color(4, 1.6, 0.3, 1), unlit: true });
  const bracket = Material.create({ color: new Color(0.05, 0.04, 0.03, 1), smoothness: 0.2 });
  // Each: a point on a wall's face and the way into the room.
  const torches: [Vector3, Vector3][] = [
    [new Vector3(-1.5, 2, 4.5), new Vector3(1, 0, 0)],
    [new Vector3(1.5, 2, 10.5), new Vector3(-1, 0, 0)],
    [new Vector3(-7, 2.4, 21), new Vector3(1, 0, 0)],
    [new Vector3(7, 2.4, 21), new Vector3(-1, 0, 0)],
    [new Vector3(-3.5, 2.6, HALL_END), new Vector3(0, 0, -1)],
    [new Vector3(3.5, 2.6, HALL_END), new Vector3(0, 0, -1)],
  ];
  for (const [wall, dir] of torches) {
    block(wall.add(dir.scale(0.12)), new Vector3(0.12, 0.3, 0.12).add(new Vector3(Math.abs(dir.x), 0, Math.abs(dir.z)).scale(0.12)), bracket, tomb);
    const fire = block(wall.add(dir.scale(0.2)).add(new Vector3(0, 0.25, 0)), Vector3.one.scale(0.16), flame, tomb);
    fire.name = "Flame";
    const light = new GameObject("Torch", { position: wall.add(dir.scale(0.5)).add(new Vector3(0, 0.3, 0)), parent: tomb });
    light.addComponent(Light, { type: "point", color: new Color(1, 0.5, 0.18, 1), intensity: 5, range: 7 });
    flickers.push(light.getComponent(Light)!);
  }
}

const flickers: Light[] = [];

/** The walk: up the corridor, round the pool, up the steps, across the dais and back. */
const PATH = [
  new Vector3(0.5, 0, 2),
  new Vector3(0.5, 0, 14),
  new Vector3(1.5, 0, 17.6),
  new Vector3(4, 0, 19.6),
  new Vector3(4, 0, 24.3),
  new Vector3(1.8, 0, 25.1),
  new Vector3(1.8, 0, 27.6),
  new Vector3(-1.8, 0, 27.6),
  new Vector3(-1.8, 0, 25.1),
  new Vector3(-4, 0, 24.3),
  new Vector3(-4, 0, 19.6),
  new Vector3(-1.5, 0, 17.6),
  new Vector3(-0.5, 0, 14),
  new Vector3(-0.5, 0, 2),
];

class Tomb extends Component {
  private camera!: Camera;
  private walker!: GameObject;
  private legs: GameObject[] = [];
  private arms: GameObject[] = [];
  private target = 1;
  private yaw = 0;
  private stride = 0;
  private t = 0;

  override onStart() {
    Random.initState(1996);
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.035, 0.028, 0.022, 1);
    buildTomb();
    this.buildWalker();

    const eye = new GameObject("Main Camera", { position: new Vector3(0, 1.9, -0.5) });
    this.camera = eye.addComponent(Camera, { fieldOfView: 60, backgroundColor: DARK, farClipPlane: 60 });
    apply();
  }

  private buildWalker() {
    const skin = Material.create({ color: new Color(0.6, 0.38, 0.26, 1), smoothness: 0.1 });
    const top = Material.create({ color: new Color(0.08, 0.28, 0.3, 1), smoothness: 0.1 });
    // Dark khaki: clearly not skin, even in orange torchlight.
    const shorts = Material.create({ color: new Color(0.09, 0.075, 0.04, 1), smoothness: 0.1 });
    const pack = Material.create({ color: new Color(0.14, 0.09, 0.05, 1), smoothness: 0.1 });
    const hair = Material.create({ color: new Color(0.1, 0.05, 0.02, 1), smoothness: 0.1 });

    // Facing -z (the engine's forward), so her back (and backpack) is at +z.
    const root = new GameObject("Explorer", { position: PATH[0]! });
    const part = (mesh: "cube" | "sphere" | "capsule", at: Vector3, size: Vector3, m: Material, parent: GameObject) => {
      const go = new GameObject("Part", { position: at, scale: size, parent });
      go.addComponent(MeshRenderer, { mesh, material: m });
      return go;
    };
    part("cube", new Vector3(0, 0.82, 0), new Vector3(0.42, 0.22, 0.26), shorts, root);
    part("cube", new Vector3(0, 1.17, 0), new Vector3(0.44, 0.5, 0.26), top, root);
    part("cube", new Vector3(0, 1.15, 0.2), new Vector3(0.3, 0.32, 0.14), pack, root);
    part("sphere", new Vector3(0, 1.6, 0), Vector3.one.scale(0.32), skin, root);
    part("cube", new Vector3(0, 1.5, 0.2), new Vector3(0.08, 0.36, 0.08), hair, root);
    for (const side of [-1, 1]) {
      const hip = new GameObject("Hip", { position: new Vector3(side * 0.11, 0.8, 0), parent: root });
      part("cube", new Vector3(0, -0.4, 0), new Vector3(0.16, 0.8, 0.18), skin, hip);
      // The shorts' legs, over the tops of the thighs (moving with them).
      part("cube", new Vector3(0, -0.13, 0), new Vector3(0.19, 0.28, 0.21), shorts, hip);
      this.legs.push(hip);
      const shoulder = new GameObject("Shoulder", { position: new Vector3(side * 0.29, 1.38, 0), parent: root });
      part("cube", new Vector3(0, -0.3, 0), new Vector3(0.12, 0.6, 0.12), skin, shoulder);
      this.arms.push(shoulder);
    }
    this.walker = root;
    const first = PATH[1]!.sub(PATH[0]!);
    this.yaw = Math.atan2(first.x, first.z);
  }

  override onUpdate(dt: number) {
    this.t += dt;
    if (input.getKeyDown("ArrowUp") || input.getKeyDown("Equal") || input.getKeyDown("NumpadAdd")) setEnd(state.end + 1);
    if (input.getKeyDown("ArrowDown") || input.getKeyDown("Minus") || input.getKeyDown("NumpadSubtract")) setEnd(state.end - 1);
    if (input.getKeyDown("KeyP")) toggle("lowRes");
    if (input.getKeyDown("KeyF")) toggle("fog");

    // Walk towards the next waypoint, turning smoothly, slowing for sharp turns.
    const me = this.walker.transform;
    const pos = me.position;
    const goal = PATH[this.target]!;
    const to = new Vector3(goal.x - pos.x, 0, goal.z - pos.z);
    if (to.magnitude < 0.5) this.target = (this.target + 1) % PATH.length;
    let diff = Math.atan2(to.x, to.z) - this.yaw;
    diff = Math.atan2(Math.sin(diff), Math.cos(diff));
    const turn = 2.4 * dt;
    this.yaw += Math.max(-turn, Math.min(turn, diff));
    const heading = new Vector3(Math.sin(this.yaw), 0, Math.cos(this.yaw));
    const speed = 1.7 * Math.max(0.15, Math.cos(diff));
    const step = pos.add(heading.scale(speed * dt));
    const ground = groundAt(step.x, step.z);
    const next = new Vector3(step.x, pos.y + (ground - pos.y) * Math.min(1, dt * 8), step.z);
    me.position = next;
    me.rotation = Quaternion.lookRotation(heading);

    // Swing the legs and arms with the stride.
    this.stride += speed * dt * 4.2;
    const swing = Math.sin(this.stride) * 32 * Math.min(1, speed);
    this.legs.forEach((leg, i) => (leg.transform.localRotation = Quaternion.euler(i === 0 ? swing : -swing, 0, 0)));
    this.arms.forEach((arm, i) => (arm.transform.localRotation = Quaternion.euler(i === 0 ? -swing * 0.8 : swing * 0.8, 0, 0)));

    // The camera: behind and above her, catching up with a little lag.
    const cam = this.camera.transform;
    const wanted = keepInside(next.add(heading.scale(-2.8)).add(new Vector3(0, 1.8, 0)));
    cam.position = cam.position.lerp(wanted, 1 - Math.exp(-dt * 3));
    cam.rotation = Quaternion.lookRotation(next.add(new Vector3(0, 1.1, 0)).sub(cam.position));

    // Torches flicker a little.
    flickers.forEach((l, i) => (l.intensity = 5 + Math.sin(this.t * 9 + i * 1.7) * 0.5 + Math.sin(this.t * 23 + i) * 0.35));
  }
}

register({ Tomb });

const YELLOW = "#f2c94c";

function Row(props: { keys: string; label: string; onClick?: () => void }) {
  return (
    <box onClick={props.onClick} style={{ flexDirection: "row", alignItems: "center", gap: 10, cursor: props.onClick ? "pointer" : "default" }}>
      <box style={{ padding: "2px 6px", minWidth: 64, backgroundColor: "#3a2a14", borderWidth: 2, borderColor: "#8a6a32", alignItems: "center" }}>
        <text style={{ color: YELLOW, fontSize: 13, fontWeight: 700 }}>{props.keys}</text>
      </box>
      <text style={{ color: "#e8dcc0", fontSize: 13 }}>{props.label}</text>
    </box>
  );
}

function Hud() {
  const [, setTick] = useState(0);
  refreshHud = () => setTick((t) => t + 1);
  const start = (state.end * START_RATIO).toFixed(1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ pointerEvents: "auto", alignSelf: "flex-start", padding: 12, gap: 8, flexDirection: "column", backgroundColor: "rgba(10, 6, 2, 0.8)", borderWidth: 3, borderColor: "#8a6a32" }}>
        <text style={{ color: YELLOW, fontSize: 18, fontWeight: 700, letterSpacing: 2 }}>{state.fog ? `FOG: START ${start} M, END ${state.end} M` : "FOG: OFF"}</text>
        <Row keys="UP / DOWN" label="Fog end -/+ 1 m (start at 20%)" />
        <Row keys="P" label={state.lowRes ? `Low-res look: on (${LOW_RES * 100}% scale)` : "Low-res look: off"} onClick={() => toggle("lowRes")} />
        <Row keys="F" label={state.fog ? "Fog: on" : "Fog: off"} onClick={() => toggle("fog")} />
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
