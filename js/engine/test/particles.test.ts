import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { Color, GameObject, ParticleSystem } from "../src/index.ts";
import { _resetParticlesForTests } from "../src/particles.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  _resetParticlesForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const particleOps = () => ops.filter((o) => String(o[0]).startsWith("render.particle"));
const setting = (op: unknown[], key: string) => {
  const pairs = op[3] as unknown[];
  return pairs[pairs.indexOf(key) + 1];
};

// M5-PRT-02
describe("ParticleSystem", () => {
  test("sends its settings on awake, and again only when they change", () => {
    const go = new GameObject("sparks");
    const ps = go.addComponent(ParticleSystem, {
      startLifetime: [0.5, 1],
      startColor: [Color.white, new Color(1, 0.5, 0, 1)],
      shape: { type: "sphere", radius: 0.2 },
      blendMode: "additive",
      bursts: [{ time: 0, count: 30 }],
      sizeOverLifetime: [
        { time: 0, value: 1 },
        { time: 1, value: 0 },
      ],
    });
    const [first] = particleOps();
    expect(first!.slice(0, 3)).toEqual(["render.particles", ps.id, go.id]);
    expect(first![4]).toBe(true);
    expect(setting(first!, "lifetime")).toEqual([0.5, 1]);
    expect(setting(first!, "color")).toEqual([1, 1, 1, 1, 1, 0.5, 0, 1]);
    expect(setting(first!, "shape")).toEqual(["sphere", 0.2]);
    expect(setting(first!, "additive")).toBe(true);
    expect(setting(first!, "bursts")).toEqual([0, 30, 1, 0.01]);
    expect(setting(first!, "sizeOverLifetime")).toEqual([0, 1, 1, 0]);
    ps.onPreRender();
    expect(particleOps().length).toBe(1);
    (ps.shape as { radius: number }).radius = 0.5;
    ps.onPreRender();
    expect(setting(particleOps().at(-1)!, "shape")).toEqual(["sphere", 0.5]);
  });

  test("controls become ops, and reports update the count and fire onStopped", () => {
    const go = new GameObject("boom");
    let stopped = 0;
    const ps = go.addComponent(ParticleSystem, { loop: false, stopAction: "destroy", onStopped: () => stopped++ });
    ops = [];
    ps.emit(12);
    ps.stop(true);
    ps.pause();
    expect(particleOps()).toEqual([
      ["render.particleControl", ps.id, "emit", 12],
      ["render.particleControl", ps.id, "stopClear"],
      ["render.particleControl", ps.id, "pause"],
    ]);
    expect(ps.isPlaying).toBe(false);
    ps.play();
    globalThis.__iron_particles([ps.id, 40, 1]);
    expect([ps.particleCount, ps.isPlaying]).toEqual([40, true]);
    globalThis.__iron_particles([ps.id, 0, 0]);
    expect(stopped).toBe(1);
    // stopAction "destroy" removed the object.
    expect(ops.some((o) => o[0] === "scene.destroy" && o[1] === go.id)).toBe(true);
  });
});
