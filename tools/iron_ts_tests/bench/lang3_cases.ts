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
