// String methods that take a regular expression: match, matchAll, search, replace, replaceAll,
// split. Every exported function without parameters returns a string; `gen_regexp2_expected.ts`
// runs this file in bun and `tests/regexp2.rs` compares.

export function search(): string {
  return (
    "hello world".search(/o/) + "," + "hello".search(/z/) + "," + "a1b2".search(/\d/) + "," +
    "héllo wörld".search(/w/) + "," + "x.y".search(".") + "," + "abc".search("c")
  );
}

export function matchPlain(): string {
  const m = "tel 555-1234 end".match(/(\d+)-(\d+)/);
  const none = "abc".match(/\d/);
  if (!m) return "no match";
  return m[0] + "|" + m[1] + "|" + m[2] + "|" + m.index + "|" + (none === null);
}

export function matchGlobal(): string {
  const all = "a1 b22 c333".match(/\d+/g);
  const none = "abc".match(/\d/g);
  const re = /a/g;
  re.lastIndex = 5;
  const reset = "banana".match(re);
  return (all ? all.join("+") + "/" + all.length : "null") + "|" + (none === null) + "|" +
    (reset ? reset.length : -1) + "|" + re.lastIndex;
}

export function matchString(): string {
  const m = "a.b".match(".");
  return m ? m[0] + m.index : "null";
}

export function matchAll(): string {
  let out = "";
  for (const m of "a1b22c333".matchAll(/([a-z])(\d+)/g)) {
    out += m[0] + ":" + m[1] + ":" + m[2] + ":" + m.index + ";";
  }
  const empty = [..."abc".matchAll(/x*/g)].map((m) => m.index).join(",");
  return out + empty;
}

export function matchAllNeedsG(): string {
  try {
    "abc".matchAll(/b/);
    return "no error";
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function replaceString(): string {
  return (
    "a-b-c".replace("-", "+") + "|" + "a.b".replace(".", "[$&]") + "|" + "abc".replace("x", "y") +
    "|" + "aaa".replaceAll("a", "bb") + "|" + "a.b.c".replaceAll(".", "$$") + "|" +
    "x".replace("x", "$'$`$$")
  );
}

export function replaceRegex(): string {
  return (
    "John Smith".replace(/(\w+)\s(\w+)/, "$2, $1") + "|" +
    "aaa".replace(/a/, "b") + "|" + "aaa".replace(/a/g, "b") + "|" +
    "abc".replace(/(?<x>b)/, "[$<x>]") + "|" + "abc".replace(/b/, "$0$1") + "|" +
    "abc".replace(/(b)/, "$01$10$2") + "|" + "abc".replace(/b/, "$&$&$`$'") + "|" +
    "abc".replace(/x*/g, "-") + "|" + "héllo".replace(/l/g, "L") + "|" + "aaa".replaceAll(/a/g, "x")
  );
}

export function replaceFunction(): string {
  const doubled = "a1b22".replace(/\d+/g, (m) => String(Number(m) * 2));
  const pair = "k=v;x=y".replace(/(\w)=(\w)/g, (m, k, v) => v + "=" + k);
  const where = "abcabc".replace(/b/g, (m, offset, whole) => offset + whole.length + "");
  const first = "abab".replace(/b/, (m) => m.toUpperCase());
  const str = "a-b".replace("-", (m) => "[" + m + "]");
  const word = "hi you".replace(/\w+/g, (w) => w.charAt(0).toUpperCase() + w.slice(1));
  return doubled + "|" + pair + "|" + where + "|" + first + "|" + str + "|" + word;
}

export function replaceFunctionGroups(): string {
  const re = /(\d+)(x)?/g;
  const got = "1x 22".replace(re, (m, a, b) => "<" + a + "," + (b ?? "none") + ">");
  const named = "2026-10".replace(/(?<y>\d+)-(?<m>\d+)/, (m, y, mo, off, whole, g) => g!.m + g!.y + off + whole);
  return got + "|" + named + "|" + re.lastIndex;
}

export function replaceAllNeedsG(): string {
  try {
    "abc".replaceAll(/b/, "x");
    return "no error";
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function splitRegex(): string {
  return (
    "a1b2c3".split(/\d/).join("|") + "#" + "a, b ,c".split(/\s*,\s*/).join("|") + "#" +
    "abc".split(/(b)/).join("|") + "#" + "abc".split(/(?:)/).join("|") + "#" +
    "a1b2c3".split(/\d/, 2).join("|") + "#" + "".split(/x/).length + "#" + "".split(/(?:)/).length +
    "#" + "one  two".split(/ /).length + "#" + "😀a".split(/(?:)/u).length + "#" +
    "A1b".split(/(\d)/).length
  );
}

export function splitString(): string {
  return "a,b,c".split(",").join("|") + "#" + "abc".split("").length + "#" + "a,b,c".split(",", 2).length;
}
