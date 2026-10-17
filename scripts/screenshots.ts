// Renders every example to tests/output/examples-<name>.png (git-ignored), for
// checking by eye. Run `bun run build` first so each example's dist/ is current.
//
//   bun run screenshots            all examples
//   bun run screenshots css form   just these

import { mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const output = join(root, "tests", "output");
mkdirSync(output, { recursive: true });

const all = readdirSync(join(root, "examples"), { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name);
const names = process.argv.length > 2 ? process.argv.slice(2) : all;

function run(cmd: string[]): boolean {
  const result = Bun.spawnSync(cmd, { cwd: root, stdout: "inherit", stderr: "inherit" });
  return result.exitCode === 0;
}

// The app binary isn't rebuilt by `cargo test`, so build it once up front.
if (!run(["cargo", "build", "-p", "iron_app"])) process.exit(1);
const exe = join(root, "target", "debug", process.platform === "win32" ? "iron_app.exe" : "iron_app");

let failed = 0;
for (const name of names) {
  const out = join(output, `examples-${name}.png`);
  const ok = run([exe, join("examples", name), "--screenshot", out, "--frames", "30"]);
  console.log(ok ? `wrote ${out}` : `failed: ${name}`);
  if (!ok) failed++;
}
process.exit(failed ? 1 : 0);
