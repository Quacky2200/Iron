# Benchmarks

Frame times of every example on the native backend (iron_ts, JIT-compiled `game.irbundle`) against the QuickJS original
of the same example on `main` (its own `bun run build`), measured before this branch is merged back. Both run in the
same release host, without a window (`iron --bench`: an offscreen target, no vsync, no frame cap; each frame waits for
the GPU work of the frame before it), at 1280x720 and a fixed step of 1/60 s, with the same scripted input
(`tools/bench/inputs/<name>.json`). How it works and the flags: `docs/iron-build.md`, "Benchmarks". Raw numbers:
`docs/benchmarks/results.json`.

Re-run (about 40 minutes for all 44; `--only a,b` re-runs some and keeps the rest):

```bash
cd tools && cargo run -q -p iron_build -- bench ../examples
```

The columns: fps over the measured frames, frame ms (mean, median, p95, p99, worst), and the mean frame split into
**script** (game code: hooks, timers, the React UI it renders), **sync** (native only: `sync_native_render`, the
per-frame copy of the compiled game's registries into the renderer, plus animation, splines and camera follow, which
QuickJS games do inside script time), **render** (UI layout and draw list, the world, the GPU wait) and **other**
(physics, audio, applying script output). Start-up is process start to the first frame (it includes the JIT
compile for native games); peak MB is the peak working set.

## Findings

Run of 2026-10-23 at `a01f76f1` (all 44 examples in one batch, nothing else building or running), against `main`
(`5b8aa521`). Machine: AMD Ryzen 9 5900X (12 cores, 32 GB), NVIDIA GeForce RTX 5080 (Vulkan), Windows 11 Pro 22631.
"First" below is the first full run, at `41fb369` (`git show 41fb369:docs/benchmarks/results.json`), before the render
sync was made incremental (`31be6440`, `97756c28`, `a8dd52dc`).

- **Best speed-ups:** raindrop-demo 6.87x, fishtank 5.12x, godrays-room 4.58x, world 3.94x, splines 3.35x, physics
  2.68x, fog-world 2.43x, animation 2.41x, emoji 2.27x, clock 2.03x. They are the script-heavy examples: script time is
  5-25x lower native (raindrop-demo 2.98 to 0.10 ms, fishtank 2.94 to 0.14, godrays-room 4.40 to 0.20, world 1.79 to
  0.05, splines 0.89 to 0.05).
- **Nothing is slower than QuickJS beyond noise.** The examples within 0.9-1.1x were re-run twice more (fps of the
  full run / re-run 1 / re-run 2):

  | Example | native fps | QuickJS fps | speed-up | spread |
  |---|---|---|---|---:|
  | boxes | 3256 / 3020 / 3253 | 3040 / 2711 / 3177 | 1.07 / 1.11 / 1.02 | 1.02-1.11x |
  | cloth | 914 / 845 / 878 | 840 / 850 / 860 | 1.09 / 0.99 / 1.02 | 0.99-1.09x |
  | dark-room | 355 / 351 / 329 | 344 / 348 / 322 | 1.03 / 1.01 / 1.02 | 1.01-1.03x |
  | fog | 1405 / 1368 / 1291 | 1277 / 1276 / 1316 | 1.10 / 1.07 / 0.98 | 0.98-1.10x |
  | form | 3400 / 3408 / 3360 | 3502 / 3328 / 3419 | 0.97 / 1.02 / 0.98 | 0.97-1.02x |
  | godrays-lift | 1499 / 1463 / 1426 | 1490 / 1472 / 1386 | 1.01 / 0.99 / 1.03 | 0.99-1.03x |
  | godrays-shed | 226 / 222 / 219 | 225 / 221 / 219 | 1.00 / 1.00 / 1.00 | 1.00x |
  | hello | 6223 / 8321 / 8127 | 6129 / 8359 / 7981 | 1.02 / 1.00 / 1.02 | 1.00-1.02x |
  | materials | 615 / 540 / 581 | 569 / 483 / 553 | 1.08 / 1.12 / 1.05 | 1.05-1.12x |
  | shapes | 2900 / 3032 / 2643 | 2938 / 2717 / 2852 | 0.99 / 1.12 / 0.93 | 0.93-1.12x |
  | text | 2303 / 2206 / 2275 | 2304 / 1569 / 2306 | 1.00 / 1.41 / 0.99 | 0.99-1.41x |
  | tunnel | 637 / 589 / 624 | 618 / 617 / 617 | 1.03 / 0.96 / 1.01 | 0.96-1.03x |

  The three below 1.0x in the full run (form 0.97x, shapes 0.99x, text 1.00x) are UI or 2D scenes at 2300-3500 fps
  with 0.01 ms of script and sync, so the frame is the draw and the GPU; their spread crosses 1.0x. Of the clock faces,
  Morphing read 0.82x and Alarm clock 1.08x; two more clock runs gave Morphing 1.00x and 1.11x and Alarm clock 0.98x and
  0.95x: draw-bound faces (0.15-0.20 ms of QuickJS script), noise. Every other face is 1.16-3.53x.
- **GPU-bound scenes are equal** (godrays-shed, godrays-lift, text, tunnel, dark-room: 1.00-1.03x). Each frame waits
  for the previous frame's GPU work, and the **render** column holds that wait: under QuickJS the script time overlaps
  the GPU, so godrays-shed shows 2.19 ms script + 2.19 render against 4.26 ms native render, and godrays-cell 2.01 +
  0.92 against 2.35, at the same GPU floor. (The previous page read this as the originals drawing less.)
