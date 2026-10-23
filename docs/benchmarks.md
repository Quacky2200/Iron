# Benchmarks

Frame times of every example on the native backend (iron_ts, JIT-compiled `game.irbundle`) against the QuickJS original
of the same example on `main` (its own `bun run build`), measured before this branch is merged back. Both run in the
same release host, without a window (`iron --bench`: an offscreen target, no vsync, no frame cap; each frame waits for
the GPU work of the frame before it), at 1280x720 and a fixed step of 1/60 s, with the same scripted input
(`tools/bench/inputs/<name>.json`). How it works and the flags: `docs/iron-build.md`, "Benchmarks". Raw numbers:
`docs/benchmarks/results.json`.

Re-run (about 40 minutes for all 43; `--only a,b` re-runs some and keeps the rest):

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

- **Script time is 5-25x lower native** wherever there is script work: raindrop-demo 3.07 to 0.13 ms, fishtank 2.95 to
  0.17, godrays-room 4.43 to 0.21, world 1.85 to 0.05, splines 0.89 to 0.05. Whole-frame speed-ups follow where scripts
  dominated: raindrop-demo 6.2x, world 4.1x, fishtank 3.3x, splines 2.9x, godrays-room 2.7x, physics 2.5x, animation
  2.0x, emoji 1.7x, arcade 1.6x.
- **GPU-bound scenes are equal** (materials, cloth, cornell, text, shapes, tunnel, godrays-lift: 0.99-1.03x): the
  backends share the renderer.
- **The render sync is incremental now** (2026-10-23, `docs/iron-api.md`, "3D from a native game"). The first run found
  native regressions in fog-world (0.86x) and six clock faces (Dials, Meter, Sundial, Sand timer, Morphing, Alarm clock:
  0.5-0.8x), all from the **sync** column: `sync_native_render` rebuilt every component every frame, and 1.85-1.95 ms of
  it was the mesh renderers alone (every renderer copied its script mesh's vertices to read the mesh's name; clock: 0.42 ms
  of 2D lines rebuilt from `Value`s). Before -> after (sync ms; native fps):

  | Example | sync before | sync after | native fps before | native fps after | QuickJS fps | speed-up now |
  |---|---:|---:|---:|---:|---:|---:|
  | fog-world | 1.83 | 0.03 | 323 | 917 | 389 | 2.36x (was 0.86x) |
  | fog-park | 1.95 | 0.03 | 223 | 462 | 225 | 2.06x (was 1.07x) |
  | godrays-clouds | 1.87 | 0.03 | 206 | 305 | 175 | 1.74x (was 1.16x) |
  | godrays-forest | 1.44 | 0.02 | 276 | 367 | 211 | 1.74x (was 1.09x) |
  | clock (all faces) | 0.62 | 0.07 | 725 | 1526 | 720 | 2.12x (was 1.08x) |

  No clock face is slower native any more: Dials 1.52x, Meter 1.38x, Sundial 1.39x, Sand timer 1.58x, Morphing 1.08x,
  Alarm clock 1.00x (draw-bound: equal script and sync, the render is the frame).
  Renders before and after are pixel-identical (fog-world, godrays-clouds, godrays-forest, fog-park, emoji, materials,
  and all twelve clock faces, at the same fixed step). fog-blocks (1.07x), form (0.97x) and hello (0.88x) differ by 0.01
  ms or less in the median at 3000-8000 fps: noise (an A/B of the old and new hosts on hello gives 8431 and 8480 fps).
- **Like-for-like script time:** a QuickJS game's **script** column holds the JavaScript, the native API calls it makes
  (math, scene, rendering data: QuickJS games call the same `#[iron_api]` functions) and its animation, spline and camera
  follow components; native counts those components under **sync**. `opMarshalMs` in `results.json` (copying `__iron_op`
  arguments out of JavaScript) is at most 0.3 ms (raindrop-demo 0.31, godrays-room 0.17, fishtank 0.12, else under
  0.1) and `opsApplyMs` (applying those ops, under **other**) at most 0.27 (godrays-clouds): QuickJS script time is almost
  all JavaScript plus native calls, which cannot be told apart without a timer in every binding.
