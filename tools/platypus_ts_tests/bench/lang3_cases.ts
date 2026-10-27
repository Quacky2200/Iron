// SPDX-License-Identifier: Apache-2.0

// Cases of the TypeScript-semantics fixes: `flat(depth)` with a run-time depth, `this` in object
// literal methods and accessors, `Object.assign` into objects by key, string array parameters.
// Each case (split at `//## name`) defines `function t(): string` and is compared with bun.
//## flat_run_time_depth_two_levels
function show(xs: number[][], d: number): string {
  const r = xs.flat(d);
  let out = "";
  for (const x of r) {
    out += typeof x === "number" ? "n" + x + " " : "a" + x.length + " ";
  }
  return out;
}
function t(): string {
  const xs = [[1, 2], [3], []];
  return [show(xs, 0), show(xs, 1), show(xs, 5), show(xs, -3), show(xs, NaN), show(xs, 0.9), show(xs, 1.7), show(xs, Infinity)].join("|");
}
//## flat_run_time_depth_three_levels
function show(xs: number[][][], d: number): string {
  const r = xs.flat(d);
  let out = "";
  for (const x of r) {
    if (typeof x === "number") out += x + ";";
    else out += "[" + x.length + "]";
  }
  return r.length + ":" + out;
}
function count(xs: string[][][], d: number): number {
  let n = 0;
  for (const x of xs.flat(d)) {
    if (typeof x === "string") n += 100;
    else if (Array.isArray(x)) n += x.length;
  }
  return n;
}
function t(): string {
  const xs = [[[1, 2], [3]], [[4]], []];
  const ss = [[["a"], ["b", "c"]], [["d"]]];
  const parts: string[] = [];
  for (const d of [0, 1, 2, 3, -1, NaN, 1.5, Infinity, -Infinity]) {
    parts.push(show(xs, d));
    parts.push(count(ss, d) + "");
  }
  return parts.join(" ");
}
//## flat_constant_depths_keep_exact_types
function t(): string {
  const xs = [[[1, 2], [3]], [[4]]];
  const a: number[][] = xs.flat();
  const b: number[] = xs.flat(2);
  const c: number[][][] = xs.flat(0);
  const d: number[] = xs.flat(Infinity) as number[];
  return a.length + " " + b.join("") + " " + c.length + " " + d.length;
}
//## flat_cast_with_a_too_shallow_depth
function flatten(xs: number[][][], depth: number): number[] {
  return xs.flat(depth) as number[];
}
function t(): string {
  try {
    return flatten([[[1]]], 1).length + "";
  } catch (e) {
    return "threw:" + (e as Error).message;
  }
}
//## this_in_object_literal_methods_and_accessors
function t(): string {
  const o = {
    n: 1,
    get double() {
      return this.n * 2;
    },
    set double(v: number) {
      this.n = v / 2;
    },
    inc() {
      this.n++;
    },
    add(k: number): number {
      this.n += k;
      return this.n;
    },
    label: "x",
    get tag(): string {
      return this.label + this.n;
    },
  };
  const out: string[] = [];
  out.push(o.double + "");
  o.inc();
  out.push(o.n + "," + o.double);
  o.double = 20;
  out.push(o.n + "," + o.add(5) + "," + o.tag);
  o.label = "y";
  out.push(o.tag);
  o.double += 2;
  out.push(o.n + "");
  return out.join("|");
}
//## this_in_literal_methods_with_a_declared_type_and_nesting
interface Counter {
  count: number;
  step: number;
  bump(): number;
  twice(): void;
  readonly label: string;
}
function make(step: number): Counter {
  return {
    count: 0,
    step,
    bump() {
      this.count += this.step;
      return this.count;
    },
    twice() {
      this.bump();
      this.bump();
    },
    get label() {
      return "c" + this.count;
    },
  };
}
const shared = {
  total: 100,
  take(n: number) {
    this.total -= n;
    return this.total;
  },
  inner: {
    v: 7,
    get plus() {
      return this.v + 1;
    },
  },
};
function t(): string {
  const a = make(2);
  const b = make(10);
  a.bump();
  a.twice();
  b.twice();
  const fn = () => shared.take(30);
  const arrow = {
    k: 3,
    run() {
      const f = () => this.k * 2;
      return f();
    },
    list: [1, 2, 3],
    sum() {
      let s = 0;
      for (const x of this.list) s += x * this.k;
      return s;
    },
  };
  return `${a.count} ${b.count} ${a.label} ${b.label} ${fn()} ${shared.take(5)} ${shared.inner.plus} ${arrow.run()} ${arrow.sum()}`;
}
//## object_assign_a_dictionary_into_an_object
interface Settings {
  volume: number;
  speed: number;
  gain: number;
}
function show(s: Settings): string {
  return s.volume + "/" + s.speed + "/" + s.gain;
}
function load(entries: [string, number][]): Record<string, number> {
  const d: Record<string, number> = {};
  for (const [k, v] of entries) d[k] = v;
  return d;
}
function t(): string {
  const a: Settings = { volume: 1, speed: 2, gain: 3 };
  const d = load([["speed", 20], ["unknown", 99], ["gain", 30]]);
  const r = Object.assign(a, d);
  const b: Settings = { volume: 1, speed: 2, gain: 3 };
  Object.assign(b, { volume: 7 }, d, { gain: 0.5 });
  const c: Settings = { volume: 4, speed: 4, gain: 4 };
  Object.assign(c, load([]));
  return show(a) + " " + (r === a) + " " + show(b) + " " + show(c);
}
//## spread_a_dictionary_into_an_object_type_and_into_a_dictionary
interface Pose {
  x: number;
  y: number;
}
function mk(d: Record<string, number>): Pose {
  return { x: 0, y: 0, ...d };
}
function t(): string {
  const d: Record<string, number> = {};
  d["y"] = 5;
  d["w"] = 8;
  const p = mk(d);
  const copy = { ...d, z: 1 };
  copy["y"] = 6;
  const merged: Record<string, number> = { ...copy, ...{ q: 2 } };
  return p.x + "," + p.y + " " + Object.keys(copy).length + " " + d["y"] + " " + copy["y"] + " " + merged["z"] + merged["q"] + " " + Object.keys(merged).length;
}
//## object_keys_values_and_entries_of_objects
interface Box {
  w: number;
  h: number;
  d: number;
}
function t(): string {
  const b: Box = { w: 2, h: 3, d: 4 };
  const keys = Object.keys(b).join(",");
  const vals = Object.values(b).join(",");
  let pairs = "";
  for (const [k, v] of Object.entries(b)) pairs += k + "=" + v + ";";
  let area = 1;
  for (const k of Object.keys(b)) area *= b[k as keyof Box];
  return keys + " " + vals + " " + pairs + " " + area;
}
//## object_assign_a_dictionary_value_of_the_wrong_type
interface Named {
  title: string;
  n: number;
}
function t(): string {
  const o: Named = { title: "a", n: 1 };
  const d: Record<string, number> = { n: 2, title: 9 };
  try {
    Object.assign(o, d);
  } catch (e) {
    return "threw:" + (e as Error).message;
  }
  return o.title;
}
//## string_arrays_into_a_host_function
function t(): string {
  const words = ["alpha", "", "gamma"];
  const parts: string[] = [];
  parts.push(Words.join(words, "+"));
  parts.push(Words.count(words) + "");
  parts.push(Words.join([], "+") + "|" + Words.count([]));
  parts.push(Words.longest(words.map((w) => w + "!")));
  let extra = "";
  for (let i = 0; i < 40; i++) extra += "é";
  const many = [extra, extra + "ü", "x" + extra];
  parts.push(Words.longest(many).length + "");
  parts.push(Words.join(["one"], ",") + Words.count(["a", "b"]));
  return parts.join(" ");
}
