// SPDX-License-Identifier: Apache-2.0

// What the clock faces share: colours, easings, and builders that put everything a face makes
// under that face's root object (so switching faces is turning one object on or off).

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
export abstract class Face {
  abstract readonly name: string;
  abstract readonly root: GameObject;
  /** `now` is the wall clock; `t` seconds since the app started. */
  abstract update(now: DateTime, t: number, dt: number): void;
  show(camera: Camera): void {}
  hide(camera: Camera): void {}
}

let parent: GameObject | null = null;

/** Runs `build` with new objects going under `root`. */
export function under(root: GameObject, build: () => void): void {
  const before = parent;
  parent = root;
  build();
  parent = before;
}

/** A new object under the face being built (or under `inside`). */
export function obj(name: string, at: Vector3 = Vector3.zero, inside?: GameObject): GameObject {
  const p = inside !== undefined ? inside : parent;
  if (p !== null) return new GameObject(name, { position: at, parent: p });
  return new GameObject(name, { position: at });
}

/** What a shape is made with (a LineRenderer2D's own settings). */
export interface ShapeLook {
  points?: Vector2[];
  width?: number;
  color?: Color;
  loop?: boolean;
  fillColor?: Color;
  fillRule?: string;
  sortingOrder?: number;
}

/** A LineRenderer2D on its own object. */
export function shape(name: string, look: ShapeLook, at: Vector3 = Vector3.zero, inside?: GameObject): LineRenderer2D {
  const r = obj(name, at, inside).addComponent(LineRenderer2D);
  r.width = look.width !== undefined ? look.width : 0.05;
  r.color = look.color !== undefined ? look.color : INK;
  if (look.points !== undefined) r.points = look.points;
  if (look.loop !== undefined) r.loop = look.loop;
  if (look.fillColor !== undefined) r.fillColor = look.fillColor;
  if (look.fillRule !== undefined) r.fillRule = look.fillRule;
  if (look.sortingOrder !== undefined) r.sortingOrder = look.sortingOrder;
  return r;
}

/** What a label is made with (a TextMesh's own settings). */
export interface LabelLook {
  text?: string;
  fontSize?: number;
  color?: Color;
  font?: string;
  fontWeight?: number;
  alignment?: string;
  maxWidth?: number;
  pixelsPerUnit?: number;
  resolution?: number;
}

/** A TextMesh on its own object. */
export function label(name: string, look: LabelLook, at: Vector3 = Vector3.zero, inside?: GameObject): TextMesh {
  const t = obj(name, at, inside).addComponent(TextMesh);
  t.fontSize = look.fontSize !== undefined ? look.fontSize : 24;
  t.color = look.color !== undefined ? look.color : INK;
  if (look.text !== undefined) t.text = look.text;
  if (look.font !== undefined) t.font = look.font;
  if (look.fontWeight !== undefined) t.fontWeight = look.fontWeight;
  if (look.alignment !== undefined) t.alignment = look.alignment;
  if (look.maxWidth !== undefined) t.maxWidth = look.maxWidth;
  if (look.pixelsPerUnit !== undefined) t.pixelsPerUnit = look.pixelsPerUnit;
  if (look.resolution !== undefined) t.resolution = look.resolution;
  return t;
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
