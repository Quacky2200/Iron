// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The Scene view: an editor camera drawing the world into the panel's box (the panel
// is see-through, so the world shows under it), Unity's mouse and key controls
// (scene-camera.ts), the x/y/z orientation gizmo in the corner, SceneIcons (an icon at
// each camera, light, audio source and probe, at a constant size, that selects its
// object when clicked) and wire gizmos on the selection (frustums, light ranges and
// cones, probe boxes, audio distances) drawn by the renderer as thin lines. The active
// (last selected) object gets Unity's transform handles (Q view, W move, E rotate, R
// scale; math in handles.ts), drawn as SVG over the view.

import type { Rect as DockRect } from "./dock.ts";
import { onEditorFrame } from "./frame.ts";
import {
  beginDrag,
  dragTo,
  type HandleDrag,
  type HandlePart,
  type HandleShape,
  type HandleTool,
  type HandleView,
  handleShapes,
  pickHandle,
} from "./handles.ts";
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
  /** SceneIcons and the selection's wire gizmos (default on). */
  gizmos?: boolean;
  /** Where the camera starts. */
  initial?: SceneCameraState;
  /** After a handle drag: the object, and its transform before and after (for undo). */
  onTransformed?: (object: GameObject, before: ObjectPose, after: ObjectPose) => void;
}

/** An object's world position and rotation and its local scale. */
export interface ObjectPose {
  position: Vector3;
  rotation: Quaternion;
  scale: Vector3;
}

/** The Scene view's tools: `view` (Q) shows no handles. */
export type SceneTool = "view" | HandleTool;

type Drag =
  | { mode: "look" | "orbit" | "pan" | "click"; x: number; y: number; moved: boolean }
  | { mode: "handle"; x: number; y: number; moved: boolean; handle: HandleDrag; object: GameObject };

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

