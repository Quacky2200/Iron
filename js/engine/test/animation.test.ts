import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import {
  Animation,
  AnimationClip,
  AnimationCurve,
  Animator,
  AnimatorController,
  Color,
  Component,
  GameObject,
  Keyframe,
  Light,
  Quaternion,
  SceneManager,
} from "../src/index.ts";
import { _resetSceneForTests } from "../src/scene.ts";

let ops: unknown[][] = [];

beforeEach(() => {
  _resetSceneForTests();
  ops = [];
  (globalThis as { __iron_op?: unknown }).__iron_op = (...args: unknown[]) => ops.push(args);
});

afterEach(() => {
  delete (globalThis as { __iron_op?: unknown }).__iron_op;
});

const near = (a: number, b: number, e = 1e-4) => Math.abs(a - b) < e;

// M5-ANI-01
describe("AnimationCurve", () => {
  test("Hermite keys, steps, wrap modes and saving", () => {
    const line = AnimationCurve.linear(0, 0, 2, 10);
    expect(line.evaluate(1)).toBeCloseTo(5);
    expect(line.evaluate(-1)).toBe(0);
    expect(line.evaluate(5)).toBe(10);
    // Flat tangents ease in and out: halfway is halfway, a quarter in is less than a quarter.
    const ease = AnimationCurve.easeInOut(0, 0, 1, 1);
    expect(ease.evaluate(0.5)).toBeCloseTo(0.5);
    expect(ease.evaluate(0.25)).toBeLessThan(0.25);
    // Steps hold.
    const step = AnimationCurve.fromPoints([[0, 1], [1, 5], [2, 3]], "step");
    expect(step.evaluate(0.99)).toBe(1);
    expect(step.evaluate(1.5)).toBe(5);
    // Wrapping past the end.
    const loop = AnimationCurve.linear(0, 0, 1, 1);
    loop.postWrapMode = "loop";
    expect(loop.evaluate(2.25)).toBeCloseTo(0.25);
    loop.postWrapMode = "pingPong";
    expect(loop.evaluate(1.25)).toBeCloseTo(0.75);
    // Smooth keys pass through their points.
    const smooth = AnimationCurve.fromPoints([[0, 0], [1, 2], [2, 0]], "smooth");
    expect(smooth.evaluate(1)).toBeCloseTo(2);
    expect(smooth.keys[1]!.inTangent).toBe(0);
    // Keys stay sorted; saving keeps steps (Infinity).
    const c = new AnimationCurve(new Keyframe(1, 1));
    expect(c.addKey(0, 5)).toBe(0);
    const back = AnimationCurve._fromJSON(JSON.parse(JSON.stringify(step._toJSON())));
    expect(back.evaluate(1.5)).toBe(5);
    expect(back.keys[0]!.outTangent).toBe(Infinity);
  });
});

class Flasher extends Component {
  hits: unknown[] = [];
  flash(p: unknown) {
    this.hits.push(p);
  }
}

// M5-ANI-02
describe("AnimationClip", () => {
  test("curves pose transforms and component properties by path; events and saving", () => {
    const root = new GameObject("Root");
    const arm = new GameObject("Arm", { parent: root });
    const hand = new GameObject("Hand", { parent: arm });
    const light = hand.addComponent(Light, { intensity: 1, color: Color.white });
    const clip = new AnimationClip("wave");
    clip.setCurve("Arm/Hand", "Transform", "localPosition.y", AnimationCurve.linear(0, 0, 1, 2));
    clip.setCurve("Arm", "Transform", "localEulerAngles.z", AnimationCurve.linear(0, 0, 1, 90));
    clip.setCurve("Arm/Hand", Light, "intensity", AnimationCurve.linear(0, 1, 1, 3));
    clip.setCurve("Arm/Hand", "Light", "color.g", AnimationCurve.linear(0, 1, 1, 0));
    clip.setCurve("Nowhere", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, 1, 1));
    expect(clip.length).toBe(1);
    clip.sampleAnimation(root, 0.5);
    expect(hand.transform.localPosition.y).toBeCloseTo(1);
    expect(arm.transform.localEulerAngles.z).toBeCloseTo(45);
    expect(light.intensity).toBeCloseTo(2);
    expect(light.color.g).toBeCloseTo(0.5);
    expect(light.color).toBeInstanceOf(Color);
    // One op per animated transform.
    globalThis.__iron_flush();
    const sent = ops.filter((o) => o[0] === "scene.locals").flatMap((o) => o[1] as number[]);
    expect(sent.filter((v, i) => i % 11 === 0 && v === hand.id).length).toBe(1);
    // Replacing a curve, and saving and loading.
    clip.setCurve("Arm/Hand", Light, "intensity", null);
    expect(clip.curves.length).toBe(4);
    const back = AnimationClip.fromJSON(JSON.parse(JSON.stringify(clip.toJSON())));
    expect(back.curves.map((c) => c.property)).toEqual(clip.curves.map((c) => c.property));
  });

  test("events call methods on the animated object's components as clips pass them", () => {
    const go = new GameObject("Flash");
    const f = go.addComponent(Flasher);
    const clip = new AnimationClip("blink");
    clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, 1, 1));
    clip.addEvent({ time: 0.5, functionName: "flash", parameter: "half" });
    clip.wrapMode = "loop";
    const anim = go.addComponent(Animation, { clip });
    anim.play();
    globalThis.__iron_animate(0.4);
    expect(f.hits).toEqual([]);
    globalThis.__iron_animate(0.2);
    expect(f.hits).toEqual(["half"]);
    // Looping: once more each pass.
    globalThis.__iron_animate(1.0);
    expect(f.hits).toEqual(["half", "half"]);
  });
});

