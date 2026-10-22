// The TypeScript `@iron/engine` members the native engine API has in another shape, written over
// the native functions. A chunk is appended to a program that names one of its triggers (the text
// after `when`, separated by `|`), when the engine API has the names after an optional `@`
// (`//# when Spline @ Vector3`). The naming scheme is in `facade.rs`. A chunk may also be plain
// TypeScript classes (`Shapes`, `Spline`, `SplineAnimate`); their declarations for `tsc` are in
// `engine_ts.d.ts`. Keep module-level state out of a chunk: a top-level initialiser of the
// program may run a chunk's code before the chunk's own `const`s are set.

//# when .rect|rect:
function __xg_Camera_rect(self: Camera): Rect {
  return new Rect(self.rectX, self.rectY, self.rectWidth, self.rectHeight);
}
function __xs_Camera_rect(self: Camera, r: Rect): void {
  self.setRect(r.x, r.y, r.width, r.height);
}

//# when layerCullDistances
function __xg_Camera_layerCullDistances(self: Camera): number[] {
  const out: number[] = [];
  for (let i = 0; i < 32; i++) out.push(self.layerCullDistance(i));
  return out;
}
function __xs_Camera_layerCullDistances(self: Camera, v: number[]): void {
  for (let i = 0; i < 32; i++) self.setLayerCullDistance(i, i < v.length ? v[i]! : 0);
}

//# when horizontalFieldOfView
function __xg_Camera_horizontalFieldOfView(self: Camera): number {
  const aspect = Screen.aspect * (self.rectWidth / self.rectHeight);
  return (Math.atan(Math.tan((self.fieldOfView * Math.PI) / 360) * aspect) * 360) / Math.PI;
}
function __xs_Camera_horizontalFieldOfView(self: Camera, v: number): void {
  const aspect = Screen.aspect * (self.rectWidth / self.rectHeight);
  self.fieldOfView = (Math.atan(Math.tan((v * Math.PI) / 360) / aspect) * 360) / Math.PI;
}

//# when getRootGameObjects
function __xm_Scene_getRootGameObjects(self: Scene): GameObject[] {
  const out: GameObject[] = [];
  const n = self.rootCount;
  for (let i = 0; i < n; i++) out.push(self.rootAt(i));
  return out;
}

//# when Screen.windowSize
interface WindowSize {
  width: number;
  height: number;
}
function __xp_Screen_windowSize(): WindowSize {
  return { width: Screen.windowWidth, height: Screen.windowHeight };
}

//# when Screen.currentResolution|Screen.resolutions
interface Resolution {
  width: number;
  height: number;
  refreshRate: number;
}
function __xp_Screen_currentResolution(): Resolution {
  return { width: Screen.currentResolutionWidth, height: Screen.currentResolutionHeight, refreshRate: Screen.currentResolutionRefreshRate };
}
function __xp_Screen_resolutions(): Resolution[] {
  const out: Resolution[] = [];
  const n = Screen.resolutionCount;
  for (let i = 0; i < n; i++) out.push({ width: Screen.resolutionWidth(i), height: Screen.resolutionHeight(i), refreshRate: Screen.resolutionRefreshRate(i) });
  return out;
}

//# when QualitySettings.names
function __xp_QualitySettings_names(): string[] {
  const out: string[] = [];
  const n = QualitySettings.levelCount;
  for (let i = 0; i < n; i++) out.push(QualitySettings.levelName(i));
  return out;
}

//# when setLODs
interface LodLevel {
  screenRelativeTransitionHeight: number;
  renderers: MeshRenderer[];
}
function __xm_LODGroup_setLODs(self: LODGroup, levels: LodLevel[]): LODGroup {
  self.clearLods();
  for (let i = 0; i < levels.length; i++) {
    self.addLod(levels[i]!.screenRelativeTransitionHeight);
    const rs = levels[i]!.renderers;
    for (let j = 0; j < rs.length; j++) self.addLodRenderer(i, rs[j]!);
  }
  return self;
}

//# when pinBox|pins:|sphereColliders|capsuleColliders
interface ClothBox {
  min: Vector3;
  max: Vector3;
}
function __xs_Cloth_pinBox(self: Cloth, b: ClothBox): void {
  self.setPinBox(b.min, b.max);
}
function __xs_Cloth_pins(self: Cloth, v: Vector3[]): void {
  self.clearPins();
  for (let i = 0; i < v.length; i++) self.addPin(v[i]!);
}
function __xs_Cloth_sphereColliders(self: Cloth, v: GameObject[]): void {
  for (let i = 0; i < v.length; i++) self.addSphereCollider(v[i]!);
}
function __xs_Cloth_capsuleColliders(self: Cloth, v: GameObject[]): void {
  for (let i = 0; i < v.length; i++) self.addCapsuleCollider(v[i]!);
}