- **Start-up** is about the same (0.4-1.0 s). The 2.2 s native emoji start-up of the first run did not come back (0.9-1.0 s
  now, QuickJS 1.2-1.5 s). `startupParts` in a bench report splits it: for emoji (4701 lines + 443 of prelude) 342 ms
  are the headless GPU device (every backend pays it), 174 ms Cranelift code generation, 121 ms the first frame (the
  performers' assets), 93 ms the asset index (`.meta` files), 36 ms type inference and lowering, 31 ms source rewrites
  (JSX, async, components), 12 ms engine manifests (6 to build, 6 to read), 32 ms renderer set-up; parsing (2 ms), IR
  passes (2 ms) and JIT linking (1 ms) are small.
- Some originals on `main` are older versions of the examples (the branch changed them since), so a few scenes draw
  slightly different work: godrays-shed and godrays-cell render more native-side (4.1 / 1.9 ms against 2.2 / 1.1 ms)
  with equal script cost. emoji is random (each run picks other performers).
- **n/a:** launcher, md-reader and windows exist only on this branch (native only). css's original needs the Charm font,
  which is kept out of git; the runner copies it in from the checkout's `Charm/` folder. No original failed to build
  or run, and no run logged errors.

<!-- bench:begin (written by `iron-build bench`) -->
Machine: AMD Ryzen 9 5900X 12-Core Processor (32 GB RAM), NVIDIA GeForce RTX 5080 (Vulkan, DiscreteGpu), Microsoft Windows 11 Pro 10.0.22631.

Commit 311074be against main (5b8aa521); every run 1280x720 px, a fixed step of 1/60 s per frame (`--fixed-dt`), no window, no vsync or frame cap, release host.

## Summary

| Example | Native fps | QuickJS fps | Speed-up | Native median / p99 ms | QuickJS median / p99 ms | Native script / sync ms | QuickJS script ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| [animation](#animation) | 1841 | 912 | 2.02x | 0.46 / 1.18 | 1.02 / 1.86 | 0.02 / 0.05 | 0.62 |
| [arcade](#arcade) | 2037 | 1241 | 1.64x | 0.39 / 1.34 | 0.69 / 2.12 | 0.03 / 0.03 | 0.38 |
| [boxes](#boxes) | 2394 | 2123 | 1.13x | 0.32 / 1.42 | 0.38 / 1.46 | 0.01 / 0.01 | 0.04 |
| [classics](#classics) | 2401 | 1802 | 1.33x | 0.38 / 1.34 | 0.49 / 1.42 | 0.01 / 0.03 | 0.15 |
| [clock](#clock) | 1526 | 720 | 2.12x | 0.58 / 1.12 | 1.14 / 3.71 | 0.07 / 0.07 | 0.80 |
| [cloth](#cloth) | 800 | 808 | 0.99x | 1.22 / 2.55 | 1.22 / 2.38 | 0.01 / 0.02 | 0.07 |
| [cornell](#cornell) | 1871 | 1849 | 1.01x | 0.48 / 1.24 | 0.50 / 1.36 | 0.01 / 0.02 | 0.03 |
| [css](#css) | 2386 | 2211 | 1.08x | 0.40 / 1.03 | 0.37 / 3.90 | 0.01 / 0.01 | 0.04 |
| [dark-room](#dark-room) | 349 | 340 | 1.03x | 2.75 / 3.73 | 2.76 / 4.74 | 0.10 / 0.04 | 0.70 |
| [emoji](#emoji) | 654 | 454 | 1.44x | 1.19 / 6.74 | 1.95 / 4.14 | 0.06 / 0.03 | 0.79 |
| [fishtank](#fishtank) | 1297 | 267 | 4.85x | 0.74 / 0.99 | 3.71 / 4.24 | 0.14 / 0.04 | 2.97 |
| [fog](#fog) | 1295 | 1199 | 1.08x | 0.72 / 2.03 | 0.81 / 1.99 | 0.01 / 0.14 | 0.26 |
| [fog-blocks](#fog-blocks) | 2798 | 2617 | 1.07x | 0.35 / 0.42 | 0.37 / 0.45 | 0.01 / 0.01 | 0.05 |
| [fog-park](#fog-park) | 462 | 225 | 2.06x | 2.10 / 2.90 | 4.41 / 5.27 | 0.02 / 0.03 | 2.28 |
| [fog-raider](#fog-raider) | 1936 | 1318 | 1.47x | 0.51 / 0.89 | 0.75 / 0.84 | 0.02 / 0.01 | 0.39 |
| [fog-teapots](#fog-teapots) | 2157 | 2107 | 1.02x | 0.41 / 0.80 | 0.44 / 0.80 | 0.01 / 0.02 | 0.05 |
| [fog-world](#fog-world) | 917 | 389 | 2.36x | 0.99 / 5.12 | 1.38 / 15.76 | 0.23 / 0.03 | 1.66 |
| [form](#form) | 3381 | 3489 | 0.97x | 0.29 / 0.34 | 0.28 / 0.31 | 0.01 / 0.01 | 0.01 |
| [godrays-cell](#godrays-cell) | 388 | 329 | 1.18x | 2.50 / 3.08 | 3.04 / 3.39 | 0.12 / 0.03 | 2.00 |
| [godrays-clouds](#godrays-clouds) | 305 | 175 | 1.74x | 3.25 / 4.56 | 5.67 / 6.89 | 0.29 / 0.03 | 2.54 |
| [godrays-forest](#godrays-forest) | 367 | 211 | 1.74x | 2.73 / 3.40 | 4.71 / 5.51 | 0.01 / 0.02 | 1.93 |
| [godrays-lift](#godrays-lift) | 1435 | 1393 | 1.03x | 0.67 / 1.23 | 0.68 / 1.61 | 0.01 / 0.05 | 0.10 |
| [godrays-room](#godrays-room) | 767 | 168 | 4.58x | 1.28 / 1.83 | 5.97 / 7.58 | 0.21 / 0.05 | 4.55 |
| [godrays-shed](#godrays-shed) | 222 | 221 | 1.00x | 4.33 / 6.07 | 4.34 / 6.12 | 0.10 / 0.33 | 2.23 |
| [godrays-sunset](#godrays-sunset) | 802 | 718 | 1.12x | 1.09 / 2.30 | 1.24 / 2.57 | 0.02 / 0.25 | 0.39 |
| [hello](#hello) | 7440 | 8461 | 0.88x | 0.12 / 0.23 | 0.12 / 0.17 | 0.01 / 0.01 | 0.01 |
| [launcher](#launcher) | 655 | n/a | - | 1.99 / 3.25 | n/a | 0.01 / 0.01 | n/a |
| [layers](#layers) | 1392 | 1257 | 1.11x | 0.70 / 1.12 | 0.78 / 0.90 | 0.01 / 0.21 | 0.31 |
| [light-units](#light-units) | 1428 | 1328 | 1.08x | 0.67 / 1.41 | 0.71 / 1.27 | 0.02 / 0.03 | 0.10 |
| [materials](#materials) | 485 | 483 | 1.00x | 1.71 / 3.40 | 1.78 / 3.33 | 0.01 / 0.04 | 0.12 |
| [md-reader](#md-reader) | 1350 | n/a | - | 0.58 / 1.41 | n/a | 0.01 / 0.01 | n/a |
| [physics](#physics) | 1889 | 751 | 2.51x | 0.44 / 1.20 | 1.24 / 2.41 | 0.26 / 0.01 | 0.86 |
| [platformer](#platformer) | 2095 | 1436 | 1.46x | 0.45 / 0.77 | 0.67 / 1.04 | 0.08 / 0.09 | 0.31 |
| [quality](#quality) | 1972 | 1728 | 1.14x | 0.47 / 0.81 | 0.52 / 1.09 | 0.01 / 0.07 | 0.15 |
| [raindrop-demo](#raindrop-demo) | 1986 | 282 | 7.04x | 0.49 / 0.58 | 3.50 / 4.16 | 0.10 / 0.08 | 3.01 |
| [scenes](#scenes) | 2717 | 2172 | 1.25x | 0.36 / 0.41 | 0.43 / 0.79 | 0.01 / 0.02 | 0.10 |
| [shapes](#shapes) | 2850 | 2836 | 1.01x | 0.32 / 0.58 | 0.34 / 0.43 | 0.01 / 0.04 | 0.08 |
| [sine-circle-demo](#sine-circle-demo) | 3045 | 1998 | 1.52x | 0.30 / 0.43 | 0.44 / 0.96 | 0.02 / 0.02 | 0.12 |
| [splines](#splines) | 2360 | 812 | 2.91x | 0.38 / 0.82 | 1.18 / 1.62 | 0.05 / 0.06 | 0.89 |
| [text](#text) | 2250 | 2228 | 1.01x | 0.43 / 0.64 | 0.42 / 0.89 | 0.01 / 0.01 | 0.02 |
| [tunnel](#tunnel) | 613 | 610 | 1.01x | 1.68 / 3.06 | 1.68 / 3.07 | 0.01 / 0.27 | 0.37 |
| [windows](#windows) | 1367 | n/a | - | 0.82 / 1.24 | n/a | 0.02 / 0.01 | n/a |
| [world](#world) | 1200 | 271 | 4.43x | 0.82 / 1.03 | 4.23 / 5.33 | 0.04 / 0.01 | 1.82 |

## Per example

### animation

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1841 | 0.54 | 0.46 | 0.84 | 1.18 | 2.19 | 0.02 | 0.05 | 0.46 | 0.01 | 802 ms | 164 |
| QuickJS | 912 | 1.10 | 1.02 | 1.52 | 1.86 | 3.87 | 0.62 | 0.00 | 0.46 | 0.01 | 799 ms | 166 |

### arcade

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2037 | 0.49 | 0.39 | 0.91 | 1.34 | 3.65 | 0.03 | 0.03 | 0.42 | 0.01 | 697 ms | 165 |
| QuickJS | 1241 | 0.81 | 0.69 | 1.48 | 2.12 | 2.49 | 0.38 | 0.00 | 0.42 | 0.01 | 793 ms | 167 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1624 | 1041 | 1.56x | 0.49 / 1.41 | 0.82 / 2.33 | 0.05 | 0.49 |
| steer | 720 | 2278 | 1349 | 1.69x | 0.38 / 1.22 | 0.66 / 1.54 | 0.02 | 0.33 |

### boxes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2394 | 0.42 | 0.32 | 0.80 | 1.42 | 1.71 | 0.01 | 0.01 | 0.39 | 0.01 | 700 ms | 164 |
| QuickJS | 2123 | 0.47 | 0.38 | 0.86 | 1.46 | 2.92 | 0.04 | 0.00 | 0.42 | 0.01 | 790 ms | 166 |

### classics

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2401 | 0.42 | 0.38 | 0.56 | 1.34 | 2.33 | 0.01 | 0.03 | 0.37 | 0.01 | 775 ms | 164 |
| QuickJS | 1802 | 0.56 | 0.49 | 0.79 | 1.42 | 2.81 | 0.15 | 0.00 | 0.40 | 0.01 | 805 ms | 166 |

### clock

Input: the right arrow clicked through all twelve faces.

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1526 | 0.66 | 0.58 | 1.07 | 1.12 | 2.63 | 0.07 | 0.07 | 0.46 | 0.05 | 741 ms | 233 |
| QuickJS | 720 | 1.39 | 1.14 | 3.55 | 3.71 | 5.13 | 0.80 | 0.00 | 0.55 | 0.04 | 639 ms | 230 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Analog | 240 | 1859 | 663 | 2.81x | 0.52 / 0.68 | 1.49 / 1.87 | 0.05 | 1.07 |
| Dials | 240 | 1824 | 1198 | 1.52x | 0.53 / 0.69 | 0.83 / 0.89 | 0.01 | 0.33 |
| Meter | 240 | 1375 | 995 | 1.38x | 0.71 / 0.88 | 1.00 / 1.16 | 0.02 | 0.32 |
| Sundial | 240 | 1772 | 1275 | 1.39x | 0.55 / 0.71 | 0.74 / 1.21 | 0.03 | 0.22 |
| Sand timer | 240 | 1969 | 1250 | 1.58x | 0.50 / 0.65 | 0.77 / 1.27 | 0.03 | 0.34 |
| Lava lamp | 240 | 921 | 278 | 3.31x | 1.07 / 1.25 | 3.57 / 4.05 | 0.42 | 2.84 |
| Underwater | 240 | 1728 | 811 | 2.13x | 0.57 / 0.70 | 1.20 / 1.71 | 0.04 | 0.62 |
| Chandelier | 240 | 1274 | 683 | 1.87x | 0.77 / 0.96 | 1.43 / 1.76 | 0.03 | 0.58 |
| Rainbow | 240 | 1689 | 601 | 2.81x | 0.58 / 0.73 | 1.59 / 2.41 | 0.06 | 0.99 |
| Morphing | 240 | 1968 | 1817 | 1.08x | 0.49 / 0.65 | 0.54 / 0.67 | 0.02 | 0.14 |
| Flip | 240 | 1640 | 415 | 3.95x | 0.59 / 0.77 | 2.35 / 2.79 | 0.15 | 1.94 |
| Alarm clock | 240 | 1222 | 1226 | 1.00x | 0.81 / 0.88 | 0.78 / 1.38 | 0.02 | 0.18 |

### cloth

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 800 | 1.25 | 1.22 | 2.14 | 2.55 | 2.92 | 0.01 | 0.02 | 0.45 | 0.77 | 644 ms | 164 |
| QuickJS | 808 | 1.24 | 1.22 | 2.08 | 2.38 | 2.85 | 0.07 | 0.00 | 0.42 | 0.75 | 595 ms | 166 |

### cornell

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1871 | 0.53 | 0.48 | 0.64 | 1.24 | 2.56 | 0.01 | 0.02 | 0.50 | 0.01 | 674 ms | 186 |
| QuickJS | 1849 | 0.54 | 0.50 | 0.66 | 1.36 | 2.18 | 0.03 | 0.00 | 0.51 | 0.01 | 668 ms | 187 |

### css

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2386 | 0.42 | 0.40 | 0.47 | 1.03 | 1.42 | 0.01 | 0.01 | 0.39 | 0.01 | 557 ms | 166 |
| QuickJS | 2211 | 0.45 | 0.37 | 0.46 | 3.90 | 4.43 | 0.04 | 0.00 | 0.40 | 0.01 | 608 ms | 168 |

### dark-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 349 | 2.87 | 2.75 | 3.40 | 3.73 | 4.98 | 0.10 | 0.04 | 2.71 | 0.01 | 514 ms | 212 |
| QuickJS | 340 | 2.94 | 2.76 | 4.25 | 4.74 | 7.45 | 0.70 | 0.00 | 2.22 | 0.02 | 552 ms | 281 |

### emoji

Input: the show is random: each run picks other performers.

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 654 | 1.53 | 1.19 | 4.62 | 6.74 | 14.84 | 0.06 | 0.03 | 1.36 | 0.08 | 968 ms | 184 |
| QuickJS | 454 | 2.20 | 1.95 | 3.25 | 4.14 | 25.95 | 0.79 | 0.00 | 1.33 | 0.08 | 1164 ms | 200 |

### fishtank

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1297 | 0.77 | 0.74 | 0.93 | 0.99 | 2.49 | 0.14 | 0.04 | 0.58 | 0.01 | 507 ms | 165 |
| QuickJS | 267 | 3.74 | 3.71 | 4.02 | 4.24 | 5.60 | 2.97 | 0.00 | 0.68 | 0.09 | 530 ms | 167 |

### fog

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1295 | 0.77 | 0.72 | 0.97 | 2.03 | 2.44 | 0.01 | 0.14 | 0.61 | 0.01 | 561 ms | 186 |
| QuickJS | 1199 | 0.83 | 0.81 | 1.06 | 1.99 | 2.65 | 0.26 | 0.00 | 0.57 | 0.01 | 602 ms | 187 |

### fog-blocks

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2798 | 0.36 | 0.35 | 0.37 | 0.42 | 2.05 | 0.01 | 0.01 | 0.33 | 0.01 | 502 ms | 165 |
| QuickJS | 2617 | 0.38 | 0.37 | 0.40 | 0.45 | 1.97 | 0.05 | 0.00 | 0.33 | 0.01 | 571 ms | 167 |

### fog-park

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 462 | 2.16 | 2.10 | 2.60 | 2.90 | 5.57 | 0.02 | 0.03 | 2.11 | 0.01 | 568 ms | 211 |
| QuickJS | 225 | 4.45 | 4.41 | 4.81 | 5.27 | 10.71 | 2.28 | 0.00 | 2.16 | 0.01 | 734 ms | 214 |

### fog-raider

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1936 | 0.52 | 0.51 | 0.61 | 0.89 | 2.15 | 0.02 | 0.01 | 0.45 | 0.03 | 482 ms | 164 |
| QuickJS | 1318 | 0.76 | 0.75 | 0.79 | 0.84 | 2.32 | 0.39 | 0.00 | 0.36 | 0.01 | 518 ms | 165 |

### fog-teapots

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2157 | 0.46 | 0.41 | 0.62 | 0.80 | 2.21 | 0.01 | 0.02 | 0.43 | 0.01 | 610 ms | 166 |
| QuickJS | 2107 | 0.47 | 0.44 | 0.62 | 0.80 | 2.17 | 0.05 | 0.00 | 0.41 | 0.01 | 652 ms | 167 |

### fog-world

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 917 | 1.09 | 0.99 | 1.33 | 5.12 | 10.74 | 0.23 | 0.03 | 0.83 | 0.01 | 643 ms | 240 |
| QuickJS | 389 | 2.57 | 1.38 | 9.90 | 15.76 | 19.46 | 1.66 | 0.00 | 0.89 | 0.02 | 850 ms | 274 |

### form

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3381 | 0.30 | 0.29 | 0.31 | 0.34 | 0.36 | 0.01 | 0.01 | 0.27 | 0.01 | 466 ms | 165 |
| QuickJS | 3489 | 0.29 | 0.28 | 0.30 | 0.31 | 0.35 | 0.01 | 0.00 | 0.27 | 0.01 | 501 ms | 166 |

### godrays-cell

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 388 | 2.58 | 2.50 | 2.90 | 3.08 | 5.20 | 0.12 | 0.03 | 2.41 | 0.01 | 498 ms | 208 |
| QuickJS | 329 | 3.04 | 3.04 | 3.19 | 3.39 | 6.48 | 2.00 | 0.00 | 0.95 | 0.08 | 558 ms | 208 |

### godrays-clouds

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 305 | 3.28 | 3.25 | 3.73 | 4.56 | 7.84 | 0.29 | 0.03 | 2.96 | 0.01 | 494 ms | 207 |
| QuickJS | 175 | 5.72 | 5.67 | 6.09 | 6.89 | 9.40 | 2.54 | 0.00 | 2.87 | 0.31 | 655 ms | 213 |

### godrays-forest

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 367 | 2.72 | 2.73 | 3.18 | 3.40 | 5.53 | 0.01 | 0.02 | 2.67 | 0.01 | 499 ms | 211 |
| QuickJS | 211 | 4.74 | 4.71 | 5.17 | 5.51 | 8.55 | 1.93 | 0.00 | 2.80 | 0.02 | 691 ms | 204 |

### godrays-lift

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1435 | 0.70 | 0.67 | 0.81 | 1.23 | 2.54 | 0.01 | 0.05 | 0.63 | 0.01 | 518 ms | 191 |
| QuickJS | 1393 | 0.72 | 0.68 | 0.87 | 1.61 | 2.76 | 0.10 | 0.00 | 0.61 | 0.01 | 569 ms | 190 |

### godrays-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 767 | 1.30 | 1.28 | 1.55 | 1.83 | 6.15 | 0.21 | 0.05 | 1.04 | 0.01 | 513 ms | 208 |
| QuickJS | 168 | 5.97 | 5.97 | 6.55 | 7.58 | 10.26 | 4.55 | 0.00 | 1.24 | 0.18 | 581 ms | 253 |

### godrays-shed

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 222 | 4.51 | 4.33 | 5.38 | 6.07 | 6.91 | 0.10 | 0.33 | 4.06 | 0.01 | 558 ms | 202 |
| QuickJS | 221 | 4.53 | 4.34 | 5.43 | 6.12 | 7.35 | 2.23 | 0.00 | 2.23 | 0.07 | 619 ms | 200 |

### godrays-sunset

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 802 | 1.25 | 1.09 | 2.01 | 2.30 | 5.83 | 0.02 | 0.25 | 0.97 | 0.01 | 579 ms | 223 |
| QuickJS | 718 | 1.39 | 1.24 | 2.18 | 2.57 | 4.28 | 0.39 | 0.00 | 1.00 | 0.01 | 945 ms | 236 |

### hello

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 7440 | 0.13 | 0.12 | 0.18 | 0.23 | 0.41 | 0.01 | 0.01 | 0.11 | 0.01 | 423 ms | 162 |
| QuickJS | 8461 | 0.12 | 0.12 | 0.13 | 0.17 | 0.23 | 0.01 | 0.00 | 0.10 | 0.01 | 442 ms | 163 |

### launcher

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 655 | 1.53 | 1.99 | 2.22 | 3.25 | 6.62 | 0.01 | 0.01 | 1.48 | 0.03 | 612 ms | 168 |
| QuickJS | n/a |||||||||||| 

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1793 | - | - | 0.53 / 1.07 | - / - | 0.01 | - |
| carousel | 660 | 508 | - | - | 2.02 / 3.26 | - / - | 0.01 | - |

### layers

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1392 | 0.72 | 0.70 | 0.75 | 1.12 | 2.37 | 0.01 | 0.21 | 0.49 | 0.01 | 521 ms | 165 |
| QuickJS | 1257 | 0.80 | 0.78 | 0.83 | 0.90 | 2.41 | 0.31 | 0.00 | 0.48 | 0.01 | 581 ms | 166 |

### light-units

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1428 | 0.70 | 0.67 | 0.84 | 1.41 | 2.61 | 0.02 | 0.03 | 0.64 | 0.01 | 551 ms | 194 |
| QuickJS | 1328 | 0.75 | 0.71 | 1.06 | 1.27 | 3.22 | 0.10 | 0.00 | 0.65 | 0.01 | 575 ms | 194 |

### materials

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 485 | 2.06 | 1.71 | 2.83 | 3.40 | 4.52 | 0.01 | 0.04 | 2.01 | 0.01 | 623 ms | 186 |
| QuickJS | 483 | 2.07 | 1.78 | 2.83 | 3.33 | 4.49 | 0.12 | 0.00 | 1.94 | 0.01 | 645 ms | 194 |

### md-reader

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1350 | 0.74 | 0.58 | 1.07 | 1.41 | 41.40 | 0.01 | 0.01 | 0.68 | 0.04 | 1323 ms | 182 |
| QuickJS | n/a |||||||||||| 

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1712 | - | - | 0.57 / 0.83 | - / - | 0.01 | - |
| scroll | 800 | 1251 | - | - | 0.62 / 1.42 | - / - | 0.01 | - |

### physics

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1889 | 0.53 | 0.44 | 0.98 | 1.20 | 2.26 | 0.26 | 0.01 | 0.16 | 0.10 | 487 ms | 165 |
| QuickJS | 751 | 1.33 | 1.24 | 2.27 | 2.41 | 3.64 | 0.86 | 0.00 | 0.16 | 0.31 | 538 ms | 166 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 3347 | 1247 | 2.69x | 0.30 / 0.49 | 0.78 / 1.14 | 0.12 | 0.50 |
| drop 40 | 600 | 1551 | 627 | 2.47x | 0.63 / 1.26 | 1.55 / 2.42 | 0.33 | 1.03 |

### platformer

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2095 | 0.48 | 0.45 | 0.56 | 0.77 | 7.76 | 0.08 | 0.09 | 0.29 | 0.02 | 541 ms | 166 |
| QuickJS | 1436 | 0.70 | 0.67 | 0.89 | 1.04 | 2.23 | 0.31 | 0.00 | 0.30 | 0.09 | 808 ms | 167 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 2020 | 1455 | 1.39x | 0.49 / 0.79 | 0.66 / 1.03 | 0.07 | 0.32 |
| run and jump | 660 | 2131 | 1427 | 1.49x | 0.44 / 0.72 | 0.69 / 1.04 | 0.09 | 0.31 |

### quality

Input: each quality level clicked in the menu (this stops the demo's own cycling).

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1972 | 0.51 | 0.47 | 0.63 | 0.81 | 2.02 | 0.01 | 0.07 | 0.40 | 0.02 | 512 ms | 164 |
| QuickJS | 1728 | 0.58 | 0.52 | 0.75 | 1.09 | 2.25 | 0.15 | 0.00 | 0.41 | 0.02 | 577 ms | 166 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Low | 300 | 2051 | 1399 | 1.47x | 0.47 / 0.88 | 0.71 / 1.13 | 0.01 | 0.17 |
| Medium | 300 | 1885 | 1871 | 1.01x | 0.52 / 0.81 | 0.51 / 0.77 | 0.01 | 0.15 |
| High | 300 | 1833 | 1865 | 0.98x | 0.54 / 0.81 | 0.52 / 0.61 | 0.01 | 0.15 |
| Ultra | 300 | 2151 | 1887 | 1.14x | 0.45 / 0.51 | 0.52 / 0.67 | 0.01 | 0.15 |

### raindrop-demo

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1986 | 0.50 | 0.49 | 0.56 | 0.58 | 2.04 | 0.10 | 0.08 | 0.30 | 0.02 | 497 ms | 168 |
| QuickJS | 282 | 3.55 | 3.50 | 3.88 | 4.16 | 15.26 | 3.01 | 0.00 | 0.36 | 0.17 | 746 ms | 181 |

### scenes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2717 | 0.37 | 0.36 | 0.38 | 0.41 | 1.95 | 0.01 | 0.02 | 0.33 | 0.01 | 566 ms | 166 |
| QuickJS | 2172 | 0.46 | 0.43 | 0.58 | 0.79 | 2.11 | 0.10 | 0.00 | 0.34 | 0.01 | 589 ms | 166 |

### shapes

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2850 | 0.35 | 0.32 | 0.42 | 0.58 | 1.96 | 0.01 | 0.04 | 0.29 | 0.01 | 502 ms | 163 |
| QuickJS | 2836 | 0.35 | 0.34 | 0.37 | 0.43 | 1.87 | 0.08 | 0.00 | 0.26 | 0.01 | 528 ms | 164 |

### sine-circle-demo

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3045 | 0.33 | 0.30 | 0.38 | 0.43 | 1.79 | 0.02 | 0.02 | 0.28 | 0.01 | 534 ms | 165 |
| QuickJS | 1998 | 0.50 | 0.44 | 0.76 | 0.96 | 2.11 | 0.12 | 0.00 | 0.36 | 0.02 | 572 ms | 166 |

### splines

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2360 | 0.42 | 0.38 | 0.62 | 0.82 | 1.87 | 0.05 | 0.06 | 0.30 | 0.01 | 516 ms | 164 |
| QuickJS | 812 | 1.23 | 1.18 | 1.43 | 1.62 | 2.66 | 0.89 | 0.00 | 0.31 | 0.04 | 522 ms | 164 |

### text

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2250 | 0.44 | 0.43 | 0.46 | 0.64 | 1.42 | 0.01 | 0.01 | 0.42 | 0.01 | 539 ms | 165 |
| QuickJS | 2228 | 0.45 | 0.42 | 0.51 | 0.89 | 1.47 | 0.02 | 0.00 | 0.42 | 0.01 | 539 ms | 166 |

### tunnel

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 613 | 1.63 | 1.68 | 1.93 | 3.06 | 3.79 | 0.01 | 0.27 | 1.34 | 0.01 | 556 ms | 212 |
| QuickJS | 610 | 1.64 | 1.68 | 1.94 | 3.07 | 5.92 | 0.37 | 0.00 | 1.26 | 0.01 | 667 ms | 212 |

### windows

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1367 | 0.73 | 0.82 | 0.96 | 1.24 | 2.44 | 0.02 | 0.01 | 0.69 | 0.02 | 688 ms | 167 |
| QuickJS | n/a |||||||||||| 

### world

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1200 | 0.83 | 0.82 | 0.93 | 1.03 | 2.39 | 0.04 | 0.01 | 0.75 | 0.03 | 544 ms | 194 |
| QuickJS | 271 | 3.69 | 4.23 | 4.91 | 5.33 | 6.78 | 1.82 | 0.00 | 0.98 | 0.89 | 540 ms | 196 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1224 | 285 | 4.30x | 0.81 / 0.98 | 4.14 / 4.73 | 0.04 | 1.80 |
| pointer sweep | 600 | 1188 | 264 | 4.49x | 0.82 / 1.03 | 4.31 / 5.48 | 0.04 | 1.83 |
<!-- bench:end -->
