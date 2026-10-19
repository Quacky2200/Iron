// Runs every exported function of date.ts in bun, once per time zone, and writes what they return.
//   bun bench/gen_date_expected.ts
// The same zone list is in tests/date.rs.
export const ZONES = ["America/New_York", "Europe/London", "Asia/Kolkata", "Australia/Lord_Howe", "UTC"];
const out: Record<string, unknown> = {};
for (const zone of ZONES) {
  process.env.TZ = zone;
  const m = await import("./date.ts?" + zone);
  for (const [name, fn] of Object.entries(m)) {
    if (typeof fn !== "function") continue;
    (globalThis as any).__bench_reset?.(); // a program that keeps state starts each function fresh
  const v = (fn as () => unknown)();
    out[name + "@" + zone] = typeof v === "string" ? { s: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) } : v;
  }
}
await Bun.write(import.meta.dir + "/date_expected.json", JSON.stringify(out));
console.log("wrote", Object.keys(out).length, "results");
