// Fog: the white room, after Assassin's Creed's loading screen (the Animus). A figure on a
// bright white floor of grid tiles under a grey, cloudy sky, everything fading into white
// fog a few tiles away. The floor moves with the player like a treadmill, so the room never
// ends: no walls to reach, no edge to fall off. Tiles light up under footsteps and here and
// there on their own; pale fragments drift up out of the floor; a low ceiling of grey cloud
// drifts overhead.
// The figure is Quaternius's mannequin (CC0, see assets/CREDITS.md), driven by an Animator:
// idle, walk, jog and sprint blended by speed, and jumps and rolls.
// Keys: WASD or arrows to move, Shift to sprint, Alt to walk, Space to jump, C to roll; hold a
// mouse button and drag (or Q / E) to turn the camera, the wheel to zoom. F: fog distance,
// H: height fog, T: glowing tiles.
import {
  Animation,
  AnimationClip,
  Animator,
  AnimatorController,
  BoxCollider,
  Camera,
  CharacterController,
  Color,
  Component,
  GameObject,
  Light,
  Material,
  MeshRenderer,
  Model,
  QualitySettings,
  Quaternion,
  Random,
  RenderSettings,
  Texture,
  Vector3,
  input,
  register,
} from "@iron/engine";
import { createRoot } from "@iron/react";
import { useState } from "react";

/** A floor tile's size, metres. */
const TILE = 1.5;
/** Tiles across the floor texture's block (each tile its own shade). */
const BLOCK_TILES = 8;
const BLOCK = BLOCK_TILES * TILE;
/** The floor's width (whole blocks): it follows the player, snapped to blocks. */
const FLOOR = 8 * BLOCK;
/** The cloud layers: height, how wide one repeat of the texture is, and drift (m/s). */
const CLOUDS = [
  { y: 9, repeat: 36, drift: new Vector3(0.6, 0, 0.25), grey: 0.62, alpha: 0.85 },
  { y: 12.5, repeat: 54, drift: new Vector3(0.35, 0, 0.15), grey: 0.72, alpha: 0.7 },
];
/**
 * The village on the horizon: built at full size (metres) about `VILLAGE_R` away, then shrunk
 * into the fog's fading band, where it stays as the player walks (a
 * backdrop, never reached). Shrunk, it looks just as it would far off, but the fog leaves it
 * a faded grey silhouette instead of hiding it.
 */
const VILLAGE_R = 230;
/** The cloud planes' width: past it, the fog has covered them. */
const CLOUD_SPAN = 160;
/** How far the player gets from the origin before everything shifts back (floats stay precise). */
const RECENTRE = 240;
const WHITE = new Color(0.93, 0.94, 0.96, 1);

/**
 * Fog distances: linear, starting a few tiles out, so the player and what's near stand clear
 * in a bubble and the room fades to white beyond it.
 */
const FOGS = [
  { name: "Close (a few tiles)", start: 7, end: 26 },
  { name: "Medium", start: 10, end: 45 },
  { name: "Far", start: 14, end: 75 },
];
const state = { fog: 0, heightFog: true, tiles: true, village: true };
let refreshHud = () => {};

function apply(): void {
  RenderSettings.fog = true;
  RenderSettings.fogMode = "linear";
  RenderSettings.fogColor = WHITE;
  RenderSettings.fogStartDistance = FOGS[state.fog]!.start;
  RenderSettings.fogEndDistance = FOGS[state.fog]!.end;
  // A thin layer hugging the floor: over a few metres it's nothing, so the player stays clear,
  // but over the long look to the horizon it thickens, and the floor melts into the sky.
  RenderSettings.heightFog = state.heightFog ? { density: 0.025, baseHeight: 0, falloff: 2.5 } : null;
  refreshHud();
}

/**
 * The floor: a block of BLOCK_TILES x BLOCK_TILES white tiles, each its own shade, with grey
 * grid lines and a bevel round each (the Animus floor's raised panels). The lines glow faintly
 * (the emission map).
 */
