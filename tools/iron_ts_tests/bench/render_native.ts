// The native rendering data API from compiled code: lights, meshes, materials, textures,
// sprites, mesh renderers, LOD groups and the camera extras. The Rust test
// (tests/all/render_native.rs) reads the registries the renderer would.

export function light(): string {
  const go = new GameObject("Sun");
  const l = go.addComponent(Light, {
    type: "spot",
    intensity: 2.5,
    color: new Color(1, 0.5, 0.25, 1),
    spotAngle: 40,
    shadows: "soft",
    cullingMask: 5,
  });
  l.range = 20;
  l.type = "bogus";
  return `${l.type}|${l.intensity}|${l.color.g}|${l.spotAngle}|${l.shadows}|${l.cullingMask}|${l.range}|${go.getComponent(Light) === l}`;
}

export function meshes(): string {
  const go = new GameObject("Box");
  const mat = new Material({ color: new Color(1, 0, 0, 1), metallic: 1, smoothness: 0.9, renderingMode: "cutout" });
  const r = go.addComponent(MeshRenderer, { mesh: "sphere", material: mat, shadowCastingMode: "off" });
  const tri = Mesh.create({
    vertices: [new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(0, 1, 0)],
    triangles: [0, 1, 2],
  });
  const r2 = new GameObject("Tri").addComponent(MeshRenderer, { mesh: tri });
  const plain = new GameObject("Plain").addComponent(MeshRenderer);
  r2.material = mat.copy();
  r2.material.metallic = 0.25;
  return `${r.mesh.primitive}|${r2.mesh.vertexCount}|${r.material.metallic}|${r.material.renderingMode}|${r.shadowCastingMode}|${plain.mesh.primitive}|${plain.material.smoothness}|${r2.material.metallic}`;
}

export function textures(): string {
  const a = Texture.load("img/a.png", { filterMode: "point", anisoLevel: 4 });
  const b = Texture.create(2, 1, [new Color(1, 0, 0, 1), new Color(0, 0, 1, 0.5)]);
  const rt = Texture.createRenderTexture(64, 32);
  const cam = new GameObject("Cam").addComponent(Camera, { targetTexture: rt, clearFlags: "skybox", cullingMask: 3 });
  cam.setRect(0.5, 0, 0.5, 1);
  cam.setLayerCullDistance(4, 30);
  const mat = Material.create({ mainTexture: a });
  return `${a.filterMode}|${a.anisoLevel}|${a.isLoaded}|${b.width}x${b.height}|${rt.isRenderTexture}|${rt.width}|${cam.targetTexture.width}|${cam.clearFlags}|${cam.cullingMask}|${cam.rectX}|${cam.layerCullDistance(4)}|${mat.mainTexture.src}`;
}

export function sprites(): string {
  const px: Color[] = [];
  for (let i = 0; i < 16; i++) px.push(new Color(1, 1, 1, 1));
  const t = Texture.create(4, 4, px);
  const s = Sprite.create(t, 50);
  s.pivot = new Vector2(0, 0);
  const cell = Sprite.gridCell(t, 2, 2, 3);
  const go = new GameObject("Sprite");
  const r = go.addComponent(SpriteRenderer, { sprite: s, flipX: true, sortingOrder: 3.7 });
  return `${s.size.x}|${cell.size.y}|${r.flipX}|${r.sortingOrder}|${r.sprite === s}`;
}

export function lods(): string {
  const go = new GameObject("Tree");
  const near = new GameObject("Near").addComponent(MeshRenderer);
  const far = new GameObject("Far").addComponent(MeshRenderer, { mesh: "cube" });
  const lod = go.addComponent(LODGroup, { size: 3 });
  lod.addLod(0.6);
  lod.addLod(0.2);
  lod.addLod(0.5);
  lod.addLodRenderer(0, near);
  lod.addLodRenderer(1, far);
  lod.forceLod(1);
  return `${lod.lodCount}|${lod.lodHeight(1)}|${lod.lodRendererCount(0)}|${lod.lodRendererAt(1, 0) === far}`;
}

export function settings(): string {
  RenderSettings.ambientLight = new Color(0.5, 0.5, 0.5, 1);
  RenderSettings.skyboxEnabled = true;
  RenderSettings.skySunSize = 3;
  RenderSettings.bloomIntensity = 2;
  RenderSettings.tonemapping = "aces";
  RenderSettings.tonemapping = "bogus";
  return `${RenderSettings.ambientLight.r}|${RenderSettings.skyboxEnabled}|${RenderSettings.skySunSize}|${RenderSettings.bloomIntensity}|${RenderSettings.bloomEnabled}|${RenderSettings.tonemapping}|${RenderSettings.skyHasSun}|${RenderSettings.ambientMode}`;
}

export function cursor(): string {
  Cursor.visible = false;
  Cursor.lockState = CursorLockMode.Locked;
  Cursor.setCursor("ui/hand.png", { hotspotX: 4, hotspotY: 5 });
  return `${Cursor.visible}|${Cursor.lockState}`;
}

export function behaviours(): void {
  const target = new GameObject("Target", { position: new Vector3(10, 0, 0) });
  const cam = new GameObject("Follower");
  cam.addComponent(CameraFollow, { target: target, offset: new Vector3(0, 2, 5), smoothTime: 0 });
  const orbiter = new GameObject("Orbiter");
  orbiter.addComponent(CameraOrbit, { center: new Vector3(0, 0, 0), distance: 4, yaw: 90, pitch: 0, autoRotate: 90 });
}