//# when bones:
function __xs_SkinnedMeshRenderer_bones(self: SkinnedMeshRenderer, v: GameObject[]): void {
  self.clearBones();
  for (let i = 0; i < v.length; i++) self.addBone(v[i]!);
}

//# when new Material(
function __xn_Material(
  color: Color = Color.white,
  emission: Color = Color.black,
  smoothness: number = 0.5,
  unlit: boolean = false,
  mainTexture: Texture | null = null,
  mainTextureScale: Vector2 = Vector2.one,
  mainTextureOffset: Vector2 = Vector2.zero,
  metallic: number = 0,
): Material {
  const m = Material.create();
  m.mainTextureScale = mainTextureScale;
  m.mainTextureOffset = mainTextureOffset;
  m.color = color;
  m.emission = emission;
  m.smoothness = smoothness;
  m.unlit = unlit;
  if (mainTexture !== null) m.mainTexture = mainTexture;
  m.metallic = metallic;
  return m;
}

//# when withTexture
function __xm_Material_withTexture(self: Material, texture: Texture): Material {
  const m = self.copy();
  m.mainTexture = texture;
  return m;
}

//# when Sprite.grid
function __xt_Sprite_grid(texture: Texture, columns: number, rows: number, pivot?: Vector2, pixelsPerUnit?: number): Sprite[] {
  const out: Sprite[] = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < columns; c++) {
      const s = Sprite.gridCell(texture, columns, rows, r * columns + c);
      if (pivot !== null) s.pivot = pivot;
      if (pixelsPerUnit !== null) s.pixelsPerUnit = pixelsPerUnit;
      out.push(s);
    }
  }
  return out;
}

//# when AnimatorController|getCurrentAnimatorStateInfo
interface AnimatorStateInfo {
  name: string;
  normalizedTime: number;
  length: number;
}
function __xm_Animator_getCurrentAnimatorStateInfo(self: Animator): AnimatorStateInfo {
  return { name: self.currentStateName, normalizedTime: self.currentNormalizedTime, length: self.currentLength };
}
function __xm_AnimationController_addParameter(self: AnimationController, name: string, type: string, defaultValue: number = 0): AnimationController {
  self.addParameter$n(AnimationParameter.create(name, type, defaultValue));
  return self;
}
function __xm_AnimationController_addState(self: AnimationController, name: string, motion: AnimationClip): AnimationController {
  self.addNode(AnimationNode.create(name, motion));
  return self;
}
interface BlendChild {
  motion: AnimationClip;
  threshold: number;
  timeScale?: number;
}
interface BlendSpec {
  blendParameter: string;
  children: BlendChild[];
}
interface StateOpts {
  speed?: number;
  speedParameter?: string;
}
function __xm_AnimationController_addState$o(self: AnimationController, name: string, motion: BlendSpec, opts?: StateOpts): AnimationController {
  const tree = AnimationBlendTree.create(motion.blendParameter);
  for (let i = 0; i < motion.children.length; i++) {
    const c = motion.children[i]!;
    tree.addChild(c.motion, c.threshold, c.timeScale ?? 1);
  }
  const node = AnimationNode.blend(name, tree);
  if (opts) {
    if (opts.speed !== undefined) node.speed = opts.speed;
    if (opts.speedParameter !== undefined) node.speedParameter = opts.speedParameter;
  }
  self.addNode(node);
  return self;
}
interface ConditionSpec {
  parameter: string;
  mode: string;
  threshold?: number;
}
interface TransitionSpec {
  from: string;
  to: string;
  conditions?: ConditionSpec[];
  hasExitTime?: boolean;
  exitTime?: number;
  duration?: number;
  offset?: number;
}
function __xm_AnimationController_addTransition(self: AnimationController, t: TransitionSpec): AnimationController {
  const tr = AnimationTransition.create(t.from, t.to);
  if (t.hasExitTime !== undefined) tr.hasExitTime = t.hasExitTime;
  if (t.exitTime !== undefined) tr.exitTime = t.exitTime;
  if (t.duration !== undefined) tr.duration = t.duration;
  if (t.offset !== undefined) tr.offset = t.offset;
  if (t.conditions) {
    for (let i = 0; i < t.conditions.length; i++) {
      const c = t.conditions[i]!;
      tr.addCondition(AnimationCondition.create(c.parameter, c.mode, c.threshold ?? 0));
    }
  }
  self.addTransition$n(tr);
  return self;
}

