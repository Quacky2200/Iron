// The Animator and its controller (parameters, states, transitions, blend trees, triggers), made
// with the chained builder and with the explicit classes, compiled against the native API and run
// in bun against js/engine/src/animation.ts (an adapter in the prelude gives the original the same
// class names). The program advances time with Animations.updateAll.

function f(x: number): string {
  return `${x}`;
}

function line(clip: AnimationClip, x0: number, x1: number, end: number, mode: string): AnimationClip {
  clip.setCurve("", "Transform", "localPosition.x", AnimationCurve.linear(0, x0, end, x1));
  clip.wrapMode = mode;
  return clip;
}

function clips(): AnimationClip[] {
  const idle = line(new AnimationClip("idle"), 0, 1, 2, "loop");
  const run = line(new AnimationClip("run"), 10, 14, 1, "loop");
  run.addEvent(0.5, "footstep", 1);
  const jump = line(new AnimationClip("jump"), 20, 30, 1, "once");
  jump.addEvent(1, "done");
  return [idle, run, jump];
}

function viaBuilder(c: AnimationClip[]): AnimationController {
  return AnimationController.build()
    .parameter("speed", "float")
    .parameter("jump", "trigger")
    .parameter("grounded", "bool", 1)
    .state("Idle", c[0])
    .state("Run", c[1], 1, "speed")
    .state("Jump", c[2])
    .transition("Idle", "Run").when("speed", "greater", 0.1).duration(0.2)
    .transition("Run", "Idle").when("speed", "less", 0.1).duration(0.2)
    .transition("Any", "Jump").when("jump", "if").when("grounded", "if").duration(0.1)
    .transition("Jump", "Idle").exitTime(0.9).duration(0.1)
    .create();
}

function viaClasses(c: AnimationClip[]): AnimationController {
  const controller = new AnimationController();
  controller.addParameter(new AnimationParameter("speed", "float"));
  controller.addParameter(new AnimationParameter("jump", "trigger"));
  controller.addParameter(new AnimationParameter("grounded", "bool", 1));
  controller.addNode(new AnimationNode("Idle", c[0]));
  controller.addNode(new AnimationNode("Run", c[1], { speedParameter: "speed" }));
  controller.addNode(new AnimationNode("Jump", c[2]));
  const toRun = new AnimationTransition("Idle", "Run", { duration: 0.2 });
  toRun.addCondition(new AnimationCondition("speed", "greater", 0.1));
  controller.addTransition(toRun);
  const toIdle = new AnimationTransition("Run", "Idle", { duration: 0.2 });
  toIdle.addCondition(new AnimationCondition("speed", "less", 0.1));
  controller.addTransition(toIdle);
  const toJump = new AnimationTransition("Any", "Jump", { duration: 0.1 });
  toJump.addCondition(new AnimationCondition("jump", "if"));
  toJump.addCondition(new AnimationCondition("grounded", "if"));
  controller.addTransition(toJump);
  controller.addTransition(new AnimationTransition("Jump", "Idle", { hasExitTime: true, exitTime: 0.9, duration: 0.1 }));
  return controller;
}

function px(go: GameObject): number {
  return go.transform.localPosition.x;
}

function state(a: Animator): string {
  return `${a.currentStateName}/${f(a.currentNormalizedTime)}/${f(a.currentLength)}${a.isInTransition ? ">" + a.nextStateName + "/" + f(a.nextNormalizedTime) : ""}`;
}

function run(controller: AnimationController): string {
  const go = new GameObject("Hero");
  const a = go.addComponent(Animator);
  a.controller = controller;
  let s = `${state(a)};`;
  const script = [0, 0, 0.5, 0.5, 0.5, 0.5, 1, 1, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05, 0.05];
  for (let i = 0; i < script.length; i++) {
    a.setFloat("speed", script[i]);
    if (i === 12) a.setTrigger("jump");
    if (i === 14) a.setBool("grounded", false);
    if (i === 16) a.setTrigger("jump");
    if (i === 17) a.setBool("grounded", true);
    Animations.updateAll(0.2);
    s += `${f(px(go))} ${state(a)} ${Animations.takeEvents()};`;
  }
  s += `${a.getFloat("speed")},${a.getInteger("speed")},${a.getBool("jump")},${a.getBool("grounded")}`;
  return s;
}