- **Before and after the incremental sync.** The **sync** column (`sync_native_render`) was 0.2-1.95 ms a frame in the
  first run for 3D scenes (every component rebuilt every frame) and is 0.08 ms or less now everywhere but
  fog-white-room. Native fps rose most where sync was large: fog-world 323 to 978 (3.0x; it was 0.86x QuickJS and is
  2.43x), clock 725 to 1437, godrays-room 475 to 810, fog-park 223 to 376, godrays-clouds 206 to 324, godrays-forest 276
  to 416, fishtank 886 to 1381. QuickJS did not change between the runs, yet its fps moved by a median of +5% (0.81x to
  1.43x per example: boxes +43%, scenes -19%): machine state between batches, so changes of 10-20% in native fps alone
  are not from the sync. hello's native 8156 to 6223 is that noise (its re-runs gave 8321 and 8127, equal to QuickJS).

| Example | sync first | sync now | native fps first | native fps now | QuickJS fps now | speed-up first | speed-up now |
|---|---:|---:|---:|---:|---:|---:|---:|
| animation | 0.05 | 0.04 | 1841 | 2300 | 954 | 2.02x | 2.41x |
| arcade | 0.03 | 0.01 | 2037 | 2203 | 1335 | 1.64x | 1.65x |
| boxes | 0.01 | 0.01 | 2394 | 3256 | 3040 | 1.13x | 1.07x |
| classics | 0.03 | 0.01 | 2401 | 2881 | 1965 | 1.33x | 1.47x |
| clock | 0.62 | 0.06 | 725 | 1437 | 707 | 1.08x | 2.03x |
| cloth | 0.02 | 0.01 | 800 | 914 | 840 | 0.99x | 1.09x |
| cornell | 0.02 | 0.01 | 1871 | 2225 | 2011 | 1.01x | 1.11x |
| css | 0.01 | 0.01 | 2386 | 2635 | 2343 | 1.08x | 1.12x |
| dark-room | 0.23 | 0.04 | 338 | 355 | 344 | 1.04x | 1.03x |
| emoji | 0.50 | 0.03 | 477 | 819 | 361 | 1.68x | 2.27x |
| fishtank | 0.29 | 0.04 | 886 | 1381 | 270 | 3.26x | 5.12x |
| fog | 0.14 | 0.01 | 1295 | 1405 | 1277 | 1.08x | 1.10x |
| fog-blocks | 0.06 | 0.01 | 2267 | 2915 | 2239 | 0.93x | 1.30x |
| fog-park | 1.95 | 0.02 | 223 | 376 | 233 | 1.07x | 1.61x |
| fog-raider | 0.21 | 0.01 | 1670 | 2176 | 1356 | 1.40x | 1.61x |
| fog-teapots | 0.02 | 0.01 | 2157 | 2522 | 2246 | 1.02x | 1.12x |
| fog-white-room | n/a | 0.58 | n/a | 861 | n/a | - | - |
| fog-world | 1.83 | 0.03 | 323 | 978 | 402 | 0.86x | 2.43x |
| form | 0.01 | 0.01 | 3293 | 3400 | 3502 | 0.97x | 0.97x |
| godrays-cell | 0.59 | 0.03 | 386 | 398 | 332 | 1.23x | 1.20x |
| godrays-clouds | 1.87 | 0.02 | 206 | 324 | 181 | 1.16x | 1.79x |
| godrays-forest | 1.44 | 0.02 | 276 | 416 | 220 | 1.09x | 1.89x |
| godrays-lift | 0.05 | 0.01 | 1435 | 1499 | 1490 | 1.03x | 1.01x |
| godrays-room | 0.80 | 0.04 | 475 | 810 | 177 | 2.73x | 4.58x |
| godrays-shed | 0.33 | 0.06 | 222 | 226 | 225 | 1.00x | 1.00x |
| godrays-sunset | 0.25 | 0.02 | 802 | 987 | 772 | 1.12x | 1.28x |
| hello | 0.01 | 0.01 | 8156 | 6223 | 6129 | 1.20x | 1.02x |
| launcher | 0.01 | 0.01 | 655 | 655 | n/a | - | - |
| layers | 0.21 | 0.01 | 1392 | 1759 | 1244 | 1.11x | 1.41x |
| light-units | 0.03 | 0.01 | 1428 | 1566 | 1402 | 1.08x | 1.12x |
| materials | 0.04 | 0.01 | 485 | 615 | 569 | 1.00x | 1.08x |
| md-reader | 0.01 | 0.01 | 1350 | 1506 | n/a | - | - |
| physics | 0.01 | 0.01 | 1889 | 2029 | 758 | 2.51x | 2.68x |
| platformer | 0.09 | 0.02 | 2095 | 3043 | 1565 | 1.46x | 1.94x |
| quality | 0.07 | 0.01 | 1972 | 2067 | 1719 | 1.14x | 1.20x |
| raindrop-demo | 0.10 | 0.08 | 1696 | 1967 | 286 | 6.19x | 6.87x |
| scenes | 0.02 | 0.01 | 2717 | 2579 | 1769 | 1.25x | 1.46x |
| shapes | 0.04 | 0.01 | 2850 | 2900 | 2938 | 1.01x | 0.99x |
| sine-circle-demo | 0.02 | 0.01 | 3045 | 3169 | 2376 | 1.52x | 1.33x |
| splines | 0.06 | 0.04 | 2360 | 2745 | 820 | 2.91x | 3.35x |
| text | 0.01 | 0.01 | 2250 | 2303 | 2304 | 1.01x | 1.00x |
| tunnel | 0.27 | 0.01 | 613 | 637 | 618 | 1.01x | 1.03x |
| windows | 0.01 | 0.01 | 1367 | 1906 | n/a | - | - |
| world | 0.04 | 0.01 | 1093 | 1112 | 282 | 4.07x | 3.94x |