//# when Sprite.create(
function __xt_Sprite_create$3(texture: Texture, rect: Rect | null, pivot?: Vector2): Sprite {
  return __sprite_make(texture, rect, pivot, 100);
}
function __xt_Sprite_create$4(texture: Texture, rect: Rect | null, pivot?: Vector2, pixelsPerUnit?: number): Sprite {
  return __sprite_make(texture, rect, pivot, pixelsPerUnit ?? 100);
}
function __sprite_make(texture: Texture, rect: Rect | null, pivot: Vector2 | undefined, pixelsPerUnit: number): Sprite {
  const s = Sprite.create(texture, pixelsPerUnit);
  if (rect !== null) s.setRect(rect.x, rect.y, rect.width, rect.height);
  if (pivot !== undefined) s.pivot = pivot;
  return s;
}

//# when loadSceneAsync
function __xt_SceneManager_loadSceneAsync(path: string, mode?: string): Promise<Scene> {
  return new Promise<Scene>((resolve, reject) => {
    let loaded = 0;
    let failed = 0;
    loaded = SceneManager.onSceneLoaded$n((scene: Scene, m: string) => {
      if (scene.path === path) {
        SceneManager.offSceneEvent(loaded);
        SceneManager.offSceneEvent(failed);
        resolve(scene);
      }
    });
    failed = SceneManager.onSceneLoadFailed((p: string, error: string) => {
      if (p === path) {
        SceneManager.offSceneEvent(loaded);
        SceneManager.offSceneEvent(failed);
        reject(new Error("could not load scene " + path + ": " + error));
      }
    });
    SceneManager.loadSceneAsync$n(path, mode ?? "single");
  });
}

//# when Prefab.load
function __xt_Prefab_load(path: string): Promise<Prefab> {
  return new Promise<Prefab>((resolve, reject) => {
    let loaded = 0;
    let failed = 0;
    loaded = SceneManager.onPrefabLoaded((p: string, prefab: Prefab) => {
      if (p === path) {
        SceneManager.offSceneEvent(loaded);
        SceneManager.offSceneEvent(failed);
        resolve(prefab);
      }
    });
    failed = SceneManager.onSceneLoadFailed((p: string, error: string) => {
      if (p === path) {
        SceneManager.offSceneEvent(loaded);
        SceneManager.offSceneEvent(failed);
        reject(new Error("could not load prefab " + path + ": " + error));
      }
    });
    SceneManager.loadPrefabAsync(path);
  });
}

//# when whenLoaded
function __poll_texture(self: Texture, resolve: (v: Texture) => void, reject: (e: Error) => void): void {
  if (self.failed) reject(new Error("texture failed to load"));
  else if (self.isLoaded) resolve(self);
  else setTimeout(() => { __poll_texture(self, resolve, reject); }, 0);
}
function __poll_model(self: Model, resolve: (v: Model) => void, reject: (e: Error) => void): void {
  if (self.failed) reject(new Error("model failed to load"));
  else if (self.isLoaded) resolve(self);
  else setTimeout(() => { __poll_model(self, resolve, reject); }, 0);
}
function __xm_Texture_whenLoaded(self: Texture): Promise<Texture> {
  return new Promise<Texture>((resolve, reject) => { __poll_texture(self, resolve, reject); });
}
function __xm_Model_whenLoaded(self: Model): Promise<Model> {
  return new Promise<Model>((resolve, reject) => { __poll_model(self, resolve, reject); });
}

//# when .serialize()
interface SceneObjectData {
  name: string;
}
interface SceneData {
  name: string;
  objects: SceneObjectData[];
}
function __xm_Scene_serialize(self: Scene): SceneData {
  const d: SceneData = JSON.parse(self.serialize$n());
  return d;
}