function floorTextures(): { pane: Texture; glow: Texture } {
  const N = 48;
  const M = N * BLOCK_TILES;
  const shade = Array.from({ length: BLOCK_TILES * BLOCK_TILES }, () => 0.86 + 0.12 * Random.value);
  const pane: Color[] = [];
  const glow: Color[] = [];
  for (let y = 0; y < M; y++) {
    for (let x = 0; x < M; x++) {
      const tx = x % N;
      const ty = y % N;
      const edge = Math.min(tx, ty, N - 1 - tx, N - 1 - ty);
      const line = edge < 1 ? 1 : edge < 2 ? 0.45 : 0;
      // Lit from above: the bevel's top and left edges a little lighter, bottom and right darker.
      const bevel = edge >= 2 && edge < 5 ? (tx < 5 || ty < 5 ? 1.03 : 0.9) : 1;
      const v = shade[Math.floor(y / N) * BLOCK_TILES + Math.floor(x / N)]! * bevel * (1 - 0.42 * line);
      pane.push(new Color(v, v, v * 1.015, 1));
      glow.push(new Color(0.75 * line, 0.82 * line, 1 * line, 1));
    }
  }
  return {
    pane: Texture.create(M, M, pane, { anisoLevel: 16 }),
    glow: Texture.create(M, M, glow, { anisoLevel: 16 }),
  };
}

/**
 * Soft grey clouds that tile: a few octaves of smooth value noise on a wrapping grid, its
 * alpha the cloud's cover (thin at the edges), lighter where it's thick.
 */
function cloudTexture(grey: number, alpha: number): Texture {
  const N = 256;
  const lattice = (size: number) => Array.from({ length: size * size }, () => Random.value);
  const octaves = [4, 8, 16, 32].map((size) => ({ size, v: lattice(size) }));
  const smooth = (t: number) => t * t * (3 - 2 * t);
  const px: Color[] = [];
  for (let y = 0; y < N; y++) {
    for (let x = 0; x < N; x++) {
      let n = 0;
      let amp = 0.5;
      for (const o of octaves) {
        const fx = (x / N) * o.size;
        const fy = (y / N) * o.size;
        const x0 = Math.floor(fx);
        const y0 = Math.floor(fy);
        const at = (i: number, j: number) => o.v[((j + o.size) % o.size) * o.size + ((i + o.size) % o.size)]!;
        const u = smooth(fx - x0);
        const w = smooth(fy - y0);
        const top = at(x0, y0) * (1 - u) + at(x0 + 1, y0) * u;
        const bottom = at(x0, y0 + 1) * (1 - u) + at(x0 + 1, y0 + 1) * u;
        n += amp * (top * (1 - w) + bottom * w);
        amp /= 2;
      }
      // Cover: clear sky between the clouds, soft edges.
      const c = Math.min(1, Math.max(0, (n - 0.42) / 0.3));
      const g = grey * (0.85 + 0.25 * c);
      px.push(new Color(g, g, g * 1.03, c * alpha));
    }
  }
  return Texture.create(N, N, px);
}

/** A tile lit up: under a footstep, or on its own nearby. */
interface Glow {
  r: MeshRenderer;
  go: GameObject;
  /** Lit (in use); its object is hidden while it's too faint to show. */
  alive: boolean;
  /** Seconds since it lit, and how long it lasts. */
  age: number;
  life: number;
  peak: number;
}
/** A pale fragment drifting up out of the floor. */
interface Fragment {
  go: GameObject;
  p: Vector3;
  speed: number;
  spin: Vector3;
}

/** Where the floor's tiles are lit: brightness levels to switch between. */
const GLOW_LEVELS = 10;

function shape(name: string, mesh: "cube" | "sphere", at: Vector3, scale: Vector3, material: Material, parent?: GameObject) {
  const go = new GameObject(name, { position: at, scale, parent });
  const r = go.addComponent(MeshRenderer, { mesh, material });
  r.shadowCastingMode = "off";
  return go;
}

/** The middle of the tile `v` is in (tiles run between multiples of TILE). */
const tileCentre = (v: number) => (Math.floor(v / TILE) + 0.5) * TILE;
const snapBlock = (v: number) => Math.round(v / BLOCK) * BLOCK;

