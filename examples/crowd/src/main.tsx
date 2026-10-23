// SPDX-License-Identifier: Apache-2.0

// Crowd: two busy sidewalks. Two city blocks far apart, each a ring of pavement round a block of
// buildings with a road round that, and on each a crowd of Quaternius's mannequin (CC0, see
// assets/CREDITS.md) walking round in both directions: each figure its own Animator (idle, walk
// and jog blended by speed), colours, pace and lane, slowing behind others or stepping into a
// free lane, now and then stopping for a while. A test of animation culling (the Animators'
// `cullingMode`) and frustum culling (what the cameras see of the blocks): view 4 looks at
// block B only, so block A is entirely off screen, its shadows included.
// Keys: 1-4 views (street level on A, over A, both blocks, over B), 5-9 figures per block
// (50, 100, 250, 500, 1000), M the animators' culling mode.

/** Figures per block, on keys 5-9. */
const COUNTS = [50, 100, 250, 500, 1000];
const START_COUNT = 1;
const MODES: AnimatorCullingMode[] = ["alwaysAnimate", "cullUpdateTransforms", "cullCompletely"];
const MODE_NAMES = ["AlwaysAnimate", "CullUpdateTransforms", "CullCompletely"];

/** A block of buildings (half its size, metres), the pavement round it, the road round that. */
const HX = 36;
const HZ = 13;
const WALK = 6;
const ROAD = 9;
/** The pavement's top: figures walk on it. */
const PAVE = 0.15;
/** Lanes across the pavement, from the buildings out: anticlockwise walkers keep to the inner half. */
const LANES = 8;
const laneOffset = (k: number) => 1.0 + k * 0.6;
/** The ring's length along the middle of the pavement: the walkers' common measure along it. */
const MID = 4 * HX + 4 * HZ + 2 * Math.PI * 3;
/** Neighbour cells along the ring, metres. */
const CELL = 2.5;
const CELLS = Math.ceil(MID / CELL);
/** At most this many figures made per frame (instantiating a skinned model is the costly part). */
const SPAWN_PER_FRAME = 24;
/** Each figure decides (lanes, speed, stops) every this many frames, staggered. */
const THINK_EVERY = 6;
/** Speeds, m/s: the blend tree's thresholds (each clip at the rate whose stride matches). */
const WALK_SPEED = 1.35;
const JOG_SPEED = 4;

const state = { view: 0, count: START_COUNT, mode: 0 };
let refreshHud = () => {};
/** The running scene, for the HUD. */
let crowd: Crowd | null = null;

/** A point on the ring `d` metres out from the buildings, `s` metres round it, and the way on. */
const at = { x: 0, z: 0, tx: 0, tz: 0 };

function ringLength(d: number): number {
  return 4 * HX + 4 * HZ + 2 * Math.PI * d;
}

function arc(cx: number, cz: number, d: number, from: number, s: number) {
  const a = from + s / d;
  at.x = cx + d * Math.cos(a);
  at.z = cz + d * Math.sin(a);
  at.tx = -Math.sin(a);
  at.tz = Math.cos(a);
}

/** Fills `at`: anticlockwise from the south-west corner (seen from above, +z north). */
function ring(d: number, s: number) {
  const q = (Math.PI * d) / 2;
  if (s < 2 * HX) {
    at.x = -HX + s;
    at.z = -HZ - d;
    at.tx = 1;
    at.tz = 0;
    return;
  }
  s -= 2 * HX;
  if (s < q) return arc(HX, -HZ, d, -Math.PI / 2, s);
  s -= q;
  if (s < 2 * HZ) {
    at.x = HX + d;
    at.z = -HZ + s;
    at.tx = 0;
    at.tz = 1;
    return;
  }
  s -= 2 * HZ;
  if (s < q) return arc(HX, HZ, d, 0, s);
  s -= q;
  if (s < 2 * HX) {
    at.x = HX - s;
    at.z = HZ + d;
    at.tx = -1;
    at.tz = 0;
    return;
  }
  s -= 2 * HX;
  if (s < q) return arc(-HX, HZ, d, Math.PI / 2, s);
  s -= q;
  if (s < 2 * HZ) {
    at.x = -HX - d;
    at.z = HZ - s;
    at.tx = 0;
    at.tz = -1;
    return;
  }
  s -= 2 * HZ;
  arc(-HX, -HZ, d, Math.PI, Math.min(s, q));
}

