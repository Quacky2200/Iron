// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// Renders the picture of every example for docs/PROGRESS.md. Each is a windowless bench run of
// the release host (`--bench`: an offscreen target, no OS window, a fixed step of 1/60 s, so the
// game's clock reaches the same moment on any machine) that saves its last frame, then is shrunk
// into docs/progress/<name>.jpg (3D scenes) or .png (UI) with System.Drawing (Windows PowerShell).
//
//   cargo build --release -p platypus_app
//   cd tools && cargo run -q -p platypus_build -- check ../examples   (builds every example)
//   bun scripts/progress-shots.ts              every example
//   bun scripts/progress-shots.ts fog tunnel   just these
//
// The full-size renders stay in tests/output/progress/ (git-ignored).

import { mkdirSync, readdirSync, existsSync, rmSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const raw = join(root, "tests", "output", "progress");
const out = join(root, "docs", "progress");
mkdirSync(raw, { recursive: true });
mkdirSync(out, { recursive: true });

/** Frames before the picture (60 a second); 120 unless an effect needs longer to settle. */
const FRAMES: Record<string, number> = {
  "dark-room": 900, // the shutters open 10 s in, then the eye adapts
  tunnel: 420, // the car is inside the tunnel
  "godrays-lift": 360, // the doors have opened
  "light-units": 600, // the camera is in the lamplit hall
  crowd: 240,
  emoji: 300,
  fishtank: 300,
};
for (const n of ["fog", "fog-blocks", "fog-park", "fog-raider", "fog-teapots", "fog-white-room", "fog-world"]) FRAMES[n] = 600;
for (const n of ["godrays-cell", "godrays-clouds", "godrays-forest", "godrays-room", "godrays-shed", "godrays-sunset"]) FRAMES[n] = 600;

/** UI examples keep sharp edges as PNG; the rest are JPEG. */
const PNG = new Set(["hello", "boxes", "text", "css", "form", "windows", "launcher", "md-reader", "shapes", "physics"]);
/** Showpieces get 800 px; the rest 640. */
const WIDE = new Set(["crowd", "fog-park", "fog-world", "godrays-forest", "godrays-sunset", "godrays-clouds"]);

const host = join(root, "target", "release", process.platform === "win32" ? "platypus-game.exe" : "platypus-game");
if (!existsSync(host)) {
  console.error(`no ${host}: cargo build --release -p platypus_app --bin platypus-game`);
  process.exit(1);
}

/** Examples that draw nothing worth a picture (hello only logs). */
const SKIP = new Set(["hello"]);

const all = readdirSync(join(root, "examples"), { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(root, "examples", d.name, "project.json")))
  .map((d) => d.name)
  .filter((n) => !SKIP.has(n));
const names = process.argv.length > 2 ? process.argv.slice(2) : all;

// Resize with high-quality bicubic and save (JPEG at quality 85, or PNG).
const SHRINK = `$src = $env:SHOT_SRC; $dst = $env:SHOT_DST; $width = [int]$env:SHOT_WIDTH
Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile($src)
$h = [int]([math]::Round($img.Height * $width / $img.Width))
$bmp = New-Object System.Drawing.Bitmap ([int]$width), $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g.DrawImage($img, 0, 0, [int]$width, $h)
$img.Dispose()
if ($dst.EndsWith('.jpg')) {
  $codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
  $p = New-Object System.Drawing.Imaging.EncoderParameters 1
  $p.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 85L
  $bmp.Save($dst, $codec, $p)
} else {
  $bmp.Save($dst, [System.Drawing.Imaging.ImageFormat]::Png)
}`;

let failed = 0;
for (const name of names) {
  const frames = FRAMES[name] ?? 120;
  const dir = join(raw, name);
  rmSync(dir, { recursive: true, force: true });
  const run = Bun.spawnSync(
    [host, join("examples", name), "--bench", "--warmup", String(frames - 1), "--frames", "1",
      "--fixed-dt", String(1 / 60), "--size", "1280x720", "--bench-shots", dir, "--bench-out", join(raw, `${name}.json`)],
    { cwd: root, stdout: "ignore", stderr: "inherit" },
  );
  const shot = existsSync(dir) ? readdirSync(dir).find((f) => f.endsWith(".png")) : undefined;
  if (run.exitCode !== 0 || !shot) {
    console.log(`failed: ${name}`);
    failed++;
    continue;
  }
  const dst = join(out, `${name}.${PNG.has(name) ? "png" : "jpg"}`);
  const width = WIDE.has(name) ? 800 : 640;
  const shrink = Bun.spawnSync(["powershell", "-NoProfile", "-Command", SHRINK], {
    env: { ...process.env, SHOT_SRC: join(dir, shot), SHOT_DST: dst, SHOT_WIDTH: String(width) },
    stdout: "inherit",
    stderr: "inherit",
  });
  if (shrink.exitCode !== 0) {
    console.log(`failed to shrink: ${name}`);
    failed++;
    continue;
  }
  console.log(`wrote ${dst} (${frames} frames)`);
}
process.exit(failed ? 1 : 0);
