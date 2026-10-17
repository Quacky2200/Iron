// The Scene view: an editor camera drawing the world into the panel's box (the panel
// is see-through, so the world shows under it), Unity's mouse and key controls
// (scene-camera.ts), the x/y/z orientation gizmo in the corner, and SceneIcons: an
// icon at each camera, light, audio source and probe, at a constant size, that selects
// its object when clicked.

import { AudioSource, Camera, Color, GameObject, HideFlags, input, Light, Rect, ReflectionProbe, Screen, Time, Vector3 } from "@iron/engine";
import { type IronKeyboardEvent, type IronPointerEvent, type IronWheelEvent, Text } from "@iron/react";
import { useEffect, useRef, useState } from "react";

import type { Rect as DockRect } from "./dock.ts";
import { onEditorFrame } from "./frame.ts";
import {
  defaultSceneCamera,
  eyePosition,
  fly,
  frame,
  gizmoAxes,
  look,
  lookAlong,
  orbit,
  orthographicSize,
  pan,
  SCENE_FOV,
  type SceneCameraState,
  sceneRotation,
  zoom,
} from "./scene-camera.ts";

export interface SceneViewProps {
  /** The panel's content box on screen, in UI pixels. */
  rect: DockRect;
  /** Selected objects (outlined icons, framed by F). */
  selected: readonly GameObject[];
  /** A click on an icon (or on nothing: `null`); `additive` with Ctrl or Shift. */
  onSelect?: (object: GameObject | null, additive: boolean) => void;
  /** The ground grid (default on). */
  grid?: boolean;
  /** SceneIcons (default on). */
  icons?: boolean;
  /** Where the camera starts. */
  initial?: SceneCameraState;
}

type Drag = { mode: "look" | "orbit" | "pan" | "click"; x: number; y: number; moved: boolean };

/** A SceneIcon: what it stands for and where it is in the panel. */
export interface SceneIcon {
  object: GameObject;
  kind: "camera" | "directional" | "point" | "spot" | "audio" | "probe";
  x: number;
  y: number;
}

const LIGHT = "#f2cc5a";

/** The icons, drawn in a 16 × 16 box (fixed markup, so each is rasterized once). */
function IconArt({ kind }: { kind: SceneIcon["kind"] }) {
  const art = (() => {
    switch (kind) {
      case "camera":
        return (
          <>
            <rect x={1.5} y={4.5} width={9} height={7} rx={1.5} fill="#8ab4f8" />
            <polygon points="10.5,8 15,5 15,11" fill="#8ab4f8" />
          </>
        );
      case "directional":
        return (
          <>
            <circle cx={8} cy={8} r={3.2} fill={LIGHT} />
            <path d="M8 1v2.5M8 12.5V15M1 8h2.5M12.5 8H15M3 3l1.8 1.8M11.2 11.2 13 13M13 3l-1.8 1.8M4.8 11.2 3 13" stroke={LIGHT} strokeWidth={1.4} strokeLinecap="round" />
          </>
        );
      case "point":
        return (
          <>
            <path d="M8 1.5a4.5 4.5 0 0 0-2.6 8.2V11.5h5.2V9.7A4.5 4.5 0 0 0 8 1.5z" fill={LIGHT} />
            <rect x={5.6} y={12.4} width={4.8} height={1.6} rx={0.6} fill={LIGHT} />
          </>
        );
      case "spot":
        return (
          <>
            <rect x={5.5} y={1.5} width={5} height={3.5} rx={1} fill={LIGHT} />
            <polygon points="6,5.5 10,5.5 14.5,14.5 1.5,14.5" fill={LIGHT} opacity={0.75} />
          </>
        );
      case "audio":
        return (
          <>
            <polygon points="2,6 5,6 9,2.5 9,13.5 5,10 2,10" fill="#7fc6a4" />
            <path d="M11 5.5a3.5 3.5 0 0 1 0 5M12.8 3.5a6.3 6.3 0 0 1 0 9" stroke="#7fc6a4" strokeWidth={1.3} fill="none" strokeLinecap="round" />
          </>
        );
      case "probe":
        return (
          <>
            <circle cx={8} cy={8} r={5.5} fill="#c792ea" />
            <ellipse cx={8} cy={8} rx={5.5} ry={2} fill="none" stroke="#3a2a48" strokeWidth={1} />
            <circle cx={6.2} cy={5.8} r={1.4} fill="#efe2fa" />
          </>
        );
    }
  })();
  return (
    <svg width={16} height={16} viewBox="0 0 16 16">
      {art}
    </svg>
  );
}

