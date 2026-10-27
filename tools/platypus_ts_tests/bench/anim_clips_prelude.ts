// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The original animation module over a scene with no engine behind it, plus the native API's names
// where they differ (see docs/research/platypus-native-animation.md): text JSON, event arguments instead
// of an object, a transform read back at f32 precision (the native scene stores floats), and
// `Transform.setParent`.
import "./hypot_patch.ts";
import { AnimationClip, AnimationCurve, Keyframe } from "../../../js/engine/src/animation.ts";
import { Quaternion, Vector3 } from "../../../js/engine/src/math.ts";
import { GameObject, Transform, _resetSceneForTests } from "../../../js/engine/src/scene.ts";

(globalThis as any).__platypus_op = () => {};
_resetSceneForTests();

const fr = Math.fround;
const proto = Transform.prototype as any;
for (const [name, make] of [
  ["localPosition", (v: any) => new Vector3(fr(v.x), fr(v.y), fr(v.z))],
  ["localScale", (v: any) => new Vector3(fr(v.x), fr(v.y), fr(v.z))],
  ["localRotation", (q: any) => new Quaternion(fr(q.x), fr(q.y), fr(q.z), fr(q.w))],
] as const) {
  const d = Object.getOwnPropertyDescriptor(proto, name)!;
  Object.defineProperty(proto, name, { ...d, get() { return make(d.get!.call(this)); } });
}
proto.setParent = function (p: Transform | null, keepWorld: boolean) {
  this.gameObject.setParent(p ? p.gameObject : null, keepWorld);
};

const c = AnimationCurve.prototype as any;
c.key = function (i: number) { return this.keys[i] ?? new Keyframe(0, 0); };
c.addKeyframe = function (k: Keyframe) { return this.addKey(k); };

const clip = AnimationClip.prototype as any;
const addEvent = clip.addEvent;
clip.addEvent = function (time: number | object, functionName?: string, parameter?: number) {
  return addEvent.call(this, typeof time === "object" ? time : { time, functionName, parameter });
};
clip.removeCurve = function (path: string, type: string, property: string) { this.setCurve(path, type, property, null); };
Object.defineProperty(clip, "curveCount", { get() { return this._bindings.length; } });
Object.defineProperty(clip, "eventCount", { get() { return this.events.length; } });
clip.curveAt = function (i: number) { const b = this._bindings[i]; return b ? `${b.path}|${b.type}|${b.property}` : ""; };
clip.curve = function (path: string, type: string, property: string) {
  return this._bindings.find((b: any) => b.path === path && b.type === type && b.property === property)?.curve;
};
clip.toJson = function () { return JSON.stringify(this.toJSON()); };
(AnimationClip as any).fromJson = (text: string) => AnimationClip.fromJSON(JSON.parse(text));
void GameObject;

(globalThis as any).__bench_reset = () => {
  _resetSceneForTests();
};