- **fog-white-room** (new, native only: not on `main`): 861 fps, median 1.25 ms. Its 0.58 ms of sync is the
  `Animator` on the skinned mannequin (a speed blend tree plus one-shot states: `syncParts` "animation, splines, camera
  follow" 0.57 ms), the largest sync left in any example; QuickJS games evaluate animation inside script time.
  Since cut to about 0.03-0.04 ms (see "Animator" below, after the generated tables).
- **Like-for-like script time:** a QuickJS game's **script** column holds the JavaScript, the native API calls it makes
  (math, scene, rendering data: QuickJS games call the same `#[iron_api]` functions) and its animation, spline and camera
  follow components; native counts those components under **sync**. `opMarshalMs` (copying `__iron_op` arguments out
  of JavaScript) and `opsApplyMs` (applying them, under **other**) stay at 0.3 ms or less.
- **Start-up** is 0.4-1.3 s for both backends. emoji's native start-up was 1.94 s in this batch (QuickJS 1.37 s), but
  the compiler's share is unchanged (about 250 ms: Cranelift 176, type inference and lowering 35, source rewrites 33);
  the rest was file reads (asset mount 537 ms against 122 for QuickJS, first frame 519), cold right after its native
  build. The breakdown and what a compile cache would save: `docs/research/iron-ts-startup-and-hot-reload.md`.
- emoji is random (each run picks other performers), so its numbers move between runs.
- **n/a:** fog-white-room, launcher, md-reader and windows exist only on this branch (native only). css's original
  needs the Charm font, which is kept out of git; the runner copies it in from the checkout's `Charm/` folder. No
  build or run failed and no run logged errors.

<!-- bench:begin (written by `iron-build bench`) -->
Machine: AMD Ryzen 9 5900X 12-Core Processor (32 GB RAM), NVIDIA GeForce RTX 5080 (Vulkan, DiscreteGpu), Microsoft Windows 11 Pro 10.0.22631.

Commit a01f76f1 against main (5b8aa521); every run 1280x720 px, a fixed step of 1/60 s per frame (`--fixed-dt`), no window, no vsync or frame cap, release host.

## Summary

| Example | Native fps | QuickJS fps | Speed-up | Native median / p99 ms | QuickJS median / p99 ms | Native script / sync ms | QuickJS script ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| [animation](#animation) | 2300 | 954 | 2.41x | 0.41 / 0.64 | 1.01 / 1.50 | 0.01 / 0.04 | 0.62 |
| [arcade](#arcade) | 2203 | 1335 | 1.65x | 0.42 / 0.86 | 0.68 / 1.70 | 0.03 / 0.01 | 0.36 |
| [boxes](#boxes) | 3256 | 3040 | 1.07x | 0.30 / 0.44 | 0.31 / 0.76 | 0.01 / 0.01 | 0.02 |
| [classics](#classics) | 2881 | 1965 | 1.47x | 0.34 / 0.39 | 0.49 / 0.66 | 0.01 / 0.01 | 0.14 |
| [clock](#clock) | 1437 | 707 | 2.03x | 0.67 / 1.13 | 1.12 / 4.30 | 0.08 / 0.06 | 0.81 |
| [cloth](#cloth) | 914 | 840 | 1.09x | 1.10 / 1.89 | 1.15 / 2.47 | 0.01 / 0.01 | 0.06 |
| [cornell](#cornell) | 2225 | 2011 | 1.11x | 0.44 / 0.50 | 0.47 / 0.80 | 0.01 / 0.01 | 0.03 |
| [css](#css) | 2635 | 2343 | 1.12x | 0.37 / 0.90 | 0.36 / 3.86 | 0.01 / 0.01 | 0.04 |
| [dark-room](#dark-room) | 355 | 344 | 1.03x | 2.74 / 3.55 | 2.75 / 5.06 | 0.10 / 0.04 | 0.70 |
| [emoji](#emoji) | 819 | 361 | 2.27x | 1.12 / 2.65 | 2.44 / 9.80 | 0.05 / 0.03 | 0.99 |
| [fishtank](#fishtank) | 1381 | 270 | 5.12x | 0.71 / 1.04 | 3.68 / 4.33 | 0.14 / 0.04 | 2.94 |
| [fog](#fog) | 1405 | 1277 | 1.10x | 0.71 / 0.88 | 0.79 / 1.09 | 0.01 / 0.01 | 0.25 |
| [fog-blocks](#fog-blocks) | 2915 | 2239 | 1.30x | 0.34 / 0.37 | 0.46 / 0.67 | 0.01 / 0.01 | 0.06 |
| [fog-park](#fog-park) | 376 | 233 | 1.61x | 2.60 / 3.49 | 4.26 / 4.95 | 0.01 / 0.02 | 2.22 |
| [fog-raider](#fog-raider) | 2176 | 1356 | 1.61x | 0.46 / 0.69 | 0.73 / 0.81 | 0.02 / 0.01 | 0.38 |
| [fog-teapots](#fog-teapots) | 2522 | 2246 | 1.12x | 0.39 / 0.43 | 0.44 / 0.47 | 0.01 / 0.01 | 0.05 |
| [fog-white-room](#fog-white-room) | 861 | n/a | - | 1.25 / 1.64 | n/a | 0.05 / 0.58 | n/a |
| [fog-world](#fog-world) | 978 | 402 | 2.43x | 0.92 / 4.99 | 1.34 / 15.15 | 0.22 / 0.03 | 1.64 |
| [form](#form) | 3400 | 3502 | 0.97x | 0.29 / 0.32 | 0.28 / 0.31 | 0.01 / 0.01 | 0.01 |
| [godrays-cell](#godrays-cell) | 398 | 332 | 1.20x | 2.50 / 2.90 | 3.01 / 3.46 | 0.12 / 0.03 | 2.01 |
| [godrays-clouds](#godrays-clouds) | 324 | 181 | 1.79x | 3.10 / 3.40 | 5.50 / 6.21 | 0.28 / 0.02 | 2.46 |
| [godrays-forest](#godrays-forest) | 416 | 220 | 1.89x | 2.43 / 3.00 | 4.49 / 5.22 | 0.01 / 0.02 | 1.89 |
| [godrays-lift](#godrays-lift) | 1499 | 1490 | 1.01x | 0.66 / 0.81 | 0.66 / 0.85 | 0.01 / 0.01 | 0.10 |
| [godrays-room](#godrays-room) | 810 | 177 | 4.58x | 1.22 / 1.45 | 5.70 / 6.66 | 0.20 / 0.04 | 4.40 |
| [godrays-shed](#godrays-shed) | 226 | 225 | 1.00x | 4.33 / 5.23 | 4.35 / 5.29 | 0.10 / 0.06 | 2.19 |
| [godrays-sunset](#godrays-sunset) | 987 | 772 | 1.28x | 0.92 / 2.03 | 1.18 / 2.21 | 0.02 / 0.02 | 0.38 |
| [hello](#hello) | 6223 | 6129 | 1.02x | 0.16 / 0.31 | 0.17 / 0.29 | 0.01 / 0.01 | 0.02 |
| [launcher](#launcher) | 655 | n/a | - | 2.02 / 3.19 | n/a | 0.01 / 0.01 | n/a |
| [layers](#layers) | 1759 | 1244 | 1.41x | 0.57 / 0.84 | 0.78 / 1.08 | 0.01 / 0.01 | 0.30 |
| [light-units](#light-units) | 1566 | 1402 | 1.12x | 0.63 / 0.82 | 0.69 / 1.11 | 0.01 / 0.01 | 0.09 |
| [materials](#materials) | 615 | 569 | 1.08x | 1.59 / 3.11 | 1.73 / 3.24 | 0.01 / 0.01 | 0.12 |
| [md-reader](#md-reader) | 1506 | n/a | - | 0.57 / 1.28 | n/a | 0.01 / 0.01 | n/a |
| [physics](#physics) | 2029 | 758 | 2.68x | 0.41 / 1.00 | 1.24 / 2.42 | 0.23 / 0.01 | 0.85 |
| [platformer](#platformer) | 3043 | 1565 | 1.94x | 0.32 / 0.40 | 0.63 / 0.80 | 0.04 / 0.02 | 0.31 |
| [quality](#quality) | 2067 | 1719 | 1.20x | 0.50 / 0.72 | 0.54 / 0.88 | 0.02 / 0.01 | 0.15 |
| [raindrop-demo](#raindrop-demo) | 1967 | 286 | 6.87x | 0.49 / 0.67 | 3.45 / 3.94 | 0.10 / 0.08 | 2.98 |
| [scenes](#scenes) | 2579 | 1769 | 1.46x | 0.34 / 0.64 | 0.56 / 0.81 | 0.01 / 0.01 | 0.11 |
| [shapes](#shapes) | 2900 | 2938 | 0.99x | 0.36 / 0.55 | 0.34 / 0.36 | 0.01 / 0.01 | 0.08 |
| [sine-circle-demo](#sine-circle-demo) | 3169 | 2376 | 1.33x | 0.29 / 0.39 | 0.34 / 0.63 | 0.02 / 0.01 | 0.11 |
| [splines](#splines) | 2745 | 820 | 3.35x | 0.33 / 0.56 | 1.19 / 1.44 | 0.05 / 0.04 | 0.89 |
| [text](#text) | 2303 | 2304 | 1.00x | 0.43 / 0.58 | 0.43 / 0.80 | 0.01 / 0.01 | 0.01 |
| [tunnel](#tunnel) | 637 | 618 | 1.03x | 1.65 / 1.94 | 1.67 / 2.10 | 0.01 / 0.01 | 0.36 |
| [windows](#windows) | 1906 | n/a | - | 0.51 / 0.58 | n/a | 0.01 / 0.01 | n/a |
| [world](#world) | 1112 | 282 | 3.94x | 0.82 / 1.48 | 4.18 / 4.73 | 0.05 / 0.01 | 1.79 |

## Per example

### animation

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2300 | 0.43 | 0.41 | 0.54 | 0.64 | 1.98 | 0.01 | 0.04 | 0.38 | 0.01 | 998 ms | 164 |
| QuickJS | 954 | 1.05 | 1.01 | 1.38 | 1.50 | 3.18 | 0.62 | 0.00 | 0.42 | 0.01 | 768 ms | 167 |

### arcade

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2203 | 0.45 | 0.42 | 0.62 | 0.86 | 2.21 | 0.03 | 0.01 | 0.40 | 0.01 | 801 ms | 166 |
| QuickJS | 1335 | 0.75 | 0.68 | 1.36 | 1.70 | 2.55 | 0.36 | 0.00 | 0.37 | 0.01 | 710 ms | 167 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 2089 | 1218 | 1.71x | 0.44 / 0.98 | 0.67 / 1.70 | 0.05 | 0.44 |
| steer | 720 | 2255 | 1390 | 1.62x | 0.42 / 0.69 | 0.68 / 1.60 | 0.02 | 0.33 |

### boxes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3256 | 0.31 | 0.30 | 0.39 | 0.44 | 0.83 | 0.01 | 0.01 | 0.28 | 0.01 | 549 ms | 165 |
| QuickJS | 3040 | 0.33 | 0.31 | 0.36 | 0.76 | 0.80 | 0.02 | 0.00 | 0.30 | 0.01 | 563 ms | 166 |

### classics

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2881 | 0.35 | 0.34 | 0.36 | 0.39 | 1.94 | 0.01 | 0.01 | 0.32 | 0.01 | 766 ms | 164 |
| QuickJS | 1965 | 0.51 | 0.49 | 0.59 | 0.66 | 2.19 | 0.14 | 0.00 | 0.36 | 0.01 | 808 ms | 166 |

### clock

Input: the right arrow clicked through all twelve faces.

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1437 | 0.70 | 0.67 | 1.06 | 1.13 | 2.62 | 0.08 | 0.06 | 0.50 | 0.05 | 1178 ms | 227 |
| QuickJS | 707 | 1.41 | 1.12 | 3.55 | 4.30 | 7.68 | 0.81 | 0.00 | 0.56 | 0.04 | 698 ms | 231 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Analog | 240 | 1652 | 704 | 2.35x | 0.59 / 0.77 | 1.41 / 1.56 | 0.05 | 1.00 |
| Dials | 240 | 1829 | 1197 | 1.53x | 0.53 / 0.74 | 0.83 / 1.06 | 0.01 | 0.33 |
| Meter | 240 | 1392 | 995 | 1.40x | 0.72 / 0.86 | 1.00 / 1.11 | 0.02 | 0.32 |
| Sundial | 240 | 1741 | 1495 | 1.16x | 0.57 / 0.70 | 0.67 / 0.72 | 0.04 | 0.19 |
| Sand timer | 240 | 2046 | 1386 | 1.48x | 0.48 / 0.61 | 0.72 / 0.77 | 0.03 | 0.32 |
| Lava lamp | 240 | 927 | 263 | 3.53x | 1.06 / 1.25 | 3.59 / 5.93 | 0.44 | 3.00 |
| Underwater | 240 | 1754 | 764 | 2.30x | 0.56 / 0.72 | 1.34 / 1.74 | 0.04 | 0.65 |
| Chandelier | 240 | 1329 | 598 | 2.22x | 0.74 / 0.90 | 1.65 / 2.34 | 0.03 | 0.62 |
| Rainbow | 240 | 1335 | 620 | 2.15x | 0.73 / 1.15 | 1.55 / 2.13 | 0.07 | 1.00 |
| Morphing | 240 | 1506 | 1848 | 0.82x | 0.64 / 0.97 | 0.53 / 0.64 | 0.02 | 0.15 |
| Flip | 240 | 1275 | 404 | 3.16x | 0.75 / 1.05 | 2.44 / 2.85 | 0.17 | 2.00 |
| Alarm clock | 240 | 1217 | 1123 | 1.08x | 0.81 / 1.04 | 0.89 / 1.01 | 0.02 | 0.20 |

### cloth

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 914 | 1.09 | 1.10 | 1.84 | 1.89 | 2.62 | 0.01 | 0.01 | 0.36 | 0.71 | 525 ms | 164 |
| QuickJS | 840 | 1.19 | 1.15 | 2.01 | 2.47 | 2.73 | 0.06 | 0.00 | 0.39 | 0.73 | 519 ms | 165 |

### cornell

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2225 | 0.45 | 0.44 | 0.47 | 0.50 | 2.10 | 0.01 | 0.01 | 0.43 | 0.01 | 542 ms | 186 |
| QuickJS | 2011 | 0.50 | 0.47 | 0.64 | 0.80 | 2.29 | 0.03 | 0.00 | 0.46 | 0.01 | 590 ms | 189 |

### css

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2635 | 0.38 | 0.37 | 0.39 | 0.90 | 1.15 | 0.01 | 0.01 | 0.35 | 0.01 | 729 ms | 166 |
| QuickJS | 2343 | 0.43 | 0.36 | 0.42 | 3.86 | 4.16 | 0.04 | 0.00 | 0.37 | 0.01 | 648 ms | 167 |

### dark-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 355 | 2.82 | 2.74 | 3.37 | 3.55 | 11.87 | 0.10 | 0.04 | 2.67 | 0.01 | 649 ms | 212 |
| QuickJS | 344 | 2.90 | 2.75 | 4.26 | 5.06 | 8.24 | 0.70 | 0.00 | 2.18 | 0.02 | 598 ms | 211 |

### emoji

Input: the show is random: each run picks other performers.

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 819 | 1.22 | 1.12 | 1.66 | 2.65 | 24.94 | 0.05 | 0.03 | 1.07 | 0.07 | 1938 ms | 191 |
| QuickJS | 361 | 2.77 | 2.44 | 5.76 | 9.80 | 43.65 | 0.99 | 0.00 | 1.67 | 0.12 | 1373 ms | 206 |

### fishtank

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1381 | 0.72 | 0.71 | 0.82 | 1.04 | 2.27 | 0.14 | 0.04 | 0.54 | 0.01 | 540 ms | 166 |
| QuickJS | 270 | 3.71 | 3.68 | 3.95 | 4.33 | 5.32 | 2.94 | 0.00 | 0.69 | 0.08 | 542 ms | 169 |

### fog

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1405 | 0.71 | 0.71 | 0.74 | 0.88 | 2.28 | 0.01 | 0.01 | 0.68 | 0.01 | 491 ms | 189 |
| QuickJS | 1277 | 0.78 | 0.79 | 0.96 | 1.09 | 2.76 | 0.25 | 0.00 | 0.53 | 0.01 | 518 ms | 190 |

### fog-blocks

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2915 | 0.34 | 0.34 | 0.35 | 0.37 | 1.97 | 0.01 | 0.01 | 0.32 | 0.01 | 485 ms | 165 |
| QuickJS | 2239 | 0.45 | 0.46 | 0.54 | 0.67 | 2.25 | 0.06 | 0.00 | 0.38 | 0.01 | 566 ms | 166 |

### fog-park

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 376 | 2.66 | 2.60 | 3.37 | 3.49 | 7.05 | 0.01 | 0.02 | 2.62 | 0.01 | 541 ms | 210 |
| QuickJS | 233 | 4.29 | 4.26 | 4.47 | 4.95 | 6.59 | 2.22 | 0.00 | 2.05 | 0.01 | 727 ms | 214 |

### fog-raider

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2176 | 0.46 | 0.46 | 0.55 | 0.69 | 2.05 | 0.02 | 0.01 | 0.40 | 0.03 | 469 ms | 164 |
| QuickJS | 1356 | 0.74 | 0.73 | 0.76 | 0.81 | 2.29 | 0.38 | 0.00 | 0.34 | 0.01 | 511 ms | 166 |

### fog-teapots

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2522 | 0.40 | 0.39 | 0.41 | 0.43 | 2.01 | 0.01 | 0.01 | 0.37 | 0.01 | 560 ms | 164 |
| QuickJS | 2246 | 0.45 | 0.44 | 0.46 | 0.47 | 2.06 | 0.05 | 0.00 | 0.39 | 0.01 | 589 ms | 165 |

### fog-white-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 861 | 1.16 | 1.25 | 1.44 | 1.64 | 2.90 | 0.05 | 0.58 | 0.52 | 0.01 | 646 ms | 240 |
| QuickJS | n/a |||||||||||| 

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1042 | - | - | 0.95 / 1.02 | - / - | 0.04 | - |
| jog, sprint, jump and roll | 720 | 803 | - | - | 1.27 / 1.66 | - / - | 0.05 | - |

### fog-world

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 978 | 1.02 | 0.92 | 1.13 | 4.99 | 11.01 | 0.22 | 0.03 | 0.77 | 0.01 | 624 ms | 212 |
| QuickJS | 402 | 2.49 | 1.34 | 9.40 | 15.15 | 18.32 | 1.64 | 0.00 | 0.82 | 0.02 | 846 ms | 174 |

### form

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3400 | 0.29 | 0.29 | 0.30 | 0.32 | 0.34 | 0.01 | 0.01 | 0.27 | 0.01 | 454 ms | 165 |
| QuickJS | 3502 | 0.29 | 0.28 | 0.29 | 0.31 | 0.40 | 0.01 | 0.00 | 0.27 | 0.01 | 498 ms | 166 |

### godrays-cell

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 398 | 2.51 | 2.50 | 2.58 | 2.90 | 4.87 | 0.12 | 0.03 | 2.35 | 0.01 | 492 ms | 203 |
| QuickJS | 332 | 3.02 | 3.01 | 3.14 | 3.46 | 7.53 | 2.01 | 0.00 | 0.92 | 0.08 | 564 ms | 203 |

### godrays-clouds

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 324 | 3.08 | 3.10 | 3.26 | 3.40 | 6.89 | 0.28 | 0.02 | 2.76 | 0.01 | 485 ms | 206 |
| QuickJS | 181 | 5.53 | 5.50 | 5.85 | 6.21 | 9.23 | 2.46 | 0.00 | 2.76 | 0.31 | 618 ms | 214 |

### godrays-forest

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 416 | 2.40 | 2.43 | 2.76 | 3.00 | 6.93 | 0.01 | 0.02 | 2.35 | 0.01 | 492 ms | 211 |
| QuickJS | 220 | 4.54 | 4.49 | 4.93 | 5.22 | 7.61 | 1.89 | 0.00 | 2.64 | 0.01 | 682 ms | 210 |

### godrays-lift

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1499 | 0.67 | 0.66 | 0.70 | 0.81 | 2.22 | 0.01 | 0.01 | 0.64 | 0.01 | 464 ms | 194 |
| QuickJS | 1490 | 0.67 | 0.66 | 0.71 | 0.85 | 2.36 | 0.10 | 0.00 | 0.56 | 0.01 | 514 ms | 194 |

### godrays-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 810 | 1.24 | 1.22 | 1.38 | 1.45 | 6.15 | 0.20 | 0.04 | 0.98 | 0.01 | 496 ms | 208 |
| QuickJS | 177 | 5.66 | 5.70 | 6.00 | 6.66 | 10.14 | 4.40 | 0.00 | 1.08 | 0.17 | 567 ms | 217 |

### godrays-shed

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 226 | 4.42 | 4.33 | 5.01 | 5.23 | 5.53 | 0.10 | 0.06 | 4.26 | 0.01 | 503 ms | 204 |
| QuickJS | 225 | 4.44 | 4.35 | 5.04 | 5.29 | 5.87 | 2.19 | 0.00 | 2.19 | 0.06 | 543 ms | 204 |

### godrays-sunset

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 987 | 1.01 | 0.92 | 1.81 | 2.03 | 4.69 | 0.02 | 0.02 | 0.97 | 0.01 | 505 ms | 227 |
| QuickJS | 772 | 1.30 | 1.18 | 2.02 | 2.21 | 4.47 | 0.38 | 0.00 | 0.91 | 0.01 | 897 ms | 233 |

### hello

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 6223 | 0.16 | 0.16 | 0.22 | 0.31 | 0.40 | 0.01 | 0.01 | 0.12 | 0.03 | 419 ms | 162 |
| QuickJS | 6129 | 0.16 | 0.17 | 0.22 | 0.29 | 0.41 | 0.02 | 0.00 | 0.12 | 0.03 | 430 ms | 162 |

### launcher

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 655 | 1.53 | 2.02 | 2.25 | 3.19 | 4.67 | 0.01 | 0.01 | 1.48 | 0.03 | 538 ms | 167 |
| QuickJS | n/a |||||||||||| 

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1873 | - | - | 0.53 / 0.71 | - / - | 0.01 | - |
| carousel | 660 | 506 | - | - | 2.06 / 3.22 | - / - | 0.01 | - |

### layers

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1759 | 0.57 | 0.57 | 0.66 | 0.84 | 2.13 | 0.01 | 0.01 | 0.54 | 0.01 | 477 ms | 164 |
| QuickJS | 1244 | 0.80 | 0.78 | 0.89 | 1.08 | 2.40 | 0.30 | 0.00 | 0.49 | 0.01 | 525 ms | 165 |

### light-units

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1566 | 0.64 | 0.63 | 0.69 | 0.82 | 2.51 | 0.01 | 0.01 | 0.61 | 0.01 | 502 ms | 194 |
| QuickJS | 1402 | 0.71 | 0.69 | 0.97 | 1.11 | 2.41 | 0.09 | 0.00 | 0.61 | 0.01 | 523 ms | 193 |

### materials

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 615 | 1.63 | 1.59 | 1.69 | 3.11 | 3.17 | 0.01 | 0.01 | 1.60 | 0.01 | 541 ms | 164 |
| QuickJS | 569 | 1.76 | 1.73 | 1.81 | 3.24 | 3.31 | 0.12 | 0.00 | 1.63 | 0.01 | 582 ms | 166 |

### md-reader

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1506 | 0.66 | 0.57 | 1.01 | 1.28 | 5.28 | 0.01 | 0.01 | 0.60 | 0.04 | 1267 ms | 182 |
| QuickJS | n/a |||||||||||| 

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1742 | - | - | 0.56 / 0.81 | - / - | 0.01 | - |
| scroll | 800 | 1433 | - | - | 0.61 / 1.30 | - / - | 0.01 | - |

### physics

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2029 | 0.49 | 0.41 | 0.89 | 1.00 | 2.25 | 0.23 | 0.01 | 0.15 | 0.10 | 436 ms | 163 |
| QuickJS | 758 | 1.32 | 1.24 | 2.26 | 2.42 | 3.66 | 0.85 | 0.00 | 0.16 | 0.31 | 481 ms | 165 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 3419 | 1311 | 2.61x | 0.30 / 0.46 | 0.78 / 1.00 | 0.12 | 0.48 |
| drop 40 | 600 | 1687 | 626 | 2.69x | 0.61 / 1.02 | 1.56 / 2.44 | 0.29 | 1.04 |

### platformer

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3043 | 0.33 | 0.32 | 0.38 | 0.40 | 1.88 | 0.04 | 0.02 | 0.26 | 0.01 | 507 ms | 164 |
| QuickJS | 1565 | 0.64 | 0.63 | 0.75 | 0.80 | 2.22 | 0.31 | 0.00 | 0.26 | 0.07 | 744 ms | 166 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 3086 | 1591 | 1.94x | 0.32 / 0.41 | 0.61 / 0.84 | 0.04 | 0.31 |
| run and jump | 660 | 3023 | 1553 | 1.95x | 0.33 / 0.39 | 0.64 / 0.77 | 0.04 | 0.30 |

### quality

Input: each quality level clicked in the menu (this stops the demo's own cycling).

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2067 | 0.48 | 0.50 | 0.55 | 0.72 | 2.18 | 0.02 | 0.01 | 0.44 | 0.02 | 463 ms | 164 |
| QuickJS | 1719 | 0.58 | 0.54 | 0.67 | 0.88 | 2.19 | 0.15 | 0.00 | 0.41 | 0.02 | 518 ms | 165 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Low | 300 | 2491 | 1870 | 1.33x | 0.40 / 0.42 | 0.53 / 0.56 | 0.01 | 0.14 |
| Medium | 300 | 1935 | 1563 | 1.24x | 0.50 / 0.73 | 0.64 / 0.86 | 0.01 | 0.15 |
| High | 300 | 1907 | 1915 | 1.00x | 0.51 / 0.71 | 0.52 / 0.55 | 0.01 | 0.14 |
| Ultra | 300 | 2030 | 1586 | 1.28x | 0.47 / 0.75 | 0.61 / 0.91 | 0.04 | 0.17 |

### raindrop-demo

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1967 | 0.51 | 0.49 | 0.58 | 0.67 | 2.02 | 0.10 | 0.08 | 0.31 | 0.02 | 482 ms | 168 |
| QuickJS | 286 | 3.49 | 3.45 | 3.76 | 3.94 | 14.30 | 2.98 | 0.00 | 0.34 | 0.17 | 718 ms | 181 |

### scenes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2579 | 0.39 | 0.34 | 0.48 | 0.64 | 1.95 | 0.01 | 0.01 | 0.36 | 0.01 | 497 ms | 165 |
| QuickJS | 1769 | 0.56 | 0.56 | 0.70 | 0.81 | 2.12 | 0.11 | 0.00 | 0.45 | 0.01 | 517 ms | 165 |

### shapes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2900 | 0.34 | 0.36 | 0.38 | 0.55 | 1.81 | 0.01 | 0.01 | 0.32 | 0.01 | 453 ms | 162 |
| QuickJS | 2938 | 0.34 | 0.34 | 0.35 | 0.36 | 1.88 | 0.08 | 0.00 | 0.25 | 0.01 | 470 ms | 163 |

### sine-circle-demo

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3169 | 0.32 | 0.29 | 0.37 | 0.39 | 1.80 | 0.02 | 0.01 | 0.28 | 0.01 | 483 ms | 165 |
| QuickJS | 2376 | 0.42 | 0.34 | 0.61 | 0.63 | 2.12 | 0.11 | 0.00 | 0.29 | 0.01 | 516 ms | 166 |

### splines

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2745 | 0.36 | 0.33 | 0.54 | 0.56 | 1.84 | 0.05 | 0.04 | 0.27 | 0.01 | 463 ms | 163 |
| QuickJS | 820 | 1.22 | 1.19 | 1.41 | 1.44 | 2.68 | 0.89 | 0.00 | 0.29 | 0.04 | 474 ms | 162 |

### text

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2303 | 0.43 | 0.43 | 0.45 | 0.58 | 0.61 | 0.01 | 0.01 | 0.41 | 0.01 | 464 ms | 164 |
| QuickJS | 2304 | 0.43 | 0.43 | 0.44 | 0.80 | 0.92 | 0.01 | 0.00 | 0.41 | 0.01 | 490 ms | 165 |

### tunnel

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 637 | 1.57 | 1.65 | 1.82 | 1.94 | 4.47 | 0.01 | 0.01 | 1.54 | 0.01 | 500 ms | 213 |
| QuickJS | 618 | 1.62 | 1.67 | 1.94 | 2.10 | 5.31 | 0.36 | 0.00 | 1.25 | 0.01 | 626 ms | 214 |

### windows

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1906 | 0.53 | 0.51 | 0.54 | 0.58 | 2.04 | 0.01 | 0.01 | 0.50 | 0.01 | 640 ms | 166 |
| QuickJS | n/a |||||||||||| 

### world

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1112 | 0.90 | 0.82 | 1.16 | 1.48 | 2.49 | 0.05 | 0.01 | 0.81 | 0.03 | 528 ms | 193 |
| QuickJS | 282 | 3.55 | 4.18 | 4.42 | 4.73 | 8.37 | 1.79 | 0.00 | 0.88 | 0.88 | 525 ms | 206 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 939 | 289 | 3.25x | 1.03 / 1.51 | 4.09 / 4.71 | 0.05 | 1.78 |
| pointer sweep | 600 | 1224 | 278 | 4.40x | 0.80 / 1.07 | 4.23 / 4.78 | 0.04 | 1.79 |
<!-- bench:end -->

## Animator

Hand-written (kept by `iron-build bench`). The native `Animator` made cheap (2026-10-23), one commit per step, and
Unity's animation culling. Measured with the release host (`cargo build --release -p iron_app`), no window, on the
machine above (AMD Ryzen 9 5900X, NVIDIA GeForce RTX 5080 on Vulkan, Windows 11 Pro 22631):

```bash
target/release/iron_app.exe examples/fog-white-room --bench --bench-input tools/bench/inputs/fog-white-room.json \
  --fixed-dt 0.0166667 --bench-out out.json
```

The numbers are `syncParts` means in ms a frame: "animation, splines, camera follow" (the whole update; splines and
camera follow are next to nothing here) and the animation timers added in the first step (`animation: pose add`,
`pose apply`, `set_local`, which is timed per bone, so part of its share is the timer). Median of 3 runs per step (5
for the last; 1 for "before"), segments "idle" (one clip) and "jog, sprint, jump and roll" (2 to 4 clips blended).

| Step | Commit | Idle: update | add | apply | set_local | Moving: update | add | apply | Frame mean idle / moving |
|---|---|---|---|---|---|---|---|---|---|
| Before (timers only) | `8b0d59aa` | 0.375 | 0.202 | 0.163 | 0.029 | 0.635 | 0.456 | 0.166 | 0.97 / 1.24 |
| Bind tracks once per (clip, root) | `b722f24e` | 0.058 | 0.016 | 0.033 | 0.029 | 0.073 | 0.027 | 0.033 | 0.64 / 0.67 |
| `mark_moved` stops at listed objects | `6fe4775d` | 0.033 | 0.017 | 0.008 | 0.003 | 0.049 | 0.028 | 0.007 | 0.61 / 0.63 |
| Reused pose buffer, shared key search, kept clip lengths | `3abe1f58` | 0.029 | 0.017 | 0.008 | 0.003 | 0.042 | 0.030 | 0.007 | 0.63 / 0.64 |
| With culling (default `alwaysAnimate`) | `b168dc3c` | 0.027 | 0.014 | 0.008 | 0.003 | 0.039 | 0.027 | 0.007 | 0.62 / 0.63 |

Overall 14x idle and 16x moving for the 65-bone mannequin (about 650 curves a clip). The first step is nearly all of it
(the string work per curve per frame); the rest are microseconds and near the noise between runs (add moved between
0.012 and 0.017 ms idle across runs of one build). Frames of `examples/animation` and fog-white-room rendered at fixed
frames before and after every step are identical byte for byte.

**A crowd and culling.** `tools/bench/crowd` (a bench scene, not an example: copy the mannequin into its `assets/`):
50 mannequins jogging, each with its own `Animator` (65 bones, one looping clip), the camera on them or turned away;
`tools/bench/crowd/bench.json` switches the mode (keys 1 / 2 / 3) and the view (L). Same build as the last row, median of
3 runs:

| Segment | Animation update (ms) | Per character | Frame mean (ms) |
|---|---|---|---|
| `alwaysAnimate`, in view | 0.812 | 0.016 | 1.65 |
| `alwaysAnimate`, looking away | 0.808 | 0.016 | 1.65 |
| `cullUpdateTransforms`, looking away | 0.098 | 0.002 | 0.94 |
| `cullCompletely`, looking away | 0.085 | 0.002 | 0.92 |
| `cullUpdateTransforms`, in view | 0.834 | 0.017 | 1.80 |
| `cullCompletely`, in view | 0.837 | 0.017 | 1.78 |

Culled, what is left is walking the scene for players and the state machines (`cullUpdateTransforms`) and the watch over
the renderers (about 0.02 ms). In view with culling on, the frame costs about 0.14 ms more: the renderer works out a box
for each of the 100 skinned renderers from its 65 bones in every view, then tests it against the view and the shadow
maps (a follow-up in TODO.md).
