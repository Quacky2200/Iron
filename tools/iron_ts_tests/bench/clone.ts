// structuredClone of data: nested objects and arrays, nullable fields, a recursive tree, typed
// arrays, independence of the copy. Every exported function without parameters returns a string;
// `gen_clone_expected.ts` runs this file in bun.

interface Point {
  x: number;
  y: number;
}

interface Shape {
  name: string;
  tags: string[];
  points: Point[];
  origin: Point;
  parent: Point | null;
  weight: number | null;
}

interface Tree {
  value: number;
  children: Tree[];
}

function sampleShape(): Shape {
  return {
    name: "tri",
    tags: ["a", "b"],
    points: [{ x: 1, y: 2 }, { x: 3, y: 4 }],
    origin: { x: 0, y: 0 },
    parent: { x: 9, y: 9 },
    weight: 2.5,
  };
}

function describe(s: Shape): string {
  const p = s.parent === null ? "none" : String(s.parent.x);
  const w = s.weight === null ? "none" : String(s.weight);
  return s.name + "," + s.tags.join("+") + "," + s.points.map((q) => q.x + ":" + q.y).join("/") + "," + s.origin.x + "," + p + "," + w;
}

export function equalCopy(): string {
  const a = sampleShape();
  const b = structuredClone(a);
  return describe(a) + "|" + describe(b) + "|" + (a === b) + "|" + (a.points === b.points) + "|" + (a.origin === b.origin);
}

export function copyIsIndependent(): string {
  const a = sampleShape();
  const b = structuredClone(a);
  b.name = "changed";
  b.tags.push("c");
  b.points[0].x = 100;
  b.origin.x = 7;
  if (b.parent !== null) b.parent.x = 50;
  b.weight = null;
  return describe(a) + "|" + describe(b);
}

export function nullFields(): string {
  const a = sampleShape();
  a.parent = null;
  a.weight = null;
  const b = structuredClone(a);
  return describe(b);
}

function sum(t: Tree): number {
  let n = t.value;
  for (const c of t.children) n += sum(c);
  return n;
}

export function recursiveTree(): string {
  const t: Tree = { value: 1, children: [{ value: 2, children: [{ value: 4, children: [] }] }, { value: 3, children: [] }] };
  const c = structuredClone(t);
  c.children[0].children[0].value = 40;
  c.children.push({ value: 5, children: [] });
  return sum(t) + "," + sum(c) + "," + t.children.length + "," + c.children.length;
}

export function arrays(): string {
  const grid: number[][] = [[1, 2], [3, 4], []];
  const g = structuredClone(grid);
  g[0][0] = 9;
  g[2].push(7);
  const words = structuredClone(["x", "y"]);
  words.push("z");
  return grid.map((r) => r.join(".")).join("/") + "|" + g.map((r) => r.join(".")).join("/") + "|" + words.join("");
}

export function typedArrays(): string {
  const a = new Float32Array([1, 2, 3]);
  const b = structuredClone(a);
  b[0] = 10;
  return a.join(",") + "|" + b.join(",") + "|" + (a === b);
}

export function primitives(): string {
  const n = structuredClone(42);
  const s = structuredClone("text");
  const t = structuredClone(true);
  const big = structuredClone(12345678901234567890n);
  return n + "," + s + "," + t + "," + big;
}

export function cloneUnderCollection(): string {
  let total = 0;
  for (let i = 0; i < 300; i++) {
    const s = sampleShape();
    s.points.push({ x: i, y: i });
    const c = structuredClone(s);
    c.points[2].x += 1;
    total += c.points[2].x + s.points[2].x + c.points.length;
  }
  return String(total);
}
