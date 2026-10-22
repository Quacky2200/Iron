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
- **Native regressions:** fog-world (0.86x), fog-blocks (0.93x), form (0.97x), and in clock the Dials, Meter, Sundial,
  Sand timer, Morphing and Alarm clock faces (0.5-0.8x). The cause is the **sync** column: `sync_native_render` costs
  1.8 ms a frame in fog-world, 1.95 in fog-park, 1.9 in godrays-clouds, 1.4 in godrays-forest and 0.6 in clock, often
  more than the script time it replaces. It walks the registries every frame whether or not anything changed; making it
  incremental (dirty flags per registry) is the obvious next optimisation. fog-blocks and form differ by 0.03 ms or less a
  frame (noise at 2000+ fps).
- **Start-up** is about the same (0.5-0.8 s), except emoji (2.2 s native against 1.2 s QuickJS: JIT-compiling about
  4700 lines) and clock (1.0 s both).
- Some originals on `main` are older versions of the examples (the branch changed them since), so a few scenes draw
  slightly different work: godrays-shed and godrays-cell render more native-side (4.1 / 1.9 ms against 2.2 / 1.1 ms)
  with equal script cost. emoji is random (each run picks other performers).
- **n/a:** launcher, md-reader and windows exist only on this branch (native only). css's original needs the Charm font,
  which is kept out of git; the runner copies it in from the checkout's `Charm/` folder. No original failed to build
  or run, and no run logged errors.

<!-- bench:begin (written by `iron-build bench`) -->
Machine: AMD Ryzen 9 5900X 12-Core Processor (32 GB RAM), NVIDIA GeForce RTX 5080 (Vulkan, DiscreteGpu), Microsoft Windows 11 Pro 10.0.22631.

Commit 189b820 against main (5b8aa52); every run 1280x720 px, a fixed step of 1/60 s per frame (`--fixed-dt`), no window, no vsync or frame cap, release host.

## Summary

