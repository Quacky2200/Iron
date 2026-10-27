// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// JSON in compiled code, typed from the program's static types. Every exported function without
// parameters returns a string; `gen_json_expected.ts` runs this file in bun and `tests/json.rs`
// compares.

interface Meta {
  ok: boolean;
  note: string | null;
}

interface Item {
  id: number;
  name: string;
  tags: string[];
  price?: number;
  meta: Meta;
}

class Base {
  kind = "base";
  id = 1;
}

class Derived extends Base {
  extra = "x";
}

type Mode = "idle" | "run";

interface State {
  mode: Mode;
  xs: number[];
  nested: { a: number }[];
}

export function stringifyBasic(): string {
  const it: Item = {
    id: 1,
    name: 'A "quoted" \n name\t\u0001',
    tags: ["x", "y"],
    meta: { ok: true, note: null },
  };
  const parts = [
    JSON.stringify(it),
    JSON.stringify(it, null, 2),
    JSON.stringify([1, 2.5, -0, NaN, 1e21, 1e-7]),
    JSON.stringify("s \ud800é😀"),
    JSON.stringify(true),
    JSON.stringify(12),
    JSON.stringify([]),
    JSON.stringify({}),
    JSON.stringify([[], [1]], null, "--"),
    JSON.stringify(it, null, 20),
  ];
  return parts.join("\n");
}

export function stringifyUnionsAndNesting(): string {
  const s: State = { mode: "run", xs: [1, 2], nested: [{ a: 1 }, { a: 2 }] };
  return JSON.stringify(s) + JSON.stringify(s, null, 1);
}

export function polymorphic(): string {
  const items: Base[] = [new Base(), new Derived()];
  return JSON.stringify(items);
}

export function optionalFieldsAreOmitted(): string {
  const a: Item = { id: 1, name: "n", tags: [], meta: { ok: false, note: "z" } };
  const b: Item = { id: 2, name: "m", tags: ["t"], price: 2.5, meta: { ok: false, note: null } };
  return JSON.stringify(a) + JSON.stringify(b);
}

export function roundTrip(): string {
  const text =
    '{"id":5,"name":"n\\u00e9\\ud83d\\ude00","tags":["a","b"],"price":9.5,"meta":{"ok":false,"note":"hi"}}';
  const it: Item = JSON.parse(text);
  const again = JSON.stringify(it);
  const second = JSON.parse(again) as Item;
  return (
    it.id + "|" + it.name + "|" + it.tags.join("+") + "|" + it.meta.ok + "|" + it.meta.note + "|" +
    (it.price ?? -1) + "|" + again + "|" + second.name.length
  );
}

/** Fields the type does not have are checked as JSON and then ignored. */
export function unknownFieldsAreSkipped(): string {
  const it = JSON.parse(
    '{"zzz":[1,{"a":"b"},null],"id":3,"name":"n","tags":[],"qqq":{"x":{"y":[]}},"meta":{"ok":true,"note":null}}',
  ) as Item;
  return it.id + it.name + it.tags.length + it.meta.ok + it.meta.note;
}

export function parseValues(): string {
  const nums = JSON.parse("[1, -2.5e3, 0, 1E2, 0.5, 123456789012345680000]") as number[];
  const strs = JSON.parse('["a\\nb", "\\"q\\"", "\\\\", "\\/", "\\u0041"]') as string[];
  const flags = JSON.parse(" [ true , false ] ") as boolean[];
  const maybe = JSON.parse("[1, null, 3]") as (number | null)[];
  return (
    nums.join(",") + "|" + strs.join("~") + "|" + flags.join(",") + "|" + maybe.map((x) => x ?? -1).join(",")
  );
}

export function parseUnions(): string {
  const s = JSON.parse('{"mode":"idle","xs":[],"nested":[{"a":3}]}') as State;
  return s.mode + s.xs.length + s.nested[0]!.a;
}

const BAD: string[] = [
  "{", "[1,", "abc", '{"a":}', "", "1 2", "{'a':1}", "[1,]", "01", '{"a":1,}', '"abc', "[1 2]", "tru",
  "nul", '{"a" 1}', "1e", "-", '"\\x"', '"a\nb"', '{"a":1}}', "[", '{"a"', "nan", "[1,2", '{"a":1',
  '"\\u12"', "+1", ".5", "1.", "[}", "@", "#", ")", "(", ";", "/", "*", "=", "<", "`", '"\\',
  "true false", "nulll", "truex", "[1}", '{"a":1]', "{1:2}", '{"a":1 "b":2}', "é", "a1", "_x", "$",
  "-x", "-01", "1.5e+", "[01]", "tru e", "{,}", "[,]", ":", "NaN", "Infinity", "undefined", "'x'",
  '"\\u00zz"', "1 ",
];

export function parseErrors(): string {
  let out = "";
  for (const s of BAD) {
    try {
      JSON.parse(s) as number[];
      out += "ok;";
    } catch (e) {
      out += (e as Error).name + ":" + (e as Error).message + ";";
    }
  }
  return out;
}

export function manyObjects(): string {
  const items: Item[] = [];
  for (let i = 0; i < 300; i++) {
    items.push({ id: i, name: "item" + i, tags: ["a" + i], meta: { ok: i % 2 === 0, note: i % 3 === 0 ? null : "n" + i } });
  }
  const text = JSON.stringify(items);
  const back = JSON.parse(text) as Item[];
  let total = 0;
  for (const it of back) total += it.id + it.name.length + it.tags.length + (it.meta.ok ? 1 : 0);
  return total + ":" + text.length + ":" + (JSON.stringify(back) === text);
}

/** Data that does not have the shape of the type: a TypeError here (JavaScript would carry on). */
export function shapeMismatches(): string {
  const texts = [
    "[1]",
    '{"id":"x","name":"n","tags":[],"meta":{"ok":true,"note":null}}',
    '{"id":1}',
    "[1",
  ];
  let out = "";
  for (const t of texts) {
    try {
      JSON.parse(t) as Item;
      out += "ok;";
    } catch (e) {
      out += (e as Error).name + ":" + (e as Error).message + ";";
    }
  }
  try {
    JSON.parse('{"mode":"zzz","xs":[],"nested":[]}') as State;
  } catch (e) {
    out += (e as Error).name + ":" + (e as Error).message + ";";
  }
  return out;
}

function jsonTextOfItems(n: number): string {
  let s = "[";
  for (let i = 0; i < n; i++) {
    s += (i ? "," : "") + '{"id":' + i + ',"name":"item' + i + '","tags":["a","b"],"meta":{"ok":true,"note":null}}';
  }
  return s + "]";
}

/** `JSON.parse` of a call result: the fresh text is only an operand while the parser allocates. */
export function parseOfCall(): string {
  const items = JSON.parse(jsonTextOfItems(50)) as Item[];
  let total = 0;
  for (const it of items) total += it.id + it.name.length;
  return items.length + ":" + total;
}