//# when ParticleSystem
function __xs_ParticleSystem_startLifetime$a(self: ParticleSystem, v: number[]): void {
  self.setStartLifetimeRange(v[0]!, v[1]!);
}
function __xs_ParticleSystem_startSpeed$a(self: ParticleSystem, v: number[]): void {
  self.setStartSpeedRange(v[0]!, v[1]!);
}
function __xs_ParticleSystem_startSize$a(self: ParticleSystem, v: number[]): void {
  self.setStartSizeRange(v[0]!, v[1]!);
}
function __xs_ParticleSystem_startRotation$a(self: ParticleSystem, v: number[]): void {
  self.setStartRotationRange(v[0]!, v[1]!);
}
function __xs_ParticleSystem_angularVelocity$a(self: ParticleSystem, v: number[]): void {
  self.setAngularVelocityRange(v[0]!, v[1]!);
}
function __xs_ParticleSystem_startColor$a(self: ParticleSystem, v: Color[]): void {
  self.setStartColorRange(v[0]!, v[1]!);
}
interface ParticleBurst {
  time: number;
  count: number;
  cycles?: number;
  interval?: number;
}
function __xs_ParticleSystem_bursts(self: ParticleSystem, v: ParticleBurst[]): void {
  self.clearBursts();
  for (let i = 0; i < v.length; i++) {
    const b = v[i]!;
    self.addBurst(b.time, b.count, b.cycles ?? 1, b.interval ?? 0.01);
  }
}
interface ParticleShapeSpec {
  type: string;
  angle?: number;
  radius?: number;
  size?: Vector3;
}
function __xs_ParticleSystem_shape(self: ParticleSystem, s: ParticleShapeSpec): void {
  self.shapeType = s.type;
  if (s.angle !== undefined) self.shapeAngle = s.angle;
  if (s.radius !== undefined) self.shapeRadius = s.radius;
  if (s.size !== undefined) self.shapeSize = s.size;
}
interface ColorKey {
  time: number;
  color: Color;
}
function __xs_ParticleSystem_colorOverLifetime(self: ParticleSystem, v: ColorKey[]): void {
  self.clearColorOverLifetime();
  for (let i = 0; i < v.length; i++) self.addColorKey(v[i]!.time, v[i]!.color);
}
interface SizeKey {
  time: number;
  value: number;
}
function __xs_ParticleSystem_sizeOverLifetime(self: ParticleSystem, v: SizeKey[]): void {
  self.clearSizeOverLifetime();
  for (let i = 0; i < v.length; i++) self.addSizeKey(v[i]!.time, v[i]!.value);
}

//# when Mesh.create({
interface MeshData {
  vertices: Vector3[];
  triangles: number[];
  normals?: Vector3[];
  uv?: Vector2[];
  boneWeights?: { indices: number[]; weights: number[] }[];
  bindposes?: Matrix4x4[];
}
function __xt_Mesh_create$o(o: MeshData): Mesh {
  const m = Mesh.create(o.vertices, o.triangles, o.normals ?? [], o.uv ?? []);
  if (o.boneWeights) {
    const joints: number[] = [];
    const weights: number[] = [];
    for (let i = 0; i < o.boneWeights.length; i++) {
      const b = o.boneWeights[i]!;
      for (let k = 0; k < 4; k++) {
        joints.push(k < b.indices.length ? b.indices[k]! : 0);
        weights.push(k < b.weights.length ? b.weights[k]! : 0);
      }
    }
    m.setBoneWeights(joints, weights);
  }
  if (o.bindposes) {
    const flat: number[] = [];
    for (let i = 0; i < o.bindposes.length; i++) {
      const p = o.bindposes[i]!;
      for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) flat.push(p.get(r, c));
    }
    m.setBindposes(flat);
  }
  return m;
}

//# when addStyleSheet(|removeStyleSheet(
function addStyleSheet(css: string): number {
  return UiResources.addStyleSheet(css);
}
function removeStyleSheet(id: number): void {
  UiResources.removeStyleSheet(id);
}

//# when loadFont(
interface FontFace {
  family: string;
  src: string;
  weight?: number;
  style?: string;
}
function loadFont(face: FontFace): void {
  UiResources.loadFont(face.family, face.src, face.weight ?? 0, face.style ?? "");
}