export function builder(): string {
  return run(viaBuilder(clips()));
}

export function classes(): string {
  return run(viaClasses(clips()));
}

export function sameEitherWay(): string {
  return `${run(viaBuilder(clips())) === run(viaClasses(clips()))}`;
}

export function blendTree(): string {
  const a = line(new AnimationClip("walk"), 0, 2, 2, "loop");
  const b = line(new AnimationClip("jog"), 0, 6, 1, "loop");
  const c = line(new AnimationClip("sprint"), 0, 12, 0.5, "loop");
  const controller = AnimationController.build()
    .parameter("speed", "float")
    .blendState("Move", "speed")
    .child(c, 2)
    .child(a, 0)
    .child(b, 1, 0.5)
    .create();
  const go = new GameObject("Mover");
  const anim = go.addComponent(Animator);
  anim.controller = controller;
  let s = "";
  for (const v of [-1, 0, 0.25, 0.5, 1, 1.5, 2, 3, 0.75]) {
    anim.setFloat("speed", v);
    Animations.updateAll(0.1);
    s += `${f(v)}:${f(px(go))},${state(anim)};`;
  }
  const tree = new AnimationBlendTree("speed");
  tree.addChild(a, 0);
  tree.addChild(b, 4);
  const node = AnimationNode.blend("Explicit", tree);
  const second = new AnimationController();
  second.addNode(node);
  const g2 = new GameObject("Second");
  const a2 = g2.addComponent(Animator);
  a2.controller = second;
  a2.setFloat("speed", 1);
  Animations.updateAll(0.5);
  s += `${f(px(g2))},${state(a2)},${tree.childCount},${tree.parameter}`;
  return s;
}

export function jumping(): string {
  const c = clips();
  const controller = AnimationController.build()
    .parameter("go", "bool")
    .state("A", c[0])
    .state("B", c[1])
    .state("C", c[2])
    .transition("A", "B").exitTime(0.5).duration(0)
    .transition("B", "C").when("go", "if").duration(0.3).offset(0.25)
    .transition("C", "A").exitTime(1).duration(0.2)
    .defaultState("A")
    .create();
  const go = new GameObject("Jumper");
  const a = go.addComponent(Animator);
  a.controller = controller;
  let s = `${controller.defaultState},${controller.nodeCount},${controller.transitionCount},${controller.parameterCount};`;
  for (let i = 0; i < 24; i++) {
    if (i === 8) a.setBool("go", true);
    Animations.updateAll(0.25);
    s += `${f(px(go))} ${state(a)} ${Animations.takeEvents()};`;
  }
  a.play("B", 0.5);
  s += `${state(a)};`;
  a.crossFade("A", 0.5, 0.25);
  Animations.updateAll(0.25);
  s += `${state(a)};`;
  a.crossFade("Nothing");
  a.play("Nothing");
  a.speed = 2;
  Animations.updateAll(0.25);
  s += `${f(px(go))} ${state(a)}`;
  return s;
}

export function persistence(): string {
  const c = clips();
  const controller = viaBuilder(c);
  const copy = AnimationController.fromJson(controller.toJson());
  const first = run(controller);
  const second = run(copy);
  const empty = AnimationController.fromJson("{}");
  return `${first === second},${copy.parameterCount},${copy.nodeCount},${copy.transitionCount},${copy.defaultState || "none"},${empty.nodeCount},${copy.nodeAt(1).speedParameter},${copy.transitionAt(2).conditionCount},${copy.transitionAt(3).exitTime},${Number(copy.parameterAt(2).defaultValue)}`;
}
