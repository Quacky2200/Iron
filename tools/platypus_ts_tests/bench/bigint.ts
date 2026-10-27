// SPDX-License-Identifier: Apache-2.0

// BigInt: literals, operators, comparison, conversions, BigInt(), asIntN/asUintN. Every exported
// function without parameters returns a string; `gen_bigint_expected.ts` runs this file in bun.

function fail(f: () => string): string {
  try {
    return f();
  } catch (e) {
    return (e as Error).name + ": " + (e as Error).message;
  }
}

export function literals(): string {
  const xs: bigint[] = [0n, 123n, 0xffn, 0b101n, 0o17n, 1_000n, 123456789012345678901234567890n, -5n, 0x7fffffffffffffffn, 18446744073709551616n];
  return xs.map((x) => String(x)).join(",");
}

export function arithmetic(): string {
  const a = 123456789012345678901234567890n;
  const b = -987654321n;
  const c = 7n;
  return [
    a + b, a - b, a * b, a / b, a % b, b / c, b % c, -b / c, (-b) % c, a ** 3n, 2n ** 200n, (-3n) ** 3n, (-3n) ** 4n, 0n ** 0n, 5n ** 0n,
    -a, -b, a + a - a, (a * a) / a, 10n % -3n, -10n % 3n, -10n / 3n, 10n / -3n,
  ].map((x) => String(x)).join(",");
}

export function bitwise(): string {
  const xs: bigint[] = [0n, 5n, -3n, 255n, -256n, 12345678901234567890n, -12345678901234567890n];
  let out = "";
  for (const x of xs) {
    for (const y of xs) out += [x & y, x | y, x ^ y].map((v) => String(v)).join("/") + ";";
    out += [~x, x << 3n, x >> 3n, x << 100n, x >> 100n, x << -2n, x >> -2n].map((v) => String(v)).join("/") + "|";
  }
  return out;
}

export function comparison(): string {
  const big = [0n, 1n, -1n, 2n ** 64n, -(2n ** 64n), 5n];
  const nums = [0, 1, -1, 1.5, -1.5, 5, 5.5, 1e19, -1e19, NaN, Infinity, -Infinity, 18446744073709551616, 9007199254740993];
  let out = "";
  for (const a of big) {
    for (const b of big) out += [a < b, a <= b, a > b, a >= b, a == b, a != b, a === b, a !== b].map((v) => (v ? "1" : "0")).join("") + ",";
    out += "|";
    for (const n of nums) out += [a < n, a <= n, a > n, a >= n, a == n, a != n].map((v) => (v ? "1" : "0")).join("") + ",";
    out += "|";
    for (const n of nums) out += [n < a, n <= a, n > a, n >= a, n == a, n != a].map((v) => (v ? "1" : "0")).join("") + ",";
    out += "\n";
  }
  return out;
}

export function conversions(): string {
  const big = 2n ** 64n + 5n;
  return [
    String(BigInt(5)), String(BigInt(-5)), String(BigInt(1e21)), String(BigInt("0x1f")), String(BigInt(" 12 ")), String(BigInt("")), String(BigInt("-42")),
    String(BigInt("0b101")), String(BigInt("0o17")), String(BigInt(true)), String(BigInt(false)), String(BigInt(9007199254740993n)),
    String(Number(big)), String(Number(1n)), String(Number(-(2n ** 1100n))), String(Number(2n ** 53n + 1n)), String(Number(0n)),
    big.toString(), big.toString(2), big.toString(16), big.toString(36), (-255n).toString(16), "a" + 1n, `${big}`, String(big),
    typeof big, typeof 0n, String(big.valueOf()),
  ].join("|");
}

export function truthiness(): string {
  const xs: bigint[] = [0n, 1n, -1n, 0n - 0n, 5n - 5n];
  return xs.map((x) => (x ? "t" : "f")).join("") + (!0n) + (!1n) + (0n ? "a" : "b");
}

export function asN(): string {
  const xs: bigint[] = [255n, 256n, -1n, 127n, 128n, -129n, 2n ** 64n - 1n, 2n ** 63n, -(2n ** 63n) - 1n];
  let out = "";
  for (const x of xs) {
    out += [BigInt.asIntN(8, x), BigInt.asUintN(8, x), BigInt.asIntN(64, x), BigInt.asUintN(64, x), BigInt.asIntN(1, x), BigInt.asUintN(0, x), BigInt.asIntN(200, x)].map((v) => String(v)).join("/") + ";";
  }
  return out;
}

export function loops(): string {
  let fact = 1n;
  for (let i = 1n; i <= 30n; i++) fact *= i;
  let a = 0n;
  let b = 1n;
  for (let i = 0; i < 150; i++) {
    const t = a + b;
    a = b;
    b = t;
  }
  let down = 10n;
  while (down > 0n) down--;
  let sum = 0n;
  for (const x of [1n, 2n, 3n, 4n]) sum += x * x;
  const arr: bigint[] = [5n, 3n, 9n];
  arr.push(2n ** 70n);
  return [String(fact), String(a), String(down), String(sum), String(arr.length), String(arr[3])].join("|");
}

export function errors(): string {
  const zero = 0n;
  const neg = -1n;
  return [
    fail(() => String(BigInt(1.5))),
    fail(() => String(BigInt(NaN))),
    fail(() => String(BigInt(Infinity))),
    fail(() => String(BigInt("abc"))),
    fail(() => String(BigInt("1.5"))),
    fail(() => String(BigInt("0x"))),
    fail(() => String(5n / zero)),
    fail(() => String(5n % zero)),
    fail(() => String(2n ** neg)),
    fail(() => (5n).toString(1)),
    fail(() => (5n).toString(37)),
    fail(() => String(1n << 100000000000n)),
    fail(() => String(BigInt.asIntN(-1, 5n))),
  ].join("\n");
}
