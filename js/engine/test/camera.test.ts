// SPDX-License-Identifier: Apache-2.0

import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Camera, CameraFollow, CameraOrbit, GameObject, Light, Matrix4x4, QualitySettings, Quaternion, Rect, RenderSettings, RenderTexture, Vector3 } from "../src/index.ts";
import { _resetCamerasForTests } from "../src/rendering.ts";
import { _resetSceneForTests } from "../src/scene.ts";
import { _setScreenForTests } from "../src/screen.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetCamerasForTests();
  _setScreenForTests(800, 600);
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const near = (a: Vector3, b: Vector3, eps = 1e-4) => a.distance(b) < eps;

function camera(position: Vector3, props: Partial<Camera> = {}): Camera {
  const go = new GameObject("cam", { position });
  const cam = go.addComponent(Camera, props);
  cam.onStart();
  return cam;
}

// M5-CAM-01
describe("Matrix4x4", () => {
  test("TRS, inverse and points", () => {
    const m = Matrix4x4.trs(new Vector3(1, 2, 3), Quaternion.euler(0, 90, 0), new Vector3(2, 2, 2));
    const p = new Vector3(0, 0, -1);
    // Scale 2, turn left 90°, then move.
    expect(near(m.multiplyPoint3x4(p), new Vector3(-1, 2, 3))).toBe(true);
    expect(near(m.inverse.multiplyPoint3x4(m.multiplyPoint3x4(p)), p)).toBe(true);
    const id = m.mul(m.inverse);
    for (let i = 0; i < 16; i++) expect(id.m[i]).toBeCloseTo(Matrix4x4.identity.m[i]!, 6);
    expect(m.determinant).toBeCloseTo(8, 5);
    expect(m.transpose.get(0, 3)).toBeCloseTo(m.get(3, 0));
  });
});

// M5-CAM-01
describe("Camera conversions", () => {
  test("perspective: rays through the screen and back", () => {
    const cam = camera(new Vector3(0, 0, 10), { fieldOfView: 90 });
    const center = cam.screenPointToRay({ x: 400, y: 300 });
    expect(near(center.direction, Vector3.forward)).toBe(true);
    expect(near(cam.worldToScreenPoint(Vector3.zero), new Vector3(400, 300, 10))).toBe(true);
    // 90° vertical fov: 10 m away the view is 20 m tall, so y = 10 is the top edge.
    expect(near(cam.worldToScreenPoint(new Vector3(0, 10, 0)), new Vector3(400, 0, 10))).toBe(true);
    const p = cam.screenToWorldPoint(new Vector3(100, 450, 5));
    expect(near(cam.worldToScreenPoint(p), new Vector3(100, 450, 5))).toBe(true);
    // Viewport points are 0 to 1 from the top-left.
    expect(near(cam.worldToViewportPoint(new Vector3(0, 10, 0)), new Vector3(0.5, 0, 10))).toBe(true);
  });

  test("orthographic and a camera rect", () => {
    const cam = camera(new Vector3(0, 0, 10), { orthographic: true, orthographicSize: 5, rect: new Rect(0.5, 0, 0.5, 1) });
    // The right half of an 800×600 screen: 400×600, so 5 m up is the top and the
    // view is 5 × (400/600) m to each side.
    expect(cam.pixelRect).toEqual(new Rect(400, 0, 400, 600));
    expect(near(cam.worldToScreenPoint(new Vector3(0, 5, 0)), new Vector3(600, 0, 10))).toBe(true);
    const halfWidth = 5 * (400 / 600);
    expect(near(cam.worldToScreenPoint(new Vector3(halfWidth, 0, 0)), new Vector3(800, 300, 10))).toBe(true);
    // Orthographic rays are parallel.
    const a = cam.screenPointToRay({ x: 450, y: 50 });
    const b = cam.screenPointToRay({ x: 750, y: 550 });
    expect(near(a.direction, b.direction)).toBe(true);
    expect(ops.at(-1)).toEqual(["render.camera", cam.id, cam.gameObject.id, 60, 0.1, 1000, [0.05, 0.06, 0.09, 1], 0, true, 5, [0.5, 0, 0.5, 1], "color", 0xffffffff, null, false, "none", null, false, true]);
    // Post anti-aliasing (M5-RND-18).
    cam.antialiasing = "fxaa";
    expect(ops.at(-1)![15]).toBe("fxaa");
  });

  // M5-RTX-01
  test("render textures: cameras draw into them, and they leave Camera.main to the screen's", () => {
    const screen = camera(new Vector3(0, 0, 10));
    const rt = new RenderTexture(256.4, 128);
    expect(ops.at(-1)).toEqual(["render.renderTexture", rt.id, 256, 128]);
    expect(rt.isLoaded).toBe(true);
    expect([rt.width, rt.height]).toEqual([256, 128]);
    const security = camera(new Vector3(5, 5, 5));
    security.targetTexture = rt;
    expect(ops.at(-1)![0]).toBe("render.camera");
    expect(ops.at(-1)![16]).toBe(rt.id);
    expect(Camera.main).toBe(screen);
    rt.setSize(64, 64);
    expect(ops.at(-1)).toEqual(["render.renderTexture", rt.id, 64, 64]);
    security.targetTexture = null;
    expect(ops.at(-1)![16]).toBeNull();
  });

  test("main camera and draw order", () => {
    const a = camera(Vector3.zero, { depth: 5 });
    const b = camera(Vector3.zero, { depth: -1 });
    expect(Camera.main).toBe(a);
    expect(Camera.allCameras).toEqual([b, a]);
    a.gameObject.enabled = false;
    expect(Camera.main).toBe(b);
  });

  test("follow and orbit helpers", () => {
    const target = new GameObject("target", { position: new Vector3(10, 0, 0) });
    const cam = new GameObject("cam");
    const follow = cam.addComponent(CameraFollow, { target, offset: new Vector3(0, 2, 5), smoothTime: 0 });
    follow.onLateUpdate(1 / 60);
    expect(near(cam.transform.position, new Vector3(10, 2, 5))).toBe(true);
    expect(near(cam.transform.forward, new Vector3(0, -2, -5).normalized)).toBe(true);

    const orbiter = new GameObject("orbiter");
    const orbit = orbiter.addComponent(CameraOrbit, { target, distance: 4, pitch: 0, yaw: 90 });
    orbit.onLateUpdate(0);
    // The camera starts behind the target (+z); yaw 90° turns that offset to +x.
    expect(near(orbiter.transform.position, new Vector3(14, 0, 0))).toBe(true);
    expect(near(orbiter.transform.forward, new Vector3(-1, 0, 0))).toBe(true);
  });
});