/** Icon size in UI pixels. */
const ICON = 22;
/** The orientation gizmo's size. */
const GIZMO = 84;

/** The icon an object gets, if any (the first of its components that has one). */
export function iconKind(go: GameObject): SceneIcon["kind"] | null {
  const light = go.getComponent(Light);
  if (light) return light.type;
  if (go.getComponent(Camera)) return "camera";
  if (go.getComponent(AudioSource)) return "audio";
  if (go.getComponent(ReflectionProbe)) return "probe";
  return null;
}

/** Every object, depth first, leaving out hidden ones (and what's under them). */
export function visibleObjects(): GameObject[] {
  const out: GameObject[] = [];
  const stack = [...GameObject.roots].reverse();
  while (stack.length) {
    const go = stack.pop()!;
    if (go.destroyed || (go.hideFlags & HideFlags.HideInHierarchy) !== 0) continue;
    out.push(go);
    for (let i = go.children.length - 1; i >= 0; i--) stack.push(go.children[i]!);
  }
  return out;
}

export function SceneView({ rect, selected, onSelect, grid = true, icons = true, initial }: SceneViewProps) {
  const [cameraObject] = useState(() => {
    const go = new GameObject("Scene Camera");
    go.hideFlags = HideFlags.HideAndDontSave;
    GameObject.dontDestroyOnLoad(go);
    return go;
  });
  const [camera] = useState(() =>
    cameraObject.addComponent(Camera, { clearFlags: "skybox", backgroundColor: new Color(0.19, 0.2, 0.22, 1), depth: -100 }),
  );
  const state = useRef<SceneCameraState>(initial ?? defaultSceneCamera());
  const drag = useRef<Drag | null>(null);
  const rectRef = useRef(rect);
  rectRef.current = rect;
  const [view, setView] = useState<{ state: SceneCameraState; icons: SceneIcon[] }>(() => ({ state: state.current, icons: [] }));

  useEffect(() => () => cameraObject.destroy(), [cameraObject]);
  useEffect(() => {
    camera.sceneGrid = grid;
  }, [camera, grid]);

  // Every frame: fly while right-dragging, place the camera, and update the icons
  // (React only hears of it when something changed).
  useEffect(() => {
    let last = "";
    return onEditorFrame(() => {
      const r = rectRef.current;
      if (drag.current?.mode === "look") {
        const key = (code: string) => (input.getKey(code) ? 1 : 0);
        const move = new Vector3(key("KeyD") - key("KeyA"), key("KeyE") - key("KeyQ"), key("KeyW") - key("KeyS"));
        if (move.sqrMagnitude > 0) {
          state.current = fly(state.current, move, Time.unscaledDeltaTime, input.getKey("ShiftLeft") || input.getKey("ShiftRight"));
        }
      }
      place(camera, state.current, r);
      const found = icons ? sceneIcons(camera, r) : [];
      const key = JSON.stringify([state.current, found.map((i) => [i.object.id, i.kind, Math.round(i.x), Math.round(i.y)])]);
      if (key !== last) {
        last = key;
        setView({ state: state.current, icons: found });
      }
    });
  }, [camera, icons]);

  const set = (next: SceneCameraState) => {
    state.current = next;
    place(camera, next, rectRef.current);
  };

  const onPointerDown = (e: IronPointerEvent) => {
    const mode: Drag["mode"] | null =
      e.button === 2 ? "look" : e.button === 1 ? "pan" : e.button === 0 ? (e.altKey ? "orbit" : "click") : null;
    if (!mode) return;
    drag.current = { mode, x: e.clientX, y: e.clientY, moved: false };
    e.currentTarget?.setPointerCapture();
    e.currentTarget?.focus();
  };
  const onPointerMove = (e: IronPointerEvent) => {
    const d = drag.current;
    if (!d) return;
    const dx = e.clientX - d.x;
    const dy = e.clientY - d.y;
    if (dx === 0 && dy === 0) return;
    drag.current = { ...d, x: e.clientX, y: e.clientY, moved: true };
    if (d.mode === "look") set(look(state.current, dx, dy));
    else if (d.mode === "orbit") set(orbit(state.current, dx, dy));
    else if (d.mode === "pan") set(pan(state.current, dx, dy, rectRef.current.height));
  };
  const onPointerUp = (e: IronPointerEvent) => {
    const d = drag.current;
    drag.current = null;
    e.currentTarget?.releasePointerCapture();
    // A click on nothing clears the selection.
    if (d?.mode === "click" && !d.moved) onSelect?.(null, e.ctrlKey || e.shiftKey);
  };
  const onWheel = (e: IronWheelEvent) => {
    e.preventDefault();
    set(zoom(state.current, e.deltaY));
  };
  const onKeyDown = (e: IronKeyboardEvent) => {
    if (e.code === "KeyF" && !drag.current) {
      const target = framing(selected);
      if (target) set(frame(state.current, target.center, target.radius));
      e.preventDefault();
    }
  };

  return (
    <box
      className="iron-scene-view"
      style={{ width: rect.width, height: rect.height }}
      tabIndex={-1}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onWheel={onWheel}
      onKeyDown={onKeyDown}
      onContextMenu={(e) => e.preventDefault()}
    >
      {view.icons.map((icon) => (
        <box
          key={icon.object.id}
          className={`iron-scene-icon ${icon.kind}${selected.includes(icon.object) ? " selected" : ""}`}
          style={{ left: icon.x - ICON / 2, top: icon.y - ICON / 2, width: ICON, height: ICON }}
          onPointerDown={(e) => {
            if (e.button !== 0 || e.altKey) return;
            e.stopPropagation();
            onSelect?.(icon.object, e.ctrlKey || e.shiftKey);
          }}
        >
          <IconArt kind={icon.kind} />
        </box>
      ))}
      <OrientationGizmo state={view.state} onLook={(dir) => set(lookAlong(state.current, dir))} onToggle={() => set({ ...state.current, orthographic: !state.current.orthographic })} />
    </box>
  );
}