class WhiteRoom extends Component {
  private camera!: Camera;
  private player!: GameObject;
  private controller!: CharacterController;
  private body: GameObject | null = null;
  private animator: Animator | null = null;
  private floor!: GameObject;
  private clouds: { go: GameObject; offset: Vector3; repeat: number; drift: Vector3 }[] = [];
  private village!: GameObject;
  private glows: Glow[] = [];
  private glowMats: Material[] = [];
  private fragments: Fragment[] = [];
  private yaw = 0;
  private pitch = 12;
  private distance = 4.2;
  /** Which way the figure faces (radians from +z): at first away from the camera. */
  private facing = Math.PI;
  private vy = 0;
  private speed = 0;
  private rollTime = 0;
  private lastTile = "";
  private nextAmbient = 0;
  private pivot = new Vector3(0, 1.4, 0);

  override onStart() {
    QualitySettings.shadowDistance = 30;
    RenderSettings.ambientMode = "color";
    RenderSettings.ambientLight = new Color(0.38, 0.4, 0.44, 1);
    RenderSettings.tonemapping = "neutral";
    // A touch under: bright, but the whites not quite blown.
    RenderSettings.postExposure = -0.3;
    RenderSettings.bloom = { threshold: 1.1, intensity: 0.35, scatter: 0.75 };
    RenderSettings.skybox = {
      zenith: new Color(0.7, 0.72, 0.76, 1),
      horizon: new Color(0.94, 0.95, 0.97, 1),
      ground: new Color(0.94, 0.95, 0.97, 1),
      sunDirection: null,
    };
    Random.initState(7);

    // Soft light from high above: the room has no sun, just a bright overcast.
    const sky = new GameObject("Sky light", { rotation: Quaternion.lookRotation(new Vector3(0.25, -1, 0.35)) });
    sky.addComponent(Light, { type: "directional", color: new Color(1, 0.99, 0.97, 1), intensity: 1.45, shadows: "soft", shadowStrength: 0.6 });

    const { pane, glow } = floorTextures();
    const floorMat = Material.create({
      color: new Color(0.9, 0.91, 0.93, 1),
      mainTexture: pane,
      mainTextureScale: { x: FLOOR / BLOCK, y: FLOOR / BLOCK },
      emissionMap: glow,
      emission: new Color(0.22, 0.24, 0.27, 1),
      smoothness: 0.45,
    });
    this.floor = new GameObject("Floor", { position: new Vector3(0, -0.1, 0), scale: new Vector3(FLOOR, 0.2, FLOOR) });
    this.floor.addComponent(MeshRenderer, { mesh: "cube", material: floorMat });
    this.floor.addComponent(BoxCollider);

    // A low ceiling of grey cloud, two layers drifting: clear overhead, fading into the white
    // towards the horizon as the fog takes them.
    for (const layer of CLOUDS) {
      const mat = Material.create({
        color: Color.white,
        mainTexture: cloudTexture(layer.grey, layer.alpha),
        mainTextureScale: { x: CLOUD_SPAN / layer.repeat, y: CLOUD_SPAN / layer.repeat },
        renderingMode: "fade",
        unlit: true,
        doubleSided: true,
      });
      const go = new GameObject("Clouds", { position: new Vector3(0, layer.y, 0), scale: new Vector3(CLOUD_SPAN, 0.01, CLOUD_SPAN) });
      const r = go.addComponent(MeshRenderer, { mesh: "cube", material: mat });
      r.shadowCastingMode = "off";
      this.clouds.push({ go, offset: new Vector3(Random.range(0, layer.repeat), 0, Random.range(0, layer.repeat)), repeat: layer.repeat, drift: layer.drift });
    }

    this.village = buildVillage();

    // Lit tiles: a pool of thin panels laid on the floor, faded through brightness levels.
    this.glowMats = Array.from({ length: GLOW_LEVELS }, (_, i) => {
      const k = (i + 1) / GLOW_LEVELS;
      // Brighter than the white floor even at its faintest, so a fading tile never looks grey.
      const e = 1 + 3 * k;
      return Material.create({ color: Color.black, emission: new Color(0.88 * e, 0.94 * e, 1.05 * e, 1), unlit: true });
    });
    for (let i = 0; i < 48; i++) {
      const go = shape("Lit tile", "cube", Vector3.zero, new Vector3(TILE - 0.06, 0.006, TILE - 0.06), this.glowMats[0]!);
      go.enabled = false;
      this.glows.push({ r: go.getComponent(MeshRenderer)!, go, alive: false, age: 0, life: 0, peak: 0 });
    }

    // Fragments: small pale cubes rising slowly, spinning.
    const frag = Material.create({ color: Color.black, emission: new Color(1.3, 1.35, 1.45, 1), unlit: true });
    for (let i = 0; i < 70; i++) {
      const s = 0.03 + 0.09 * Random.value ** 2;
      const go = shape("Fragment", "cube", Vector3.zero, new Vector3(s, s, s), frag);
      const f: Fragment = { go, p: Vector3.zero, speed: 0, spin: new Vector3(Random.range(-60, 60), Random.range(-60, 60), Random.range(-60, 60)) };
      this.spawnFragment(f, Random.range(0, 5));
      this.fragments.push(f);
    }

    // The player: a capsule the CharacterController moves; the mannequin hangs below its middle.
    this.player = new GameObject("Player", { position: new Vector3(0, 0.95, 0) });
    this.controller = this.player.addComponent(CharacterController, { radius: 0.3, height: 1.8 });
    this.loadFigure();

    const eye = new GameObject("Main Camera");
    this.camera = eye.addComponent(Camera, { fieldOfView: 55, backgroundColor: WHITE, nearClipPlane: 0.1, farClipPlane: 200 });
    apply();
  }

