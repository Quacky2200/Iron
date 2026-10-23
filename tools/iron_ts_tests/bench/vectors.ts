// SPDX-License-Identifier: Apache-2.0

// Vector2, Vector3, Vector4 and the integer grid vectors, compiled against the native API and run
// in bun against js/engine/src/math.ts. Every number is printed in full.

function s2(v: Vector2): string {
  return `(${v.x},${v.y})`;
}
function s3(v: Vector3): string {
  return `(${v.x},${v.y},${v.z})`;
}
function s4(v: Vector4): string {
  return `(${v.x},${v.y},${v.z},${v.w})`;
}

export function vector2(): string {
  const vs = [new Vector2(3, 4), new Vector2(-1.5, 0.25), new Vector2(0, 0), new Vector2(1e-3, -7)];
  let s = "";
  for (const a of vs) {
    for (const b of vs) {
      s += `${s2(a.add(b))}${s2(a.sub(b))}${s2(a.mul(b))}${a.dot(b)}${a.distance(b)}${s2(a.lerp(b, 0.3))}${s2(a.moveTowards(b, 1))}${a.equals(b)}${a.equals(b, 10)}${Vector2.angle(a, b)}${Vector2.signedAngle(a, b)};`;
    }
    s += `${s2(a.scale(2.5))}${a.magnitude}${a.sqrMagnitude}${s2(a.normalized)}${s2(a.rotate(33))}${s3(a.toVector3())}${s3(a.toVector3(9))}${a.toString()};`;
  }
  s += `${s2(Vector2.zero)}${s2(Vector2.one)}${s2(Vector2.up)}${s2(Vector2.down)}${s2(Vector2.left)}${s2(Vector2.right)}${s2(new Vector2())}${s2(new Vector2(5))}`;
  return s;
}

export function vector3(): string {
  const vs = [new Vector3(1, 2, 2), new Vector3(-1.5, 0.25, 8), new Vector3(0, 0, 0), new Vector3(1e-3, -7, 0.1)];
  let s = "";
  for (const a of vs) {
    for (const b of vs) {
      s += `${s3(a.add(b))}${s3(a.sub(b))}${s3(a.mul(b))}${s3(a.div(b))}${a.dot(b)}${s3(a.cross(b))}${a.distance(b)}`;
      s += `${s3(a.lerp(b, 0.3))}${s3(a.moveTowards(b, 1))}${s3(a.project(b))}${s3(a.projectOnPlane(b))}${s3(a.reflect(b.normalized))}`;
      s += `${a.equals(b)}${a.equals(b, 10)}${Vector3.angle(a, b)};`;
    }
    s += `${s3(a.scale(2.5))}${s3(a.negate())}${a.magnitude}${a.sqrMagnitude}${s3(a.normalized)}${s3(a.clampMagnitude(2))}${s3(a.clampMagnitude(100))}${s2(a.toVector2())}${a.toString()};`;
  }
  s += `${s3(Vector3.zero)}${s3(Vector3.one)}${s3(Vector3.up)}${s3(Vector3.down)}${s3(Vector3.left)}${s3(Vector3.right)}${s3(Vector3.forward)}${s3(Vector3.back)}${s3(new Vector3())}${s3(new Vector3(5, 6))}`;
  return s;
}

export function vector4(): string {
  const vs = [new Vector4(1, 2, 2, 4), new Vector4(-1.5, 0.25, 8, -2), new Vector4(0, 0, 0, 0)];
  let s = "";
  for (const a of vs) {
    for (const b of vs) {
      s += `${s4(a.add(b))}${s4(a.sub(b))}${s4(a.mul(b))}${a.dot(b)}${s4(a.lerp(b, 0.3))}${a.equals(b)}${a.equals(b, 100)};`;
    }
    s += `${s4(a.scale(2.5))}${a.magnitude}${s4(a.normalized)}${s3(a.toVector3())}${a.toString()};`;
  }
  s += `${s4(Vector4.zero)}${s4(Vector4.one)}${s4(Vector4.fromVector3(new Vector3(1, 2, 3)))}${s4(Vector4.fromVector3(new Vector3(1, 2, 3), 0))}${s4(new Vector4(1))}`;
  return s;
}

export function grid(): string {
  const a = new Vector2Int(3.9, -2.9);
  const b = new Vector2Int(1, 1);
  const c = new Vector3Int(1.5, -2.5, 3.5);
  const f = Vector2Int.floor(new Vector2(-0.5, 2.7));
  const r = Vector2Int.round(new Vector2(-0.5, 2.5));
  const f3 = Vector3Int.floor(new Vector3(-0.5, 2.7, 0.2));
  const r3 = Vector3Int.round(new Vector3(-0.5, 2.5, -2.5));
  let s = `${a.toString()}${a.add(b).toString()}${a.sub(b).toString()}${a.scale(2.5).toString()}${a.manhattan(b)}${a.equals(a)}${a.equals(b)}${s2(a.toVector2())};`;
  s += `${c.toString()}${c.add(new Vector3(1, 1, 1)).toString()}${c.sub(new Vector3(0.5, 0, 0)).toString()}${c.equals(c)}${s3(c.toVector3())};`;
  s += `${f.toString()}${r.toString()}${f3.toString()}${r3.toString()}`;
  s += `${Vector2Int.zero.toString()}${Vector2Int.one.toString()}${Vector2Int.up.toString()}${Vector2Int.down.toString()}${Vector2Int.left.toString()}${Vector2Int.right.toString()}${Vector3Int.zero.toString()}${Vector3Int.one.toString()}`;
  return s;
}