// M5-ANI-03
describe("Animation", () => {
  test("play, crossFade by weight, and once-clips hold their end", () => {
    const go = new GameObject("Mover");
    const left = new AnimationClip("left");
    left.setCurve("", "Transform", "localPosition.x", AnimationCurve.constant(0, 1, -1));
    left.wrapMode = "loop";
    const right = new AnimationClip("right");
    right.setCurve("", "Transform", "localPosition.x", AnimationCurve.constant(0, 1, 1));
    right.wrapMode = "loop";
    const anim = go.addComponent(Animation, { clips: [left, right] });
    expect(anim.play("nope")).toBe(false);
    anim.play("left");
    globalThis.__iron_animate(0.1);
    expect(go.transform.localPosition.x).toBeCloseTo(-1);
    anim.crossFade("right", 1);
    globalThis.__iron_animate(0.5);
    // Halfway: equal weights.
    expect(go.transform.localPosition.x).toBeCloseTo(0);
    globalThis.__iron_animate(0.6);
    expect(go.transform.localPosition.x).toBeCloseTo(1);
    expect(anim.isPlaying("left")).toBe(false);
    // A clip that plays once holds its last frame and stops.
    const once = new AnimationClip("rise");
    once.setCurve("", "Transform", "localPosition.y", AnimationCurve.linear(0, 0, 1, 4));
    anim.addClip(once);
    anim.play("rise");
    globalThis.__iron_animate(2);
    expect(go.transform.localPosition.y).toBeCloseTo(4);
    expect(anim.isPlaying()).toBe(false);
    // Inactive objects don't animate.
    go.enabled = false;
    anim.play("left");
    globalThis.__iron_animate(0.1);
    expect(go.transform.localPosition.x).toBeCloseTo(1);
  });
});

/** A loop holding x (and a rotation about y) still. */
function pose(name: string, x: number, yaw = 0): AnimationClip {
  const clip = new AnimationClip(name);
  clip.wrapMode = "loop";
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.constant(0, 1, x));
  const q = Quaternion.euler(0, yaw, 0);
  for (const a of ["x", "y", "z", "w"] as const) {
    clip.setCurve("", "Transform", `localRotation.${a}`, AnimationCurve.constant(0, 1, q[a]));
  }
  return clip;
}

