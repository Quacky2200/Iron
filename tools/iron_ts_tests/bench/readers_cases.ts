// SPDX-License-Identifier: Apache-2.0

// Cases from the md-reader and launcher examples (merged from `md-reader-example`): TypeScript the
// compiler learnt for them, each compared with bun. Each case (split at `//## name`) defines
// `function t(): string`.
//## intersection_with_a_recursive_union
type Inline = { kind: "text"; text: string } | { kind: "em"; children: Inline[] };
type Block = BlockKind & { line?: number };
type BlockKind =
  | { kind: "para"; children: Inline[] }
  | { kind: "quote"; children: Block[] }
  | { kind: "rule" };
function words(xs: Inline[]): number {
  let n = 0;
  for (const x of xs) n += x.kind === "text" ? x.text.split(" ").length : words(x.children);
  return n;
}
function weight(b: Block): number {
  switch (b.kind) {
    case "rule":
      return 1;
    case "para":
      return words(b.children);
    case "quote":
      return b.children.reduce((n, c) => n + weight(c), 0);
  }
}
function t(): string {
  const doc: Block[] = [
    { kind: "para", children: [{ kind: "text", text: "a b c" }], line: 0 },
    { kind: "quote", children: [{ kind: "rule" }, { kind: "para", children: [{ kind: "em", children: [{ kind: "text", text: "d e" }] }] }] },
  ];
  return doc.map((b) => `${b.kind}:${weight(b)}:${b.line ?? "-"}`).join(" ");
}
//## a_tag_test_narrows_a_field_of_another_type
type Node = { kind: "leaf"; items: string[] } | { kind: "branch"; items: Node[] };
function count(n: Node): number {
  if (n.kind === "leaf") return n.items.length;
  let c = 0;
  for (const k of n.items) c += count(k);
  return c;
}
function first(n: Node): string {
  if (n.kind !== "branch") return n.items[0] ?? "none";
  return "branch of " + n.items.length;
}
function t(): string {
  const tree: Node = { kind: "branch", items: [{ kind: "leaf", items: ["a", "b"] }, { kind: "leaf", items: [] }] };
  return `${count(tree)} ${first(tree)} ${first({ kind: "leaf", items: ["x"] })}`;
}
//## filter_boolean_and_boolean_calls
function t(): string {
  const words = " a  b c ".split(/\s+/).filter(Boolean);
  const nums = [0, 1, 2, NaN, 3].filter(Boolean);
  return `${words.join(",")}|${nums.join(",")}|${Boolean("")}${Boolean("x")}${Boolean(0)}${Boolean(7)}`;
}
//## function_declarations_inside_a_body_are_hoisted
function collect(xs: number[]): string {
  const out: number[] = [];
  let i = 0;
  while (i < xs.length) {
    if (xs[i]! < 0) return finish();
    out.push(xs[i]! * 2);
    i++;
  }
  return finish();

  function finish() {
    return out.join(",") + "@" + i;
  }
}
function t(): string {
  return collect([1, 2, 3]) + " " + collect([4, -1, 5]);
}
//## or_null_and_a_string_or_a_number
function arg(args: string[], flag: string): string | undefined {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
}
function t(): string {
  const args = ["--steps", "4", "--name", ""];
  const name = arg(args, "--name") || null;
  const steps = Number(arg(args, "--steps") ?? 1);
  const fallback = Number(arg(args, "--none") ?? 1);
  const missing = Number(arg(args, "--none"));
  return `${name === null} ${steps} ${fallback} ${Number.isNaN(missing)}`;
}
//## closures_infer_results_and_default_parameter_types
function t(): string {
  const index = 3;
  const value = (args: string[], name: string) => {
    const i = args.indexOf(name);
    if (i < 0) return null;
    return args[i + 1] ?? null;
  };
  const pick = (i = index) => i * 10;
  return `${value(["a", "b"], "a")} ${value(["a"], "z")} ${pick()} ${pick(1)}`;
}
//## negative_number_literal_types
function step(at: number, by: -1 | 1): number {
  return at + by;
}
function t(): string {
  return `${step(5, -1)} ${step(5, 1)}`;
}
//## sets_and_maps_copied_from_others
function t(): string {
  const a = new Set<string>(["x", "y"]);
  const b = new Set(a);
  b.add("z");
  const m = new Map<string, number>([["one", 1]]);
  const n = new Map(m);
  n.set("two", 2);
  return `${a.size} ${b.size} ${[...b].join("")} ${m.size} ${n.get("one")} ${n.get("two")}`;
}
//## find_over_a_tuple_of_literals
type Pad = "xbox" | "playstation";
function t(): string {
  const want = "playstation";
  const found = (["xbox", "playstation"] as const).find((s) => s === want);
  const none = (["xbox", "playstation"] as const).find((s) => s === "nintendo");
  const style: Pad = found ? found : "xbox";
  return `${found} ${none === undefined} ${style}`;
}
//## optional_index_of_a_match
function at(re: RegExp, src: string, i: number): string | null {
  re.lastIndex = i;
  return re.exec(src)?.[0] ?? null;
}
function matchAt(re: RegExp, src: string): RegExpExecArray | null {
  return re.exec(src);
}
function t(): string {
  const m = matchAt(/(\d+)/, "ab12");
  return `${at(/\s+/y, "a  b", 1)?.length} ${at(/\s+/y, "a  b", 0)} ${m ? m[1] : "none"} ${matchAt(/x/, "ab") === null}`;
}
//## strings_that_mention_code_keep_their_text
class Spot {
  x = 1;
}
function t(): string {
  const doc = "Use `Temporal.PlainDate` or `win.request<{ n: number }>(1)`; class Foo is in a string.";
  const s = new Spot();
  return doc + " " + s.x;
}
//## literal_unions_from_and_to_strings_that_may_be_missing
type Mode = "browse" | "details";
function label(m: Mode | null): string {
  const s: string | null = m;
  return s ?? "none";
}
function mode(s: string | undefined): Mode | null {
  return s === "details" ? s : null;
}
function t(): string {
  const keys: Record<string, Mode> = { Enter: "details" };
  const a = keys["Enter"];
  const set = new Set<Mode>(["details"]);
  return `${label("details")} ${label(null)} ${mode("details")} ${mode(undefined)} ${a && set.has(a)}`;
}
//## a_generic_function_infers_a_closure_type
function id<F>(f: F): F {
  return f;
}
function t(): string {
  const twice = id((x: number) => x * 2);
  const greet = id((name: string, n: number) => name.repeat(n));
  return `${twice(21)} ${greet("ab", 2)}`;
}
//## a_copy_where_a_nullable_type_goes_keeps_null
interface Mark {
  at: number;
  ref: { current: number };
}
function read(m: Mark | null): string {
  if (!m) return "none";
  m.ref.current += 1;
  return "at" + m.at;
}
function t(): string {
  const ref = { current: 0 };
  const marks = [0, 1].map((i) => (i === 1 ? { at: i, faded: false, ref } : null));
  return `${read(marks[0]!)} ${read(marks[1]!)} ${ref.current}`;
}
//## clear_timeout_takes_undefined
function t(): string {
  let timer: ReturnType<typeof setTimeout> | undefined;
  clearTimeout(timer);
  clearInterval(undefined);
  return "ok";
}
