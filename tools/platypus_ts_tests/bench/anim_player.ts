// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Animation component (named clip states, play, crossFade, stop, events, the default clip,
// engine component properties), compiled against the native API and run in bun against
// js/engine/src/animation.ts and scene.ts. The program advances time with Animations.updateAll.

function f(x: number): string {
  return `${x}`;
}

function line(clip: AnimationClip, x0: number, x1: number, end: number): AnimationClip {
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, x0, end, x1));
  return clip;
}

function px(go: GameObject): number {
  return go.transform.localPosition.x;
}

export function playing(): string {
  const go = new GameObject("Player");
  const anim = go.addComponent(Animation);
  const a = new AnimationClip("A");
  const b = new AnimationClip("B");
  line(a, 0, 10, 2);
  line(b, 100, 130, 3);
  anim.addClip(a);
  anim.addClip(b, "Second");
  let s = `${anim.clipCount},${anim.isPlaying()},${anim.isPlaying("A")},${anim.play("Nope")};`;
  const st = anim.state("Second");
  s += `${st.name},${st.length},${st.weight},${st.enabled},${st.speed},${st.wrapMode};`;
  s += `${anim.play("A")},${anim.isPlaying()},${anim.isPlaying("A")},${anim.isPlaying("Second")};`;
  for (let i = 0; i < 4; i++) {
    Animations.updateAll(0.3);
    s += `${f(px(go))}@${f(anim.state("A").time)},${f(anim.state("A").normalizedTime)};`;
  }
  anim.play("Second");
  s += `${anim.isPlaying("A")},${anim.state("A").weight},${anim.state("Second").weight};`;
  Animations.updateAll(0.5);
  s += `${f(px(go))};`;
  anim.state("Second").speed = 2;
  Animations.updateAll(0.5);
  s += `${f(px(go))};`;
  anim.state("Second").normalizedTime = 0.25;
  Animations.updateAll(0);
  s += `${f(px(go))},${f(anim.state("Second").time)};`;
  anim.stop("Second");
  s += `${anim.isPlaying()},${anim.state("Second").weight},${anim.state("Second").time};`;
  anim.play("A");
  anim.stop();
  s += `${anim.isPlaying()}`;
  return s;
}

export function fading(): string {
  const go = new GameObject("Fader");
  const anim = go.addComponent(Animation);
  const a = line(new AnimationClip("A"), 0, 10, 10);
  const b = line(new AnimationClip("B"), 100, 200, 10);
  a.wrapMode = "loop";
  b.wrapMode = "loop";
  anim.addClip(a);
  anim.addClip(b);
  anim.play("A");
  Animations.updateAll(1);
  let s = `${f(px(go))};`;
  anim.crossFade("B", 0.5);
  for (let i = 0; i < 8; i++) {
    Animations.updateAll(0.1);
    s += `${f(px(go))}|${f(anim.state("A").weight)},${f(anim.state("B").weight)},${anim.state("A").enabled};`;
  }
  anim.crossFade("A");
  for (let i = 0; i < 5; i++) {
    Animations.updateAll(0.1);
    s += `${f(px(go))}|${f(anim.state("A").weight)},${f(anim.state("B").weight)},${anim.state("B").enabled};`;
  }
  anim.crossFade("Missing", 1);
  return s;
}

export function wrapModes(): string {
  let s = "";
  for (const mode of ["once", "loop", "pingPong", "clampForever"]) {
    const go = new GameObject(mode);
    const anim = go.addComponent(Animation);
    const clip = line(new AnimationClip(mode), 0, 8, 2);
    clip.wrapMode = mode;
    anim.addClip(clip);
    anim.play(mode);
    s += `${mode}:`;
    for (let i = 0; i < 12; i++) {
      Animations.updateAll(0.45);
      s += `${f(px(go))}${anim.isPlaying() ? "+" : "-"},`;
    }
    s += "|";
  }
  return s;
}

export function events(): string {
  const go = new GameObject("Walker");
  const anim = go.addComponent(Animation);
  const clip = line(new AnimationClip("walk"), 0, 4, 2);
  clip.addEvent(0.5, "footstep", 1);
  clip.addEvent(1, "footstep", 2);
  clip.addEvent(2, "done");
  clip.wrapMode = "loop";
  anim.addClip(clip);
  anim.play("walk");
  let s = "";
  for (let i = 0; i < 14; i++) {
    Animations.updateAll(0.35);
    s += `[${Animations.takeEvents()}]`;
  }
  const once = new GameObject("Once");
  const a2 = once.addComponent(Animation);
  const c2 = line(new AnimationClip("one"), 0, 1, 1);
  c2.addEvent(0.4, "footstep", 7);
  c2.addEvent(1, "done");
  a2.addClip(c2);
  a2.play("one");
  for (let i = 0; i < 6; i++) {
    Animations.updateAll(0.3);
    s += `{${Animations.takeEvents()}}`;
  }
  return s;
}

