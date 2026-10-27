// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Quaternion and Matrix4x4, compiled against the native API and run in bun against
// js/engine/src/math.ts. Every number is printed in full.

function s3(v: Vector3): string {
  return `(${v.x},${v.y},${v.z})`;
}
function sq(q: Quaternion): string {
  return `(${q.x},${q.y},${q.z},${q.w})`;
}
function sm(m: Matrix4x4): string {
  let s = "[";
  for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) s += `${m.get(r, c)},`;
  return s + "]";
}

export function construction(): string {
  const a = new Quaternion();
  const b = new Quaternion(1, 2, 3, 4);
  let s = `${sq(a)}${sq(b)}${sq(Quaternion.identity)}`;
  s += sq(Quaternion.angleAxis(90, new Vector3(0, 1, 0)));
  s += sq(Quaternion.angleAxis(33.3, new Vector3(1, 2, 3)));
  s += sq(Quaternion.angleAxis(45, new Vector3(0, 0, 0)));
  s += sq(Quaternion.euler(10, 20, 30));
  s += sq(Quaternion.euler(new Vector3(-45, 170, 5)));
  s += sq(Quaternion.euler(0, 0, 0));
  return s;
}

export function products(): string {
  const qs = [
    Quaternion.euler(10, 20, 30),
    Quaternion.euler(-80, 135, 7),
    Quaternion.angleAxis(180, new Vector3(1, 1, 0)),
    Quaternion.identity,
  ];
  const vs = [new Vector3(1, 0, 0), new Vector3(0.3, -2, 5), new Vector3(0, 0, 0)];
  let s = "";
  for (const a of qs) {
    for (const b of qs) {
      s += `${sq(a.mul(b))}${Quaternion.angle(a, b)}${sq(a.slerp(b, 0.4))}${sq(a.slerp(b, 1.5))}${sq(a.rotateTowards(b, 10))}${sq(a.rotateTowards(b, 400))};`;
    }
    s += `${sq(a.inverse())}${sq(a.normalized)}${s3(a.eulerAngles)};`;
    for (const v of vs) s += s3(a.rotate(v));
    s += "|";
  }
  s += `${sq(new Quaternion(0, 0, 0, 0).normalized)}${sq(new Quaternion(2, 0, 0, 0).normalized)}`;
  return s;
}

export function euler(): string {
  let s = "";
  for (const x of [-90, -30, 0, 45, 89, 90]) {
    for (const y of [0, 100, 200, 359]) {
      for (const z of [0, 60, -170]) {
        s += s3(Quaternion.euler(x, y, z).eulerAngles);
      }
    }
  }
  return s;
}

export function directions(): string {
  const dirs = [new Vector3(0, 0, -1), new Vector3(1, 0, 0), new Vector3(0, 1, 0), new Vector3(0, -1, 0), new Vector3(1, 1, 1), new Vector3(-0.2, 0.7, -3), new Vector3(0, 0, 0), new Vector3(0, 0, 1)];
  let s = "";
  for (const d of dirs) {
    s += `${sq(Quaternion.lookRotation(d))}${sq(Quaternion.lookRotation(d, new Vector3(1, 0, 0)))};`;
    for (const e of dirs) s += sq(Quaternion.fromToRotation(d, e));
    s += "|";
  }
  return s;
}

export function matrices(): string {
  const t = Matrix4x4.trs(new Vector3(1, 2, 3), Quaternion.euler(10, 20, 30), new Vector3(2, 0.5, 3));
  const u = Matrix4x4.trs(new Vector3(-4, 0, 1), Quaternion.euler(-30, 5, 100));
  let s = `${sm(Matrix4x4.identity)}${sm(t)}${sm(u)}${sm(t.mul(u))}${sm(u.mul(t))}`;
  s += `${sm(t.inverse)}${sm(t.transpose)}${t.determinant}${u.determinant}${sm(t.mul(t.inverse))}`;
  s += `${sm(Matrix4x4.perspective(60, 16 / 9, 0.1, 100))}${sm(Matrix4x4.orthographic(-8, 8, -4.5, 4.5, 0.1, 50))}`;
  const singular = Matrix4x4.trs(new Vector3(0, 0, 0), Quaternion.identity, new Vector3(1, 0, 1));
  s += `${singular.determinant}${sm(singular.inverse)}`;
  return s;
}

export function transformingPoints(): string {
  const t = Matrix4x4.trs(new Vector3(1, 2, 3), Quaternion.euler(10, 20, 30), new Vector3(2, 0.5, 3));
  const p = Matrix4x4.perspective(60, 1.5, 0.1, 100);
  const v = new Vector3(0.5, -1, -4);
  let s = `${s3(t.multiplyPoint(v))}${s3(t.multiplyPoint3x4(v))}${s3(t.multiplyVector(v))}`;
  s += `${s3(p.multiplyPoint(v))}${s3(p.multiplyPoint3x4(v))}${s3(p.multiplyVector(v))}`;
  const v4 = t.multiplyVector4(new Vector4(1, 2, 3, 1));
  s += `(${v4.x},${v4.y},${v4.z},${v4.w})`;
  const w = Matrix4x4.fromColumns(new Vector4(1, 2, 3, 4), new Vector4(5, 6, 7, 8), new Vector4(9, 10, 11, 12), new Vector4(13, 14, 15, 16));
  s += `${sm(w)}${w.get(0, 3)}${w.get(3, 0)}${w.get(1, 1)}`;
  return s;
}