// M7-EXP-05
describe("exposure", () => {
  test("cameras learn their exposure from the engine and can reset it", () => {
    const cam = camera(Vector3.zero);
    expect(Number.isNaN(cam.exposure)).toBe(true);
    globalThis.__iron_exposure(cam.id, 3.5, 2.25);
    expect(cam.exposure).toBe(3.5);
    expect(cam.sceneBrightness).toBe(2.25);
    globalThis.__iron_exposure(cam.id, 1, null);
    expect(Number.isNaN(cam.sceneBrightness)).toBe(true);
    cam.resetExposure();
    expect(ops.at(-1)).toEqual(["render.exposureReset", cam.id]);
  });

  test("freezing keeps the main camera's exposure as the manual one", () => {
    const cam = camera(Vector3.zero);
    RenderSettings.autoExposure = {};
    globalThis.__iron_exposure(cam.id, 4, 4);
    RenderSettings.freezeExposure();
    const post = ops.filter((o) => o[0] === "render.post").at(-1)!;
    expect(post[4]).toBe(null);
    expect(post[5]).toBe(4);
    RenderSettings.fixedExposure = null;
  });

  test("the player's quality option turns auto exposure off and on", () => {
    RenderSettings.autoExposure = { minEV: 1 };
    QualitySettings.autoExposure = false;
    expect(ops.filter((o) => o[0] === "render.post").at(-1)![4]).toBe(null);
    QualitySettings.autoExposure = true;
    expect((ops.filter((o) => o[0] === "render.post").at(-1)![4] as number[])[0]).toBe(1);
    RenderSettings.autoExposure = null;
  });
});

// M7-HD-02
describe("real-world light", () => {
  test("a physical camera's exposure and a light meter", () => {
    const cam = camera(Vector3.zero);
    // HDRP's defaults: f/16, 1/200 s, ISO 200.
    expect(cam.physicalExposure).toBeCloseTo(Math.log2((16 * 16) / (1 / 200)) - 1, 6);
    cam.aperture = 2.8;
    cam.shutterSpeed = 1 / 60;
    cam.iso = 800;
    expect(cam.physicalExposure).toBeCloseTo(Math.log2(2.8 * 2.8 * 60) - 3, 6);
    // A sun of 100000 lux straight down, and an 800 lumen bulb 2 m above the point.
    const sun = new GameObject("sun", { rotation: Quaternion.lookRotation(new Vector3(0, -1, 0)) });
    sun.addComponent(Light, { type: "directional", intensity: 100000 });
    const bulb = new GameObject("bulb", { position: new Vector3(0, 2, 0) });
    bulb.addComponent(Light, { type: "point", intensity: 800, range: 1000 });
    const up = new Vector3(0, 1, 0);
    const lux = Light.illuminanceAt(Vector3.zero, up);
    expect(lux).toBeCloseTo(100000 + 800 / (4 * Math.PI) / 4, 0);
    // Facing away, neither lights it.
    expect(Light.illuminanceAt(Vector3.zero, new Vector3(0, -1, 0))).toBe(0);
  });
});