//# when input.gamepads()|input.on(
interface Gamepad {
  id: number;
  name: string;
  connected: boolean;
  buttons: Record<string, number>;
  axes: Record<string, number>;
}
interface GamepadEvent {
  gamepad: Gamepad;
}
// A pad as the TypeScript API has it: the standard buttons and axes by name (0 when not reported).
function __pad(id: number): Gamepad {
  const buttons: Record<string, number> = {};
  for (const b of ["South", "East", "West", "North", "C", "Z", "LeftBumper", "RightBumper", "LeftTrigger", "RightTrigger", "Select", "Start", "Mode", "LeftStick", "RightStick", "DPadUp", "DPadDown", "DPadLeft", "DPadRight"]) buttons[b] = input.gamepadButton(id, b);
  const axes: Record<string, number> = {};
  for (const a of ["LeftStickX", "LeftStickY", "RightStickX", "RightStickY", "LeftZ", "RightZ", "DPadX", "DPadY"]) axes[a] = input.gamepadAxis(id, a);
  return { id, name: input.gamepadName(id), connected: input.gamepadConnected(id), buttons, axes };
}
function __xt_input_gamepads(): Gamepad[] {
  const out: Gamepad[] = [];
  const n = input.gamepadCount();
  for (let i = 0; i < n; i++) out.push(__pad(input.gamepadId(i)));
  return out;
}
function __xt_input_on(type: string, fn: (e: GamepadEvent) => void): () => void {
  const token = type === "gamepaddisconnected" ? input.onGamepadDisconnected((id: number) => { fn({ gamepad: __pad(id) }); }) : input.onGamepadConnected((id: number) => { fn({ gamepad: __pad(id) }); });
  return () => { input.off(token); };
}

//# when AnimationCurve.fromPoints
function __xt_AnimationCurve_fromPoints(points: number[][], mode?: string): AnimationCurve {
  const v: Vector2[] = [];
  for (let i = 0; i < points.length; i++) v.push(new Vector2(points[i]![0]!, points[i]![1]!));
  return AnimationCurve.fromPoints$n(v, mode ?? "linear");
}

//# when QualitySettings.onLevelChanged
function __xt_QualitySettings_onLevelChanged(fn: (level: number) => void): () => void {
  const token = QualitySettings.onLevelChanged$n(fn);
  return () => { QualitySettings.offLevelChanged(token); };
}

//# when Screen.onChange
function __xt_Screen_onChange(fn: () => void): () => void {
  const token = Screen.onChange$n(fn);
  return () => { Screen.offChange(token); };
}

//# when Assets.onChanged
function __xt_Assets_onChanged(fn: (path: string) => void): () => void {
  const token = Assets.onChanged$n(fn);
  return () => { Assets.offChanged(token); };
}

//# when Application.onFocusChanged
function __xt_Application_onFocusChanged(fn: (focused: boolean) => void): () => void {
  const token = Application.onFocusChanged$n(fn);
  return () => { Application.offFocusChanged(token); };
}

//# when SceneManager.onSceneLoaded|SceneManager.onSceneUnloaded|SceneManager.onActiveSceneChanged
function __xt_SceneManager_onSceneLoaded(fn: (scene: Scene, mode: string) => void): () => void {
  const token = SceneManager.onSceneLoaded$n(fn);
  return () => { SceneManager.offSceneEvent(token); };
}
function __xt_SceneManager_onSceneUnloaded(fn: (scene: Scene) => void): () => void {
  const token = SceneManager.onSceneUnloaded$n(fn);
  return () => { SceneManager.offSceneEvent(token); };
}
function __xt_SceneManager_onActiveSceneChanged(fn: (from: Scene, to: Scene) => void): () => void {
  const token = SceneManager.onActiveSceneChanged$n(fn);
  return () => { SceneManager.offSceneEvent(token); };
}

