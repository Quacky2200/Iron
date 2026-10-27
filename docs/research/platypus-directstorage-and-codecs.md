# DirectStorage and the pack codec (research)

Status: **research, 2026-10-21.** Question from the user: should Windows DirectStorage decide the compression codec of asset packs
(`crates/iron_pack`, design `iron-asset-packs.md`), given that xz-like codecs decompress slowly on the CPU, and how do zstd, gzip/deflate
and LZ4 compare with what DirectStorage can decompress? Answer in short: **no, not as the one codec**. DirectStorage pays off through
its I/O path and GPU decompression of large GPU-bound payloads (textures, vertex buffers); the pack format already carries a codec per
chunk, so a Windows-only fast path can be a **second codec on texture and mesh chunks of a Windows build**, not the format's default.
The default stays LZ4 for now; zstd is the likely second codec everywhere; GDeflate vs zstd for the DirectStorage path waits for a
prototype (section 6, TODO `CODEC-1`).

Sources were read on 2026-10-21. Every claim is marked with its source; section 7 lists what could not be verified.

## 1. DirectStorage on Windows

### 1.1 What it is

A user-mode Windows API (COM, `dstorage.h`, MIT-licensed headers) shipped as two redistributable DLLs, `dstorage.dll` and
`dstoragecore.dll`, in the NuGet package `Microsoft.Direct3D.DirectStorage`. The game creates a factory (`DStorageGetFactory`), opens
files (`IDStorageFactory::OpenFile`), creates queues (`CreateQueue` with a D3D12 device, a priority and a source type: file or memory),
enqueues **requests** (`DSTORAGE_REQUEST`: source file + offset + size, compression format, uncompressed size, destination), then
`EnqueueSignal(ID3D12Fence, value)` or `EnqueueSetEvent` and `Submit`. DirectStorage reads (only reads: there is no write path), stages
in its own staging buffers, decompresses on the CPU or GPU, and copies into the destination. Destinations (Developer Guidance):

| Destination | What |
|---|---|
| `MEMORY` | a CPU pointer + size (always CPU decompression) |
| `BUFFER` | a region of an `ID3D12Resource` buffer (written like `CopyBufferRegion`) |
| `TEXTURE_REGION` | a box of one subresource; source data laid out as `GetCopyableFootprints()` gives (256-byte row pitch) |
| `MULTIPLE_SUBRESOURCES` | the first subresource and all after it, footprint layout |
| `MULTIPLE_SUBRESOURCES_RANGE` (1.3) | a count of contiguous subresources from an index |
| `TILES` | a region of tiles of a reserved (tiled) resource, arranged as `CopyTiles()` |

Guidance from Microsoft: read at least 32 KiB per request; the uncompressed size of a request must fit the staging buffer (default
32 MiB; NVIDIA recommends 128 MiB to saturate the pipeline); GPU decompression uses two VRAM staging buffers of that size. After 1.2.1
the destination is left in the `COMMON` state.

### 1.2 Versions

From the NuGet listing and release notes (nuget.org, read 2026-10-21):