/** `v` wrapped into [-MID / 2, MID / 2). */
function wrapHalf(v: number): number {
  return v - Math.floor(v / MID + 0.5) * MID;
}

interface Walker {
  go: GameObject;
  animator: Animator;
  /** The body (the figure has two skinned parts): its `isVisible` stands for the figure. */
  body: SkinnedMeshRenderer | null;
  /** How far round the ring, 0..1, and which way: +1 anticlockwise, -1 clockwise. */
  u: number;
  dir: number;
  /** Metres out from the buildings, and the lane it is heading for. */
  d: number;
  lane: number;
  /** Its own pace, the speed it is after now, and its speed. */
  cruise: number;
  want: number;
  speed: number;
  /** Seconds left standing still (a look in a shop window). */
  pause: number;
  /** Where it is along the ring's middle, metres (for neighbours). */
  a: number;
}

interface Block {
  name: string;
  centre: Vector3;
  walkers: Walker[];
  cells: Walker[][];
  /** Counted at the last HUD refresh. */
  seen: number;
}

interface View {
  name: string;
  position: Vector3;
  target: Vector3;
  /**
   * The far clip plane. Kept short where it can be: a shadow caster counts as seen when its
   * shadow, swept along the light without end, meets the view's frustum (the ground can't stop
   * it), so a deep frustum under the far block would see its figures' shadows there.
   */
  far: number;
}

const A = new Vector3(-110, 0, 0);
const B = new Vector3(110, 0, 0);

const VIEWS: View[] = [
  // From the road's edge at the east end, looking west along the south side: block B behind.
  { name: "1 Street level, block A", position: A.add(new Vector3(HX + 3, 1.7, -HZ - WALK - 0.9)), target: A.add(new Vector3(-HX * 0.4, 1.0, -HZ - 2.4)), far: 260 },
  { name: "2 Over block A", position: A.add(new Vector3(0, 58, -56)), target: A.add(new Vector3(0, 0, -4)), far: 200 },
  { name: "3 Both blocks", position: new Vector3(0, 130, -225), target: new Vector3(0, 0, 10), far: 700 },
  { name: "4 Over block B (A off screen)", position: B.add(new Vector3(0, 58, -56)), target: B.add(new Vector3(0, 0, -4)), far: 200 },
];

/** A box with its own material, not casting shadows unless `shadows`. */
function box(name: string, parent: GameObject, at: Vector3, size: Vector3, material: Material, shadows = false) {
  const go = new GameObject(name, { position: at, scale: size, parent });
  const r = go.addComponent(MeshRenderer, { mesh: "cube", material });
  r.shadowCastingMode = shadows ? "on" : "off";
  return go;
}

function mat(r: number, g: number, b: number, smoothness = 0.2): Material {
  return Material.create({ color: new Color(r, g, b, 1), smoothness });
}

/** Clothes: the figures' main material, and their joints'. */
const CLOTHES: number[][] = [
  [0.72, 0.18, 0.16], [0.16, 0.3, 0.62], [0.2, 0.46, 0.28], [0.85, 0.66, 0.2], [0.42, 0.42, 0.46],
  [0.12, 0.12, 0.14], [0.86, 0.85, 0.82], [0.5, 0.28, 0.55], [0.15, 0.5, 0.56], [0.62, 0.42, 0.28],
  [0.9, 0.45, 0.55], [0.3, 0.22, 0.16],
];
const JOINTS: number[][] = [[0.12, 0.12, 0.13], [0.85, 0.82, 0.76], [0.55, 0.36, 0.25], [0.3, 0.32, 0.36]];
const FACADES: number[][] = [
  [0.72, 0.62, 0.52], [0.55, 0.32, 0.26], [0.78, 0.76, 0.7], [0.45, 0.48, 0.52], [0.66, 0.55, 0.4],
  [0.36, 0.38, 0.42], [0.82, 0.74, 0.6], [0.5, 0.42, 0.36],
];

