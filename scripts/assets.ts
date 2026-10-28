// SPDX-License-Identifier: Apache-2.0

// Asset `.meta` files, Unity's way: every asset in a game folder gets `<file>.meta` next
// to it holding a GUID (stable across renames, so scenes can refer to assets by it)
// and its importer settings (how a texture is filtered, whether a model gets colliders,
// ...). Run in a game's folder, or with folders:
//
//   bun ../../scripts/assets.ts           this folder
//   bun scripts/assets.ts examples/world  given folders
//   ... --prune                            also delete `.meta` files whose asset is gone
//
// Existing `.meta` files are kept. A file moved with its `.meta` left behind keeps its
// GUID when the move is unambiguous (one orphaned `.meta` and one new file with that
// name). `scripts/dev.ts` runs this before building.
import { existsSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative } from "node:path";

/** Asset kinds by extension. */
const KINDS: Record<string, "texture" | "model" | "audio" | "font" | "data"> = {
  png: "texture",
  jpg: "texture",
  jpeg: "texture",
  svg: "texture",
  hdr: "texture",
  glb: "model",
  gltf: "model",
  bin: "model",
  wav: "audio",
  ogg: "audio",
  mp3: "audio",
  flac: "audio",
  ttf: "font",
  otf: "font",
  woff: "font",
  woff2: "font",
  json: "data",
};

/** Folders that hold no assets: builds, packages, code, tools' state. */
const SKIP = new Set(["build", "node_modules", "src", "target"]);
/** Files that aren't assets even with an asset extension. */
const NOT_ASSETS = new Set(["package.json", "project.json", "tsconfig.json", "bun.lock", "bun.lockb"]);

export interface Meta {
  guid: string;
  importer: Record<string, unknown>;
}

/** An asset kind's default importer settings. */
export function defaultImporter(file: string): Record<string, unknown> {
  const kind = KINDS[file.split(".").pop()!.toLowerCase()] ?? "data";
  switch (kind) {
    case "texture":
      return { type: "texture", filterMode: "bilinear", wrapMode: "repeat", mipmaps: true, anisoLevel: 1, linear: false };
    case "model":
      return { type: "model", colliders: "none" };
    case "audio":
      return { type: "audio", stream: false };
    case "font":
      return { type: "font" };
    default:
      return { type: "data" };
  }
}

export function isAsset(file: string): boolean {
  if (file.endsWith(".meta") || NOT_ASSETS.has(basename(file))) return false;
  return (file.split(".").pop()!.toLowerCase()) in KINDS;
}

export function newGuid(): string {
  return crypto.randomUUID().replaceAll("-", "");
}

/** Every asset file and `.meta` file under `root` (game-relative, forward slashes). */
export function scan(root: string): { assets: string[]; metas: string[] } {
  const assets: string[] = [];
  const metas: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      if (name.startsWith(".")) continue;
      const full = join(dir, name);
      if (statSync(full).isDirectory()) {
        if (!SKIP.has(name)) walk(full);
        continue;
      }
      const rel = relative(root, full).replaceAll("\\", "/");
      if (name.endsWith(".meta")) metas.push(rel);
      else if (isAsset(name)) assets.push(rel);
    }
  };
  walk(root);
  return { assets: assets.sort(), metas: metas.sort() };
}

export interface Report {
  created: string[];
  moved: [from: string, to: string][];
  orphans: string[];
}

/** Makes sure every asset under `root` has a `.meta`; returns what changed. */
export function update(root: string, options: { prune?: boolean } = {}): Report {
  const { assets, metas } = scan(root);
  const assetSet = new Set(assets);
  const orphans = metas.filter((m) => !assetSet.has(m.slice(0, -".meta".length)));
  const missing = assets.filter((a) => !existsSync(join(root, `${a}.meta`)));
  const report: Report = { created: [], moved: [], orphans: [] };
  // Moves: an orphaned meta and a meta-less file sharing a name, each the only one.
  const byName = <T extends string>(list: T[]) => {
    const m = new Map<string, T[]>();
    for (const x of list) m.set(basename(x), [...(m.get(basename(x)) ?? []), x]);
    return m;
  };
  const orphanNames = byName(orphans.map((o) => o.slice(0, -".meta".length)));
  const missingNames = byName(missing);
  const moved = new Set<string>();
  for (const [name, files] of missingNames) {
    const from = orphanNames.get(name);
    if (files.length === 1 && from?.length === 1) {
      renameSync(join(root, `${from[0]}.meta`), join(root, `${files[0]}.meta`));
      report.moved.push([from[0]!, files[0]!]);
      moved.add(files[0]!);
      moved.add(`${from[0]}.meta`);
    }
  }
  for (const a of missing) {
    if (moved.has(a)) continue;
    const meta: Meta = { guid: newGuid(), importer: defaultImporter(a) };
    writeFileSync(join(root, `${a}.meta`), `${JSON.stringify(meta, null, 2)}\n`);
    report.created.push(a);
  }
  for (const o of orphans) {
    if (moved.has(o)) continue;
    if (options.prune) rmSync(join(root, o));
    report.orphans.push(o);
  }
  return report;
}

/** Reads a `.meta` file (null when it isn't one). */
export function readMeta(path: string): Meta | null {
  try {
    const m = JSON.parse(readFileSync(path, "utf8")) as Meta;
    return typeof m.guid === "string" ? m : null;
  } catch {
    return null;
  }
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const prune = args.includes("--prune");
  const folders = args.filter((a) => !a.startsWith("--"));
  for (const folder of folders.length ? folders : ["."]) {
    const r = update(folder, { prune });
    for (const c of r.created) console.log(`${folder}: + ${c}.meta`);
    for (const [from, to] of r.moved) console.log(`${folder}: ${from}.meta -> ${to}.meta`);
    for (const o of r.orphans) console.log(`${folder}: ${prune ? "deleted" : "orphaned (its asset is gone; --prune deletes)"} ${o}`);
  }
}
