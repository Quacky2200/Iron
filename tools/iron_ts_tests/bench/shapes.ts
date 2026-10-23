// SPDX-License-Identifier: Apache-2.0

// Rect, RectInt, Ray, Plane, Bounds and Pose, compiled against the native API and run in bun
// against js/engine/src/math.ts. Every number is printed in full.

function s2(v: Vector2): string {
  return `(${v.x},${v.y})`;
}
function s3(v: Vector3): string {
  return `(${v.x},${v.y},${v.z})`;
}
function sq(q: Quaternion): string {
  return `(${q.x},${q.y},${q.z},${q.w})`;
}

export function rects(): string {
  const a = new Rect(1, 2, 10, 5);
  const b = new Rect(8, 6, 4, 4);
  const c = new Rect(20, 20, 1, 1);
  const m = Rect.minMax(new Vector2(-1, -2), new Vector2(3, 4));
  const i = new RectInt(1.9, 2.9, 10.5, 5.5);
  let s = `${a.xMin},${a.yMin},${a.xMax},${a.yMax},${s2(a.center)},${s2(a.size)},`;
  s += `${a.contains(new Vector2(1, 2))},${a.contains(new Vector2(11, 7))},${a.contains(new Vector2(5, 5))},`;
  s += `${a.overlaps(b)},${a.overlaps(c)},${b.overlaps(a)},${m.x},${m.y},${m.width},${m.height},`;
  s += `${i.x},${i.y},${i.width},${i.height},${i.xMax},${s2(i.center)},${i.contains(new Vector2(2, 3))},${i.overlaps(a)},`;
  const e = new Rect();
  s += `${e.width},${e.xMax}`;
  return s;
}

export function rays(): string {
  const r = new Ray(new Vector3(1, 2, 3), new Vector3(0, 3, 4));
  const p = new Plane(new Vector3(0, 2, 0), -4);
  const q = Plane.fromPoint(new Vector3(1, 1, 0), new Vector3(2, 0, 0));
  const t = Plane.fromPoints(new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(0, 0, -1));
  let s = `${s3(r.origin)}${s3(r.direction)}${s3(r.getPoint(2.5))};`;
  s += `${s3(p.normal)}${p.distance}${q.distance}${s3(q.normal)}${s3(t.normal)}${t.distance};`;
  const pts = [new Vector3(0, 0, 0), new Vector3(1, 5, 2), new Vector3(-3, 2, 0.5)];
  for (const pt of pts) {
    s += `${p.getDistanceToPoint(pt)}${p.getSide(pt)}${s3(p.closestPointOnPlane(pt))}${s3(p.projectVector(pt))};`;
  }
  s += `${p.sameSide(pts[0], pts[1])}${p.sameSide(pts[1], pts[2])}${p.sameSide(pts[0], pts[2])};`;
  const down = new Ray(new Vector3(0, 10, 0), new Vector3(0, -1, 0));
  const up = new Ray(new Vector3(0, 10, 0), new Vector3(0, 1, 0));
  const parallel = new Ray(new Vector3(0, 10, 0), new Vector3(1, 0, 0));
  s += `${p.raycast(down)}${s3(p.intersect(down))}${p.raycast(up)}${p.raycast(parallel)};`;
  const f = p.flipped;
  const m = p.translate(new Vector3(0, 1, 0));
  s += `${s3(f.normal)}${f.distance}${s3(m.normal)}${m.distance}`;
  return s;
}

export function bounds(): string {
  const b = new Bounds(new Vector3(1, 2, 3), new Vector3(4, 2, 6));
  const mm = Bounds.minMax(new Vector3(-1, -1, -1), new Vector3(3, 1, 5));
  const o = new Bounds(new Vector3(4, 2, 3), new Vector3(2, 2, 2));
  const far = new Bounds(new Vector3(100, 0, 0), new Vector3(1, 1, 1));
  let s = `${s3(b.center)}${s3(b.size)}${s3(b.extents)}${s3(b.min)}${s3(b.max)}${s3(mm.center)}${s3(mm.size)};`;
  s += `${b.contains(new Vector3(1, 2, 3))}${b.contains(new Vector3(3, 3, 6))}${b.contains(new Vector3(3.1, 3, 6))}${b.intersects(o)}${b.intersects(far)};`;
  const g = b.encapsulate(new Vector3(10, -5, 3)).encapsulate(new Vector3(0, 0, 20));
  s += `${s3(g.center)}${s3(g.size)}${s3(g.min)}${s3(g.max)}`;
  return s;
}

export function poses(): string {
  const a = new Pose(new Vector3(1, 2, 3), Quaternion.euler(0, 90, 0));
  const b = new Pose(new Vector3(-4, 0, 1), Quaternion.euler(30, 0, 45));
  let s = `${s3(a.position)}${sq(a.rotation)}${s3(a.forward)}${s3(a.right)}${s3(a.up)};`;
  const c = a.transformedBy(b);
  s += `${s3(c.position)}${sq(c.rotation)};`;
  const p = new Vector3(1, 0, 0);
  const w = a.transformPoint(p);
  s += `${s3(w)}${s3(a.inverseTransformPoint(w))};`;
  const l = a.lerp(b, 0.3);
  s += `${s3(l.position)}${sq(l.rotation)};`;
  const i = Pose.identity;
  s += `${s3(i.position)}${sq(i.rotation)}`;
  return s;
}
