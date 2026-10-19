// Error cause and AggregateError. Every exported function without parameters returns a string;
// `gen_errors2_expected.ts` runs this file in bun.

export function causeChain(): string {
  const low = new RangeError("low level");
  const mid = new Error("mid", { cause: low });
  const top = new TypeError("top", { cause: mid });
  let out = "";
  let e: Error | null = top;
  while (e) {
    out += e.name + ":" + e.message + ">";
    e = e.cause;
  }
  return out + (new Error("none").cause == null) + (new Error("x", {}).cause == null);
}

export function causeInCatch(): string {
  try {
    try {
      throw new SyntaxError("inner");
    } catch (inner) {
      throw new Error("outer", { cause: inner as Error });
    }
  } catch (e) {
    const err = e as Error;
    return err.message + "<-" + (err.cause ? err.cause.message : "none") + (err.cause instanceof SyntaxError);
  }
}

export function aggregate(): string {
  const a = new AggregateError([new Error("one"), new RangeError("two")], "several");
  const b = new AggregateError([], "");
  const c = new AggregateError([new TypeError("t")], "with cause", { cause: new Error("why") });
  return [
    a.name, a.message, String(a.errors.length), a.errors.map((x) => x.name + ":" + x.message).join("/"), String(a), String(b.errors.length), String(b),
    c.cause ? c.cause.message : "none", String(a instanceof Error), String(a instanceof AggregateError), String(a instanceof RangeError),
  ].join("|");
}

export function aggregateCaught(): string {
  try {
    throw new AggregateError([new Error("x"), new Error("y")], "all failed");
  } catch (e) {
    if (e instanceof AggregateError) return e.message + ":" + e.errors.map((x) => x.message).join(",");
    return "not aggregate";
  }
}
