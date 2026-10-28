// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Dev builds with React Fast Refresh. Run in a game's folder (its `bun run dev`):
//
//   bun ../../scripts/dev.ts          build, then rebuild the game's code on every edit
//   bun ../../scripts/dev.ts --once   build once
//
// It writes two bundles to build/: `vendor.js` (React, the reconciler, the refresh
// runtime, @platypus/engine and @platypus/react; built once) and `game.js` (the game's own code,
// CommonJS with those left external, with Fast Refresh registrations, marked
// `// @platypus-dev`). A running `platypus-game` window then reloads game.js in place: React keeps
// its state and the world stays as it is. A production `bun run build` makes one
// self-contained game.js instead.
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";

import { update as updateMetas } from "./assets.ts";

const once = process.argv.includes("--once");
const entry = ["src/main.tsx", "src/main.ts"].find((f) => existsSync(f));
if (!entry) throw new Error("no src/main.tsx or src/main.ts here");
const define = { "process.env.NODE_ENV": '"development"' };

// New assets get their `.meta` (GUID and importer settings).
for (const created of updateMetas(".").created) console.log(`+ ${created}.meta`);

const vendor = await Bun.build({
  entrypoints: [join(import.meta.dir, "../js/react/src/vendor.ts")],
  outdir: "build",
  naming: "vendor.js",
  format: "iife",
  target: "browser",
  define,
});
if (!vendor.success) {
  for (const log of vendor.logs) console.error(log);
  process.exit(1);
}
// A game.css from an older build would otherwise linger.
if (existsSync("build/game.css")) rmSync("build/game.css");

const args = [
  "build",
  entry,
  "--outdir",
  "build",
  "--entry-naming",
  "game.[ext]",
  "--format",
  "cjs",
  "--target",
  "browser",
  "--react-fast-refresh",
  "--external",
  "react",
  "--external",
  "react/*",
  "--external",
  "@platypus/*",
  "--define",
  `process.env.NODE_ENV=${define["process.env.NODE_ENV"]}`,
  "--banner",
  "// @platypus-dev",
  ...(once ? [] : ["--watch", "--no-clear-screen"]),
];
const game = Bun.spawn([process.execPath, ...args], { stdout: "inherit", stderr: "inherit" });
process.exit(await game.exited);