/** Puts the editor camera where the state says, drawing into `r` (nothing when empty). */
function place(camera: Camera, s: SceneCameraState, r: DockRect): void {
  const t = camera.transform;
  t.position = eyePosition(s);
  t.rotation = sceneRotation(s);
  if (camera.orthographic !== s.orthographic) camera.orthographic = s.orthographic;
  if (s.orthographic) camera.orthographicSize = orthographicSize(s);
  if (camera.fieldOfView !== SCENE_FOV) camera.fieldOfView = SCENE_FOV;
  const near = s.orthographic ? 0.1 : Math.max(0.01, s.distance * 0.002);
  const far = s.orthographic ? Math.max(s.distance * 8, 1000) : Math.max(1000, s.distance * 200);
  if (Math.abs(camera.nearClipPlane - near) > near * 0.1) camera.nearClipPlane = near;
  if (Math.abs(camera.farClipPlane - far) > far * 0.1) camera.farClipPlane = far;
  const sw = Math.max(1, Screen.width);
  const sh = Math.max(1, Screen.height);
  const next = new Rect(r.x / sw, r.y / sh, r.width / sw, r.height / sh);
  const cur = camera.rect;
  if (cur.x !== next.x || cur.y !== next.y || cur.width !== next.width || cur.height !== next.height) camera.rect = next;
}