//# when postMessage&Window|onMessage&Window|onRequest&Window|.request<&Window|.request(&Window|broadcast&Window
// Messages between windows travel as JSON of the static type (the compiler refuses handles in it).
function __xm_Window_postMessage<T>(self: Window, data: T): boolean {
  return self.postMessageJson(JSON.stringify(data));
}
function __xm_Window_onMessage<T>(self: Window, f: (from: Window, data: T) => void): number {
  return self.onMessageJson((from: Window, text: string) => {
    const data: T = JSON.parse(text);
    f(from, data);
  });
}
function __xm_Window_onRequest<T, R, U = number>(self: Window, f: (from: Window, data: T, reply: (answer: R) => void) => U): number {
  return self.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    });
  });
}
// An async handler (it returns a promise, found by the compiler): a rejection fails the asker's promise.
function __xm_Window_onRequestP<T, R>(self: Window, f: (from: Window, data: T, reply: (answer: R) => void) => Promise<void>): number {
  return self.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }).then(() => {}, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xt_Window_onMessage<T>(f: (from: Window, data: T) => void): number {
  return Window.onMessageJson((from: Window, text: string) => {
    const data: T = JSON.parse(text);
    f(from, data);
  });
}
function __xt_Window_onRequest<T, R, U = number>(f: (from: Window, data: T, reply: (answer: R) => void) => U): number {
  return Window.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    });
  });
}
function __xt_Window_onRequestP<T, R>(f: (from: Window, data: T, reply: (answer: R) => void) => Promise<void>): number {
  return Window.onRequestJson((from: Window, text: string, id: number) => {
    const data: T = JSON.parse(text);
    f(from, data, (answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }).then(() => {}, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xt_Window_broadcast<T>(data: T): number {
  return Window.broadcastJson(JSON.stringify(data));
}
// `request<R>(data)` is written per reply type by facade.rs (a promise class exists per written type).

//# when .emit(&Window|.emit<&Window|.on(&Window|.on<&Window|.ask(&Window|.ask<&Window|.answer(&Window|.answer<&Window|emitAll
// Named events: sugar over the raw messages. The wire form is `{"$e":<name>,"d":<payload>}` (no `d`
// without a payload). The host delivers envelopes only to the `on` / `answer` listeners of their name;
// raw `onMessage` / `onRequest` listeners never see them.
function __win_event_text(name: string, payload: string | null): string {
  if (name === "") throw new Error("Window event names must not be empty");
  const head = '{"$e":' + JSON.stringify(name);
  return payload === null ? head + "}" : head + ',"d":' + payload + "}";
}
function __xm_Window_emit<T>(self: Window, name: string, payload: T): boolean {
  return self.postMessageJson(__win_event_text(name, JSON.stringify(payload)));
}
function __xm_Window_emit0<U>(self: Window, name: string): boolean {
  return self.postMessageJson(__win_event_text(name, null));
}
function __xt_Window_emitAll<T>(name: string, payload: T): number {
  return Window.broadcastJson(__win_event_text(name, JSON.stringify(payload)));
}
function __xt_Window_emitAll0<U>(name: string): number {
  return Window.broadcastJson(__win_event_text(name, null));
}
function __xm_Window_on<T>(self: Window, name: string, f: (payload: T, from: Window) => void): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onEventJson(name, (from: Window, body: string) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    f(data, from);
  });
}
function __xm_Window_answer<T, R>(self: Window, name: string, f: (payload: T, from: Window) => R): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    const answer: R = f(data, from);
    Window.replyJson(id, JSON.stringify(answer));
  });
}
// An async handler (it returns a promise, found by the compiler): the reply is sent when it resolves, a
// rejection fails the asker's promise with the error's message.
function __xm_Window_answerP<T, R>(self: Window, name: string, f: (payload: T, from: Window) => Promise<R>): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const data: T = JSON.parse(body === "" ? "null" : body);
    f(data, from).then((answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }, (e: Error) => { Window.replyError(id, e.message); });
  });
}
function __xm_Window_answer0P<R>(self: Window, name: string, f: () => Promise<R>): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    f().then((answer: R) => {
      Window.replyJson(id, JSON.stringify(answer));
    }, (e: Error) => { Window.replyError(id, e.message); });
  });
}
// A handler without parameters (`win.on("ping", () => ..)`): there is no payload type to decode.
function __xm_Window_on0<U>(self: Window, name: string, f: () => void): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onEventJson(name, (from: Window, body: string) => {
    f();
  });
}
function __xm_Window_answer0<R>(self: Window, name: string, f: () => R): number {
  if (name === "") throw new Error("Window event names must not be empty");
  return self.onAnswerJson(name, (from: Window, body: string, id: number) => {
    const answer: R = f();
    Window.replyJson(id, JSON.stringify(answer));
  });
}
// `ask<R>(name, payload?, timeoutMs?)` is written per reply type by facade.rs.

//# when RenderSettings.heightFog
interface HeightFogSettings {
  density: number;
  baseHeight: number;
  falloff: number;
}
function __xw_RenderSettings_heightFog(h: HeightFogSettings | null): void {
  RenderSettings.heightFogEnabled = false;
  if (h !== null) {
    RenderSettings.heightFogEnabled = true;
    RenderSettings.heightFogDensity = h.density;
    RenderSettings.heightFogBaseHeight = h.baseHeight;
    RenderSettings.heightFogFalloff = h.falloff;
  }
}

