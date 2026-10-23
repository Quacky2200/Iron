// SPDX-License-Identifier: Apache-2.0

// encodeURI, encodeURIComponent, decodeURI, decodeURIComponent, escape, unescape. Every exported
// function without parameters returns a string; `gen_uri_expected.ts` runs this file in bun.

function fail(f: () => string): string {
  try {
    return f();
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message + " " + (e instanceof URIError);
  }
}

export function encode(): string {
  const xs = ["a b&c=d/é€😀;,?:@$+#", "http://x.y/a b?q=é&r=1#h;,/?:@&=+$-_.!~*'()", "", "plain", "ünïcödé", "\u0000\u007f\u0080߿ࠀ￿", "😀😀"];
  let out = "";
  for (const x of xs) out += encodeURIComponent(x) + " | " + encodeURI(x) + "\n";
  return out;
}

export function decode(): string {
  const xs = ["%E2%82%AC%20%F0%9F%98%80%2F%3F", "%E2%82%AC %2F%3F%23%3b%41", "plain", "%41%42%43", "%c3%a9", "a+b%2Bc", "%25%32%35", "", "%7E%7e"];
  let out = "";
  for (const x of xs) out += decodeURIComponent(x) + " | " + decodeURI(x) + "\n";
  return out;
}

export function roundTrip(): string {
  const xs = ["a b&c=d/é€😀;,?:@$+#", "日本語/テスト?x=1&y=ü", "😀 smile"];
  return xs.map((x) => decodeURIComponent(encodeURIComponent(x)) === x && decodeURI(encodeURI(x)) === x).join(",");
}

export function errors(): string {
  return [
    fail(() => encodeURIComponent("\uD800")),
    fail(() => encodeURI("\uDC00x")),
    fail(() => encodeURIComponent("a\uD83Db")),
    fail(() => decodeURI("%")),
    fail(() => decodeURI("%E2%82")),
    fail(() => decodeURIComponent("%C0%80")),
    fail(() => decodeURIComponent("%zz")),
    fail(() => decodeURIComponent("%ED%A0%80")),
    fail(() => decodeURIComponent("%F4%90%80%80")),
    fail(() => decodeURIComponent("100%")),
    fail(() => decodeURIComponent("%80")),
    fail(() => decodeURIComponent("%E2%82%2")),
  ].join("\n");
}

export function legacy(): string {
  return [escape("a b é€ä@*_+-./~!"), unescape("%u20AC%E9%41%zz%u12"), unescape(escape("日本 ü"))].join("|");
}