export function SceneView({ rect, selected, onSelect, grid = true, gizmos = true, initial, onTransformed }: SceneViewProps) {
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
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const [view, setView] = useState<{ state: SceneCameraState; icons: SceneIcon[] }>(() => ({ state: state.current, icons: [] }));
  const [tool, setTool] = useState<SceneTool>("move");
  const [hover, setHover] = useState<HandlePart | null>(null);
  const active = selected.length > 0 ? selected[selected.length - 1]! : null;
  const activeRef = useRef(active);
  activeRef.current = active && !active.destroyed ? active : null;

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
      if (gizmos) for (const go of selectedRef.current) drawGizmos(go);
      Gizmos.present(camera);
      const found = gizmos ? sceneIcons(camera, r) : [];
      // The active object's transform too: its handles follow it.
      const a = activeRef.current;
      const pose = a ? [a.transform.position, a.transform.rotation] : null;
      const key = JSON.stringify([state.current, pose, found.map((i) => [i.object.id, i.kind, Math.round(i.x), Math.round(i.y)])]);
      if (key !== last) {
        last = key;
        setView({ state: state.current, icons: found });
      }
    });
  }, [camera, gizmos]);

  const set = (next: SceneCameraState) => {
    state.current = next;
    place(camera, next, rectRef.current);
  };

  // The handles of the active object, as the camera sees them now.
  const handleView = (): HandleView => handleViewOf(camera, rectRef.current);
  const handlesNow = (): { shapes: HandleShape[]; object: GameObject } | null => {
    const a = activeRef.current;
    if (tool === "view" || !a || !gizmos || !a.activeInHierarchy) return null;
    const v = handleView();
    if (v.toPanel(a.transform.position).z <= camera.nearClipPlane) return null;
    return { shapes: handleShapes(tool, v, frameOf(a)), object: a };
  };

  const onPointerDown = (e: PlatypusPointerEvent) => {
    e.currentTarget?.focus();
    const r = rectRef.current;
    if (e.button === 0 && !e.altKey && tool !== "view") {
      const h = handlesNow();
      const part = h ? pickHandle(h.shapes, e.clientX - r.x, e.clientY - r.y) : null;
      if (h && part) {
        const t = h.object.transform;
        const pose = { position: t.position, rotation: t.rotation, scale: t.localScale };
        const handle = beginDrag(tool, part, handleView(), pose, frameOf(h.object), e.clientX - r.x, e.clientY - r.y);
        drag.current = { mode: "handle", x: e.clientX, y: e.clientY, moved: false, handle, object: h.object };
        e.currentTarget?.setPointerCapture(e.pointerId);
        return;
      }
    }
    const mode = e.button === 2 ? "look" : e.button === 1 ? "pan" : e.button === 0 ? (e.altKey ? "orbit" : "click") : null;
    if (!mode) return;
    drag.current = { mode, x: e.clientX, y: e.clientY, moved: false };
    e.currentTarget?.setPointerCapture(e.pointerId);
  };
  const onPointerMove = (e: PlatypusPointerEvent) => {
    const d = drag.current;
    if (!d) {
      // Hovering lights up the part that a press would grab.
      const h = handlesNow();
      const r = rectRef.current;
      const part = h ? pickHandle(h.shapes, e.clientX - r.x, e.clientY - r.y) : null;
      if (part !== hover) setHover(part);
      return;
    }
    if (d.mode === "handle") {
      const r = rectRef.current;
      const next = dragTo(d.handle, handleView(), e.clientX - r.x, e.clientY - r.y, e.ctrlKey);
      apply(d.object, next);
      drag.current = { ...d, moved: true };
      return;
    }
    const dx = e.clientX - d.x;
    const dy = e.clientY - d.y;
    if (dx === 0 && dy === 0) return;
    drag.current = { ...d, x: e.clientX, y: e.clientY, moved: true };
    if (d.mode === "look") set(look(state.current, dx, dy));
    else if (d.mode === "orbit") set(orbit(state.current, dx, dy));
    else if (d.mode === "pan") set(pan(state.current, dx, dy, rectRef.current.height));
  };
  const onPointerUp = (e: PlatypusPointerEvent) => {
    const d = drag.current;
    drag.current = null;
    e.currentTarget?.releasePointerCapture(e.pointerId);
    if (d?.mode === "handle" && d.moved && !d.object.destroyed) {
      const t = d.object.transform;
      onTransformed?.(d.object, d.handle.start, { position: t.position, rotation: t.rotation, scale: t.localScale });
    }
    // A click on nothing clears the selection.
    if (d?.mode === "click" && !d.moved) onSelect?.(null, e.ctrlKey || e.shiftKey);
  };
  const onWheel = (e: PlatypusWheelEvent) => {
    e.preventDefault();
    set(zoom(state.current, e.deltaY));
  };
  const onKeyDown = (e: PlatypusKeyboardEvent) => {
    const tools: Record<string, SceneTool> = { KeyQ: "view", KeyW: "move", KeyE: "rotate", KeyR: "scale" };
    // Not while flying: then W and E move the camera.
    if (tools[e.code] && !drag.current && !e.ctrlKey && !e.altKey) {
      setTool(tools[e.code]!);
      e.preventDefault();
      return;
    }
    if (e.code === "KeyF" && !drag.current) {
      const target = framing(selected);
      if (target) set(frame(state.current, target.center, target.radius));
      e.preventDefault();
    }
  };

  return (
    <box
      className="platypus-scene-view"
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
          className={`platypus-scene-icon ${icon.kind}${selected.includes(icon.object) ? " selected" : ""}`}
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
      <Handles handles={handlesNow()} tool={tool} rect={rect} hot={drag.current?.mode === "handle" ? drag.current.handle.part : hover} />
      <box className="platypus-scene-tools" onPointerDown={(e) => e.stopPropagation()}>
        {TOOLS.map(([t, label, key]) => (
          <box key={t} className={t === tool ? "platypus-scene-tool active" : "platypus-scene-tool"} onClick={() => setTool(t)}>
            <Text>{`${label} (${key})`}</Text>
          </box>
        ))}
      </box>
      <OrientationGizmo state={view.state} onLook={(dir) => set(lookAlong(state.current, dir))} onToggle={() => set({ ...state.current, orthographic: !state.current.orthographic })} />
    </box>
  );
}

const TOOLS: [SceneTool, string, string][] = [
  ["view", "View", "Q"],
  ["move", "Move", "W"],
  ["rotate", "Rotate", "E"],
  ["scale", "Scale", "R"],
];

/** The handles' frame for an object: its place, turned with it (Unity's Local). */
function frameOf(go: GameObject) {
  return { position: go.transform.position, rotation: go.transform.rotation };
}