| Version | Date | Added |
|---|---|---|
| 1.0.0 | 2022-03-14 | first release: the I/O path (file -> memory / D3D12 resources), CPU-side **custom decompression** queue; no GPU decompression |
| 1.0.2 | 2022-05-24 | package layout change, custom decompression flags |
| 1.1.0 | 2022-11-07 | **GDeflate** (`DSTORAGE_COMPRESSION_FORMAT_GDEFLATE`) with **GPU decompression** (DirectCompute fallback shader plus vendor metacommands) and a CPU fallback; `DStorageCreateCompressionCodec` / `IDStorageCompressionCodec` (compress at build time with the runtime's own codec); `EnqueueSetEvent` |
| 1.1.1 | 2022-12-07 | fixes |
| 1.2.0 | 2023-04-18 | buffered file I/O for HDDs (`ForceFileBuffering`, needs `DisableBypassIO`), `IDStorageQueue2::GetCompressionSupport` (GPU optimized / GPU fallback / CPU fallback), a `.winmd` for non-C++ bindings |
| 1.2.1 to 1.2.4 | 2023-05 to 2025-04 | destination left in `COMMON`; ARM32 removed; shader fixes (HLSL 2021, TDRs on older GPUs) |
| 1.3.0 | 2025-06-27 | `IDStorageQueue3::EnqueueRequests` (a batch under one fence value), `MULTIPLE_SUBRESOURCES_RANGE`; CPU GDeflate rejects corrupt data |
| 1.4.0-preview1 | 2026-03-09 (blog 2026-03-11, GDC) | **Zstandard** (`DSTORAGE_COMPRESSION_FORMAT_ZSTD`, CPU and GPU decompression; the GPU shader is open source on the `development` branch and "initially optimized for content chunked to 256KB or smaller"); **GACL** 1.0 preview (Game Asset Conditioning Library: build-time BCn shuffling and entropy reduction for better zstd ratios, "up to 50%"; the runtime un-shuffles BC1/BC3/BC4/BC5 after decompression, BC7 later); `DStorageSetConfiguration2` (D3D12 CreatorID) |
| 1.4.0-preview2 | 2026-06-16 | a GACL enqueue throughput fix |

**Latest stable: 1.3.0. zstd exists only in the 1.4 previews.** Known issues of preview1: the zstd GPU fallback shader "may cause TDRs
with certain compressed content on some GPUs"; staging buffers over 256 MB may fail. The `zstd` folder's README: "in development and
should NOT be used in production". Vendor-tuned zstd drivers: AMD and NVIDIA "2H 2026", Qualcomm "before the end of 2026", Intel "in
the months ahead" (blog, 2026-03-11). No GA date was found.

### 1.3 Requirements and behaviour by device

- **OS**: Windows 10 and 11 (1.0 blog: "compatible with Windows 10 devices", Windows 11 recommended). The often-quoted minimum "Windows
  10 1909" comes from the 2021 developer-preview blog; no current Microsoft page states a minimum version (not verified for 1.3/1.4).
- **BypassIO** (the Windows 11 fast read path, Microsoft Learn, 2024-09): Windows 11 client only, **NVMe only, NTFS only, noncached reads
  only**, files only; a storage or filter driver that does not opt in sends reads down the classic path. Windows 10 uses "an optimized IO
  layer built on top of the Win32 APIs".
- **SATA SSD / HDD**: DirectStorage works on any storage ("benefits on any storage device"); the gains are largest on NVMe. HDDs need
  `ForceFileBuffering` (1.2) to hide seek times. Not stated as a requirement anywhere: NVMe or a minimum drive size (third-party sites
  claiming "1 TB NVMe" are wrong as far as Microsoft's pages go).
- **GPU decompression**: "supported on all DirectX 12 + Shader Model 6.0 GPUs" (1.1 blog). Vendor metacommands in drivers speed it up
  (NVIDIA "RTX IO" from Game Ready 526.47); otherwise the DirectCompute fallback runs on a compute queue; with neither, CPU fallback.
  `GetCompressionSupport` tells which path is in use. GPU decompression applies only to D3D12 destinations (buffers, textures, tiles),
  never to `MEMORY`.
- **Feature level**: DirectStorage needs a D3D12 device; no feature level beyond what SM 6.0 implies is documented (not verified).

### 1.4 Numbers that exist (vendor and press, not controlled)

- Microsoft's 1.1 sample: a load took 0.8 s with GPU GDeflate against 2.36 s with CPU decompression, with much lower CPU use.
- Intel (Arc A770, i9-12900K): 21.67 GB/s GPU GDeflate against 7.88 GB/s on the CPU path (all cores).
- IgorsLab/Compusemble benchmark: GPUs about 2.5x the i9-12900K; A770 16.8 GB/s, RX 7900 XT 14.6 GB/s.
- NVIDIA: uncompressed streaming tops out near 3 GB/s on PCIe Gen3; NVMe up to 7 GB/s; GPU decompression multiplies the effective rate
  by the ratio.

These are whole-pipeline numbers on game-like data with ideal staging buffers; none is for Iron's data or this machine.

### 1.5 Iron's renderer and DirectStorage

- **Backend**: DirectStorage writes only into **D3D12** resources. Iron runs **Vulkan by default on Windows** (`WGPU_BACKEND=dx12`
  switches, CLAUDE.md). A DirectStorage GPU path therefore means making DX12 the default on Windows, or keeping it DX12-only; on Vulkan
  only the CPU path (`MEMORY` destination) works.
- **Interop with wgpu 30** (Cargo.lock: `wgpu` 30.0.1, `wgpu-hal` 30.0.1 with `windows` 0.62.2): `wgpu::Device::as_hal::<Dx12>()` gives
  the hal device whose `raw_device()` is the `ID3D12Device` (hal source, read 2026-10-21); `raw_queue()` gives the `ID3D12CommandQueue`.
  Resources: either create them in wgpu and take the raw `ID3D12Resource` through `Buffer::as_hal` / `Texture::as_hal` (`raw_resource()`),
  or create them with D3D12 and import them with `wgpu_hal::dx12::Device::texture_from_raw(resource, format, dimension, size, mips,
  samples, drop_callback)` / `buffer_from_raw(resource, size, drop_callback)` plus `wgpu::Device::create_texture_from_hal` /
  `create_buffer_from_hal`.
- **Synchronisation**: DirectStorage signals an `ID3D12Fence` the game creates. wgpu has no public "wait on external fence"; the
  workable form is calling `ID3D12CommandQueue::Wait(fence, value)` on the raw queue before the wgpu submission that uses the data, or
  polling the fence on the CPU and only then letting wgpu use the resource (simplest, one frame of latency). **Resource states**:
  DirectStorage leaves the destination in `COMMON`; wgpu tracks its own states. Buffers promote and decay implicitly from `COMMON` in
  D3D12, so a **buffer destination is the safe first target**; texture destinations need the wgpu-side state to agree (not verified,
  prototype needed).
- **Rust bindings**: no DirectStorage module was found in `windows` 0.62 (DirectStorage is not in the Windows SDK metadata; it ships its
  own `Microsoft.Direct3D.DirectStorage.winmd` since 1.2, so `windows-bindgen` can generate bindings from it). Crates: `direct-storage`
  0.7.1 (Tsukisoft, MIT/Apache-2.0, 2025-09-09, binds DirectStorage 1.3, loads the DLLs with `libloading`, unofficial, 16 stars);
  `microsoft-directstorage` is yanked. Neither is in Iron's lock files.
- **Licence**: headers MIT; the DLLs under "Microsoft Software License Terms for the Microsoft DirectStorage SDK": object code from the
  distributables list may ship with an app that adds significant functionality; no reverse engineering. The DLLs are Windows only and
  sit next to the exe (about the same model as the Agility SDK). The GDeflate reference code and the zstd shader live in
  `github.com/microsoft/DirectStorage` (repo licence MIT; the 1.1 blog announced GDeflate under Apache-2.0: which applies to the
  `GDeflate/` folder was not verified, both are permissive).

## 2. Formats DirectStorage decompresses

| Format | Since | Where | Notes |
|---|---|---|---|
| none | 1.0 | - | stored data, any destination |
| custom (`DSTORAGE_CUSTOM_COMPRESSION_0 + n`) | 1.0 | the game's CPU threads | `IDStorageCustomDecompressionQueue(1)`: the game waits on `GetEvent`, takes requests (`GetRequests`: compressed bytes already read into a system-memory staging buffer, a destination pointer), decompresses, calls `SetRequestResults`; for VRAM destinations the output goes to an upload heap and DirectStorage copies it on. **So LZ4 (or anything) works through DirectStorage's I/O path, decoded on the CPU.** |
| GDeflate | 1.1 | GPU (metacommand or fallback shader) or CPU | Built-in. Encoded at build time with `IDStorageCompressionCodec` (the runtime's codec) or the open-source library. |
| zstd | **1.4 preview only** | GPU (fallback shader, vendor drivers from 2H 2026) or CPU | GPU shader tuned for chunks of 256 KiB or less; GACL shuffle as a per-request option (BC1/3/4/5). Not in a stable SDK yet. |

Not supported: LZ4, LZMA/xz, Brotli, Oodle, standard deflate/zlib/gzip streams (only through the custom queue on the CPU).

**GDeflate** (`GDeflate/GDeflate/README.md` in the DirectStorage repo; NVIDIA blog): the input is split into **64 KiB tiles** compressed
independently (thread-level parallelism; 64 KiB is also the D3D12/Vulkan sparse tile size). Inside a tile it is an RFC 1951 DEFLATE
stream (same block types, Huffman codes, LZ77 window) whose bits are **swizzled into 32 sub-streams**: codes are dealt round-robin so 32
GPU lanes decode one symbol each per round (64-bit bit buffers refilled 32 bits at a time; stored blocks dealt byte by byte; about 128
bytes of zero padding at the end). Ratio "should match DEFLATE, with small caveats about end effects", so GDeflate is **not** readable by
zlib/gzip and gzip is not GDeflate, but a GDeflate encoder is a DEFLATE encoder plus a bit shuffler, and its ratio is deflate's (about
libdeflate level 6 to 12). The reference CPU codec is a modified libdeflate (`3rdparty/libdeflate`, MIT). Rust: `gdeflate` 0.4.1
(2024-07-07, MIT/Apache-2.0, over `gdeflate-sys` 0.4.1, which builds the C++ code: a `cc` build, a download, not in the lock files).

Vulkan has the same format: `VK_NV_memory_decompression` (Vulkan 1.3.233) and the ratified **`VK_EXT_memory_decompression`** (revision
1, 2025-01-23; in the Vulkan 1.4.330 headers, 2025-10-24) with a `GDEFLATE_1_0` method and at most 65,536 decompressed bytes per region
(one tile per region). So GDeflate is the one GPU-decompressed format with both a D3D12 and a Vulkan path; which drivers expose the EXT
beyond NVIDIA was not checked.

## 3. Codec comparison

### 3.1 General numbers (Silesia corpus, single thread)

lzbench 2.4, AMD EPYC 9555P 3.2 GHz, turbo off (the lzbench README, read 2026-10-21). Ratio = compressed / original (lower is better).
Iron's own numbers on its own corpus (in-crate LZ4, `miniz_oxide` deflate, Windows, 24 threads) are in `iron-asset-packs.md` section 7.

| Codec | Compress MB/s | Decompress MB/s | Size | License | Rust crate | In Iron's lock files |
|---|---|---|---|---|---|---|
| memcpy | 24,075 | 24,002 | 100% | | | |
| LZ4 1.10 | 550 | 3,832 | 47.6% | BSD-2 | `lz4_flex` (pure Rust, MIT; safe decode about 85% of C) | no (in-crate decoder in `iron_pack/src/lz4.rs`) |
| LZ4HC -9 / -12 | 31.9 / 10.4 | 3,703 / 3,813 | 36.8% / 36.5% | BSD-2 | `lz4_flex` has no HC encoder; C `lz4` crate | no |
| zstd 1.5.7 -1 | 411 | 1,351 | 34.5% | BSD or GPLv2 | `zstd` (C, `cc`), `ruzstd` (pure Rust, MIT) | no (`cc` 1.8 is; `zstd-sys` is not) |
| zstd -22 (`--ultra`) | 1.99 | 1,087 | 24.7% | | | |
| libdeflate 1.26 -1 / -6 / -12 | 202 / 82 / 4.7 | 783 / 820 / 832 | 34.7% / 31.9% / 30.5% | MIT | `libdeflater` (C) | no |
| zlib 1.3.2 -1 / -6 / -9 | 89 / 26.8 / 11.3 | 297 / 314 / 318 | 36.5% / 32.2% / 31.9% | zlib | `flate2` over `miniz_oxide` (pure Rust) or `zlib-rs` (pure Rust) | **yes**: `flate2` 1.1.10, `miniz_oxide` 0.8.9 and 0.9.1, `zlib-rs` 0.6.8 (both locks) |
| Brotli 1.2 -5 / -11 | 53.3 / 0.56 | 419 / 383 | 28.1% / 23.8% | MIT | `brotli` (pure Rust) | decoder only: `brotli-decompressor` 5.0.3 (both locks) |
| xz 5.8 -9 | 2.75 | 122 | 23.0% | 0BSD / PD | `xz2` (C), `lzma-rs` (pure Rust, slow) | no |
| LZMA 26.03 -9 | 2.55 | 78 | 23.0% | PD | as above | no |
| GDeflate | about libdeflate's | CPU: deflate-class per core, tile-parallel; GPU: 15 to 22 GB/s whole pipeline (section 1.4) | about deflate's | MIT / Apache-2.0 | `gdeflate` (C++ via `gdeflate-sys`) | no |
| Oodle Kraken / Mermaid / Selkie | slow optimal levels | about 1.0-1.5 / 1.9-2.6 / 4.2 GB/s | 2.44 / 2.21 / 1.86 to 1 (Epic's "level 4" set) | proprietary (RAD/Epic) | none official | no |

zstd levels 3, 9 and 19 are not in this table: zstd's decode speed barely depends on the level (1.0 to 1.4 GB/s between -1 and -22
above), and the ratio moves between the -1 and -22 rows; compression speed falls from hundreds of MB/s (-1, -3) to a few MB/s (-19).
Long-window (`--long`) and dictionary modes help small entries (scene JSON, many small prefabs) most; multithreaded zstd frames speed
up compression, not decompression of one frame (Iron's chunks are independent, so decode is already parallel per chunk). `ruzstd` decodes
1.4x (incompressible data) to 3.5x (enwik9) slower than C zstd and its encoder has four levels (about 1, 3, 7, 11).

The user's worry is right: **xz/LZMA decode at 80 to 120 MB/s per core**, 30 to 50 times slower than LZ4, for about 2x smaller output
than LZ4 on text-like data. On a 2 GB scene that is 15 to 25 core-seconds: only for downloads that get recompressed on install (Unity's
LZMA bundles are recompressed to LZ4 in the cache for this reason), never for packs read at load time. Brotli has the same profile at
a smaller penalty (400 MB/s).

### 3.2 Game data

- **BCn textures** are already block compressed (fixed 4 or 8 bits per pixel). General LZ codecs gain little on photographic BC data
  (the asset-packs note estimates 1.1 to 1.4 with LZ4, 1.2 to 1.6 with deflate); the gains come from making BC data friendlier first:
  RDO BC encoders (bc7enc_rdo and similar), Oodle Texture (proprietary, used with Kraken in hardware on PS5), Microsoft's GACL (shuffle,
  plus lossy entropy reduction, for zstd; licence not verified), Basis Universal (Apache-2.0: ETC1S/UASTC transcoded to BC/ASTC/ETC at
  load, zstd supercompression in KTX2), Crunch (older, ETC1S's ancestor). This is cooker work and independent of the container codec,
  except GACL, which is tied to DirectStorage zstd.
- **Meshes** (f32 streams, u32 indices): LZ4 1.6, deflate-6 2.26 on Iron's models (measured). Byte-shuffling the vertex streams or
  meshoptimizer-style encoding gains more than any codec switch (a cooker follow-up).
- **Audio**: ogg/mp3/opus/flac are stored (1/16 rule); wav compresses a little. No codec question.
- **Scene JSON, fonts, data**: small; ratios 5 to 10 with any codec; zstd with a dictionary is best for many small entries.

### 3.3 What shipping engines do

| Engine | Pack codec | Source |
|---|---|---|
| Unreal (`.pak`, IoStore `.utoc/.ucas`) | Oodle (Kraken by default; Mermaid, Selkie, Leviathan; levels -4..9, default 5 for shipping, 7 for distribution); platforms with hardware decompression (e.g. zlib) override it | Epic docs "Oodle Data" (UE 5.0) |
| Unity AssetBundles | LZMA (whole-file stream, default, for CDN download), LZ4 chunk-based (128 KB chunks), uncompressed (16-byte aligned); downloaded LZMA bundles are recompressed to LZ4 in the cache | Unity Manual "AssetBundle compression formats" |
| Godot | `FileAccess` compression modes FastLZ, Deflate, Zstandard, gzip, Brotli (decompress only); `.pck` file compression not checked | Godot docs `FileAccess` |
| Xbox Series | hardware decompression of a standard LZ format plus BCPack for textures (Microsoft spec material) | press/forum reposts; the format (zlib) and rates (4.8 GB/s) not verified |
| PS5 | hardware Kraken decoder, Oodle Texture licensed by Sony | press (RAD statements) |

No major engine ships xz/LZMA for data read at load time; the fast path everywhere is an LZ codec with a cheap decoder (Oodle, LZ4,
zstd) or hardware/GPU decompression.

## 4. Other platforms

| Platform | Fast loading API | Codecs it decompresses | Fits Iron's `.irpk`? |
|---|---|---|---|
| Windows (D3D12) | DirectStorage | GDeflate (GPU/CPU), zstd (1.4 preview), custom (CPU) | yes: requests are (file, offset, size, format), Iron's chunks map to requests |
| Windows/Linux/Android (Vulkan) | `VK_EXT_memory_decompression` (GDeflate 1.0, GPU memory to GPU memory; the I/O is the app's) | GDeflate | yes, as chunk payloads; driver coverage unknown |
| macOS 13 / iOS 16 and later (Metal 3) | `MTLIOCommandQueue` (fast resource loading: loads into `MTLBuffer` / `MTLTexture` / bytes) | zlib, LZFSE, LZ4, LZMA, LZBitmap (`MTLIOCompressionMethod`) | **no for compressed data**: compressed files must be written by `MTLIOCreateCompressionContext(path, method, chunkSize)` in Apple's own chunked file format (WWDC22 sample: 64 KiB chunks, zlib) and opened with `makeIOFileHandle(url:compressionMethod:)`; Iron's own container can only use the uncompressed handle with offsets |
| Android | none: `AAssetManager` reads from the APK (stored entries, `AAsset_openFileDescriptor64`), mmap | none (CPU) | yes (already designed: packs stored uncompressed in the APK) |
| Linux | `io_uring` (async reads, no decompression) | none (CPU) | yes |
| Xbox (GDK) | DirectStorage for Xbox (hardware decompression) | zlib-class LZ and BCPack (not verified) | needs a console build of the packer; out of scope until a console target exists |
| PS5 | Sony's I/O API with hardware Kraken | Oodle Kraken | needs Oodle (licence); out of scope |

So **one portable codec plus platform fast paths** works only if the fast paths take the same bytes or the build writes a pack per
target. Apple's fast path wants its own file format, the consoles want their own codecs, DirectStorage wants GDeflate or zstd: no single
compressed format is a fast path everywhere. Uncompressed (stored, 4 KiB-aligned, contiguous) entries are the only payload every fast
loader reads as is.

### 4.1 Choosing a codec per platform at build time

The format already allows it: the codec is a byte **per chunk**, `index.json` names the pack codec, and the content hash is over raw
bytes. What it needs:

1. `iron-build pack --target windows|macos|linux|android|ios` (default: the host) with a per-target codec table **by entry kind**, e.g.
   Windows+DirectStorage: textures and meshes GDeflate (or zstd), everything else LZ4; others: LZ4 everywhere. The plan (entries, GUIDs,
   order, split) is the same for every target; only the stored chunk bytes differ.
2. One `dist/data-<target>/` per target; `index.json` gets `"target"` and per-kind `"codecs"`. The pack id is a TOC hash, so two
   targets get different ids, and the manifest id check keeps a Windows `data/` from being mounted by a macOS build by mistake.
3. Readers must decode every codec they might meet on the CPU (a GDeflate chunk on a machine without DirectStorage or on Vulkan), so
   each codec added to a target's table needs a CPU decoder in `iron_pack` on that target, even when it is slow. For GDeflate that is a
   C++ dependency on Windows only.
4. Stores and patches: Steam depots per OS already split builds; patch packs (plan step 13) are per target too. Disk and upload cost:
   N targets = N packs of the same assets (no change to how each is built).

## 5. What this means for Iron

1. **DirectStorage's biggest win does not need a new codec.** With stored or LZ4 chunks, DirectStorage still gives the BypassIO/NVMe
   read path, batched requests and fewer CPU copies; LZ4 then goes through the custom decompression queue (CPU) into an upload heap,
   or `MEMORY` destinations. Iron's packs are already DirectStorage-ready for that: entries over 4 KiB start on 4 KiB boundaries, chunks
   are 256 KiB raw (above the 32 KiB minimum), each chunk has its own offset, size and codec.
2. **The GPU path only pays for GPU-bound bytes** (BC textures, vertex/index buffers) and only on D3D12, which Iron does not use by
   default today. Iron's in-crate LZ4 reads 0.9 to 1.9 GB/s per core (a tuned decoder 3 to 4 GB/s) on a 24-thread machine: CPU LZ4 on
   8 threads is above NVMe speed, so GPU decompression wins on CPU time, not wall time, unless the ratio is much better than LZ4's.
3. **GDeflate vs zstd for that path.** GDeflate: stable since 1.1 (2022), deflate ratio, a Vulkan twin, but a C++ encoder/decoder
   dependency and deflate's slow CPU fallback (about 0.8 GB/s per core). zstd: better ratio than deflate, 1.0 to 1.4 GB/s CPU decode, the
   same codec could serve every platform's CPU path, GACL for BC data, but in DirectStorage only as a preview with a production warning
   and vendor drivers due in 2H 2026. Microsoft is clearly moving to zstd; GDeflate remains supported.
4. **Chunking**: Iron's 256 KiB chunk is exactly the zstd shader's sweet spot and four GDeflate tiles (a 256 KiB GDeflate stream is one
   request with four 64 KiB tiles), so no format change is needed for either; `chunk_log2 = 16` (64 KiB) would only matter if each chunk
   had to be a single GDeflate tile (Vulkan EXT regions are one tile each: a 256 KiB chunk becomes four regions).
5. **Texture layout for DirectStorage** (plan step 12, `IRTX` v2): with a `BUFFER` destination, DirectStorage writes the whole entry
   (all chunks, one request each, at `n << chunk_log2` in the buffer) and wgpu copies each mip with `copy_buffer_to_texture`; wgpu needs
   `bytes_per_row` a multiple of 256 and the offset a multiple of the block size, which is what `GetCopyableFootprints` gives too. So the
   cooker should **store each mip with a 256-byte row pitch** (only mips narrower than 128 texels in BC1 or 64 in BC3 get padding: a few
   KiB) and **mip offsets a multiple of 512**. With that layout the same bytes also work for `TEXTURE_REGION` / `MULTIPLE_SUBRESOURCES`
   later if each streamed mip group starts on a chunk boundary (pad the entry to the chunk size before each large mip: at most 256 KiB
   per texture, only for textures with mips over 256 KiB). Tiled/sparse (`TILES`) needs 64 KiB tile layout: not planned.

## 6. Recommendation

| Question | Pick | Why | Confidence |
|---|---|---|---|
| (a) default codec for packs, all platforms | **LZ4 block (codec 1)**, as built | fastest CPU decode, no new crate, every target, BC data gains little from stronger codecs anyway | firm for now |
| second codec, all platforms | **zstd (codec 3)**, when the user OKs the crate (C `zstd` via `cc`, which is in the lock; or `ruzstd` decoder + an encoder) | deflate's ratio or better at near-LZ4 speed; the same bytes later feed DirectStorage 1.4 zstd; replaces deflate as the "small download" codec | strong, needs the user's OK for the crate |
| deflate (codec 2) | keep as built (pure Rust, in the lock) for download-bound targets until zstd lands | no new dependency | firm |
| xz/LZMA, Brotli | **no** as pack codecs (80 to 420 MB/s decode) | load-time cost | firm |
| (b) GDeflate chunk mode for Windows | **reserve codec 4 = GDeflate now** (one line in `format.rs`), build it only if `CODEC-1` says so; same 256 KiB chunks, 4 KiB-aligned entries | stable DirectStorage GPU path today, but needs DX12 by default and a C++ dependency, and zstd may make it moot | **pending (`CODEC-1`)** |
| DirectStorage itself | later, after the `iron_app` seam (plan step 10), as a Windows loader behind the `Vfs` (`AssetSource::read_into_gpu` style), stored and LZ4 chunks first (custom queue) | the I/O win is codec-independent | firm direction, timing open |
| (c) ship first | LZ4 + none + deflate as built; the `IRTX` v2 layout rules of 5.5 in plan step 12; per-target codec tables (4.1) only when a second codec exists | nothing waits on DirectStorage | firm |

### TODO `CODEC-1`: decide the Windows GPU codec (GDeflate, zstd, or none)

Needs the user's OK first: new crates/downloads (`direct-storage` or bindings from the winmd, the DirectStorage NuGet DLLs, `gdeflate`,
`zstd`), and a DX12 window or headless D3D12 device on a PC with known GPU instability (prefer a headless probe, no window).

Prototype (`crates/iron_pack/examples/ds_probe.rs` or a scratch crate, not shipped): a corpus of **2 GB of photographic BC1/BC3/BC7
textures with mips and 500 MB of meshes** (not the flat renders of the asset-packs note), packed four ways: LZ4, zstd -9 (also with GACL
shuffle if available), GDeflate (DirectStorage's `IDStorageCompressionCodec`, best level), stored. Load everything into GPU buffers
with a **cold cache** (`FILE_FLAG_NO_BUFFERING` reads, or after a reboot) on this machine's NVMe:

1. `iron_pack` mmap/pread + LZ4 on 8 threads + `queue.write_buffer` (today's path);
2. DirectStorage 1.3 with stored and LZ4 chunks (custom queue), `BUFFER` destination imported into wgpu (`create_buffer_from_hal`);
3. DirectStorage 1.3 GDeflate, GPU decompression;
4. DirectStorage 1.4 zstd, GPU and CPU (only if 1.4 is GA by then, else noted as preview).

Record: wall time to last byte usable by wgpu, total CPU time, peak RAM and VRAM, pack size, and `GetCompressionSupport` per run.

Acceptance (the decision rule): add the GPU codec mode **only if** (3) or (4) is **at least 1.5x faster in wall time than (1), or
needs at most half of (1)'s CPU time at no more than 1.1x its wall time**, with its pack **no larger than 0.8x the LZ4 pack**, and the
wgpu interop passes a correctness test (bytes in the wgpu buffer equal the raw entry; a texture copied from it equals a direct upload,
`shared_gpu()` golden style). Prefer zstd over GDeflate when both pass and DirectStorage 1.4 is GA (one codec for every platform's CPU
path); otherwise GDeflate. If neither passes, keep LZ4 and use DirectStorage (2) only for its I/O path, or not at all if (2) is not at
least 1.2x faster than (1). Write the numbers into this note and the decision into `iron-asset-packs.md` section 10.

## 7. Not verified

- The exact minimum Windows 10 version for DirectStorage 1.3/1.4 (only the 2021 "1909" statement exists) and any feature-level
  requirement beyond "D3D12 + SM 6.0".
- The numeric value of `DSTORAGE_COMPRESSION_FORMAT_ZSTD`, the zstd features the GPU shader supports (window size, dictionaries,
  frame/block limits): not documented in the pages read.
- Whether `github.com/microsoft/DirectStorage/GDeflate` is MIT (repo licence) or Apache-2.0 (announced in 2022); GACL's licence and
  repository.
- Which file names are on the DirectStorage distributables list (the licence names the list, the list was not read).
- That wgpu 30's DX12 state tracking accepts a buffer or texture written by DirectStorage in `COMMON` without extra barriers, and that
  calling `ID3D12CommandQueue::Wait` on wgpu's raw queue is safe with wgpu's own fence use (prototype item).
- `windows` 0.62 having no DirectStorage module (the docs search page is script-driven; inferred from the SDK metadata).
- Xbox Series hardware formats and rates (zlib, BCPack, 4.8 GB/s), Oodle's standalone licence terms (Epic owns RAD since 2021; Oodle is
  free inside Unreal), Unity player-build compression options (LZ4/LZ4HC), Godot `.pck` compression: from memory or press.
- `VK_EXT_memory_decompression` driver support outside NVIDIA, and `ash` support for it.
- Oodle and GPU numbers are vendor or press figures, not controlled measurements.

## 8. Sources

- DirectStorage NuGet: <https://www.nuget.org/packages/Microsoft.Direct3D.DirectStorage> (versions, release notes, 1.4.0-preview2 page,
  1.3.0 licence)
- DirectStorage 1.0 blog (2022-03-14): <https://devblogs.microsoft.com/directx/directstorage-api-available-on-pc/>
- DirectStorage 1.1 blog: <https://devblogs.microsoft.com/directx/directstorage-1-1-now-available/>
- DirectStorage 1.2 blog (2023-04-18): <https://devblogs.microsoft.com/directx/directstorage-1-2-available-now/>
- DirectStorage 1.4 preview blog (2026-03-11): <https://devblogs.microsoft.com/directx/directstorage-1-4-release-adds-support-for-zstandard/>
- Developer Guidance: <https://github.com/microsoft/DirectStorage/blob/main/Docs/DeveloperGuidance.md>
- GDeflate README and bitstream: <https://github.com/microsoft/DirectStorage/blob/main/GDeflate/README.md>,
  <https://github.com/microsoft/DirectStorage/blob/main/GDeflate/GDeflate/README.md>
- zstd GPU shader: <https://github.com/microsoft/DirectStorage/tree/development/zstd>
- `DSTORAGE_COMPRESSION_FORMAT`: <https://learn.microsoft.com/en-us/windows/win32/dstorage/dstorage/ne-dstorage-dstorage_compression_format>
- `IDStorageCustomDecompressionQueue`: <https://learn.microsoft.com/en-us/windows/win32/dstorage/dstorage/nn-dstorage-idstoragecustomdecompressionqueue>
- BypassIO: <https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/bypassio>
- NVIDIA GDeflate blog: <https://developer.nvidia.com/blog/accelerating-load-times-for-directx-games-and-apps-with-gdeflate-for-directstorage>
- Intel DirectStorage 1.1 numbers: <https://www.intel.com/content/www/us/en/developer/articles/news/directstorage-on-intel-gpus.html>
- IgorsLab benchmark: <https://www.igorslab.de/en/intel-arc-a770-exceeds-amd-and-nvidia-in-directstorage-1-1-performance-benchmark/>
- Vulkan `VK_EXT_memory_decompression`: <https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_memory_decompression.html>,
  <https://www.phoronix.com/news/Vulkan-1.4.330-Released>
- wgpu-hal DX12 device (`texture_from_raw`, `buffer_from_raw`, `raw_device`, `raw_queue`):
  <https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-hal/src/dx12/device.rs>
- Crates: <https://crates.io/crates/direct-storage> (<https://github.com/Tsukisoft/direct-storage-rs>), <https://crates.io/crates/gdeflate>,
  <https://github.com/PSeitz/lz4_flex>, <https://github.com/KillingSpark/zstd-rs>
- Codec benchmarks: <https://github.com/inikep/lzbench>, <https://github.com/facebook/zstd>, <https://github.com/lz4/lz4>,
  <https://github.com/ebiggers/libdeflate>
- Apple: <https://developer.apple.com/documentation/metal/mtliocompressionmethod>, <https://developer.apple.com/documentation/metal/resource-loading>,
  WWDC22 "Load resources faster with Metal 3": <https://developer.apple.com/videos/play/wwdc2022/10104/>
- Unreal Oodle: <https://dev.epicgames.com/documentation/en-us/unreal-engine/oodle-data?application_version=5.0>; RAD numbers:
  <https://www.radgametools.com/oodlecompressors.htm>
- Unity: <https://docs.unity3d.com/Manual/assetbundles-compression-format.html>
- Godot: <https://docs.godotengine.org/en/stable/classes/class_fileaccess.html>
