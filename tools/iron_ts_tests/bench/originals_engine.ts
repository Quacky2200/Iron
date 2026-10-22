// Engine API shapes the original (QuickJS-era) example sources use, from compiled code. The Rust
// test (tests/all/originals_engine.rs) reads the same state back.

const free: LineRenderer2D[] = [];

export function handleArrays(): string {
  for (let i = 0; i < 3; i++) free.push(new GameObject(`l${i}`).addComponent(LineRenderer2D));
  const a = free.pop();
  const b = free.pop();
  free.pop();
  const none = free.pop();
  const first = free.shift();
  return `${a !== undefined && a !== null}|${b !== undefined}|${none === undefined}|${first === undefined}|${free.length}`;
}

export function lineRendererPoints(): string {
  const l = new GameObject("p").addComponent(LineRenderer2D);
  l.points = [new Vector2(1, 2), Vector2.zero, new Vector2(3, 4)];
  const n = l.points.length;
  const last = l.points[2]!;
  l.points = [];
  const empty = l.points.length === 0;
  return `${n}|${last.x}|${last.y}|${empty}|${l.pointCount}`;
}

// Compile-only: `playback.onEnded = f` is `playback.onEnded(f)`.
export function endedAssign(p: Playback, set: Set<Playback>): void {
  p.onEnded = () => set.delete(p);
}

export function colorFromArrays(): string {
  const rgba: [number, number, number, number] = [0.5, 0.25, 0.125, 0.75];
  const rgb = [1, 0.5, 0.25];
  const a = new Color(...rgba);
  const b = new Color(...rgb);
  return `${a.r}|${a.a}|${b.g}|${b.a}`;
}

export function partialFlatMesh(): string {
  const positions: number[] = [0, 0, 0, 1, 0, 0, 0, 1, 0];
  const triangles: number[] = [0, 1, 2];
  const m = Mesh.createFlat({ positions, triangles });
  const normals: number[] = [0, 0, 1, 0, 0, 1, 0, 0, 1];
  const n = Mesh.createFlat({ positions, triangles, normals });
  const o: { positions: number[]; triangles: number[]; uv?: number[] } = { positions, triangles };
  m.setFlat(o);
  return `${m.vertexCount}|${m.triangleCount}|${n.vertexCount}`;
}

const MODES: (FogMode | "off")[] = ["exponentialSquared", "exponential", "linear", "off"];
const state = { mode: MODES[0]! };

export function enumsMeetUnions(): string {
  const seen: string[] = [];
  for (let i = 0; i < 5; i++) {
    state.mode = MODES[(MODES.indexOf(state.mode) + 1) % MODES.length]!;
    if (state.mode !== "off") RenderSettings.fogMode = state.mode;
    seen.push(state.mode);
  }
  const back: FogMode = RenderSettings.fogMode;
  const same = back === "linear";
  const table: Record<FogMode | "off", string> = { linear: "L", exponential: "E", exponentialSquared: "S", off: "-" };
  return `${seen.join(",")}|${back}|${same}|${table[back]}`;
}

export function handlesInMaps(): string {
  const objects = new Map<string, GameObject>();
  const keyed = new Map<GameObject, number>();
  const a = new GameObject("alpha");
  const b = new GameObject("beta");
  objects.set("a", a);
  objects.set("b", b);
  keyed.set(a, 1);
  keyed.set(b, 2);
  const got = objects.get("b");
  const missing = objects.get("zzz");
  const sets = new Set<GameObject>([a]);
  sets.add(b);
  sets.add(a);
  let names = "";
  for (const o of sets) names += o.name;
  return `${got !== undefined && got.name}|${missing === undefined}|${keyed.get(a)! + keyed.get(b)!}|${sets.size}|${names}`;
}