function apply(go: GameObject, pose: ObjectPose): void {
  if (go.destroyed) return;
  const t = go.transform;
  if (!t.position.equals(pose.position)) t.position = pose.position;
  const r = t.rotation;
  const q = pose.rotation;
  if (r.x !== q.x || r.y !== q.y || r.z !== q.z || r.w !== q.w) t.rotation = q;
  if (!t.localScale.equals(pose.scale)) t.localScale = pose.scale;
}

/** The handles' view of a camera drawing into `r`: panel pixels to and from the world. */
function handleViewOf(camera: Camera, r: DockRect): HandleView {
  const forward = camera.transform.forward;
  const eye = camera.transform.position;
  const height = Math.max(1, r.height);
  return {
    toPanel: (p) => {
      const s = camera.worldToScreenPoint(p);
      return new Vector3(s.x - r.x, s.y - r.y, s.z);
    },
    ray: (x, y) => camera.screenPointToRay({ x: x + r.x, y: y + r.y }),
    unitsPerPixel: (p) =>
      camera.orthographic
        ? (2 * camera.orthographicSize) / height
        : (Math.max(1e-3, p.sub(eye).dot(forward)) * 2 * Math.tan(((camera.fieldOfView / 2) * Math.PI) / 180)) / height,
    forward,
  };
}

const HANDLE_COLORS: Record<HandlePart, string> = { x: "#e5484d", y: "#7ac74f", z: "#3d8ef0", center: "#d9d9d9" };
const HOT = "#f2d24b";