  /** The mannequin and its Animator, once the model has loaded. */
  private async loadFigure() {
    const model = await Model.load("assets/mannequin.glb").whenLoaded();
    const figure = model.instantiate({ parent: this.player, position: new Vector3(0, -0.9, 0) });
    // The model adds an Animation playing its first clip; the Animator takes over.
    figure.getComponent(Animation)?.destroy();
    // The Animus's figure: white, the joints a pale glowing blue.
    const [main, joints] = model.materials;
    const white = Material.create({ color: new Color(0.92, 0.93, 0.95, 1), smoothness: 0.55 });
    const glow = Material.create({ color: new Color(0.7, 0.85, 1, 1), emission: new Color(0.35, 0.6, 1.1, 1), smoothness: 0.7 });
    const paint = (go: GameObject) => {
      const r = go.getComponent(MeshRenderer);
      if (r) r.material = r.material === joints ? glow : r.material === main ? white : r.material;
      for (const c of go.children) paint(c);
    };
    paint(figure);
    const clip = (name: string, loop = true): AnimationClip => {
      const c = model.animations.find((a) => a.name === name);
      if (!c) throw new Error(`mannequin.glb has no ${name}`);
      c.wrapMode = loop ? "loop" : "once";
      return c;
    };
    const controller = new AnimatorController()
      .addParameter("speed", "float", 0)
      .addParameter("grounded", "bool", true)
      .addParameter("jump", "trigger")
      .addParameter("roll", "trigger")
      // Speeds (m/s) each clip's feet match: idle, walk, jog, sprint.
      .addState("Locomotion", {
        blendParameter: "speed",
        children: [
          { motion: clip("Idle_Loop"), threshold: 0 },
          { motion: clip("Walk_Loop"), threshold: 1.6 },
          { motion: clip("Jog_Fwd_Loop"), threshold: 4 },
          { motion: clip("Sprint_Loop"), threshold: 7 },
        ],
      })
      .addState("Jump", clip("Jump_Start", false))
      .addState("Air", clip("Jump_Loop"))
      .addState("Land", clip("Jump_Land", false))
      .addState("Roll", clip("Roll", false))
      .addTransition({ from: "Any", to: "Jump", conditions: [{ parameter: "jump", mode: "if" }], duration: 0.08 })
      .addTransition({ from: "Jump", to: "Air", hasExitTime: true, exitTime: 0.85, duration: 0.12 })
      .addTransition({ from: "Air", to: "Land", conditions: [{ parameter: "grounded", mode: "if" }], duration: 0.06 })
      .addTransition({ from: "Jump", to: "Land", conditions: [{ parameter: "grounded", mode: "if" }], hasExitTime: true, exitTime: 0.5, duration: 0.06 })
      .addTransition({ from: "Land", to: "Locomotion", hasExitTime: true, exitTime: 0.55, duration: 0.2 })
      .addTransition({ from: "Any", to: "Roll", conditions: [{ parameter: "roll", mode: "if" }], duration: 0.1 })
      .addTransition({ from: "Roll", to: "Locomotion", hasExitTime: true, exitTime: 0.85, duration: 0.2 });
    this.animator = figure.addComponent(Animator, { controller });
    this.body = figure;
  }

