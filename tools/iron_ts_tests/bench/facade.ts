// The TypeScript `@iron/engine` shapes the examples use, over the native API: object-valued
// namespace properties and `new GameObject(name, { .. })` option keys. The Rust test
// (tests/all/facade.rs) reads the same state back.

export function skyAndBloom(): string {
  RenderSettings.skybox = { sunDirection: new Vector3(0.5, 0.45, 0.4), zenith: new Color(1, 0, 0, 1), sunSize: 2 };
  const a = `${RenderSettings.skyboxEnabled}|${RenderSettings.skyZenith.r}|${RenderSettings.skySunSize}|${RenderSettings.skyHasSun}`;
  RenderSettings.skybox = { sunDirection: null };
  const b = `${RenderSettings.skyZenith.r === 1}|${RenderSettings.skyHasSun}`;
  RenderSettings.bloom = { threshold: 1.2, intensity: 0.45, scatter: 0.55 };
  const c = `${RenderSettings.bloomEnabled}|${RenderSettings.bloomThreshold}|${RenderSettings.bloomScatter}`;
  RenderSettings.bloom = null;
  RenderSettings.skybox = null;
  return `${a}|${b}|${c}|${RenderSettings.bloomEnabled}|${RenderSettings.skyboxEnabled}`;
}

export function gameObjectOptions(): string {
  const parent = new GameObject("P", { position: new Vector3(1, 2, 3), scale: new Vector3(2, 2, 2) });
  const child = new GameObject("C", { position: new Vector3(0, 1, 0), parent, layer: 3, active: false });
  const top = new GameObject("T", { parent: null });
  const w = child.transform.position;
  return `${child.parent === parent}|${top.parent === null}|${child.layer}|${child.enabled}|${child.transform.localPosition.y}|${w.x}|${w.y}|${w.z}`;
}

export function cameraAndScreen(): string {
  const cam = new GameObject("Cam").addComponent(Camera, { rect: new Rect(0.25, 0.5, 0.5, 0.25), fieldOfView: 60 });
  const far = cam.layerCullDistances;
  far[3] = 14;
  cam.layerCullDistances = far;
  const main = Camera.main;
  const hfov = cam.horizontalFieldOfView;
  cam.horizontalFieldOfView = hfov;
  const size = Screen.windowSize;
  return `${cam.rect.x}|${cam.rect.height}|${cam.layerCullDistances[3]}|${cam.layerCullDistances[4]}|${main === cam}|${Math.round(cam.fieldOfView)}|${size.width >= 0}|${QualitySettings.names.length === QualitySettings.levelCount}`;
}

export function constructors(): string {
  const a = new Color(0.1, 0.2, 0.3);
  const m = new Material(Color.white, new Color(4, 3, 2, 1), 0.25, true);
  const t = m.withTexture(Texture.create(1, 1, [Color.white]));
  const p = GameObject.createPrimitive("sphere", { position: new Vector3(1, 2, 3), scale: Vector3.one.scale(2), name: "Ball" });
  return `${a.a}|${m.smoothness}|${m.unlit}|${m.emission.r}|${t.mainTexture.width}|${p.name}|${p.transform.localScale.x}|${p.transform.position.y}`;
}

export function structured(): string {
  const go = new GameObject("g");
  const r1 = go.addComponent(MeshRenderer, { mesh: "cube" });
  const lod = go.addComponent(LODGroup, { size: 3 });
  lod.setLODs([{ screenRelativeTransitionHeight: 0.5, renderers: [r1] }, { screenRelativeTransitionHeight: 0.1, renderers: [] }]);
  const ball = new GameObject("ball");
  const cloth = go.addComponent(Cloth, {
    pins: [new Vector3(0, 1, 0), new Vector3(1, 1, 0)],
    sphereColliders: [ball],
    pinBox: { min: new Vector3(-1, -1, -1), max: new Vector3(1, 1, 1) },
  });
  const roots = SceneManager.getActiveScene().getRootGameObjects();
  return `${lod.lodCount}|${lod.lodRendererCount(0)}|${lod.lodRendererCount(1)}|${roots.length === SceneManager.getActiveScene().rootCount}|${cloth.rebuildRequests >= 0}`;
}

export function animatorController(): string {
  const clip = new AnimationClip("idle");
  const controller = new AnimatorController()
    .addParameter("speed", "float", 0)
    .addParameter("jump", "trigger")
    .addState("Locomotion", { blendParameter: "speed", children: [{ motion: clip, threshold: 0 }, { motion: clip, threshold: 1 }] }, { speed: 2 })
    .addState("Jump", clip)
    .addTransition({ from: "Locomotion", to: "Jump", conditions: [{ parameter: "jump", mode: "if" }], duration: 0.1 })
    .addTransition({ from: "Jump", to: "Locomotion", hasExitTime: true, exitTime: 0.95, duration: 0.15 });
  const animator = new GameObject("a").addComponent(Animator, { controller });
  return `${controller.parameterCount}|${controller.nodeCount}|${controller.transitionCount}|${animator.getCurrentAnimatorStateInfo().name}`;
}

export function sprites(): string {
  const tex = Texture.create(4, 2, [Color.white, Color.white, Color.white, Color.white, Color.white, Color.white, Color.white, Color.white]);
  const s = Sprite.create(tex, new Rect(0, 0, 2, 2), new Vector2(0.5, 0.5), 40);
  const grid = Sprite.grid(tex, 2, 1, undefined, 10);
  return `${s.pixelsPerUnit}|${s.hasRect}|${grid.length}|${grid[1]!.pixelsPerUnit}`;
}

