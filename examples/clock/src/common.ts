// What the clock faces share: colours, easings, and builders that put everything a face makes
// under that face's root object (so switching faces is turning one object on or off).
import { type Camera, Color, type DateTime, GameObject, LineRenderer2D, TextMesh, Vector3 } from "@iron/engine";

export const BG = Color.hex("#0e131b");
export const INK = Color.hex("#e8edf5");
export const DIM = Color.hex("#5d6b80");

/** `a` mixed toward `b`, opaque (see-through strokes look beaded where segments overlap). */
export const mix = (a: Color, b: Color, t: number) =>
  new Color(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1);

export const easeOutCubic = (x: number) => 1 - (1 - x) ** 3;
export const easeInOutCubic = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2);
/** Past the end and back: a mechanical tick. */
export const easeOutBack = (x: number) => {
  const c = 1.9;
  return 1 + (c + 1) * (x - 1) ** 3 + c * (x - 1) ** 2;
};

/** A clock face: built under `root`, updated while it's shown. */
export interface Face {
  readonly name: string;
  readonly root: GameObject;
  /** `now` is the wall clock; `t` seconds since the app started. */
  update(now: DateTime, t: number, dt: number): void;
  show?(camera: Camera): void;
  hide?(camera: Camera): void;
}

let parent: GameObject | null = null;

/** Runs `build` with new objects going under `root`. */
export function under<T>(root: GameObject, build: () => T): T {
  const before = parent;
  parent = root;
  try {
    return build();
  } finally {
    parent = before;
  }
}

/** A new object under the face being built (or under `inside`). */
export function obj(name: string, at: Vector3 = Vector3.zero, inside?: GameObject): GameObject {
  const p = inside ?? parent;
  return new GameObject(name, p ? { position: at, parent: p } : { position: at });
}

/** A LineRenderer2D on its own object. */
export function shape(name: string, look: Partial<LineRenderer2D>, at: Vector3 = Vector3.zero, inside?: GameObject): LineRenderer2D {
  return obj(name, at, inside).addComponent(LineRenderer2D, { width: 0.05, color: INK, ...look });
}

/** A TextMesh on its own object. */
export function label(name: string, look: Partial<TextMesh>, at: Vector3 = Vector3.zero, inside?: GameObject): TextMesh {
  return obj(name, at, inside).addComponent(TextMesh, { fontSize: 24, color: INK, ...look });
}

/**
 * Switches `camera` to a perspective view framed like the orthographic one (40 degrees, far
 * enough back that the same 10 units fill the height). Returns how to put it back.
 */
export function perspective(camera: Camera): () => void {
  const saved = { orthographic: camera.orthographic, position: camera.transform.position, fov: camera.fieldOfView };
  camera.orthographic = false;
  camera.fieldOfView = 40;
  camera.transform.position = new Vector3(0, 0, 5 / Math.tan((20 * Math.PI) / 180));
  return () => {
    camera.orthographic = saved.orthographic;
    camera.fieldOfView = saved.fov;
    camera.transform.position = saved.position;
  };
}

/** Two digits, zero-padded. */
export const two = (n: number) => String(n).padStart(2, "0");