  private spawnFragment(f: Fragment, height = 0) {
    const a = Random.range(0, Math.PI * 2);
    const d = Random.range(2.5, 16);
    const at = this.player ? this.player.transform.position : Vector3.zero;
    f.p = new Vector3(at.x + Math.cos(a) * d, height, at.z + Math.sin(a) * d);
    f.speed = Random.range(0.08, 0.3);
  }

  /** Lights the tile at (x, z) for `life` seconds at up to `peak` (0..1). */
  private light(x: number, z: number, life: number, peak: number) {
    const g = this.glows.find((g) => !g.alive) ?? this.glows.reduce((a, b) => (a.age / a.life > b.age / b.life ? a : b));
    g.alive = true;
    g.go.transform.position = new Vector3(tileCentre(x), 0.003, tileCentre(z));
    g.age = 0;
    g.life = life;
    g.peak = peak;
  }

  override onUpdate(dt: number) {
    if (input.getKeyDown("KeyF")) {
      state.fog = (state.fog + 1) % FOGS.length;
      apply();
    }
    if (input.getKeyDown("KeyH")) {
      state.heightFog = !state.heightFog;
      apply();
    }
    if (input.getKeyDown("KeyV")) {
      state.village = !state.village;
      this.village.enabled = state.village;
      refreshHud();
    }
    if (input.getKeyDown("KeyT")) {
      state.tiles = !state.tiles;
      if (!state.tiles) for (const g of this.glows) g.alive = g.go.enabled = false;
      refreshHud();
    }
    this.moveCamera(dt);
    this.movePlayer(dt);
    this.updateFloor(dt);
    this.updateFragments(dt);
    this.updateClouds(dt);
    this.placeCamera(dt);
    this.placeVillage();
    timing.frames++;
    timing.time += dt;
    if (timing.time >= 0.5) {
      timing.fps = timing.frames / timing.time;
      timing.frames = 0;
      timing.time = 0;
      refreshHud();
    }
  }

  /** Turning the camera: drag with a mouse button held, Q / E, the wheel to zoom. */
  private moveCamera(dt: number) {
    if (input.getMouseButton(0) || input.getMouseButton(1)) {
      const d = input.mouseDelta;
      this.yaw += d.x * 0.25;
      this.pitch = Math.min(60, Math.max(-10, this.pitch - d.y * 0.2));
    }
    if (input.getKey("KeyQ")) this.yaw -= 90 * dt;
    if (input.getKey("KeyE")) this.yaw += 90 * dt;
    const wheel = input.mouseScrollDelta.y;
    if (wheel) this.distance = Math.min(9, Math.max(2.2, this.distance - wheel * 0.4));
  }

