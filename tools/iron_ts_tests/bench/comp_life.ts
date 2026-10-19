// The lifecycle of a component a script removes or destroys: onDestroy runs when the calling hook
// returns, the focus and pause hooks get their flag, and a destroyed object takes its components
// with it. The Rust test drives the host and reads the log.

let log = "";

class Watcher extends Component {
  onAwake(): void {
    log += "A";
  }
  onStart(): void {
    log += "S";
  }
  onUpdate(dt: number): void {
    log += "U";
  }
  onDestroy(): void {
    log += "D";
  }
  onApplicationFocus(has: boolean): void {
    log += has ? "F" : "f";
  }
  onApplicationPause(paused: boolean): void {
    log += paused ? "P" : "p";
  }
  killSelf(): void {
    this.destroy();
  }
}

class Spawner extends Component {
  onUpdate(dt: number): void {
    const w = this.gameObject!.addComponent(Watcher);
    log += "+";
    this.gameObject!.removeComponent(w);
    log += "-";
  }
}

register({ Watcher, Spawner });

export function logText(): string {
  return log;
}

export function clearLog(): void {
  log = "";
}

export function removeWatcher(go: GameObject): void {
  const w = go.getComponent(Watcher);
  if (w) go.removeComponent(w);
}

export function suicide(go: GameObject): void {
  const w = go.getComponent(Watcher);
  if (w) w.killSelf();
}

export function count(go: GameObject): number {
  return go.getComponents(Watcher).length;
}

// Engine components are found with `getComponent` too.
export function engineKinds(go: GameObject): number {
  let n = 0;
  if (go.getComponent(Camera)) n += 1;
  if (go.getComponent(Animation)) n += 2;
  if (go.getComponent(Animator)) n += 4;
  return n;
}

export function addEngine(go: GameObject, which: number): void {
  if (which == 1) go.addComponent(Camera);
  if (which == 2) go.addComponent(Animation);
  if (which == 4) go.addComponent(Animator);
}
