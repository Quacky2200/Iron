// SPDX-License-Identifier: Apache-2.0

// `boolean | undefined`: optional boolean fields and parameters, `??`, truthiness, and passing them on.
// Every exported function without parameters returns a string; `gen_nullbool_expected.ts` runs this
// in bun.

interface Opts {
  loop?: boolean;
  pingPong?: boolean;
  n?: number;
}

function flags(o: Opts): string {
  const a = o.loop ?? false;
  const b = o.pingPong ?? true;
  return (a ? "1" : "0") + (b ? "1" : "0") + (o.loop ? "T" : "F");
}

export function fields(): string {
  return [flags({}), flags({ loop: true }), flags({ loop: false }), flags({ pingPong: false, loop: true })].join(",");
}

function pass(v?: boolean): string {
  const w: boolean | undefined = v;
  if (w === undefined) return "u";
  return w ? "t" : "f";
}

export function params(): string {
  return pass() + pass(true) + pass(false);
}

class Box {
  on?: boolean;
  describe(): string {
    return this.on === undefined ? "unset" : this.on ? "on" : "off";
  }
}

export function members(): string {
  const b = new Box();
  const a = b.describe();
  b.on = true;
  const c = b.describe();
  b.on = false;
  return [a, c, b.describe()].join(",");
}