export function defaultClip(): string {
  const auto = new GameObject("Auto");
  const aa = auto.addComponent(Animation);
  const clip = line(new AnimationClip("idle"), 0, 6, 3);
  clip.wrapMode = "loop";
  aa.clip = clip;
  const manual = new GameObject("Manual", {});
  const am = manual.addComponent(Animation);
  am.clip = clip;
  am.playAutomatically = false;
  let s = `${aa.playAutomatically},${am.playAutomatically},${aa.clipCount},${aa.isPlaying()};`;
  for (let i = 0; i < 4; i++) {
    Animations.updateAll(0.5);
    s += `${f(px(auto))},${f(px(manual))},${aa.isPlaying()},${am.isPlaying()};`;
  }
  am.play();
  Animations.updateAll(0.5);
  s += `${f(px(manual))},${am.isPlaying("idle")};`;
  return s;
}

export function components(): string {
  const go = new GameObject("Cam");
  const cam = go.addComponent(Camera);
  const anim = go.addComponent(Animation);
  const clip = new AnimationClip("zoom");
  clip.setCurve("", "Camera", "fieldOfView", AnimationCurve.linear(0, 30, 2, 90));
  clip.setCurve("", "Camera", "orthographicSize", AnimationCurve.linear(0, 2, 2, 8));
  clip.setCurve("", "Camera", "backgroundColor.r", AnimationCurve.linear(0, 0, 2, 1));
  clip.setCurve("", "Camera", "backgroundColor.a", AnimationCurve.linear(0, 1, 2, 0.5));
  clip.setCurve("", "Camera", "orthographic", AnimationCurve.constant(0, 2, 1));
  clip.setCurve("", "Missing", "value", AnimationCurve.constant(0, 2, 1));
  anim.addClip(clip);
  anim.play("zoom");
  let s = "";
  for (let i = 0; i < 6; i++) {
    Animations.updateAll(0.5);
    s += `${cam.fieldOfView},${cam.orthographicSize},${cam.backgroundColor.r},${cam.backgroundColor.a},${cam.orthographic};`;
  }
  const bare = new GameObject("Bare");
  const ab = bare.addComponent(Animation);
  ab.addClip(clip);
  ab.play("zoom");
  Animations.updateAll(0.5);
  return s + "ok";
}

export function blending(): string {
  const go = new GameObject("Blend");
  const anim = go.addComponent(Animation);
  const turnA = new AnimationClip("turnA");
  const turnB = new AnimationClip("turnB");
  const rotA = [0, 0, 0, 1];
  const rotB = [0, 0.6, 0, -0.8];
  const axes = ["x", "y", "z", "w"];
  for (let i = 0; i < 4; i++) {
    turnA.setCurve("", "Transform", `localRotation.${axes[i]}`, AnimationCurve.constant(0, 2, rotA[i]));
    turnB.setCurve("", "Transform", `localRotation.${axes[i]}`, AnimationCurve.constant(0, 2, rotB[i]));
  }
  line(turnA, 0, 10, 2);
  line(turnB, 20, 30, 2);
  turnA.wrapMode = "loop";
  turnB.wrapMode = "loop";
  anim.addClip(turnA);
  anim.addClip(turnB);
  anim.play("turnA");
  anim.crossFade("turnB", 1);
  let s = "";
  for (let i = 0; i < 6; i++) {
    Animations.updateAll(0.25);
    const q = go.transform.localRotation;
    s += `${q.x},${q.y},${q.z},${q.w}|${f(px(go))};`;
  }
  return s;
}

export function inactive(): string {
  const parent = new GameObject("Parent");
  const child = new GameObject("Child");
  child.transform.setParent(parent.transform, false);
  const anim = child.addComponent(Animation);
  const clip = line(new AnimationClip("m"), 0, 10, 10);
  clip.wrapMode = "loop";
  anim.addClip(clip);
  anim.play("m");
  let s = "";
  Animations.updateAll(1);
  s += `${f(px(child))};`;
  parent.enabled = false;
  Animations.updateAll(1);
  s += `${f(px(child))},${anim.state("m").time};`;
  parent.enabled = true;
  Animations.updateAll(1);
  s += `${f(px(child))}`;
  return s;
}
