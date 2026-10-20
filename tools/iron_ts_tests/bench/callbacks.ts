// Closures given to the engine: listeners with number, boolean and string arguments, captured
// variables, and removing one by its token. The Rust test (tests/all/callbacks.rs) sends the events.

let log = "";
let keyUpCount = 0;

export function subscribeKeyDown(): number {
  return input.onKeyDown((code: string, repeat: boolean) => {
    log += code + (repeat ? "r" : "") + ";";
  });
}

export function subscribeKeyUpCounting(): number {
  let seen = 0;
  return input.onKeyUp((code: string) => {
    seen += 1;
    keyUpCount = seen;
    log += "up:" + code + seen + ";";
  });
}

export function subscribeButtons(): number {
  const a = input.onGamepadButtonDown((id: number, button: string) => {
    log += "down" + id + button + ";";
  });
  input.onGamepadAxis((id: number, axis: string, value: number) => {
    log += axis + id + "=" + value.toFixed(1) + ";";
  });
  return a;
}

export function unsubscribe(token: number): boolean {
  return input.off(token);
}

export function readLog(): string {
  const r = log;
  log = "";
  return r;
}

export function upCount(): number {
  return keyUpCount;
}

export function subscribeAssets(): number {
  return Assets.onChanged$n((paths: string) => {
    log += "assets:" + paths.split("\n").join(",") + ";";
  });
}

export function subscribeFocus(): number {
  return Application.onFocusChanged$n((focused: boolean) => {
    log += focused ? "focus;" : "blur;";
  });
}

export function offFocus(token: number): boolean {
  return Application.offFocusChanged(token);
}

export function subscribeScenes(): number {
  SceneManager.onSceneUnloaded$n((scene: Scene) => {
    log += "unloaded;";
  });
  SceneManager.onActiveSceneChanged$n((previous: Scene, next: Scene) => {
    log += "active:" + previous.name + ">" + next.name + ";";
  });
  return SceneManager.onSceneLoaded$n((scene: Scene, mode: string) => {
    log += "loaded:" + scene.name + ":" + mode + ";";
  });
}

export function loadSecond(additive: boolean): void {
  SceneManager.loadScene('{"format":"iron.scene","version":1,"name":"Second","objects":[]}', additive ? "additive" : "single");
}

export function churn(): number {
  // Allocates (and, in stress mode, collects) while the engine holds the closures.
  let s = 0;
  for (let i = 0; i < 50; i++) s += ("x" + i).length;
  return s;
}
