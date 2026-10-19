// Curves, clips and sampling, compiled against the native API and run in bun against
// js/engine/src/animation.ts and scene.ts. Every number is printed in full.

function f(x: number): string {
  return `${x}`;
}

function samples(c: AnimationCurve, from: number, to: number, steps: number): string {
  let s = "";
  for (let i = 0; i <= steps; i++) s += `${c.evaluate(from + ((to - from) * i) / steps)},`;
  return s;
}

export function curves(): string {
  let s = "";
  s += samples(AnimationCurve.linear(0, 0, 2, 10), -1, 3, 16) + "|";
  s += samples(AnimationCurve.easeInOut(0, 1, 1, 5), -0.5, 1.5, 16) + "|";
  s += samples(AnimationCurve.constant(0, 3, 7), -1, 4, 5) + "|";
  const custom = new AnimationCurve();
  custom.addKeyframe(new Keyframe(0, 0, 0, 4));
  custom.addKeyframe(new Keyframe(1, 3, -2, 2));
  custom.addKeyframe(new Keyframe(2.5, -1, 1, 0));
  custom.addKeyframe(new Keyframe(4, 2, 0, 0));
  s += samples(custom, -2, 7, 36) + "|";
  const steps = new AnimationCurve();
  steps.addKeyframe(new Keyframe(0, 1, Infinity, Infinity));
  steps.addKeyframe(new Keyframe(1, 5, Infinity, Infinity));
  steps.addKeyframe(new Keyframe(2, 3, Infinity, Infinity));
  s += samples(steps, -1, 3, 20) + "|";
  return s;
}

export function wrapping(): string {
  let s = "";
  for (const mode of ["once", "loop", "pingPong", "clampForever"]) {
    const c = AnimationCurve.linear(1, 0, 3, 8);
    c.preWrapMode = mode;
    c.postWrapMode = mode;
    s += `${mode}:${c.preWrapMode},${c.postWrapMode}:` + samples(c, -4, 9, 26) + "|";
  }
  const mixed = AnimationCurve.easeInOut(0, 0, 2, 1);
  mixed.preWrapMode = "pingPong";
  mixed.postWrapMode = "loop";
  s += samples(mixed, -5, 7, 24);
  return s;
}

export function editing(): string {
  const c = new AnimationCurve();
  let s = "";
  s += `${c.length},${c.evaluate(1)},`;
  s += `${c.addKey(2, 20)},${c.addKey(1, 10)},${c.addKey(3, 30)},${c.addKey(1, 11)},${c.addKeyframe(new Keyframe(0, -5, 1, 2))},`;
  s += `${c.length};`;
  for (let i = 0; i < c.length; i++) {
    const k = c.key(i);
    s += `${k.time},${k.value},${k.inTangent},${k.outTangent};`;
  }
  c.removeKey(1);
  c.removeKey(10);
  s += `${c.length},${c.key(1).time},${c.key(c.length - 1).time},${c.evaluate(2.2)}`;
  const one = new AnimationCurve();
  one.addKeyframe(new Keyframe(5, 9));
  s += `|${one.evaluate(-3)},${one.evaluate(100)},${one.length}`;
  return s;
}

export function clips(): string {
  const clip = new AnimationClip("walk");
  let s = `${clip.name},${clip.wrapMode},${clip.frameRate},${clip.length},${clip.curveCount},${clip.eventCount};`;
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, 2, 4));
  clip.setCurve("Arm", "Transform", "localRotation.w", AnimationCurve.linear(0, 1, 3, 0));
  clip.setCurve("", "GameObject", "active", AnimationCurve.constant(0, 1, 1));
  s += `${clip.length},${clip.curveCount};`;
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, 5, 4));
  s += `${clip.length},${clip.curveCount},${clip.curve("", "Transform", "localPosition.x").evaluate(5)};`;
  clip.removeCurve("Arm", "Transform", "localRotation.w");
  clip.removeCurve("nothing", "Transform", "x");
  s += `${clip.length},${clip.curveCount},${clip.curveAt(0)},${clip.curveAt(1)},[${clip.curveAt(2)}];`;
  clip.addEvent(7, "late");
  clip.addEvent(1, "early", 3.5);
  clip.addEvent(4, "middle");
  s += `${clip.length},${clip.eventCount};`;
  clip.wrapMode = "loop";
  clip.frameRate = 24;
  clip.name = "run";
  s += `${clip.name},${clip.wrapMode},${clip.frameRate}`;
  return s;
}

function rig(): GameObject {
  const root = new GameObject("Root");
  const arm = new GameObject("Arm");
  const hand = new GameObject("Hand");
  arm.transform.setParent(root.transform, false);
  hand.transform.setParent(arm.transform, false);
  hand.transform.localPosition = new Vector3(0, 1, 0);
  return root;
}