/** The pavement, kerb, road, buildings, lamps and benches of a block round `c`. */
function buildBlock(name: string, c: Vector3) {
  const root = new GameObject(name, { position: c });
  const asphalt = mat(0.16, 0.16, 0.17, 0.35);
  const pavement = mat(0.62, 0.6, 0.57);
  const kerb = mat(0.75, 0.74, 0.72);
  const paint = mat(0.92, 0.9, 0.82);
  const iron = mat(0.12, 0.13, 0.14, 0.5);
  const wood = mat(0.45, 0.3, 0.18);
  const facades = FACADES.map((f) => mat(f[0]!, f[1]!, f[2]!));
  const shops = [mat(0.1, 0.12, 0.15, 0.8), mat(0.22, 0.16, 0.12, 0.6), mat(0.14, 0.2, 0.18, 0.7)];
  const outX = HX + WALK + ROAD;
  const outZ = HZ + WALK + ROAD;
  box("Road", root, new Vector3(0, -0.05, 0), new Vector3(2 * outX, 0.1, 2 * outZ), asphalt);
  // Pavement: the long sides run past the corners; the short ones fit between.
  for (const s of [-1, 1]) {
    box("Pavement", root, new Vector3(0, PAVE / 2, s * (HZ + WALK / 2)), new Vector3(2 * (HX + WALK), PAVE, WALK), pavement);
    box("Pavement", root, new Vector3(s * (HX + WALK / 2), PAVE / 2, 0), new Vector3(WALK, PAVE, 2 * HZ), pavement);
    box("Kerb", root, new Vector3(0, PAVE / 2 + 0.01, s * (HZ + WALK - 0.15)), new Vector3(2 * (HX + WALK), PAVE + 0.02, 0.3), kerb);
    box("Kerb", root, new Vector3(s * (HX + WALK - 0.15), PAVE / 2 + 0.01, 0), new Vector3(0.3, PAVE + 0.02, 2 * (HZ + WALK)), kerb);
    // The road's centre line, dashed.
    const lineZ = s * (HZ + WALK + ROAD / 2);
    for (let x = -outX + 3; x < outX - 2; x += 5) box("Line", root, new Vector3(x, 0.005, lineZ), new Vector3(2, 0.01, 0.15), paint);
    const lineX = s * (HX + WALK + ROAD / 2);
    for (let z = -HZ - WALK + 3; z < HZ + WALK - 2; z += 5) box("Line", root, new Vector3(lineX, 0.005, z), new Vector3(0.15, 0.01, 2), paint);
  }
  // Buildings: two rows (south and north), each a run of fronts of their own width and height.
  for (const s of [-1, 1]) {
    let x = -HX;
    while (x < HX - 0.5) {
      const w = Math.min(HX - x, Random.range(7, 13));
      const h = Random.range(9, 34);
      const cx = x + w / 2;
      const cz = (s * HZ) / 2;
      box("Building", root, new Vector3(cx, h / 2, cz), new Vector3(w - 0.2, h, HZ), facades[Math.floor(Random.value * facades.length)]!, true);
      box("Roof", root, new Vector3(cx, h + 0.4, cz), new Vector3(w - 1.2, 0.8, HZ - 1), iron);
      // A shop front at street level on the pavement side.
      box("Shop front", root, new Vector3(cx, 2, s * (HZ - 0.05)), new Vector3(w - 1.4, 3.2, 0.2), shops[Math.floor(Random.value * shops.length)]!);
      x += w;
    }
  }
  // Lamp posts along the kerb, benches against the shop fronts.
  for (const s of [-1, 1]) {
    for (let x = -HX + 4; x <= HX - 4; x += 12) {
      const z = s * (HZ + WALK - 0.55);
      box("Lamp post", root, new Vector3(x, 2.3, z), new Vector3(0.14, 4.6, 0.14), iron, true);
      box("Lamp", root, new Vector3(x, 4.6, z - s * 0.35), new Vector3(0.3, 0.14, 0.8), iron, true);
    }
    for (let x = -HX + 10; x <= HX - 10; x += 26) {
      const z = s * (HZ + 0.45);
      box("Bench seat", root, new Vector3(x, 0.6, z), new Vector3(1.8, 0.08, 0.45), wood, true);
      box("Bench back", root, new Vector3(x, 0.9, z + s * -0.2), new Vector3(1.8, 0.45, 0.06), wood, true);
    }
  }
}