  private movePlayer(dt: number) {
    const x = (input.getKey("KeyD") || input.getKey("ArrowRight") ? 1 : 0) - (input.getKey("KeyA") || input.getKey("ArrowLeft") ? 1 : 0);
    const z = (input.getKey("KeyW") || input.getKey("ArrowUp") ? 1 : 0) - (input.getKey("KeyS") || input.getKey("ArrowDown") ? 1 : 0);
    // Relative to the camera: forward is where it looks, flattened.
    const yaw = (this.yaw * Math.PI) / 180;
    const fwd = new Vector3(Math.sin(yaw), 0, -Math.cos(yaw));
    const right = new Vector3(Math.cos(yaw), 0, Math.sin(yaw));
    let dir = fwd.scale(z).add(right.scale(x));
    const moving = dir.magnitude > 0.01;
    if (moving) dir = dir.normalized;
    const grounded = this.controller.isGrounded;
    const sprint = input.getKey("ShiftLeft") || input.getKey("ShiftRight");
    const walk = input.getKey("AltLeft") || input.getKey("AltRight");
    let target = moving ? (sprint ? 7 : walk ? 1.6 : 4) : 0;
    // A roll carries on forward on its own for most of its length.
    if (this.rollTime > 0) {
      this.rollTime -= dt;
      target = 5;
      dir = new Vector3(Math.sin(this.facing), 0, Math.cos(this.facing));
    } else if (grounded && input.getKeyDown("KeyC")) {
      this.rollTime = 0.75;
      this.animator?.setTrigger("roll");
    }
    // Ease the speed: quicker to start than to stop, slower to change in the air.
    const ease = grounded ? (target > this.speed ? 8 : 10) : 2;
    this.speed += (target - this.speed) * Math.min(1, ease * dt);
    if (moving || this.rollTime > 0) {
      const want = Math.atan2(dir.x, dir.z);
      let d = want - this.facing;
      d = Math.atan2(Math.sin(d), Math.cos(d));
      this.facing += d * Math.min(1, 12 * dt);
    }
    const heading = new Vector3(Math.sin(this.facing), 0, Math.cos(this.facing));
    // Gravity and jumps.
    if (grounded && this.vy <= 0) {
      this.vy = -1;
      if (input.getKeyDown("Space") && this.rollTime <= 0) {
        this.vy = 5.8;
        this.animator?.setTrigger("jump");
      }
    } else this.vy -= 16 * dt;
    this.controller.move(heading.scale(this.speed * dt).add(new Vector3(0, this.vy * dt, 0)));
    if (this.body) this.body.transform.localRotation = Quaternion.euler(0, (this.facing * 180) / Math.PI, 0);
    if (this.animator) {
      this.animator.setFloat("speed", this.speed);
      this.animator.setBool("grounded", grounded && this.vy <= 0);
    }
    // Footsteps light the tile underfoot.
    const p = this.player.transform.position;
    const tile = `${tileCentre(p.x)},${tileCentre(p.z)}`;
    if (state.tiles && grounded && tile !== this.lastTile) {
      this.light(p.x, p.z, 1.6, 0.75);
      this.lastTile = tile;
    }
    // Far from the origin: shift everything back by whole tiles, so the grid stays put.
    if (Math.abs(p.x) > RECENTRE || Math.abs(p.z) > RECENTRE) {
      // Whole floor blocks, so the tiles' shades stay where they were.
      const shift = new Vector3(-snapBlock(p.x), 0, -snapBlock(p.z));
      this.player.transform.position = p.add(shift);
      this.pivot = this.pivot.add(shift);
      for (const f of this.fragments) f.p = f.p.add(shift);
      for (const g of this.glows) g.go.transform.position = g.go.transform.position.add(shift);
    }
  }

  /** The floor follows the player, snapped to tiles; lit tiles fade; others light on their own. */
  private updateFloor(dt: number) {
    const p = this.player.transform.position;
    // Moved only when the player crosses into another tile: moving a collider teleports it.
    const at = new Vector3(snapBlock(p.x), -0.1, snapBlock(p.z));
    if (!at.equals(this.floor.transform.position)) this.floor.transform.position = at;
    for (const g of this.glows) {
      if (!g.alive) continue;
      g.age += dt;
      if (g.age >= g.life) {
        g.alive = g.go.enabled = false;
        continue;
      }
      // Quick to light, slow to fade.
      const u = g.age / g.life;
      const k = g.peak * (u < 0.12 ? u / 0.12 : (1 - u) / 0.88) ** 1.5;
      const level = Math.round(k * GLOW_LEVELS) - 1;
      g.go.enabled = level >= 0;
      if (level >= 0) g.r.material = this.glowMats[level]!;
    }
    this.nextAmbient -= dt;
    if (state.tiles && this.nextAmbient <= 0) {
      this.nextAmbient = Random.range(0.08, 0.3);
      const a = Random.range(0, Math.PI * 2);
      const d = Random.range(3, 14);
      this.light(p.x + Math.cos(a) * d, p.z + Math.sin(a) * d, Random.range(1.5, 3.5), Random.range(0.25, 0.6));
    }
  }