/** The icons in view, in panel coordinates. */
function sceneIcons(camera: Camera, r: DockRect): SceneIcon[] {
  const out: SceneIcon[] = [];
  if (r.width <= 0 || r.height <= 0) return out;
  for (const go of visibleObjects()) {
    if (!go.activeInHierarchy) continue;
    const kind = iconKind(go);
    if (!kind) continue;
    const p = camera.worldToScreenPoint(go.transform.position);
    if (p.z <= camera.nearClipPlane) continue;
    const x = p.x - r.x;
    const y = p.y - r.y;
    if (x < -ICON || y < -ICON || x > r.width + ICON || y > r.height + ICON) continue;
    out.push({ object: go, kind, x, y });
  }
  return out;
}

/** A sphere around the selection to frame: their positions, padded. */
function framing(selected: readonly GameObject[]): { center: Vector3; radius: number } | null {
  const live = selected.filter((g) => !g.destroyed);
  if (live.length === 0) return null;
  let min = live[0]!.transform.position;
  let max = min;
  for (const g of live) {
    const p = g.transform.position;
    min = new Vector3(Math.min(min.x, p.x), Math.min(min.y, p.y), Math.min(min.z, p.z));
    max = new Vector3(Math.max(max.x, p.x), Math.max(max.y, p.y), Math.max(max.z, p.z));
  }
  const scale = live[0]!.transform.localScale;
  const size = Math.max(Math.abs(scale.x), Math.abs(scale.y), Math.abs(scale.z), 1);
  return { center: min.add(max).scale(0.5), radius: max.sub(min).magnitude / 2 + size * 0.75 };
}

const AXIS_COLORS: Record<string, string> = { x: "#e5484d", y: "#7ac74f", z: "#3d8ef0" };

/**
 * Unity's scene gizmo: the six axes as the camera sees them, the positive ones labelled.
 * Clicking an axis looks along it, towards the centre; the centre toggles perspective
 * and orthographic.
 */
function OrientationGizmo({ state, onLook, onToggle }: { state: SceneCameraState; onLook: (dir: Vector3) => void; onToggle: () => void }) {
  const c = GIZMO / 2;
  const reach = GIZMO * 0.32;
  const axes = gizmoAxes(state);
  return (
    <box className="iron-scene-gizmo" style={{ width: GIZMO, height: GIZMO + 16 }} onPointerDown={(e) => e.stopPropagation()}>
      <svg width={GIZMO} height={GIZMO} viewBox={`0 0 ${GIZMO} ${GIZMO}`}>
        {axes.map((a) => {
          const color = AXIS_COLORS[a.name.replace("-", "")]!;
          const x = c + a.x * reach;
          const y = c + a.y * reach;
          const positive = !a.name.startsWith("-");
          return (
            <g key={a.name} opacity={a.depth < -0.3 ? 0.55 : 1}>
              {positive ? <line x1={c} y1={c} x2={x} y2={y} stroke={color} strokeWidth={3} /> : null}
              <circle cx={x} cy={y} r={positive ? 8 : 5} fill={positive ? color : "#9aa0a6"} stroke={positive ? "none" : color} strokeWidth={1.5} />
              {positive ? (
                <text x={x} y={y + 3.5} textAnchor="middle" fontSize={10} fontWeight="bold" fill="#111">
                  {a.name.toUpperCase()}
                </text>
              ) : null}
            </g>
          );
        })}
        <rect x={c - 5} y={c - 5} width={10} height={10} rx={2} fill="#d9d9d9" />
      </svg>
      {axes.map((a) => (
        <box
          key={a.name}
          className="iron-scene-gizmo-hit"
          style={{ left: c + a.x * reach - 9, top: c + a.y * reach - 9, width: 18, height: 18 }}
          onClick={() => onLook(a.axis.negate())}
        />
      ))}
      <box className="iron-scene-gizmo-hit" style={{ left: c - 7, top: c - 7, width: 14, height: 14 }} onClick={onToggle} />
      <box className="iron-scene-gizmo-label">
        <Text>{state.orthographic ? "Iso" : "Persp"}</Text>
      </box>
    </box>
  );
}