function skinned(go: GameObject, out: SkinnedMeshRenderer[]) {
  const s = go.getComponent(SkinnedMeshRenderer);
  if (s) out.push(s);
  for (const c of go.children) skinned(c, out);
}

class Crowd extends Component {
  private camera!: Camera;
  private model: Model | null = null;
  private controller: AnimatorController | null = null;
  private mains: Material[] = [];
  private joints: Material[] = [];
  private blocks: Block[] = [];
  private frame = 0;
  /** The camera's ease between views. */
  private fromPos = Vector3.zero;
  private fromRot = Quaternion.identity;
  private ease = 1;
  /** Frame time from the real clock (not the game's, which a bench run fixes). */
  private lastReal = 0;
  private sampleTime = 0;
  private sampleFrames = 0;
  fps = 0;
  frameMs = 0;

  override onStart() {
    crowd = this;
    Random.initState(11);
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.42, 0.45, 0.52, 1);
    RenderSettings.tonemapping = "neutral";
    RenderSettings.skybox = {
      zenith: new Color(0.3, 0.5, 0.82, 1),
      horizon: new Color(0.75, 0.84, 0.92, 1),
      ground: new Color(0.42, 0.44, 0.42, 1),
      sunDirection: new Vector3(0, 1, -0.45),
    };
    QualitySettings.shadowDistance = 140;
    // A late-morning sun from the south. Its light has no east-west part, so neither block's
    // shadows can reach towards the other (a caster counts as seen while its shadow, swept along
    // the light, can fall in view).
    const sun = new GameObject("Sun", { rotation: Quaternion.lookRotation(new Vector3(0, -1, 0.45)) });
    sun.addComponent(Light, { type: "directional", color: new Color(1, 0.96, 0.9, 1), intensity: 1.5, shadows: "soft", shadowStrength: 0.8 });
    const grass = new GameObject("Park", { position: new Vector3(0, -0.12, 0), scale: new Vector3(700, 0.1, 400) });
    grass.addComponent(MeshRenderer, { mesh: "cube", material: mat(0.22, 0.33, 0.17) }).shadowCastingMode = "off";
    buildBlock("Block A", A);
    buildBlock("Block B", B);
    for (const [name, centre] of [["A", A], ["B", B]] as [string, Vector3][]) {
      const cells: Walker[][] = [];
      for (let i = 0; i < CELLS; i++) cells.push([]);
      this.blocks.push({ name, centre, walkers: [], cells, seen: 0 });
    }
    this.mains = CLOTHES.map((c) => mat(c[0]!, c[1]!, c[2]!, 0.35));
    this.joints = JOINTS.map((c) => mat(c[0]!, c[1]!, c[2]!, 0.5));
    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 50, nearClipPlane: 0.2, farClipPlane: 700, clearFlags: "skybox" });
    this.place(1);
    this.load();
  }

  private async load() {
    const model = await Model.load("assets/mannequin.glb").whenLoaded();
    const clip = (name: string): AnimationClip => {
      const c = model.animations.find((a) => a.name === name);
      if (!c) throw new Error(`mannequin.glb has no ${name}`);
      c.wrapMode = "loop";
      return c;
    };
    // Blended by speed (m/s), each clip at the rate whose stride matches its threshold (their
    // natural speeds from the pack's root-motion version: walk 0.97, jog 5.36 m/s).
    this.controller = new AnimatorController().addParameter("speed", "float", 0).addState("Locomotion", {
      blendParameter: "speed",
      children: [
        { motion: clip("Idle_Loop"), threshold: 0 },
        { motion: clip("Walk_Loop"), threshold: WALK_SPEED, timeScale: WALK_SPEED / 0.97 },
        { motion: clip("Jog_Fwd_Loop"), threshold: JOG_SPEED, timeScale: JOG_SPEED / 5.36 },
      ],
    });
    this.model = model;
  }

  /** A figure on block `b` at a random place, lane and pace. */
  private spawn(b: Block) {
    const model = this.model!;
    const dir = Random.value < 0.5 ? 1 : -1;
    // Anticlockwise walkers keep to the inner lanes, clockwise ones to the outer: at first.
    const lane = (dir > 0 ? 0 : LANES / 2) + Math.floor(Random.value * (LANES / 2));
    const jogger = Random.value < 0.1;
    const cruise = jogger ? Random.range(3.6, 4.4) : Random.range(1.05, 1.65);
    const go = model.instantiate({ position: b.centre });
    go.getComponent(Animation)?.destroy();
    const [main, joints] = model.materials;
    const myMain = this.mains[Math.floor(Random.value * this.mains.length)]!;
    const myJoints = this.joints[Math.floor(Random.value * this.joints.length)]!;
    const parts: SkinnedMeshRenderer[] = [];
    skinned(go, parts);
    for (const p of parts) p.material = p.material === joints ? myJoints : p.material === main ? myMain : p.material;
    const animator = go.addComponent(Animator, { controller: this.controller! });
    animator.cullingMode = MODES[state.mode]!;
    animator.setFloat("speed", cruise);
    animator.play("Locomotion", Random.value);
    const w: Walker = { go, animator, body: parts[0] ?? null, u: Random.value, dir, d: laneOffset(lane), lane, cruise, want: cruise, speed: cruise, pause: 0, a: 0 };
    // A touch of height variety, so the crowd isn't all one size.
    go.transform.localScale = Vector3.one.scale(Random.range(0.92, 1.08));
    this.move(b, w, 0);
    b.walkers.push(w);
  }

  /** Makes or removes figures towards the count, a few a frame. */
  private populate() {
    if (!this.model) return;
    const want = COUNTS[state.count]!;
    let budget = SPAWN_PER_FRAME;
    for (const b of this.blocks) {
      while (b.walkers.length > want) b.walkers.pop()!.go.destroy();
    }
    while (budget > 0) {
      // Alternate, so both blocks fill at the same pace.
      const b = this.blocks[0]!.walkers.length <= this.blocks[1]!.walkers.length ? this.blocks[0]! : this.blocks[1]!;
      if (b.walkers.length >= want) break;
      this.spawn(b);
      budget--;
    }
  }

  override onUpdate(dt: number) {
    for (let k = 0; k < VIEWS.length; k++) if (input.getKeyDown(`Digit${k + 1}`)) this.setView(k);
    for (let k = 0; k < COUNTS.length; k++) {
      if (input.getKeyDown(`Digit${k + 5}`)) {
        state.count = k;
        refreshHud();
      }
    }
    if (input.getKeyDown("KeyM")) {
      state.mode = (state.mode + 1) % MODES.length;
      for (const b of this.blocks) for (const w of b.walkers) w.animator.cullingMode = MODES[state.mode]!;
      refreshHud();
    }
    this.populate();
    for (const b of this.blocks) {
      for (const c of b.cells) c.length = 0;
      for (const w of b.walkers) b.cells[Math.floor(w.a / CELL) % CELLS]!.push(w);
      for (let i = 0; i < b.walkers.length; i++) {
        const w = b.walkers[i]!;
        if ((this.frame + i) % THINK_EVERY === 0) this.think(b, w);
        this.move(b, w, dt);
      }
    }
    this.frame++;
    if (this.ease < 1) {
      this.ease = Math.min(1, this.ease + dt / 0.8);
      this.place(this.ease);
    }
    this.measure();
  }

  private setView(k: number) {
    const cam = this.camera.transform;
    this.fromPos = cam.position;
    this.fromRot = cam.rotation;
    state.view = k;
    this.ease = 0;
    refreshHud();
  }

  /** The camera `t` of the way (eased) from where it was to the current view. */
  private place(t: number) {
    const v = VIEWS[state.view]!;
    const to = Quaternion.lookRotation(v.target.sub(v.position));
    const e = t * t * (3 - 2 * t);
    const cam = this.camera.transform;
    cam.position = t >= 1 ? v.position : this.fromPos.lerp(v.position, e);
    cam.rotation = t >= 1 ? to : this.fromRot.slerp(to, e);
    // On the way, far enough for both ends.
    this.camera.farClipPlane = t >= 1 ? v.far : 700;
  }

  /** Lanes and speed: slow behind someone, step into a free lane, drift home, stop now and then. */
  private think(b: Block, w: Walker) {
    if (w.pause > 0) return;
    let leader: Walker | null = null;
    let leaderGap = 3.5;
    let oncoming = false;
    const cell = Math.floor(w.a / CELL);
    for (let k = -1; k <= 2; k++) {
      const others = b.cells[(((cell + k * w.dir) % CELLS) + CELLS) % CELLS]!;
      for (const o of others) {
        if (o === w || Math.abs(o.d - w.d) > 0.5) continue;
        const gap = wrapHalf(o.a - w.a) * w.dir;
        if (gap <= 0) continue;
        if (o.dir === w.dir) {
          if (gap < leaderGap && (o.speed < w.cruise - 0.05 || gap < 1.2)) {
            leader = o;
            leaderGap = gap;
          }
        } else if (gap < 4.5) oncoming = true;
      }
    }
    const home = w.dir > 0 ? 0 : LANES / 2;
    const inBand = w.lane >= home && w.lane < home + LANES / 2;
    if (leader || oncoming) {
      // Try the lane nearer home first, then the other.
      const towards = w.lane < home ? 1 : w.lane >= home + LANES / 2 ? -1 : w.dir > 0 ? -1 : 1;
      if (!this.tryLane(b, w, w.lane + towards) && !this.tryLane(b, w, w.lane - towards)) {
        w.want = oncoming ? 0.5 : leaderGap < 1.1 ? 0 : Math.min(w.cruise, leader!.speed);
        return;
      }
    } else if (!inBand && Random.value < 0.3) {
      this.tryLane(b, w, w.lane + (w.lane < home ? 1 : -1));
    } else if (Random.value < 0.0025 && w.cruise < 2) {
      // A look in a shop window.
      w.pause = Random.range(2, 6);
      w.want = 0;
      return;
    }
    w.want = w.cruise;
  }

  /** Moves `w` into `lane` if no one is near it there. */
  private tryLane(b: Block, w: Walker, lane: number): boolean {
    if (lane < 0 || lane >= LANES) return false;
    const d = laneOffset(lane);
    const cell = Math.floor(w.a / CELL);
    for (let k = -1; k <= 1; k++) {
      for (const o of b.cells[(((cell + k) % CELLS) + CELLS) % CELLS]!) {
        if (o === w || Math.abs(o.d - d) > 0.45) continue;
        const gap = wrapHalf(o.a - w.a) * w.dir;
        if (gap > -1.2 && gap < 3) return false;
      }
    }
    w.lane = lane;
    return true;
  }

  private move(b: Block, w: Walker, dt: number) {
    if (w.pause > 0) {
      w.pause -= dt;
      if (w.pause <= 0) w.want = w.cruise;
    }
    // Quicker to slow than to speed up.
    const rate = w.want < w.speed ? 4 : 1.5;
    w.speed += (w.want - w.speed) * Math.min(1, rate * dt);
    const target = laneOffset(w.lane);
    const side = Math.max(-0.7 * dt, Math.min(0.7 * dt, target - w.d));
    w.d += side;
    const length = ringLength(w.d);
    w.u += (w.dir * w.speed * dt) / length;
    w.u -= Math.floor(w.u);
    w.a = w.u * MID;
    ring(w.d, w.u * length);
    // Facing along the way it goes, turned a little into a lane change (out is (tz, -tx)).
    const lateral = dt > 0 ? side / dt : 0;
    const hx = at.tx * w.dir * Math.max(w.speed, 0.3) + at.tz * lateral;
    const hz = at.tz * w.dir * Math.max(w.speed, 0.3) - at.tx * lateral;
    const t = w.go.transform;
    t.position = new Vector3(b.centre.x + at.x, PAVE, b.centre.z + at.z);
    t.rotation = Quaternion.euler(0, (Math.atan2(hx, hz) * 180) / Math.PI, 0);
    w.animator.setFloat("speed", w.speed);
  }

  /** Frame times from the real clock; every quarter second the HUD counts what was seen. */
  private measure() {
    const now = Time.realtimeSinceStartup;
    if (this.lastReal > 0) {
      this.sampleTime += now - this.lastReal;
      this.sampleFrames++;
    }
    this.lastReal = now;
    if (this.sampleTime < 0.25) return;
    this.fps = this.sampleFrames / this.sampleTime;
    this.frameMs = (this.sampleTime / this.sampleFrames) * 1000;
    this.sampleTime = 0;
    this.sampleFrames = 0;
    for (const b of this.blocks) {
      let seen = 0;
      for (const w of b.walkers) if (w.body?.isVisible) seen++;
      b.seen = seen;
    }
    refreshHud();
  }

  /** Per block: figures, seen last frame, posed this frame. */
  counts(): { name: string; total: number; seen: number; animated: number }[] {
    return this.blocks.map((b) => ({
      name: b.name,
      total: b.walkers.length,
      seen: b.seen,
      // Culling modes pose only what was seen last frame (the frame `isVisible` reports).
      animated: state.mode === 0 ? b.walkers.length : b.seen,
    }));
  }
}