  /**
   * The clouds drift; each layer stays over the player, moved back a whole repeat of its
   * texture when it has drifted that far, so the jump doesn't show.
   */
  private updateClouds(dt: number) {
    const p = this.player.transform.position;
    for (const c of this.clouds) {
      c.offset = c.offset.add(c.drift.scale(dt));
      const wrap = (v: number) => ((v % c.repeat) + c.repeat) % c.repeat;
      c.offset = new Vector3(wrap(c.offset.x), 0, wrap(c.offset.z));
      const y = c.go.transform.position.y;
      c.go.transform.position = new Vector3(Math.round(p.x / c.repeat) * c.repeat + c.offset.x, y, Math.round(p.z / c.repeat) * c.repeat + c.offset.z);
    }
  }

  private updateFragments(dt: number) {
    const p = this.player.transform.position;
    for (const f of this.fragments) {
      f.p = f.p.add(new Vector3(0, f.speed * dt, 0));
      const away = Math.hypot(f.p.x - p.x, f.p.z - p.z);
      if (f.p.y > 6 || away > 20) this.spawnFragment(f);
      f.go.transform.position = f.p;
      f.go.transform.rotation = Quaternion.euler(f.spin.x * f.p.y, f.spin.y * f.p.y, f.spin.z * f.p.y);
    }
  }

  /** The village keeps its distance, centred under the camera, standing on the floor. */
  private placeVillage() {
    // Deep in the fog's fade, whichever distance it's at: its front rows about two-thirds
    // faded, the back ones lost in the white (close, it would look like a model).
    const f = FOGS[state.fog]!;
    const k = (f.start + 0.82 * (f.end - f.start)) / VILLAGE_R;
    const c = this.camera.transform.position;
    // Only when the fog distance changes: rescaling walks every child.
    if (Math.abs(this.village.transform.localScale.x - k) > 1e-6) this.village.transform.localScale = Vector3.one.scale(k);
    this.village.transform.position = new Vector3(c.x, 0, c.z);
  }

  /** Behind and above the player, looking at the shoulders; follows smoothly. */
  private placeCamera(dt: number) {
    const target = this.player.transform.position.add(new Vector3(0, 0.55, 0));
    this.pivot = this.pivot.add(target.sub(this.pivot).scale(Math.min(1, 10 * dt)));
    const yaw = (this.yaw * Math.PI) / 180;
    const pitch = (this.pitch * Math.PI) / 180;
    const back = new Vector3(-Math.sin(yaw) * Math.cos(pitch), Math.sin(pitch), Math.cos(yaw) * Math.cos(pitch));
    const cam = this.camera.transform;
    cam.position = this.pivot.add(back.scale(this.distance));
    cam.rotation = Quaternion.lookRotation(this.pivot.sub(cam.position));
  }
}

/**
 * A grey village in clusters round the horizon: houses with pitched roofs, towers and, in one
 * cluster, a cathedral's dome on its drum. Built at full size round the origin, then shrunk.
 */