//# when RenderSettings.volumetricFog
interface VolumetricFogSettings {
  intensity?: number;
  anisotropy?: number;
  steps?: number;
  distance?: number;
  albedo?: Color;
  sharpDistance?: number;
  sharpSamples?: number;
  resolution?: string;
  denoising?: string;
  historyWeight?: number;
  samples?: number;
  motionSamples?: number;
}
function __xw_RenderSettings_volumetricFog(v: VolumetricFogSettings | null): void {
  RenderSettings.volumetricFogEnabled = false;
  if (v === null) return;
  RenderSettings.volumetricFogEnabled = true;
  if (v.intensity !== undefined) RenderSettings.volumetricFogIntensity = v.intensity;
  if (v.anisotropy !== undefined) RenderSettings.volumetricFogAnisotropy = v.anisotropy;
  if (v.steps !== undefined) RenderSettings.volumetricFogSteps = v.steps;
  if (v.distance !== undefined) RenderSettings.volumetricFogDistance = v.distance;
  if (v.albedo !== undefined) RenderSettings.volumetricFogAlbedo = v.albedo;
  if (v.sharpDistance !== undefined) RenderSettings.volumetricFogSharpDistance = v.sharpDistance;
  if (v.sharpSamples !== undefined) RenderSettings.volumetricFogSharpSamples = v.sharpSamples;
  if (v.resolution !== undefined) RenderSettings.volumetricFogResolution = v.resolution;
  if (v.denoising !== undefined) RenderSettings.volumetricFogDenoising = v.denoising;
  if (v.historyWeight !== undefined) RenderSettings.volumetricFogHistoryWeight = v.historyWeight;
  if (v.samples !== undefined) RenderSettings.volumetricFogSamples = v.samples;
  if (v.motionSamples !== undefined) RenderSettings.volumetricFogMotionSamples = v.motionSamples;
}

//# when RenderSettings.autoExposure
interface AutoExposureSettings {
  minEV?: number;
  maxEV?: number;
  compensation?: number;
  speedDarkToLight?: number;
  speedLightToDark?: number;
  metering?: string;
  lowPercent?: number;
  highPercent?: number;
  darkAdaptation?: number;
  darkBelow?: number;
  curve?: [number, number][];
  hold?: boolean;
}
function __xw_RenderSettings_autoExposure(a: AutoExposureSettings | null): void {
  RenderSettings.autoExposureEnabled = false;
  if (a === null) return;
  RenderSettings.autoExposureEnabled = true;
  if (a.minEV !== undefined) RenderSettings.autoExposureMinEv = a.minEV;
  if (a.maxEV !== undefined) RenderSettings.autoExposureMaxEv = a.maxEV;
  if (a.compensation !== undefined) RenderSettings.autoExposureCompensation = a.compensation;
  if (a.speedDarkToLight !== undefined) RenderSettings.autoExposureSpeedDarkToLight = a.speedDarkToLight;
  if (a.speedLightToDark !== undefined) RenderSettings.autoExposureSpeedLightToDark = a.speedLightToDark;
  if (a.metering !== undefined) RenderSettings.autoExposureMetering = a.metering;
  if (a.lowPercent !== undefined) RenderSettings.autoExposureLowPercent = a.lowPercent;
  if (a.highPercent !== undefined) RenderSettings.autoExposureHighPercent = a.highPercent;
  if (a.darkAdaptation !== undefined) RenderSettings.autoExposureDarkAdaptation = a.darkAdaptation;
  if (a.darkBelow !== undefined) RenderSettings.autoExposureDarkBelow = a.darkBelow;
  if (a.hold !== undefined) RenderSettings.autoExposureHold = a.hold;
  if (a.curve !== undefined) {
    for (let i = 0; i < a.curve.length; i++) RenderSettings.addAutoExposureCurvePoint(a.curve[i]![0], a.curve[i]![1]);
  }
}

//# when createFlat|setFlat
function __xt_Mesh_createFlat<D>(data: D): Mesh {
  return Mesh.createFlatArrays(data.positions, data.triangles, "normals" in data ? (data.normals ?? []) : [], "uv" in data ? (data.uv ?? []) : []);
}
function __xm_Mesh_setFlat<D>(self: Mesh, data: D): void {
  self.setFlatArrays(data.positions, data.triangles, "normals" in data ? (data.normals ?? []) : [], "uv" in data ? (data.uv ?? []) : []);
}

//# when Random.pick
function __xt_Random_pick<T>(items: T[]): T | undefined {
  return items.length > 0 ? items[Random.rangeInt(0, items.length)] : undefined;
}