| Example | Native fps | QuickJS fps | Speed-up | Native median / p99 ms | QuickJS median / p99 ms | Native script / sync ms | QuickJS script ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| [animation](#animation) | 1841 | 912 | 2.02x | 0.46 / 1.18 | 1.02 / 1.86 | 0.02 / 0.05 | 0.62 |
| [arcade](#arcade) | 2037 | 1241 | 1.64x | 0.39 / 1.34 | 0.69 / 2.12 | 0.03 / 0.03 | 0.38 |
| [boxes](#boxes) | 2394 | 2123 | 1.13x | 0.32 / 1.42 | 0.38 / 1.46 | 0.01 / 0.01 | 0.04 |
| [classics](#classics) | 2401 | 1802 | 1.33x | 0.38 / 1.34 | 0.49 / 1.42 | 0.01 / 0.03 | 0.15 |
| [clock](#clock) | 725 | 669 | 1.08x | 1.26 / 2.48 | 1.29 / 4.41 | 0.08 / 0.62 | 0.81 |
| [cloth](#cloth) | 800 | 808 | 0.99x | 1.22 / 2.55 | 1.22 / 2.38 | 0.01 / 0.02 | 0.07 |
| [cornell](#cornell) | 1871 | 1849 | 1.01x | 0.48 / 1.24 | 0.50 / 1.36 | 0.01 / 0.02 | 0.03 |
| [css](#css) | 2386 | 2211 | 1.08x | 0.40 / 1.03 | 0.37 / 3.90 | 0.01 / 0.01 | 0.04 |
| [dark-room](#dark-room) | 338 | 324 | 1.04x | 2.78 / 4.55 | 2.77 / 5.48 | 0.12 / 0.23 | 0.77 |
| [emoji](#emoji) | 477 | 284 | 1.68x | 1.90 / 4.23 | 2.63 / 13.84 | 0.06 / 0.50 | 1.53 |
| [fishtank](#fishtank) | 886 | 271 | 3.26x | 1.06 / 2.11 | 3.63 / 4.47 | 0.17 / 0.29 | 2.95 |
| [fog](#fog) | 1295 | 1199 | 1.08x | 0.72 / 2.03 | 0.81 / 1.99 | 0.01 / 0.14 | 0.26 |
| [fog-blocks](#fog-blocks) | 2267 | 2448 | 0.93x | 0.41 / 1.35 | 0.38 / 1.36 | 0.01 / 0.06 | 0.05 |
| [fog-park](#fog-park) | 223 | 209 | 1.07x | 4.28 / 6.36 | 4.70 / 6.06 | 0.02 / 1.95 | 2.43 |
| [fog-raider](#fog-raider) | 1670 | 1194 | 1.40x | 0.58 / 0.88 | 0.77 / 1.25 | 0.02 / 0.21 | 0.41 |
| [fog-teapots](#fog-teapots) | 2157 | 2107 | 1.02x | 0.41 / 0.80 | 0.44 / 0.80 | 0.01 / 0.02 | 0.05 |
| [fog-world](#fog-world) | 323 | 377 | 0.86x | 2.95 / 7.42 | 1.49 / 15.84 | 0.25 / 1.83 | 1.66 |
| [form](#form) | 3293 | 3402 | 0.97x | 0.30 / 0.34 | 0.29 / 0.31 | 0.01 / 0.01 | 0.01 |
| [godrays-cell](#godrays-cell) | 386 | 315 | 1.23x | 2.51 / 4.13 | 3.12 / 4.06 | 0.13 / 0.59 | 2.03 |
| [godrays-clouds](#godrays-clouds) | 206 | 178 | 1.16x | 4.72 / 5.94 | 5.54 / 6.43 | 0.30 / 1.87 | 2.48 |
| [godrays-forest](#godrays-forest) | 276 | 253 | 1.09x | 3.44 / 4.78 | 3.90 / 4.88 | 0.02 / 1.44 | 1.91 |
| [godrays-lift](#godrays-lift) | 1435 | 1393 | 1.03x | 0.67 / 1.23 | 0.68 / 1.61 | 0.01 / 0.05 | 0.10 |
| [godrays-room](#godrays-room) | 475 | 174 | 2.73x | 2.03 / 2.75 | 5.76 / 7.00 | 0.21 / 0.80 | 4.43 |
| [godrays-shed](#godrays-shed) | 222 | 221 | 1.00x | 4.33 / 6.07 | 4.34 / 6.12 | 0.10 / 0.33 | 2.23 |
| [godrays-sunset](#godrays-sunset) | 802 | 718 | 1.12x | 1.09 / 2.30 | 1.24 / 2.57 | 0.02 / 0.25 | 0.39 |
| [hello](#hello) | 8156 | 6794 | 1.20x | 0.12 / 0.22 | 0.12 / 0.25 | 0.01 / 0.01 | 0.02 |
| [launcher](#launcher) | 655 | n/a | - | 1.99 / 3.25 | n/a | 0.01 / 0.01 | n/a |
| [layers](#layers) | 1392 | 1257 | 1.11x | 0.70 / 1.12 | 0.78 / 0.90 | 0.01 / 0.21 | 0.31 |
| [light-units](#light-units) | 1428 | 1328 | 1.08x | 0.67 / 1.41 | 0.71 / 1.27 | 0.02 / 0.03 | 0.10 |
| [materials](#materials) | 485 | 483 | 1.00x | 1.71 / 3.40 | 1.78 / 3.33 | 0.01 / 0.04 | 0.12 |
| [md-reader](#md-reader) | 1350 | n/a | - | 0.58 / 1.41 | n/a | 0.01 / 0.01 | n/a |
| [physics](#physics) | 1889 | 751 | 2.51x | 0.44 / 1.20 | 1.24 / 2.41 | 0.26 / 0.01 | 0.86 |
| [platformer](#platformer) | 2095 | 1436 | 1.46x | 0.45 / 0.77 | 0.67 / 1.04 | 0.08 / 0.09 | 0.31 |
| [quality](#quality) | 1972 | 1728 | 1.14x | 0.47 / 0.81 | 0.52 / 1.09 | 0.01 / 0.07 | 0.15 |
| [raindrop-demo](#raindrop-demo) | 1696 | 274 | 6.19x | 0.57 / 0.91 | 3.60 / 4.87 | 0.13 / 0.10 | 3.07 |
| [scenes](#scenes) | 2717 | 2172 | 1.25x | 0.36 / 0.41 | 0.43 / 0.79 | 0.01 / 0.02 | 0.10 |
| [shapes](#shapes) | 2850 | 2836 | 1.01x | 0.32 / 0.58 | 0.34 / 0.43 | 0.01 / 0.04 | 0.08 |
| [sine-circle-demo](#sine-circle-demo) | 3045 | 1998 | 1.52x | 0.30 / 0.43 | 0.44 / 0.96 | 0.02 / 0.02 | 0.12 |
| [splines](#splines) | 2360 | 812 | 2.91x | 0.38 / 0.82 | 1.18 / 1.62 | 0.05 / 0.06 | 0.89 |
| [text](#text) | 2250 | 2228 | 1.01x | 0.43 / 0.64 | 0.42 / 0.89 | 0.01 / 0.01 | 0.02 |
| [tunnel](#tunnel) | 613 | 610 | 1.01x | 1.68 / 3.06 | 1.68 / 3.07 | 0.01 / 0.27 | 0.37 |
| [windows](#windows) | 1367 | n/a | - | 0.82 / 1.24 | n/a | 0.02 / 0.01 | n/a |
| [world](#world) | 1093 | 268 | 4.07x | 0.85 / 1.42 | 4.33 / 5.17 | 0.05 / 0.04 | 1.85 |

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
| native | 725 | 1.38 | 1.26 | 2.02 | 2.48 | 10.10 | 0.08 | 0.62 | 0.63 | 0.05 | 1018 ms | 245 |
| QuickJS | 669 | 1.50 | 1.29 | 3.76 | 4.41 | 8.39 | 0.81 | 0.00 | 0.64 | 0.05 | 999 ms | 247 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Analog | 240 | 827 | 680 | 1.21x | 1.17 / 1.79 | 1.41 / 2.10 | 0.05 | 1.00 |
| Dials | 240 | 733 | 1152 | 0.64x | 1.25 / 2.53 | 0.84 / 1.30 | 0.02 | 0.34 |
| Meter | 240 | 751 | 962 | 0.78x | 1.31 / 1.56 | 1.03 / 1.25 | 0.02 | 0.33 |
| Sundial | 240 | 478 | 608 | 0.79x | 2.02 / 3.53 | 1.59 / 2.49 | 0.04 | 0.22 |
| Sand timer | 240 | 889 | 1281 | 0.69x | 1.10 / 1.51 | 0.77 / 1.23 | 0.03 | 0.34 |
| Lava lamp | 240 | 559 | 251 | 2.23x | 1.76 / 2.11 | 3.81 / 7.00 | 0.44 | 3.08 |
| Underwater | 240 | 819 | 833 | 0.98x | 1.19 / 1.64 | 1.18 / 1.38 | 0.04 | 0.61 |
| Chandelier | 240 | 653 | 630 | 1.04x | 1.50 / 1.94 | 1.55 / 2.19 | 0.03 | 0.60 |
| Rainbow | 240 | 767 | 652 | 1.18x | 1.26 / 2.16 | 1.51 / 1.67 | 0.08 | 0.97 |
| Morphing | 240 | 937 | 1829 | 0.51x | 1.05 / 1.39 | 0.53 / 1.17 | 0.02 | 0.14 |
| Flip | 240 | 817 | 409 | 2.00x | 1.20 / 1.41 | 2.40 / 2.80 | 0.16 | 1.95 |
| Alarm clock | 240 | 770 | 1187 | 0.65x | 1.27 / 1.49 | 0.83 / 1.06 | 0.02 | 0.18 |

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
| native | 338 | 2.96 | 2.78 | 4.31 | 4.55 | 10.60 | 0.12 | 0.23 | 2.59 | 0.02 | 774 ms | 341 |
| QuickJS | 324 | 3.09 | 2.77 | 5.01 | 5.48 | 8.32 | 0.77 | 0.00 | 2.29 | 0.03 | 670 ms | 334 |

### emoji

Input: the show is random: each run picks other performers.

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 477 | 2.10 | 1.90 | 3.36 | 4.23 | 47.49 | 0.06 | 0.50 | 1.45 | 0.08 | 2183 ms | 282 |
| QuickJS | 284 | 3.52 | 2.63 | 9.86 | 13.84 | 38.58 | 1.53 | 0.00 | 1.85 | 0.14 | 1222 ms | 271 |

### fishtank

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 886 | 1.13 | 1.06 | 1.58 | 2.11 | 5.54 | 0.17 | 0.29 | 0.66 | 0.01 | 565 ms | 166 |
| QuickJS | 271 | 3.69 | 3.63 | 4.03 | 4.47 | 5.69 | 2.95 | 0.00 | 0.65 | 0.09 | 602 ms | 168 |

### fog

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1295 | 0.77 | 0.72 | 0.97 | 2.03 | 2.44 | 0.01 | 0.14 | 0.61 | 0.01 | 561 ms | 186 |
| QuickJS | 1199 | 0.83 | 0.81 | 1.06 | 1.99 | 2.65 | 0.26 | 0.00 | 0.57 | 0.01 | 602 ms | 187 |

### fog-blocks

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2267 | 0.44 | 0.41 | 0.54 | 1.35 | 2.54 | 0.01 | 0.06 | 0.36 | 0.01 | 548 ms | 166 |
| QuickJS | 2448 | 0.41 | 0.38 | 0.50 | 1.36 | 2.04 | 0.05 | 0.00 | 0.35 | 0.01 | 638 ms | 167 |

### fog-park

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 223 | 4.48 | 4.28 | 5.70 | 6.36 | 10.29 | 0.02 | 1.95 | 2.50 | 0.01 | 601 ms | 308 |
| QuickJS | 209 | 4.78 | 4.70 | 5.41 | 6.06 | 8.96 | 2.43 | 0.00 | 2.33 | 0.02 | 824 ms | 254 |

### fog-raider

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1670 | 0.60 | 0.58 | 0.72 | 0.88 | 2.28 | 0.02 | 0.21 | 0.36 | 0.01 | 566 ms | 165 |
| QuickJS | 1194 | 0.84 | 0.77 | 1.09 | 1.25 | 2.78 | 0.41 | 0.00 | 0.41 | 0.02 | 614 ms | 166 |

### fog-teapots

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 2157 | 0.46 | 0.41 | 0.62 | 0.80 | 2.21 | 0.01 | 0.02 | 0.43 | 0.01 | 610 ms | 166 |
| QuickJS | 2107 | 0.47 | 0.44 | 0.62 | 0.80 | 2.17 | 0.05 | 0.00 | 0.41 | 0.01 | 652 ms | 167 |

### fog-world

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 323 | 3.09 | 2.95 | 3.64 | 7.42 | 12.54 | 0.25 | 1.83 | 1.00 | 0.01 | 689 ms | 299 |
| QuickJS | 377 | 2.65 | 1.49 | 9.94 | 15.84 | 18.69 | 1.66 | 0.00 | 0.97 | 0.03 | 921 ms | 279 |

### form

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 3293 | 0.30 | 0.30 | 0.32 | 0.34 | 1.45 | 0.01 | 0.01 | 0.28 | 0.01 | 509 ms | 166 |
| QuickJS | 3402 | 0.29 | 0.29 | 0.30 | 0.31 | 1.44 | 0.01 | 0.00 | 0.27 | 0.01 | 545 ms | 168 |

### godrays-cell

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 386 | 2.59 | 2.51 | 2.90 | 4.13 | 6.29 | 0.13 | 0.59 | 1.85 | 0.01 | 551 ms | 199 |
| QuickJS | 315 | 3.18 | 3.12 | 3.68 | 4.06 | 5.59 | 2.03 | 0.00 | 1.06 | 0.09 | 610 ms | 206 |

### godrays-clouds

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 206 | 4.84 | 4.72 | 5.41 | 5.94 | 9.02 | 0.30 | 1.87 | 2.66 | 0.01 | 535 ms | 220 |
| QuickJS | 178 | 5.61 | 5.54 | 6.07 | 6.43 | 10.71 | 2.48 | 0.00 | 2.81 | 0.32 | 676 ms | 211 |

### godrays-forest

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 276 | 3.62 | 3.44 | 4.37 | 4.78 | 7.40 | 0.02 | 1.44 | 2.16 | 0.01 | 538 ms | 201 |
| QuickJS | 253 | 3.96 | 3.90 | 4.34 | 4.88 | 8.69 | 1.91 | 0.00 | 2.03 | 0.02 | 743 ms | 206 |

### godrays-lift

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 1435 | 0.70 | 0.67 | 0.81 | 1.23 | 2.54 | 0.01 | 0.05 | 0.63 | 0.01 | 518 ms | 191 |
| QuickJS | 1393 | 0.72 | 0.68 | 0.87 | 1.61 | 2.76 | 0.10 | 0.00 | 0.61 | 0.01 | 569 ms | 190 |

### godrays-room

| Backend | fps | mean | median | p95 | p99 | worst | script | sync | render | other | start-up | peak MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| native | 475 | 2.10 | 2.03 | 2.54 | 2.75 | 7.59 | 0.21 | 0.80 | 1.08 | 0.01 | 548 ms | 212 |
| QuickJS | 174 | 5.75 | 5.76 | 6.31 | 7.00 | 9.61 | 4.43 | 0.00 | 1.14 | 0.18 | 630 ms | 213 |

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
| native | 8156 | 0.12 | 0.12 | 0.13 | 0.22 | 1.25 | 0.01 | 0.01 | 0.10 | 0.01 | 477 ms | 163 |
| QuickJS | 6794 | 0.15 | 0.12 | 0.22 | 0.25 | 1.28 | 0.02 | 0.00 | 0.12 | 0.01 | 489 ms | 164 |

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
| native | 1696 | 0.59 | 0.57 | 0.75 | 0.91 | 2.14 | 0.13 | 0.10 | 0.34 | 0.02 | 537 ms | 170 |
| QuickJS | 274 | 3.65 | 3.60 | 4.09 | 4.87 | 14.11 | 3.07 | 0.00 | 0.41 | 0.18 | 788 ms | 183 |

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
| native | 1093 | 0.92 | 0.85 | 1.20 | 1.42 | 2.61 | 0.05 | 0.04 | 0.80 | 0.03 | 598 ms | 193 |
| QuickJS | 268 | 3.73 | 4.33 | 4.84 | 5.17 | 6.44 | 1.85 | 0.00 | 0.98 | 0.91 | 573 ms | 198 |

| Segment | frames | native fps | QuickJS fps | speed-up | native median / p99 | QuickJS median / p99 | native script | QuickJS script |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| idle | 300 | 1173 | 260 | 4.52x | 0.84 / 1.03 | 4.46 / 5.34 | 0.04 | 1.89 |
| pointer sweep | 600 | 1057 | 273 | 3.87x | 0.87 / 1.45 | 4.30 / 4.90 | 0.05 | 1.82 |
<!-- bench:end -->
