// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The original Animator and AnimatorController under the native API's class names: the pieces are
// classes (and a builder) instead of object literals, which the original reads as plain objects.
import "./anim_player_prelude.ts";
import { Animator, AnimatorController, type AnimationClip } from "../../../js/engine/src/animation.ts";

class AnimationParameter {
  name: string;
  type: string;
  defaultValue?: number | boolean;
  constructor(name: string, type: string, defaultValue?: number) {
    this.name = name;
    this.type = type;
    if (defaultValue !== undefined) this.defaultValue = type === "bool" || type === "trigger" ? defaultValue !== 0 : defaultValue;
  }
}

class AnimationCondition {
  parameter: string;
  mode: string;
  threshold?: number;
  constructor(parameter: string, mode: string, threshold?: number) {
    this.parameter = parameter;
    this.mode = mode;
    if (threshold !== undefined) this.threshold = threshold;
  }
}

class AnimationTransition {
  from: string;
  to: string;
  conditions: AnimationCondition[] = [];
  hasExitTime?: boolean;
  exitTime?: number;
  duration?: number;
  offset?: number;
  constructor(from: string, to: string, options: Partial<AnimationTransition> = {}) {
    this.from = from;
    this.to = to;
    Object.assign(this, options);
  }
  addCondition(c: AnimationCondition) {
    this.conditions.push(c);
  }
  get conditionCount() {
    return this.conditions.length;
  }
}

class AnimationBlendTree {
  blendParameter: string;
  children: { motion: AnimationClip; threshold: number; timeScale?: number }[] = [];
  constructor(parameter: string) {
    this.blendParameter = parameter;
  }
  get parameter() {
    return this.blendParameter;
  }
  get childCount() {
    return this.children.length;
  }
  addChild(motion: AnimationClip, threshold: number, timeScale?: number) {
    this.children.push({ motion, threshold, timeScale });
  }
}

class AnimationNode {
  name: string;
  motion: AnimationClip | AnimationBlendTree;
  speed?: number;
  speedParameter?: string;
  constructor(name: string, motion: AnimationClip | AnimationBlendTree, options: { speed?: number; speedParameter?: string } = {}) {
    this.name = name;
    this.motion = motion;
    Object.assign(this, options);
  }
  static blend(name: string, tree: AnimationBlendTree) {
    return new AnimationNode(name, tree);
  }
}

class AnimationController extends AnimatorController {
  override addParameter(p: any): this {
    this.parameters.push(p);
    return this;
  }
  addNode(n: AnimationNode) {
    this.states.push(n as any);
  }
  get parameterCount() { return this.parameters.length; }
  get nodeCount() { return this.states.length; }
  get transitionCount() { return this.transitions.length; }
  parameterAt(i: number) { return this.parameters[i]!; }
  nodeAt(i: number) { return this.states[i]!; }
  transitionAt(i: number) { return Object.setPrototypeOf(this.transitions[i]!, AnimationTransition.prototype); }
  toJson() { return JSON.stringify(this.toJSON()); }
  static fromJson(text: string): AnimationController {
    const c = AnimatorController.fromJSON(JSON.parse(text));
    Object.setPrototypeOf(c, AnimationController.prototype);
    return c as AnimationController;
  }
  static build() {
    return new AnimationBuilder();
  }
}

class AnimationBuilder {
  private c = new AnimationController();
  private t: AnimationTransition | null = null;
  private blend: AnimationBlendTree | null = null;
  parameter(name: string, type: string, defaultValue?: number) { this.c.addParameter(new AnimationParameter(name, type, defaultValue)); return this; }
  state(name: string, clip: AnimationClip, speed?: number, speedParameter?: string) {
    const o: { speed?: number; speedParameter?: string } = {};
    if (speed !== undefined) o.speed = speed;
    if (speedParameter !== undefined) o.speedParameter = speedParameter;
    this.c.addNode(new AnimationNode(name, clip, o));
    return this;
  }
  blendState(name: string, parameter: string) { this.blend = new AnimationBlendTree(parameter); this.c.addNode(AnimationNode.blend(name, this.blend)); return this; }
  child(clip: AnimationClip, threshold: number, timeScale?: number) { this.blend!.addChild(clip, threshold, timeScale); return this; }
  transition(from: string, to: string) { this.t = new AnimationTransition(from, to); this.c.addTransition(this.t as any); return this; }
  when(parameter: string, mode: string, threshold?: number) { this.t!.addCondition(new AnimationCondition(parameter, mode, threshold)); return this; }
  exitTime(x: number) { this.t!.hasExitTime = true; this.t!.exitTime = x; return this; }
  duration(d: number) { this.t!.duration = d; return this; }
  offset(o: number) { this.t!.offset = o; return this; }
  defaultState(name: string) { this.c.defaultState = name; return this; }
  create() { return this.c; }
}

Object.assign(globalThis, { AnimationParameter, AnimationCondition, AnimationTransition, AnimationBlendTree, AnimationNode, AnimationController, AnimationBuilder });

const a = Animator.prototype as any;
for (const [name, info, key] of [
  ["currentStateName", "getCurrentAnimatorStateInfo", "name"],
  ["currentNormalizedTime", "getCurrentAnimatorStateInfo", "normalizedTime"],
  ["currentLength", "getCurrentAnimatorStateInfo", "length"],
  ["nextStateName", "getNextAnimatorStateInfo", "name"],
  ["nextNormalizedTime", "getNextAnimatorStateInfo", "normalizedTime"],
  ["nextLength", "getNextAnimatorStateInfo", "length"],
] as const) {
  Object.defineProperty(a, name, { get() { return this[info]()[key]; } });
}
