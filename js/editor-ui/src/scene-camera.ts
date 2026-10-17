// The Scene view's camera, as Unity's works: it looks at a pivot from `distance` away,
// turned by `yaw` and `pitch`. Every control is a plain function from one state to the
// next (tested without React or the engine running): look (right-drag) turns about the
// eye, orbit (Alt+left-drag) about the pivot, pan (middle-drag) slides both, the wheel
// zooms towards the pivot, WASD/QE fly, F frames a sphere, and the axis gizmo looks
// along an axis.

import { Quaternion, Vector3 } from "@iron/engine";

const DEG = Math.PI / 180;

export interface SceneCameraState {
  /** The point the camera looks at and orbits. */
  pivot: Vector3;
  /** Degrees about +y; 0 looks along -z (forward), 90 along -x. */
  yaw: number;
  /** Degrees; positive looks down, 90 straight down. */
  pitch: number;
  /** From the eye to the pivot, in metres. In orthographic views, it sets the size. */
  distance: number;
  orthographic: boolean;
}

/** Vertical field of view, degrees. */
export const SCENE_FOV = 60;
const MIN_DISTANCE = 0.05;
const MAX_DISTANCE = 100_000;
/** Degrees turned per pixel dragged. */
const TURN = 0.25;

export function defaultSceneCamera(): SceneCameraState {
  return { pivot: Vector3.zero, yaw: -35, pitch: 25, distance: 14, orthographic: false };
}

/** The camera's rotation: yaw about up, then pitch about its right. */
export function sceneRotation(s: SceneCameraState): Quaternion {
  return Quaternion.angleAxis(s.yaw, Vector3.up).mul(Quaternion.angleAxis(-s.pitch, Vector3.right));
}

/** The way the camera looks. */
export function viewDirection(s: Pick<SceneCameraState, "yaw" | "pitch">): Vector3 {
  const y = s.yaw * DEG;
  const p = s.pitch * DEG;
  return new Vector3(-Math.sin(y) * Math.cos(p), -Math.sin(p), -Math.cos(y) * Math.cos(p));
}

/**
 * Where the eye is. Orthographic views sit farther back (their size comes from
 * `distance`, not their place), so nearby objects aren't cut by the near plane.
 */
export function eyePosition(s: SceneCameraState): Vector3 {
  const back = s.orthographic ? Math.max(s.distance * 4, 500) : s.distance;
  return s.pivot.sub(viewDirection(s).scale(back));
}

/** Half the view's height in metres at the pivot: an orthographic camera's size. */
export function orthographicSize(s: SceneCameraState): number {
  return s.distance * Math.tan((SCENE_FOV / 2) * DEG);
}

const clampPitch = (p: number) => Math.max(-90, Math.min(90, p));
const wrapYaw = (y: number) => ((((y + 180) % 360) + 360) % 360) - 180;

/** Turns the camera about its eye (right-drag), by pixels dragged. */
export function look(s: SceneCameraState, dx: number, dy: number): SceneCameraState {
  const eye = eyePosition(s);
  const turned = { ...s, yaw: wrapYaw(s.yaw - dx * TURN), pitch: clampPitch(s.pitch + dy * TURN) };
  const back = s.orthographic ? Math.max(s.distance * 4, 500) : s.distance;
  return { ...turned, pivot: eye.add(viewDirection(turned).scale(back)) };
}

/** Turns the camera about the pivot (Alt+left-drag), by pixels dragged. */
export function orbit(s: SceneCameraState, dx: number, dy: number): SceneCameraState {
  return { ...s, yaw: wrapYaw(s.yaw - dx * TURN), pitch: clampPitch(s.pitch + dy * TURN) };
}

/**
 * Slides the camera (middle-drag) so the point under the cursor at the pivot's depth
 * stays under it: pixels dragged in a view `viewHeight` pixels tall.
 */
export function pan(s: SceneCameraState, dx: number, dy: number, viewHeight: number): SceneCameraState {
  const perPixel = (2 * orthographicSize(s)) / Math.max(1, viewHeight);
  const r = sceneRotation(s);
  const move = r.rotate(Vector3.right).scale(-dx * perPixel).add(r.rotate(Vector3.up).scale(dy * perPixel));
  return { ...s, pivot: s.pivot.add(move) };
}

/** Zooms towards the pivot (wheel): positive `deltaY` (scrolling down) moves away. */
export function zoom(s: SceneCameraState, deltaY: number): SceneCameraState {
  const distance = s.distance * Math.pow(1.1, deltaY / 40);
  return { ...s, distance: Math.max(MIN_DISTANCE, Math.min(MAX_DISTANCE, distance)) };
}

/**
 * Flies (WASD / QE while right-dragging): `move` is right, up and forward in -1..1
 * for `seconds`. Speed grows with the distance to the pivot, four times with Shift.
 */
export function fly(s: SceneCameraState, move: Vector3, seconds: number, fast = false): SceneCameraState {
  const speed = Math.max(2, s.distance * 0.75) * (fast ? 4 : 1);
  const r = sceneRotation(s);
  const step = r
    .rotate(Vector3.right)
    .scale(move.x)
    .add(Vector3.up.scale(move.y))
    .add(viewDirection(s).scale(move.z))
    .scale(speed * seconds);
  return { ...s, pivot: s.pivot.add(step) };
}

/** Frames a sphere (F): the pivot at its centre, far enough back to see all of it. */
export function frame(s: SceneCameraState, center: Vector3, radius: number): SceneCameraState {
  const r = Math.max(radius, 0.1);
  return { ...s, pivot: center, distance: Math.min(MAX_DISTANCE, (r / Math.sin((SCENE_FOV / 2) * DEG)) * 1.1) };
}

/** Looks along `direction` about the same pivot (the axis gizmo's cones). */
export function lookAlong(s: SceneCameraState, direction: Vector3): SceneCameraState {
  const d = direction.normalized;
  const pitch = Math.asin(Math.max(-1, Math.min(1, -d.y))) / DEG;
  // Straight up or down keeps the yaw it had.
  const yaw = Math.abs(d.y) > 0.9999 ? s.yaw : Math.atan2(-d.x, -d.z) / DEG;
  return { ...s, yaw, pitch };
}

/** An axis of the orientation gizmo, as seen on screen. */
export interface GizmoAxis {
  /** `x`, `y`, `z`, or `-x`, `-y`, `-z`. */
  name: string;
  axis: Vector3;
  /** On screen, -1 to 1: right and down. */
  x: number;
  y: number;
  /** Towards the viewer: positive points out of the screen (drawn last). */
  depth: number;
}

/** The six axes as the camera sees them, farthest first (the drawing order). */
export function gizmoAxes(s: SceneCameraState): GizmoAxis[] {
  const r = sceneRotation(s);
  const right = r.rotate(Vector3.right);
  const up = r.rotate(Vector3.up);
  const forward = viewDirection(s);
  const axes: [string, Vector3][] = [
    ["x", Vector3.right],
    ["y", Vector3.up],
    ["z", new Vector3(0, 0, 1)],
    ["-x", Vector3.left],
    ["-y", Vector3.down],
    ["-z", new Vector3(0, 0, -1)],
  ];
  return axes
    .map(([name, axis]) => ({ name, axis, x: axis.dot(right), y: -axis.dot(up), depth: -axis.dot(forward) }))
    .sort((a, b) => a.depth - b.depth);
}