function buildVillage(): GameObject {
  const root = new GameObject("Village");
  const greys = [0.42, 0.5, 0.58, 0.66].map((g) => Material.create({ color: new Color(g, g, g * 1.02, 1), smoothness: 0.1 }));
  const grey = () => greys[Math.floor(Random.value * greys.length)]!;
  const part = (name: string, mesh: "cube" | "sphere" | "cylinder", at: Vector3, scale: Vector3, yaw: number, roll = 0) => {
    const go = new GameObject(name, { parent: root, position: at, scale, rotation: Quaternion.euler(0, yaw, roll) });
    const r = go.addComponent(MeshRenderer, { mesh, material: grey() });
    r.shadowCastingMode = "off";
    return go;
  };
  const clusters = 7;
  for (let k = 0; k < clusters; k++) {
    const a = (k / clusters) * Math.PI * 2 + Random.range(-0.25, 0.25);
    const centre = new Vector3(Math.cos(a) * VILLAGE_R, 0, Math.sin(a) * VILLAGE_R);
    const across = new Vector3(-Math.sin(a), 0, Math.cos(a));
    const houses = 10 + Math.floor(Random.value * 10);
    for (let i = 0; i < houses; i++) {
      // Spread deep through the fog: the front rows fairly clear, the back ones melting
      // into the white, so the village has distance rather than one flat ring.
      const at = centre.add(across.scale(Random.range(-70, 70))).add(new Vector3(Math.cos(a), 0, Math.sin(a)).scale(Random.range(-0.15, 0.3) * VILLAGE_R));
      const w = Random.range(6, 12);
      const d = Random.range(6, 10);
      const h = Random.range(6, 14);
      const yaw = Random.range(-20, 20) + (a * 180) / Math.PI;
      part("House", "cube", at.add(new Vector3(0, h / 2, 0)), new Vector3(w, h, d), yaw);
      // A pitched roof: a box turned 45 degrees along the ridge.
      const r = d / Math.SQRT2;
      part("Roof", "cube", at.add(new Vector3(0, h, 0)), new Vector3(w + 1, r, r), yaw, 0);
      root.children.at(-1)!.transform.localRotation = Quaternion.euler(45, yaw, 0);
    }
    // A tower or two.
    for (let t = 0; t < 1 + Math.floor(Random.value * 2); t++) {
      const at = centre.add(across.scale(Random.range(-40, 40))).add(new Vector3(Math.cos(a), 0, Math.sin(a)).scale(Random.range(-0.1, 0.25) * VILLAGE_R));
      const h = Random.range(28, 45);
      part("Tower", "cube", at.add(new Vector3(0, h / 2, 0)), new Vector3(6, h, 6), 0);
      part("Spire", "cube", at.add(new Vector3(0, h + 3, 0)), new Vector3(4.2, 4.2, 4.2), 45, 0);
      root.children.at(-1)!.transform.localRotation = Quaternion.euler(35, 45, 35);
    }
    // One cluster has the cathedral: a drum and a dome, as Florence's.
    if (k === 0) {
      part("Nave", "cube", centre.add(new Vector3(0, 12, 0)), new Vector3(20, 24, 50), (a * 180) / Math.PI);
      part("Drum", "cylinder", centre.add(new Vector3(0, 30, 0)), new Vector3(24, 6, 24), 0);
      part("Dome", "sphere", centre.add(new Vector3(0, 36, 0)), new Vector3(24, 26, 24), 0);
      part("Lantern", "cylinder", centre.add(new Vector3(0, 50, 0)), new Vector3(3, 3, 3), 0);
    }
  }
  return root;
}

register({ WhiteRoom });

/** Frames a second, measured over half a second. */
const timing = { fps: 0, frames: 0, time: 0 };

function Row(props: { label: string; value: string; keyName: string }) {
  return (
    <box style={{ flexDirection: "row", gap: 10, alignItems: "baseline" }}>
      <text style={{ color: "#5a6270", fontSize: 13, width: 84 }}>{props.label}</text>
      <text style={{ color: "#20242c", fontSize: 13, width: 150 }}>{props.value}</text>
      <text style={{ color: "#8a929e", fontSize: 12 }}>{props.keyName}</text>
    </box>
  );
}

function Hud() {
  const [, setTick] = useState(0);
  refreshHud = () => setTick((t) => t + 1);
  return (
    <box style={{ width: "100%", height: "100%", padding: 18, pointerEvents: "none" }}>
      <box style={{ alignSelf: "flex-start", padding: 14, gap: 6, borderRadius: 10, backgroundColor: "rgba(255, 255, 255, 0.55)", flexDirection: "column" }}>
        <box style={{ flexDirection: "row", justifyContent: "space-between", gap: 16 }}>
          <text style={{ color: "#20242c", fontSize: 16, fontWeight: 600 }}>Fog: the white room</text>
          <text style={{ color: "#5a6270", fontSize: 13 }}>{timing.fps.toFixed(0)} fps</text>
        </box>
        <Row label="Fog" value={FOGS[state.fog]!.name} keyName="F" />
        <Row label="Height fog" value={state.heightFog ? "On" : "Off"} keyName="H" />
        <Row label="Lit tiles" value={state.tiles ? "On" : "Off"} keyName="T" />
        <Row label="Village" value={state.village ? "On the horizon" : "Off"} keyName="V" />
        <text style={{ color: "#5a6270", fontSize: 12, marginTop: 4 }}>
          WASD move · Shift sprint · Alt walk · Space jump · C roll · drag or Q/E turn · wheel zoom
        </text>
      </box>
    </box>
  );
}

createRoot().render(<Hud />);
