// SPDX-License-Identifier: Apache-2.0

// The original Animation component over a scene with no engine behind it (see anim_clips_prelude.ts),
// with `Animations.updateAll(dt)` = the lifecycle start of new components and `__iron_animate(dt)`,
// and a component on every animated object that records the clip events it is called with
// (the native `Animations.takeEvents()` reports them as `object|function|parameter` lines).
import "./anim_clips_prelude.ts";
import { Animation, Animator } from "../../../js/engine/src/animation.ts";
import { Component, GameObject } from "../../../js/engine/src/scene.ts";

const log: string[] = [];
class EventLog extends Component {
  footstep(p?: number) { log.push(`${this.gameObject.name}|footstep|${p ?? ""}`); }
  done(p?: number) { log.push(`${this.gameObject.name}|done|${p ?? ""}`); }
}

const proto = GameObject.prototype as any;
const addComponent = proto.addComponent;
proto.addComponent = function (cls: any, ...rest: any[]) {
  const c = addComponent.call(this, cls, ...rest);
  if ((cls === Animation || cls === Animator) && !this.components.some((x: any) => x instanceof EventLog)) addComponent.call(this, EventLog);
  return c;
};

const started = new WeakSet<object>();
(globalThis as any).Animations = {
  updateAll(dt: number) {
    const visit = (go: GameObject) => {
      for (const c of go.components) if (c instanceof Animation && !started.has(c)) { started.add(c); c.onStart(); }
      go.children.forEach(visit);
    };
    GameObject.roots.forEach(visit);
    (globalThis as any).__iron_animate(dt);
  },
  takeEvents() { const text = log.join("\n"); log.length = 0; return text; },
};

// Native names for reading the clip list.
Object.defineProperty(Animation.prototype, "clipCount", { get() { return this.clips.length; } });
(Animation.prototype as any).clipAt = function (i: number) { return this.clips[i]; };

const resetScene = (globalThis as any).__bench_reset;
(globalThis as any).__bench_reset = () => {
  resetScene();
  log.length = 0;
};