// M5-ANI-04
describe("Animator", () => {
  test("blend trees, conditions, triggers, exit time and Any State", () => {
    const go = new GameObject("Hero");
    const controller = new AnimatorController()
      .addParameter("speed", "float", 0)
      .addParameter("jump", "trigger")
      .addParameter("dead", "bool", false)
      .addState("Move", { blendParameter: "speed", children: [{ motion: pose("idle", 0), threshold: 0 }, { motion: pose("run", 10), threshold: 4 }] })
      .addState("Jump", pose("jump", 100))
      .addState("Land", pose("land", 50))
      .addState("Dead", pose("dead", -5))
      .addTransition({ from: "Move", to: "Jump", conditions: [{ parameter: "jump", mode: "if" }], duration: 0 })
      .addTransition({ from: "Jump", to: "Land", hasExitTime: true, exitTime: 1, duration: 0 })
      .addTransition({ from: "Any", to: "Dead", conditions: [{ parameter: "dead", mode: "if" }], duration: 0.5 });
    const animator = go.addComponent(Animator, { controller });
    // The blend tree mixes the two clips around the parameter.
    animator.setFloat("speed", 1);
    globalThis.__iron_animate(0.1);
    expect(animator.getCurrentAnimatorStateInfo().isName("Move")).toBe(true);
    expect(go.transform.localPosition.x).toBeCloseTo(2.5);
    animator.setFloat("speed", 9);
    globalThis.__iron_animate(0.1);
    expect(go.transform.localPosition.x).toBeCloseTo(10);
    // A trigger starts a transition and is used up.
    animator.setTrigger("jump");
    globalThis.__iron_animate(0.1);
    expect(animator.getCurrentAnimatorStateInfo().name).toBe("Jump");
    expect(go.transform.localPosition.x).toBeCloseTo(100);
    // Exit time: after the jump's one second.
    globalThis.__iron_animate(0.5);
    expect(animator.getCurrentAnimatorStateInfo().name).toBe("Jump");
    globalThis.__iron_animate(0.6);
    globalThis.__iron_animate(0.01);
    expect(animator.getCurrentAnimatorStateInfo().name).toBe("Land");
    // From any state, cross-fading over half a second.
    animator.setBool("dead", true);
    globalThis.__iron_animate(0.25);
    expect(animator.isInTransition).toBe(true);
    expect(near(go.transform.localPosition.x, (50 - 5) / 2, 0.5)).toBe(true);
    globalThis.__iron_animate(0.3);
    expect(animator.getCurrentAnimatorStateInfo().name).toBe("Dead");
    expect(go.transform.localPosition.x).toBeCloseTo(-5);
    // Any State doesn't re-enter the state it's in.
    globalThis.__iron_animate(0.1);
    expect(animator.isInTransition).toBe(false);
  });

  // M5-ANI-06
  test("a blend tree keeps its phase when its weights change (clips of different lengths)", () => {
    // Two cycles whose x runs 0 to 1 through each: a 1 s walk and a 2 s run. x is the phase.
    const cycle = (name: string, length: number) => {
      const clip = new AnimationClip(name);
      clip.wrapMode = "loop";
      clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, length, 1));
      return clip;
    };
    const go = new GameObject("Runner");
    const controller = new AnimatorController()
      .addParameter("speed", "float", 0)
      .addState("Move", { blendParameter: "speed", children: [{ motion: cycle("walk", 1), threshold: 0 }, { motion: cycle("run", 2), threshold: 4 }] });
    const animator = go.addComponent(Animator, { controller });
    globalThis.__iron_animate(0.25);
    expect(go.transform.localPosition.x).toBeCloseTo(0.25);
    // Breaking into a run: the stride carries on from the same point in the cycle (it jumped
    // back to 0.125 when the state kept seconds: 0.25 s of the run's 2).
    animator.setFloat("speed", 4);
    globalThis.__iron_animate(0.001);
    expect(go.transform.localPosition.x).toBeCloseTo(0.25, 2);
    // Then at the run's pace: half a second is a quarter of its cycle.
    globalThis.__iron_animate(0.5);
    expect(go.transform.localPosition.x).toBeCloseTo(0.5, 2);
  });

  test("cross-fades blend rotations the short way; play and crossFade jump; saving", () => {
    const go = new GameObject("Turner");
    // 170 and -170 degrees are 20 apart through 180, not 340 through 0.
    const controller = new AnimatorController().addState("A", pose("a", 0, 170)).addState("B", pose("b", 0, -170));
    const animator = go.addComponent(Animator, { controller });
    globalThis.__iron_animate(0.01);
    animator.crossFade("B", 1);
    globalThis.__iron_animate(0.5);
    // Halfway is 180 degrees about y (through 0 would be the identity).
    expect(Math.abs(go.transform.localRotation.y)).toBeCloseTo(1);
    animator.play("A");
    globalThis.__iron_animate(0.01);
    expect(go.transform.localEulerAngles.y).toBeCloseTo(170, 0);
    // Controllers save as data, clips inline, and in scenes.
    const back = AnimatorController.fromJSON(JSON.parse(JSON.stringify(controller.toJSON())));
    expect(back.states.map((s) => s.name)).toEqual(["A", "B"]);
    const data = SceneManager.getActiveScene().serialize();
    SceneManager.loadScene(JSON.parse(JSON.stringify(data)));
    const loaded = GameObject.find("Turner")!.getComponent(Animator)!;
    expect(loaded.controller!.states[1]!.name).toBe("B");
  });
});
