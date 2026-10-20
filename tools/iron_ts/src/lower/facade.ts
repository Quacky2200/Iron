// The TypeScript `@iron/engine` members the native engine API has in another shape, written over
// the native functions. A chunk is appended to a program that names one of its triggers (the text
// after `when`, separated by `|`). The naming scheme is in `facade.rs`.

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

//# when Screen.supportedFullScreenModes
function __xp_Screen_supportedFullScreenModes(): string[] {
  const out: string[] = [];
  const n = Screen.supportedFullScreenModeCount;
  for (let i = 0; i < n; i++) out.push(Screen.supportedFullScreenModeAt(i));
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
  return new Promise<Scene>((resolve) => {
    let token = 0;
    token = SceneManager.onSceneLoaded((scene: Scene, m: string) => {
      if (scene.path === path) {
        SceneManager.offSceneEvent(token);
        resolve(scene);
      }
    });
    SceneManager.loadSceneAsync$n(path, mode ?? "single");
  });
}

//# when whenLoaded
function __poll_texture(self: Texture, resolve: (v: Texture) => void): void {
  if (self.isLoaded || self.failed) resolve(self);
  else setTimeout(() => { __poll_texture(self, resolve); }, 0);
}
function __poll_model(self: Model, resolve: (v: Model) => void): void {
  if (self.isLoaded || self.failed) resolve(self);
  else setTimeout(() => { __poll_model(self, resolve); }, 0);
}
function __xm_Texture_whenLoaded(self: Texture): Promise<Texture> {
  return new Promise<Texture>((resolve) => { __poll_texture(self, resolve); });
}
function __xm_Model_whenLoaded(self: Model): Promise<Model> {
  return new Promise<Model>((resolve) => { __poll_model(self, resolve); });
}