//# when illuminanceAt|measureIlluminance
function __light_lux(l: Light, point: Vector3, normal: Vector3 | undefined): number {
  if (!l.gameObject.activeInHierarchy) return 0;
  const t = l.gameObject.transform;
  const forward = t.forward;
  if (l.type === "directional") {
    const cos = normal !== undefined ? Math.max(0, -(normal.x * forward.x + normal.y * forward.y + normal.z * forward.z)) : 1;
    return l.intensity * cos;
  }
  const p = t.position;
  const dx = point.x - p.x;
  const dy = point.y - p.y;
  const dz = point.z - p.z;
  const d = Math.sqrt(dx * dx + dy * dy + dz * dz);
  if (d >= l.range || d === 0) return 0;
  // As the renderer: a smooth fade to zero at the range.
  const x = d / l.range;
  const fade = (1 - x * x * x * x) * (1 - x * x * x * x);
  // Lumens to candela, all round (HDRP's spots without reflector).
  let candela = l.intensity / (4 * Math.PI);
  if (l.type === "spot") {
    const c = (dx * forward.x + dy * forward.y + dz * forward.z) / d;
    const outer = Math.cos(((l.spotAngle / 2) * Math.PI) / 180);
    const inner = Math.cos(((l.innerSpotAngle / 2) * Math.PI) / 180);
    const u = Math.min(1, Math.max(0, (c - outer) / Math.max(inner - outer, 1e-4)));
    candela *= u * u * (3 - 2 * u);
  }
  const cos = normal !== undefined ? Math.max(0, -(normal.x * dx + normal.y * dy + normal.z * dz) / d) : 1;
  return ((candela * fade) / (d * d)) * cos;
}
function __light_meter(point: Vector3, normal: Vector3 | undefined, shadows: boolean): number {
  let sum = 0;
  const n = Light.count();
  for (let i = 0; i < n; i++) {
    const l = Light.at(i);
    let lux = __light_lux(l, point, normal);
    if (shadows && lux > 0) {
      const t = l.gameObject.transform;
      let dir = new Vector3(0, 1, 0);
      let distance = 10000;
      if (l.type === "directional") {
        const f = t.forward;
        dir = new Vector3(-f.x, -f.y, -f.z);
      } else {
        const p = t.position;
        dir = new Vector3(p.x - point.x, p.y - point.y, p.z - point.z);
        distance = dir.magnitude - 0.05;
      }
      // Start a little off the point, so the surface it is on does not block it.
      const len = dir.magnitude > 0 ? dir.magnitude : 1;
      const from = new Vector3(point.x + (dir.x / len) * 0.02, point.y + (dir.y / len) * 0.02, point.z + (dir.z / len) * 0.02);
      if (Physics.raycast(from, dir, distance) !== null) lux = 0;
    }
    sum += lux;
  }
  return sum;
}
function __xt_Light_illuminanceAt(point: Vector3, normal?: Vector3): number {
  return __light_meter(point, normal, false);
}
function __xt_Light_measureIlluminance(point: Vector3, normal?: Vector3): Promise<number> {
  // Resolves after the next physics step (the rays need the colliders as the world has them then).
  return new Promise<number>((resolve) => {
    Physics.afterNextStep(() => {
      resolve(__light_meter(point, normal, true));
    });
  });
}

//# when BloomSettings
interface BloomSettings {
  threshold: number;
  intensity: number;
  scatter: number;
}
function __xw_RenderSettings_bloom(b: BloomSettings | null): void {
  RenderSettings.bloomEnabled = false;
  if (b !== null) {
    RenderSettings.bloomEnabled = true;
    RenderSettings.bloomThreshold = b.threshold;
    RenderSettings.bloomIntensity = b.intensity;
    RenderSettings.bloomScatter = b.scatter;
  }
}

//# when screenToWorldPoint @ Camera Screen
function __xm_Camera_screenToWorldPoint(self: Camera, p: Vector3): Vector3 {
  // The camera's area of the screen in UI pixels, then the point across it in -1..1 (y up).
  const w = Math.max(1e-6, self.rectWidth * Screen.width);
  const h = Math.max(1e-6, self.rectHeight * Screen.height);
  const nx = ((p.x - self.rectX * Screen.width) / w) * 2 - 1;
  const ny = 1 - ((p.y - self.rectY * Screen.height) / h) * 2;
  const aspect = w / h;
  const t = self.gameObject.transform;
  const pos = t.position;
  const f = t.forward;
  const r = t.right;
  const u = t.up;
  const halfH = self.orthographic ? self.orthographicSize : Math.tan((self.fieldOfView * Math.PI) / 360) * p.z;
  const halfW = halfH * aspect;
  return new Vector3(
    pos.x + f.x * p.z + r.x * nx * halfW + u.x * ny * halfH,
    pos.y + f.y * p.z + r.y * nx * halfW + u.y * ny * halfH,
    pos.z + f.z * p.z + r.z * nx * halfW + u.z * ny * halfH,
  );
}