export function uiResources(): string {
  const a = addStyleSheet(".a { color: red }");
  loadFont({ family: "Charm", src: "fonts/x.ttf", weight: 700 });
  removeStyleSheet(a);
  const update = () => {};
  const stop = input.on("gamepadconnected", update);
  stop();
  return `${a}|${input.gamepads().length}`;
}

export function particlesAndPoints(): string {
  const go = new GameObject("p");
  const ps = go.addComponent(ParticleSystem, {
    startLifetime: [1.2, 1.8],
    startSpeed: 3,
    startColor: [new Color(1, 0, 0, 1), new Color(0, 0, 1, 1)],
    bursts: [{ time: 0, count: 40, cycles: 0, interval: 2 }, { time: 1, count: 5 }],
    shape: { type: "cone", angle: 14, radius: 0.08 },
    colorOverLifetime: [{ time: 0, color: Color.white }, { time: 1, color: new Color(1, 1, 1, 0) }],
    sizeOverLifetime: [{ time: 0, value: 1 }, { time: 1, value: 0 }],
  });
  const edge = go.addComponent(EdgeCollider2D, { points: [new Vector2(0, 0), new Vector2(1, 1), new Vector2(2, 1)] });
  const line = new GameObject("l").addComponent(LineRenderer, { positions: [Vector3.zero, Vector3.one], width: 0.1 });
  const sprite = new GameObject("s").addComponent(SpriteRenderer, { sprite: null });
  return `${ps.startLifetime}|${ps.startLifetimeMax}|${ps.startSpeed}|${ps.burstCount}|${ps.shapeType}|${ps.shapeAngle}|${edge.pointCount}|${line.positionCount}|${sprite.sprite === null}`;
}

// ---- round 2: the shapes the remaining examples use (M9-TS-2000..) ----

export function curveFromPairs(): string {
  const pts: number[][] = [];
  for (let i = 0; i <= 4; i++) pts.push([i, i * 2]);
  const c = AnimationCurve.fromPoints(pts, "linear");
  const d = AnimationCurve.fromPoints([[0, 0], [2, 4]]);
  return `${c.length}|${c.evaluate(1.5)}|${d.evaluate(1)}`;
}

export function unsubscribers(): string {
  let calls = 0;
  const off = QualitySettings.onLevelChanged(() => {
    calls++;
  });
  QualitySettings.setQualityLevel(0);
  const first = calls;
  off();
  QualitySettings.setQualityLevel(QualitySettings.levelCount - 1);
  QualitySettings.setQualityLevel(0);
  const modes = Screen.supportedFullScreenModes;
  const names: Record<FullScreenMode, string> = { ExclusiveFullScreen: "x", FullScreenWindow: "b", MaximizedWindow: "m", Windowed: "w" };
  const label = modes.length > 0 ? names[modes[0]!] : "none";
  return `${typeof off}|${first >= 0}|${calls === first}|${label.length}`;
}

export function bareReturnThenCleanup(): string {
  const effect = (skip: boolean) => {
    if (skip) return;
    return () => 7;
  };
  const a = effect(true);
  const b = effect(false);
  return `${a === null}|${b !== null}|${b ? b() : 0}`;
}

export function tuples(): string {
  const rows = [
    [0, -2.5, 4, false],
    [1, 2.5, 8, true],
  ] as const;
  let out = "";
  for (const [x, z, length, turn] of rows) out += `${x}:${z}:${length}:${turn ? "T" : "F"},`;
  const pair = [3, "three"];
  return `${out}${rows[1][2]}|${pair[1]}`;
}

export function defaultedClosureParams(): string {
  const row = (x: number, name: string, scale = 0.75, tag = "t") => `${x}${name}${scale}${tag}`;
  return `${row(1, "a")}|${row(2, "b", 2)}|${row(3, "c", 3, "z")}`;
}

export function voidAndExtraArguments(): string {
  let n = 0;
  const bump = () => {
    n++;
  };
  const f = () => void bump();
  f();
  f();
  const g = (a: number) => a + 1;
  return `${n}|${g(1)}`;
}

export function recordByEngineString(): string {
  const names: Record<"disable" | "hardOnly" | "all", string> = { disable: "off", hardOnly: "hard", all: "soft" };
  return names[QualitySettings.shadows as "all"];
}

export function plainPointsToValues(): string {
  const mk = (n: number): { x: number; y: number }[] => Array.from({ length: n }, (_, i) => ({ x: i, y: i * 2 }));
  const go = new GameObject("e");
  const edge = go.addComponent(EdgeCollider2D);
  edge.points = mk(4);
  const a = edge.pointCount;
  edge.points = a > 3 ? mk(2) : [];
  return `${a}|${edge.pointCount}`;
}

class Box1 {
  v = 1;
  step(d: number): void {
    this.v += d;
  }
}
class Box2 {
  s = "a";
  step(d: number): void {
    this.s += d;
  }
}

export function unrolledMixedLoop(): string {
  const a = new Box1();
  const b = new Box2();
  let seen = 0;
  for (const t of [a, b]) {
    t.step(2);
    seen++;
  }
  return `${a.v}|${b.s}|${seen}`;
}