register({ Crowd });

// ---- HUD ----

function Row(props: { label: string; value: string; keyName: string }) {
  return (
    <box style={{ flexDirection: "row", gap: 10, alignItems: "baseline" }}>
      <text style={{ color: "#a9b3c3", fontSize: 13, width: 112 }}>{props.label}</text>
      <text style={{ color: "#f0f3f8", fontSize: 13, width: 230 }}>{props.value}</text>
      <text style={{ color: "#6f7a8c", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Cell(props: { text: string; head?: boolean; width: number }) {
  return <text style={{ color: props.head ? "#a9b3c3" : "#f0f3f8", fontSize: 13, width: props.width }}>{props.text}</text>;
}

function Hud() {
  const [, setTick] = useState(0);
  refreshHud = () => setTick((t) => t + 1);
  const c = crowd;
  const rows = c ? c.counts() : [];
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 6, borderRadius: 10, backgroundColor: "rgba(12, 14, 20, 0.72)", flexDirection: "column" }}>
        <box style={{ flexDirection: "row", justifyContent: "space-between", gap: 16 }}>
          <text style={{ color: "#ffffff", fontSize: 16, fontWeight: 600 }}>Crowd: two sidewalks</text>
          <text style={{ color: "#f0f3f8", fontSize: 13 }}>{`${(c?.fps ?? 0).toFixed(0)} fps, ${(c?.frameMs ?? 0).toFixed(2)} ms`}</text>
        </box>
        <Row label="View" value={VIEWS[state.view]!.name} keyName="1-4" />
        <Row label="Per block" value={`${COUNTS[state.count]} figures`} keyName="5-9" />
        <Row label="Animator culling" value={MODE_NAMES[state.mode]!} keyName="M" />
        <box style={{ flexDirection: "row", marginTop: 4 }}>
          <Cell text="Block" head width={60} />
          <Cell text="Figures" head width={70} />
          <Cell text="Seen" head width={70} />
          <Cell text="Animated" head width={80} />
          <Cell text="Culled" head width={70} />
        </box>
        {rows.map((r) => (
          <box key={r.name} style={{ flexDirection: "row" }}>
            <Cell text={r.name} width={60} />
            <Cell text={`${r.total}`} width={70} />
            <Cell text={`${r.seen}`} width={70} />
            <Cell text={`${r.animated}`} width={80} />
            <Cell text={`${r.total - r.seen}`} width={70} />
          </box>
        ))}
        <Row label="Meshes" value={`${RenderStats.meshes} drawn, ${RenderStats.culledMeshes} culled`} keyName="" />
        <Row label="Shadows" value={`${RenderStats.shadowCasters} casters, ${RenderStats.shadowMaps} maps`} keyName="" />
        <text style={{ color: "#6f7a8c", fontSize: 12, marginTop: 4 }}>
          1-4 views · 5-9: 50 / 100 / 250 / 500 / 1000 per block · M culling mode
        </text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