let stopped = 0;

export function stoppedCount(): number {
  return stopped;
}

export function particles(): string {
  const go = new GameObject("Fire");
  const ps = go.addComponent(ParticleSystem, {
    duration: 2,
    blendMode: "additive",
    simulationSpace: "world",
    shapeType: "sphere",
    rateOverTime: 40,
    playOnAwake: false,
    stopAction: "destroy",
  });
  ps.setStartLifetimeRange(1, 2);
  ps.startSpeed = 3;
  ps.setStartColorRange(new Color(1, 0, 0, 1), new Color(1, 1, 0, 1));
  ps.addBurst(0.5, 20);
  ps.addSizeKey(0, 1);
  ps.addSizeKey(1, 0);
  ps.addColorKey(1, new Color(1, 1, 1, 0));
  ps.setTextureSheet(4, 4);
  ps.onStopped(() => {
    stopped += 1;
  });
  const before = ps.isPlaying;
  ps.play();
  ps.emit(5);
  const playing = ps.isPlaying;
  ps.pause();
  return `${ps.startLifetime}|${ps.startLifetimeMax}|${ps.burstCount}|${before}|${playing}|${ps.isPlaying}|${ps.isPaused}|${ps.startColorMax.g}`;
}

export function lines(): string {
  const go = new GameObject("Rope");
  const lr = go.addComponent(LineRenderer, { width: 0.2, sides: 6, smoothing: 1 });
  lr.setPositions([new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(1, 1, 0)]);
  lr.redraw();
  return `${lr.positionCount}|${lr.positionAt(1).x}`;
}

export function cloth(): string {
  const go = new GameObject("Flag");
  go.addComponent(MeshRenderer, { mesh: "plane" });
  const c = go.addComponent(Cloth, { bendingStiffness: 0.9, useGravity: false });
  c.addPin(new Vector3(0, 0, 0));
  c.setPinBox(new Vector3(-1, 0, -1), new Vector3(1, 0.1, -0.9));
  c.rebuild();
  return `${c.bendingStiffness}|${c.useGravity}`;
}

export function primitives(): string {
  const cube = Primitives.create(PrimitiveType.Cube, { name: "Crate" });
  const s = Primitives.create("sphere");
  return `${cube.name}|${s.name}|${cube.getComponent(MeshRenderer).mesh.primitive}`;
}

export function gizmos(): number {
  Gizmos.clear();
  Gizmos.color = new Color(1, 0, 0, 1);
  Gizmos.drawWireCube(new Vector3(0, 0, 0), new Vector3(2, 2, 2));
  Gizmos.drawRay(new Vector3(0, 0, 0), new Vector3(0, 1, 0));
  Gizmos.matrix = Matrix4x4.trs(new Vector3(5, 0, 0), Quaternion.identity, new Vector3(1, 1, 1));
  Gizmos.drawWireSphere(new Vector3(0, 0, 0), 1);
  Gizmos.drawFrustum(new Vector3(0, 0, 0), 60, 10, 1, 1.5);
  return Gizmos.lineCount;
}

let screenChanges = 0;

export function screenChangeCount(): number {
  return screenChanges;
}

export function screenApi(): string {
  Screen.onChange(() => {
    screenChanges += 1;
  });
  Screen.fullScreenMode = "FullScreenWindow";
  Screen.setResolution(800, 600, { refreshRate: 60 });
  return `${Screen.width}|${Screen.fullScreen}|${Screen.fullScreenMode}|${Screen.aspect.toFixed(2)}|${Screen.resolutionCount}`;
}

let levelChanges = 0;

export function levelChangeCount(): number {
  return levelChanges;
}

export function quality(): string {
  QualitySettings.onLevelChanged((level: number) => {
    levelChanges += level + 1;
  });
  QualitySettings.setQualityLevelByName("Low");
  const low = QualitySettings.antiAliasing;
  QualitySettings.antiAliasing = 3;
  QualitySettings.shadows = "bogus";
  QualitySettings.setQualityLevel(3);
  return `${low}|${QualitySettings.antiAliasing}|${QualitySettings.getQualityLevel()}|${QualitySettings.levelName(1)}|${QualitySettings.levelCount}|${QualitySettings.shadows}`;
}

export function model(): string {
  const m = Model.load("models/ship.glb");
  const again = Model.load("models/ship.glb");
  const a = m.instantiate({ position: new Vector3(1, 2, 3), colliders: "convex" });
  const b = m.instantiate();
  return `${m.isLoaded}|${a.name}|${b.name}|${m === again}|${m.src}`;
}

export function others(): string {
  const go = new GameObject("Stuff");
  const mirror = go.addComponent(Mirror, { cullingMask: 6 });
  const probe = go.addComponent(ReflectionProbe, { resolution: 256, refreshMode: "everyFrame" });
  probe.renderProbe();
  probe.renderProbe();
  const plane = go.addComponent(PlaneRenderer, { size: new Vector2(4, 2) });
  const panel = go.addComponent(WorldPanel, { width: 400 });
  const stroke = go.addComponent(LineRenderer2D, { width: 0.2 });
  stroke.setPoints([new Vector2(0, 0), new Vector2(1, 1)]);
  return `${mirror.cullingMask}|${probe.resolution}|${probe.refreshMode}|${plane.size.x}|${panel.worldWidth.toFixed(1)}|${stroke.pointCount}|${stroke.pointAt(1).y}`;
}