function pose(go: GameObject): string {
  const p = go.transform.localPosition;
  const q = go.transform.localRotation;
  const sc = go.transform.localScale;
  return `[${p.x},${p.y},${p.z}|${q.x},${q.y},${q.z},${q.w}|${sc.x},${sc.y},${sc.z}|${go.enabled}]`;
}

function poseAll(root: GameObject): string {
  const arm = GameObject.find("Arm");
  const hand = GameObject.find("Hand");
  return pose(root) + pose(arm) + pose(hand);
}

export function sampling(): string {
  const root = rig();
  const clip = new AnimationClip("move");
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, 0, 2, 10));
  clip.setCurve("", "Transform", "localPosition.y", AnimationCurve.easeInOut(0, 1, 2, -1));
  clip.setCurve("Arm", "Transform", "localScale.x", AnimationCurve.linear(0, 1, 2, 3));
  clip.setCurve("Arm", "Transform", "localScale.y", AnimationCurve.linear(0, 1, 2, 3));
  clip.setCurve("Arm", "Transform", "localScale.z", AnimationCurve.linear(0, 1, 2, 3));
  clip.setCurve("Arm/Hand", "Transform", "localRotation.x", AnimationCurve.linear(0, 0, 2, 0.5));
  clip.setCurve("Arm/Hand", "Transform", "localRotation.y", AnimationCurve.linear(0, 0, 2, 0.5));
  clip.setCurve("Arm/Hand", "Transform", "localRotation.z", AnimationCurve.linear(0, 0, 2, 0));
  clip.setCurve("Arm/Hand", "Transform", "localRotation.w", AnimationCurve.linear(0, 1, 2, 0.5));
  clip.setCurve("Arm/Hand", "GameObject", "active", AnimationCurve.constant(0, 1, 1));
  clip.setCurve("Missing/Path", "Transform", "localPosition.x", AnimationCurve.constant(0, 1, 9));
  let s = "";
  for (const t of [0, 0.5, 1, 1.9, 2, 3, -1]) {
    clip.sampleAnimation(root, t);
    s += `${t}:${poseAll(root)}\n`;
  }
  return s;
}

export function wrapModesWhenSampling(): string {
  const root = new GameObject("Root");
  let s = "";
  for (const mode of ["once", "loop", "pingPong", "clampForever"]) {
    const clip = new AnimationClip(mode);
    clip.wrapMode = mode;
    clip.setCurve("", "Transform", "localPosition.z", AnimationCurve.linear(0, 0, 2, 8));
    for (const t of [-1, 0.5, 2, 2.5, 3.9, 4.5, 7]) {
      clip.sampleAnimation(root, t);
      s += `${mode}@${t}=${root.transform.localPosition.z};`;
    }
  }
  return s;
}

export function euler(): string {
  const root = new GameObject("Root");
  const clip = new AnimationClip("spin");
  clip.setCurve("", "Transform", "localEulerAngles.x", AnimationCurve.linear(0, 0, 1, 45));
  clip.setCurve("", "Transform", "localEulerAngles.y", AnimationCurve.linear(0, 0, 1, 180));
  clip.setCurve("", "Transform", "localEulerAngles.z", AnimationCurve.constant(0, 1, 10));
  let s = "";
  for (const t of [0, 0.25, 0.5, 1]) {
    clip.sampleAnimation(root, t);
    const q = root.transform.localRotation;
    s += `${q.x},${q.y},${q.z},${q.w};`;
  }
  const partial = new AnimationClip("partial");
  partial.setCurve("", "Transform", "localRotation.x", AnimationCurve.constant(0, 1, 1));
  partial.sampleAnimation(root, 0.5);
  const r = root.transform.localRotation;
  s += `partial:${r.x},${r.y},${r.z},${r.w}`;
  return s;
}

export function persistence(): string {
  const clip = new AnimationClip("saved");
  clip.wrapMode = "pingPong";
  clip.frameRate = 30;
  const c = new AnimationCurve();
  c.addKeyframe(new Keyframe(0, 1, 0, 3));
  c.addKeyframe(new Keyframe(2, 5, Infinity, Infinity));
  c.postWrapMode = "loop";
  clip.setCurve("A/B", "Transform", "localPosition.y", c);
  clip.setCurve("", "Light", "color.r", AnimationCurve.linear(0, 0, 1, 1));
  clip.addEvent(1.5, "fire", 2);
  const copy = AnimationClip.fromJson(clip.toJson());
  let s = `${copy.name},${copy.wrapMode},${copy.frameRate},${copy.length},${copy.curveCount},${copy.eventCount},${copy.curveAt(0)},${copy.curveAt(1)};`;
  const cc = copy.curve("A/B", "Transform", "localPosition.y");
  s += `${cc.length},${cc.postWrapMode},${cc.preWrapMode},${cc.key(1).inTangent},${cc.evaluate(0.7)},${cc.evaluate(3.3)};`;
  const empty = AnimationClip.fromJson("{}");
  s += `${empty.name},${empty.wrapMode},${empty.frameRate},${empty.length}`;
  return s;
}
