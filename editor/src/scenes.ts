// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The scene the editor works on: new, open and save through `EditorScene`, the recent files
// (kept per project in PlayerPrefs), the scene a project starts with, and the game cameras, which
// draw only into the Game panel while the editor runs but are saved as the scene has them.

/** The most recent files kept. */
export const RECENT_LIMIT = 8;

const recentKey = (): string => `editor.recent:${Editor.project}`;

/** Recent files, newest first, from their saved text (one path a line). */
export function parseRecent(text: string): string[] {
  return text.split("\n").filter((l) => l.trim() !== "");
}

/** `path` first, without repeats, at most `RECENT_LIMIT`. */
export function addRecent(list: readonly string[], path: string): string[] {
  return [path, ...list.filter((p) => p !== path)].slice(0, RECENT_LIMIT);
}

/** The project's recent scene files, newest first. */
export function recentScenes(): string[] {
  return parseRecent(PlayerPrefs.getString(recentKey(), ""));
}

function saveRecent(list: readonly string[]): void {
  PlayerPrefs.setString(recentKey(), list.join("\n"));
  PlayerPrefs.save();
}

export function rememberScene(path: string): void {
  saveRecent(addRecent(recentScenes(), path));
}

export function forgetScene(path: string): void {
  saveRecent(recentScenes().filter((p) => p !== path));
}

export function clearRecent(): void {
  saveRecent([]);
}

/** The window title: the scene's name (or path) with `*` for unsaved changes, and the project. */
export function windowTitle(scene: string, dirty: boolean, project: string): string {
  return `${scene}${dirty ? "*" : ""} - ${project} - Platypus Editor`;
}

// --- Game cameras ---------------------------------------------------------------------------

/** Each game camera's own `rect` (as the scene has it), by object id. */
const authored = new Map<number, Rect>();

/** The game's cameras (not the editor's own, which are hidden). */
function gameCameras(): Camera[] {
  const out: Camera[] = [];
  const stack = [...GameObject.roots];
  while (stack.length > 0) {
    const go = stack.pop()!;
    if (go.destroyed || (go.hideFlags & HideFlags.HideInHierarchy) !== 0) continue;
    const c = go.getComponent(Camera);
    if (c) out.push(c);
    for (const child of go.children) stack.push(child);
  }
  return out;
}

/**
 * Keeps the game's cameras off the screen except `Camera.main` in the Game panel (`shown`: its
 * rect as fractions of the window; null when the panel is hidden). Call every frame.
 */
export function placeGameCameras(shown: Rect | null): void {
  const main = Camera.main;
  for (const c of gameCameras()) {
    const id = c.gameObject.id;
    if (!authored.has(id)) {
      const r = c.rect;
      // A zero rect (the demo scene's) means the whole view.
      authored.set(id, r.width > 0 && r.height > 0 ? r : new Rect(0, 0, 1, 1));
    }
    const want = shown && main && main.gameObject.id === id ? shown : new Rect(0, 0, 0, 0);
    const now = c.rect;
    if (now.x !== want.x || now.y !== want.y || now.width !== want.width || now.height !== want.height) c.rect = want;
  }
}

/** Runs `fn` with the game cameras' own rects back (saving), then hides them again. */
export function withAuthoredCameras(fn: () => boolean): boolean {
  const cams = gameCameras();
  const now = cams.map((c) => c.rect);
  for (const c of cams) {
    const r = authored.get(c.gameObject.id);
    if (r) c.rect = r;
  }
  const result = fn();
  cams.forEach((c, i) => {
    c.rect = now[i]!;
  });
  return result;
}

/** Forgets the cameras of a scene that was closed. */
export function forgetCameras(): void {
  authored.clear();
}

// --- Scenes --------------------------------------------------------------------------------

/** A new scene with Unity's default objects: a Main Camera and a Directional Light. */
export function newScene(): boolean {
  if (!EditorScene.newScene("Untitled")) return false;
  forgetCameras();
  const camera = new GameObject("Main Camera", { position: new Vector3(0, 1, -10) });
  camera.tag = "MainCamera";
  camera.addComponent(Camera, { clearFlags: "skybox" });
  new GameObject("Directional Light", { position: new Vector3(0, 3, 0), rotation: Quaternion.euler(50, -30, 0) }).addComponent(Light, {
    type: "directional",
    intensity: 1,
    shadows: "soft",
  });
  return true;
}

/** Opens a project scene file in place of the loaded scenes. */
export function openScene(path: string): boolean {
  if (!EditorScene.openScene(path)) return false;
  forgetCameras();
  rememberScene(path);
  return true;
}

/** Saves the active scene to a project file (with its `.meta`). */
export function saveScene(path: string): boolean {
  const ok = withAuthoredCameras(() => EditorScene.saveScene(path));
  if (!ok) return false;
  Editor.ensureMeta(path);
  rememberScene(path);
  return true;
}

/** The scene to open at start: the last one this project had open, else its start scene; "" for none. */
export function startScene(startFromProject: string): string {
  for (const path of recentScenes()) if (Editor.exists(path)) return path;
  return startFromProject !== "" && Editor.exists(startFromProject) ? startFromProject : "";
}