/** The handles as SVG, in a box just round them (so only that much is rasterized). */
function Handles({ handles, tool, rect, hot }: { handles: { shapes: HandleShape[] } | null; tool: SceneTool; rect: DockRect; hot: HandlePart | null }) {
  if (!handles) return null;
  const { shapes } = handles;
  const points = shapes.flatMap((s) => [...s.lines.flat(), ...s.back.flat()]);
  if (points.length === 0) return null;
  const pad = 16;
  const x0 = Math.max(-pad, Math.min(...points.map((p) => p.x)) - pad);
  const y0 = Math.max(-pad, Math.min(...points.map((p) => p.y)) - pad);
  const x1 = Math.min(rect.width + pad, Math.max(...points.map((p) => p.x)) + pad);
  const y1 = Math.min(rect.height + pad, Math.max(...points.map((p) => p.y)) + pad);
  if (x1 <= x0 || y1 <= y0) return null;
  const w = Math.ceil(x1 - x0);
  const h = Math.ceil(y1 - y0);
  const pts = (line: { x: number; y: number }[]) => line.map((p) => `${(p.x - x0).toFixed(1)},${(p.y - y0).toFixed(1)}`).join(" ");
  return (
    <box className="platypus-scene-handles" style={{ left: x0, top: y0, width: w, height: h }}>
      <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`}>
        {shapes.map((s) => {
          const color = s.part === hot ? HOT : HANDLE_COLORS[s.part];
          if (s.part === "center") {
            const c = s.lines[0]![0]!;
            return <rect key="center" x={c.x - x0 - 5} y={c.y - y0 - 5} width={10} height={10} fill={color} stroke="#222" strokeWidth={1} />;
          }
          return (
            <g key={s.part}>
              {s.back.map((line, i) => (
                <polyline key={`b${i}`} points={pts(line)} fill="none" stroke={color} strokeWidth={1.5} opacity={0.3} />
              ))}
              {s.lines.map((line, i) => (
                <polyline key={`f${i}`} points={pts(line)} fill="none" stroke={color} strokeWidth={s.part === hot ? 3.5 : 2.5} />
              ))}
              {tool === "move" || tool === "scale" ? <Tip line={s.lines[0]!} color={color} x0={x0} y0={y0} square={tool === "scale"} /> : null}
            </g>
          );
        })}
      </svg>
    </box>
  );
}

/** An arrow head (move) or a square (scale) at the end of an axis line. */
function Tip({ line, color, x0, y0, square }: { line: { x: number; y: number }[]; color: string; x0: number; y0: number; square: boolean }) {
  const [a, b] = [line[0]!, line[line.length - 1]!];
  const len = Math.hypot(b.x - a.x, b.y - a.y);
  if (len < 4) return null;
  const dx = (b.x - a.x) / len;
  const dy = (b.y - a.y) / len;
  const bx = b.x - x0;
  const by = b.y - y0;
  if (square) return <rect x={bx - 4.5} y={by - 4.5} width={9} height={9} fill={color} />;
  const p = (along: number, side: number) => `${(bx + dx * along - dy * side).toFixed(1)},${(by + dy * along + dx * side).toFixed(1)}`;
  return <polygon points={`${p(10, 0)} ${p(-2, 5)} ${p(-2, -5)}`} fill={color} />;
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

const LIGHT_LINE = new Color(1, 0.9, 0.45, 0.9);

/** The wire shapes Unity draws for a selected object's components. */
export function drawGizmos(go: GameObject): void {
  if (go.destroyed || !go.activeInHierarchy) return;
  const t = go.transform;
  const placed = Matrix4x4.trs(t.position, t.rotation);
  const camera = go.getComponent(Camera);
  if (camera && (go.hideFlags & HideFlags.HideInHierarchy) === 0) {
    Gizmos.color = new Color(0.85, 0.85, 0.85, 0.9);
    Gizmos.matrix = placed;
    const r = camera.pixelRect;
    const aspect = r.width > 0 && r.height > 0 ? r.width / r.height : 16 / 9;
    const near = camera.nearClipPlane;
    const far = camera.farClipPlane;
    if (camera.orthographic) {
      const h = camera.orthographicSize;
      Gizmos.drawWireCube(new Vector3(0, 0, -(near + far) / 2), new Vector3(2 * h * aspect, 2 * h, far - near));
    } else {
      Gizmos.drawFrustum(Vector3.zero, camera.fieldOfView, far, near, aspect);
    }
  }
  const light = go.getComponent(Light);
  if (light) {
    Gizmos.color = LIGHT_LINE;
    Gizmos.matrix = placed;
    if (light.type === "point") {
      Gizmos.drawWireSphere(Vector3.zero, light.range);
    } else if (light.type === "spot") {
      const radius = light.range * Math.tan(((light.spotAngle / 2) * Math.PI) / 180);
      const end = new Vector3(0, 0, -light.range);
      Gizmos.drawWireDisc(end, Vector3.back, radius);
      for (const [x, y] of [[1, 0], [-1, 0], [0, 1], [0, -1]] as const) {
        Gizmos.drawLine(Vector3.zero, end.add(new Vector3(x * radius, y * radius, 0)));
      }
    } else {
      // Rays along the light, round a small disc.
      Gizmos.drawWireDisc(Vector3.zero, Vector3.back, 0.5);
      for (let i = 0; i < 8; i++) {
        const a = (i / 8) * Math.PI * 2;
        const p = new Vector3(Math.cos(a) * 0.5, Math.sin(a) * 0.5, 0);
        Gizmos.drawRay(p, new Vector3(0, 0, -2));
      }
    }
  }
  const probe = go.getComponent(ReflectionProbe);
  if (probe) {
    Gizmos.color = new Color(0.8, 0.6, 1, 0.9);
    // Probe boxes stay lined up with the world.
    Gizmos.matrix = Matrix4x4.trs(t.position, Quaternion.identity);
    Gizmos.drawWireCube(Vector3.zero, probe.size);
  }
  const audio = go.getComponent(AudioSource);
  if (audio) {
    Gizmos.color = new Color(0.5, 0.8, 1, 0.7);
    Gizmos.matrix = Matrix4x4.trs(t.position, Quaternion.identity);
    Gizmos.drawWireSphere(Vector3.zero, audio.minDistance);
    Gizmos.drawWireSphere(Vector3.zero, audio.maxDistance);
  }
  Gizmos.matrix = Matrix4x4.identity;
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
    <box className="platypus-scene-gizmo" style={{ width: GIZMO, height: GIZMO + 16 }} onPointerDown={(e) => e.stopPropagation()}>
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
          className="platypus-scene-gizmo-hit"
          style={{ left: c + a.x * reach - 9, top: c + a.y * reach - 9, width: 18, height: 18 }}
          onClick={() => onLook(a.axis.negate())}
        />
      ))}
      <box className="platypus-scene-gizmo-hit" style={{ left: c - 7, top: c - 7, width: 14, height: 14 }} onClick={onToggle} />
      <box className="platypus-scene-gizmo-label">
        <Text>{state.orthographic ? "Iso" : "Persp"}</Text>
      </box>
    </box>
  );
}
