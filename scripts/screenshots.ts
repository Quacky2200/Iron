// Renders every example to tests/output/examples-<name>.png (git-ignored), and the
// editor (`iron_editor`) to tests/output/editor.png, for checking by eye. Run
// `iron-build build <example>` (or the editor's `bun run build`) first so each dist/ is current.
//
//   bun run screenshots            all examples and the editor
//   bun run screenshots css form   just these (`editor` for the editor)

import { mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const output = join(root, "tests", "output");
mkdirSync(output, { recursive: true });

const all = readdirSync(join(root, "examples"), { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .concat("editor");
const names = process.argv.length > 2 ? process.argv.slice(2) : all;

function run(cmd: string[]): boolean {
  const result = Bun.spawnSync(cmd, { cwd: root, stdout: "inherit", stderr: "inherit" });
  return result.exitCode === 0;
}

// The app binaries aren't rebuilt by `cargo test`, so build them once up front.
if (!run(["cargo", "build", "-p", "iron_app"])) process.exit(1);
const bin = (name: string) => join(root, "target", "debug", process.platform === "win32" ? `${name}.exe` : name);

let failed = 0;
for (const name of names) {
  const editor = name === "editor";
  const out = join(output, editor ? "editor.png" : `examples-${name}.png`);
  const game = editor ? [] : [join("examples", name)];
  const ok = run([bin(editor ? "iron_editor" : "iron_app"), ...game, "--screenshot", out, "--frames", "30"]);
  console.log(ok ? `wrote ${out}` : `failed: ${name}`);
  if (!ok) failed++;
}
process.exit(failed ? 1 : 0);
