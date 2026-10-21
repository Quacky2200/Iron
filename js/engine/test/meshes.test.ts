import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import {
  BoxCollider,
  Color,
  GameObject,
  Light,
  Material,
  Mesh,
  MeshRenderer,
  Mirror,
  ReflectionProbe,
  PrimitiveType,
  QualitySettings,
  RenderSettings,
  SphereCollider,
  Texture,
  Vector2,
  Vector3,
} from "../src/index.ts";
import { _resetPhysicsComponentsForTests } from "../src/physics-components.ts";
import { _resetPhysicsForTests } from "../src/physics.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetPhysicsForTests();
  _resetPhysicsComponentsForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const renderOps = () => ops.filter((o) => String(o[0]).startsWith("render."));

// M5-MSH-02
describe("meshes and lights", () => {
  test("MeshRenderer sends its mesh and material; changes resend", () => {
    const go = new GameObject("rock");
    const r = go.addComponent(MeshRenderer, {
      mesh: "sphere",
      material: new Material(new Color(1, 0, 0, 1), Color.black, 0.8),
    });
    expect(renderOps()).toEqual([["render.meshRenderer", r.id, go.id, "sphere", [1, 0, 0, 1], [0, 0, 0], 0.8, false, null, [1, 1, 0, 0], true, 0, [null, 1, null, false, null, 1, null, null, [1, 1, 0, 0], null, "opaque", 0.5, false]]]);
    r.material = r.material.withColor(Color.white);
    expect(renderOps().at(-1)![4]).toEqual([1, 1, 1, 1]);
  });

  // M5-MAT-02
  test("materials send their maps, specular colour and rendering mode; with() keeps them", () => {
    const normal = Texture.load("n.png", { linear: true });
    const mask = Texture.load("m.png");
    const go = new GameObject("leaf");
    const m = Material.create({
      normalMap: normal,
      normalScale: 0.5,
      metallicGlossMap: mask,
      occlusionMap: mask,
      occlusionStrength: 0.7,
      emissionMap: mask,
      detailAlbedoMap: mask,
      detailAlbedoMapScale: new Vector2(4, 4),
      specularColor: new Color(0.2, 0.3, 0.4, 1),
      renderingMode: "cutout",
      alphaCutoff: 0.3,
      doubleSided: true,
    });
    const r = go.addComponent(MeshRenderer, { mesh: "quad", material: m });
    expect(renderOps().at(-1)!.at(-1)).toEqual([normal.id, 0.5, mask.id, false, mask.id, 0.7, mask.id, mask.id, [4, 4, 0, 0], [0.2, 0.3, 0.4], "cutout", 0.3, true]);
    // Changing one thing keeps the rest.
    const faded = m.with({ renderingMode: "fade", normalMap: null });
    expect(faded.renderingMode).toBe("fade");
    expect(faded.normalMap).toBeNull();
    expect(faded.occlusionMap).toBe(mask);
    expect(faded.detailAlbedoMapScale.x).toBe(4);
    expect(m.renderingMode).toBe("cutout");
    r.material = faded;
    expect((renderOps().at(-1)!.at(-1) as unknown[])[10]).toBe("fade");
  });

  test("flat meshes send their arrays as they are, UVs flipped", () => {
    const quad = Mesh.createFlat({ positions: [0, 0, 0, 1, 0, 0, 0, 1, 0], triangles: [0, 1, 2], uv: [0, 0, 1, 0, 0, 1] });
    expect(renderOps()[0]).toEqual(["render.mesh", quad.id, [0, 0, 0, 1, 0, 0, 0, 1, 0], [], new Float32Array([0, 1, 1, 1, 0, 0]), [0, 1, 2]]);
    // Saved with scenes as plain arrays, made when asked for.
    expect(quad._data).toEqual({ vertices: [0, 0, 0, 1, 0, 0, 0, 1, 0], triangles: [0, 1, 2], normals: [], uv: [0, 0, 1, 0, 0, 1] });
    // Typed arrays go as they are.
    const typed = new Float32Array([1, 2, 3]);
    quad.setFlat({ positions: typed, triangles: new Uint32Array(0) });
    expect(renderOps().at(-1)![2]).toBe(typed);
    quad.setFlat({ positions: [1, 1, 1], triangles: [] });
    expect(renderOps().at(-1)).toEqual(["render.mesh", quad.id, [1, 1, 1], [], new Float32Array(0), []]);
  });

  test("script meshes send flat arrays", () => {
    const tri = Mesh.create({ vertices: [Vector3.zero, Vector3.right, Vector3.up], triangles: [0, 1, 2] });
    expect(renderOps()[0]).toEqual(["render.mesh", tri.id, [0, 0, 0, 1, 0, 0, 0, 1, 0], [], [], [0, 1, 2]]);
    const go = new GameObject("tri");
    go.addComponent(MeshRenderer, { mesh: tri });
    expect(renderOps().at(-1)![3]).toBe(tri.id);
    tri.destroy();
    expect(renderOps().at(-1)).toEqual(["render.mesh", tri.id, null]);
  });

  test("lights and ambient", () => {
    const sun = new GameObject("sun");
    const l = sun.addComponent(Light, { type: "spot", intensity: 2, range: 15, spotAngle: 40 });
    expect(renderOps().at(-1)).toEqual(["render.light", l.id, sun.id, "spot", [1, 1, 1], 2, 15, 40, 21.8, 0, 0, "none", 1, 0xffffffff, 1]);
    RenderSettings.ambientLight = new Color(0.1, 0.1, 0.1);
    expect(renderOps().at(-1)).toEqual(["render.ambient", 0, [0.1, 0.1, 0.1]]);
    l.haloSize = 1;
    l.beamIntensity = 0.5;
    expect(renderOps().at(-1)!.slice(9, 11)).toEqual([1, 0.5]);
    l.shadows = "soft";
    l.shadowStrength = 0.7;
    expect(renderOps().at(-1)!.slice(11, 13)).toEqual(["soft", 0.7]);
    QualitySettings.shadowDistance = 40;
    expect(renderOps().at(-1)).toEqual(["render.shadowSettings", 0, 40, 2048, 4]);
    // Cascades round to 1, 2 or 4 (M5-SHD-02).
    QualitySettings.shadowCascades = 3;
    expect(renderOps().at(-1)).toEqual(["render.shadowSettings", 0, 40, 2048, 2]);
    RenderSettings.bloom = { threshold: 1, intensity: 0.8, scatter: 0.6 };
    expect(renderOps().at(-1)).toEqual(["render.bloom", 0, 1, 0.8, 0.6]);
    RenderSettings.bloom = null;
    expect(renderOps().at(-1)).toEqual(["render.bloom", 0, null]);
    // Exposure and tonemapping (M5-RND-18).
    RenderSettings.tonemapping = "aces";
    RenderSettings.postExposure = 1.5;
    expect(renderOps().at(-1)).toEqual(["render.post", 0, 1.5, "aces", null]);
    // Auto exposure (M7-EXP-01).
    RenderSettings.autoExposure = { minEV: -2, speedLightToDark: 0.5 };
    expect(renderOps().at(-1)).toEqual(["render.post", 0, 1.5, "aces", [-2, 20, 0, 3, 0.5]]);
    RenderSettings.autoExposure = null;
    RenderSettings.skybox = { zenith: new Color(0, 0, 1, 1), sunDirection: null };
    expect(renderOps().at(-1)).toEqual(["render.sky", 0, [0, 0, 1], [0.62, 0.72, 0.82], [0.22, 0.2, 0.18], null, [20, 18, 15], 1.5, 1, true]);
    RenderSettings.ambientMode = "color";
    expect(renderOps().at(-1)!.at(-1)).toBe(false);
    RenderSettings.skybox = null;
    expect(renderOps().at(-1)).toEqual(["render.sky", 0, null]);
    // Mirrors: quads reflect along +z, planes along +y.
    const glass = new GameObject("glass");
    glass.addComponent(MeshRenderer, { mesh: "quad" });
    const mirror = glass.addComponent(Mirror);
    expect(renderOps().at(-1)).toEqual(["render.mirror", mirror.id, glass.id, [0, 0, 1], 0xffffffff]);
    const room = new GameObject("room");
    const probe = room.addComponent(ReflectionProbe, { size: new Vector3(4, 3, 6), boxProjection: true });
    expect(renderOps().at(-1)).toEqual(["render.probe", probe.id, room.id, [4, 3, 6], 128, true, false, 0xffffffff]);
    // Culling masks go out unsigned, whatever was set (M5-LAY-04).
    const only = room.addComponent(ReflectionProbe, { cullingMask: ~(1 << 5) });
    expect(renderOps().at(-1)!.at(-1)).toBe(0xffffffdf);
    expect(only.cullingMask).toBe(~(1 << 5));
    probe.renderProbe();
    expect(renderOps().at(-1)).toEqual(["render.probeRender", probe.id]);
  });

  test("createPrimitive adds a mesh and the matching collider", () => {
    const cube = GameObject.createPrimitive(PrimitiveType.Cube, { position: new Vector3(0, 5, 0) });
    expect(cube.name).toBe("Cube");
    expect(cube.getComponent(MeshRenderer)?.mesh).toBe("cube");
    expect(cube.getComponent(BoxCollider)).toBeDefined();
    const ball = GameObject.createPrimitive("sphere", { name: "Ball" });
    expect(ball.getComponent(SphereCollider)?.radius).toBe(0.5);
    expect(ball.name).toBe("Ball");
  });
});
