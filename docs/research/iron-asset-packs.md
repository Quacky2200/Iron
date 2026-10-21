# Asset packs: scene data packs for release builds (design)

Status: **design, 2026-10-21.** The container crate `crates/iron_pack` (format, LZ4 codec, writer, reader, the layered asset file
system) is built and tested (`M9-API-1100..1112`); nothing in `iron_app` or `iron-build` uses it yet. The executable step list is
`docs/research/iron-asset-packs-plan.md`. Decisions for the user are in section 10.

The user's request: games will have gigabytes of assets, so the packaged executable holds only the game code (`game.irbundle` or the
AOT object), `game.css`, the icon and the metadata. Every asset goes into **packs**: one per scene with what only that scene uses, a
shared pack for what several scenes use, a global pack for what code and UI can load at any time. No loose `png`/`glb`/`wav` files ship.

## 1. What others do, and what Iron takes

| Engine | Container | Split | Lookup | Iron takes |
|---|---|---|---|---|
| Unity (built-in) | `level<N>` + `sharedassets<N>.assets` serialized files, `resources.assets`, `.resS` streamed blobs | per scene; shared by first scene that needs it; `Resources/` folders always in | file id + path id | per-scene packs, a global pack, `resources/` folder semantics, streamed blobs as uncompressed aligned entries |
| Unity Addressables / AssetBundles | LZ4 or LZMA bundles, a catalog | by group, dependencies between bundles | address (string) or GUID | catalog = `data/index.json`, GUID and path addressing, LZ4 chunk compression |
| Unreal | `.pak` (or IoStore `.utoc`/`.ucas`), chunked, per-chunk compression, mount order and patch paks | chunk ids from asset manager rules | path hash index | per-chunk compression, a path-hash index, mount order with patch packs overriding |
| Godot | `.pck` (one file, can be appended to the exe), patches as later `.pck` | none by default | path | packs can sit inside a larger file (APK, exe) |

## 2. The pack container (`.irpk`, version 1)

Implemented in `crates/iron_pack/src/format.rs`. Little-endian everywhere (read with `from_le_bytes`, so big-endian hosts read the same
files). All offsets are `u64` from the **start of the pack** (a pack may live inside an APK or another file), so packs over 4 GiB work.

```text
+------------------------------+ 0
| Header, 64 bytes             |
+------------------------------+ 64
| entry data                   |  each entry's chunks back to back;
|   entry A: chunk 0 | 1 | 2   |  an entry of >= 4 KiB starts on a 4 KiB boundary (1 << align_log2),
|   entry B: chunk 0           |  a smaller one on 16 bytes; padding is zeros
|   ...                        |
+------------------------------+ toc_offset (16-aligned)
| TOC                          |
|   TocHeader       16 bytes   |  entry_count u32 | chunk_count u32 | strings_size u32 | reserved u32
|   Entry x E       64 bytes   |  sorted by GUID (binary search)
|   PathSlot x E    16 bytes   |  path_hash u64 | entry index u32 | cooked u32; sorted by (hash, cooked, path)
|   Chunk x C       24 bytes   |
|   strings                    |  UTF-8 paths and meta texts, each distinct text once
+------------------------------+ file_size = toc_offset + toc_size
```

**Header** (64 bytes):

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 4 | magic | `IRPK` |
| 4 | 2 | version | 1; a reader refuses other versions |
| 6 | 2 | flags | 0 (bit 0 reserved: compressed TOC; bit 1 reserved: encrypted chunks; neither built) |
| 8 | 4 | header_size | 64 |
| 12 | 1 | align_log2 | 12 (4 KiB) |
| 13 | 1 | chunk_log2 | 18 (256 KiB raw per chunk); 10..=30 accepted |
| 14 | 2 | reserved | 0 |
| 16 | 8 | toc_offset | |
| 24 | 8 | toc_size | |
| 32 | 8 | file_size | the pack's length (detects truncation) |
| 40 | 16 | pack_id | content hash of the TOC bytes: identifies the build of this pack |
| 56 | 4 | toc_crc | CRC-32 of the TOC |
| 60 | 4 | header_crc | CRC-32 of bytes 0..60 |

**Entry** (64 bytes): `guid [16]`, `content_hash [16]` (SipHash-1-3-128 of the raw bytes), `size u64` (raw), `first_chunk u32`,
`chunk_count u32`, `path_offset u32`, `path_len u16`, `kind u8`, `flags u8`, `meta_offset u32`, `meta_len u32`.
Flags: `1` tombstone (a patch deleting this GUID/path from packs mounted earlier), `2` contiguous (every chunk stored uncompressed, back
to back: the entry is one byte range), `4` cooked (the payload is the cooked form; a path may have one source and one cooked entry).
Kinds: 0 data, 1 texture, 2 model, 3 audio, 4 font, 5 scene, 6 prefab, 7 svg, 8 css, 9 material, 10 animation.

**Chunk** (24 bytes): `offset u64`, `stored_size u32`, `raw_size u32`, `codec u8`, `reserved [3]`, `crc u32` (CRC-32 of the stored
bytes). Every chunk of an entry except the last holds exactly `1 << chunk_log2` raw bytes, so byte `n` of an entry is in chunk
`first_chunk + (n >> chunk_log2)`: a range read touches only its chunks. Codecs: 0 none, 1 LZ4 block, 2 raw deflate, 3 reserved for
zstd. A chunk is stored compressed only when that saves at least 1/16 of it (mp3, ogg, png stay raw and contiguous).

**Properties.**

- *Random access without reading the file*: open reads the 64-byte header and the TOC (mapped: no read at all), checks the TOC CRC and
  that every entry's chunk and string ranges and every chunk's byte range lie inside their tables (no later bounds panics). Lookups
  are binary searches over the stored TOC (no maps built): by GUID over the entry array, by path over the path slots (hash, then the
  few equal hashes compared by string).
- *Memory-mappable*: `Pack::open` maps the file (`memmap2`, already in the tree); uncompressed entries come back as `Bytes` that are
  slices of the map (no copy; `Bytes: AsRef<[u8]> + Send + Sync + Clone`, so `std::io::Cursor<Bytes>` feeds kira's streaming decoder
  straight from the map). `OpenMode::Read` uses positional reads instead (`seek_read` / `read_exact_at`), for 32-bit targets and
  platforms where mapping is unwelcome. `Pack::from_bytes` takes a `Vec<u8>` (tests, downloaded packs).
- *Integrity*: CRC-32 per chunk (checked on every compressed chunk; on stored chunks while `set_verify(true)`, the default), CRC-32
  of the TOC and header, 128-bit content hash per entry (`Pack::verify()` and `iron-build pack --verify` check them all).
- *Dedupe*: entries with the same bytes (content hash + size + compress flag) share chunks inside a pack.
- *Deterministic*: the bytes depend only on the entries added and their order (no time, user, machine). The TOC is sorted, strings are
  interned in TOC order, chunk boundaries are fixed. `M9-API-1105` builds twice and compares.
- *Versioning*: `version` is bumped on any layout change; readers refuse unknown versions (`Error::Format`), so a stale engine fails
  loudly instead of misreading. Payload formats (`IRTX`, `IRMD`) carry their own magic and version.
- *Streaming-friendly*: large entries are page-aligned (4 KiB) so mapped reads and future unbuffered/DirectStorage-style reads line up;
  audio and future texture mip streaming use range reads (see 4.3).

### 2.1 Payloads by kind

| Kind | Payload in a pack | Form | Compress |
|---|---|---|---|
| texture | `IRTX` v1 from `cook.rs` (BC1 / BC3 / RGBA8, every mip, largest first); v2 (section 4.3) reverses the mip order and adds a mip offset table | cooked | yes |
| texture used by UI/CSS | the source PNG/JPEG/SVG | source | no (already compressed) |
| model | `IRMD` v1 from `cook.rs` (description JSON, f32 vertex streams, u32 indices, skins, its textures as `IRTX`) | cooked | yes |
| audio | the source file (wav, ogg, mp3, flac) | source | wav yes, others no |
| font | the source file (ttf, otf, woff2) | source | yes |
| scene, prefab | cooked `iron.scene` v1 JSON (section 5) | cooked | yes |
| css url() assets over 32 KiB | the file `css.rs` copied to `dist/` (`name-<hash>.ext`) | source | per kind |
| data (json, txt, bin) | the file | source | yes |

GUIDs come from the `.meta` files (`guid`, 32 hex digits, parsed by `Guid::parse`); a file without one gets `Guid::from_path` (a hash of
its normalized path: stable across builds). The entry's `meta` text holds the `importer` object of the `.meta` file (JSON), so the
release build's `Assets.importer(path)` and `Assets.pathToGuid` answer from the packs exactly as the dev loop answers from `.meta` files.

## 3. The packs of a game

```text
<release>/
  MyGame.exe                code (irbundle or AOT), game.css, icon, package.json metadata
  data/
    index.json              the manifest: packs in mount order, their ids, scene -> pack
    global.irpk             assets named by code literals, CSS, fonts, resources/ (always mounted)
    shared.irpk             assets used by two or more scenes
    scene-level1.irpk       scenes/level1.scene.json + what only it uses
    scene-level2.irpk
    patch-0001.irpk         (later) changed entries and tombstones
  dlc/<id>/index.json       (later) more packs, mounted after the base
  mods/<name>/...           (later, if the game allows mods) loose files or packs, mounted last
```

**All packs are mounted at start** (measured: opening a pack is ~0.1 ms plus ~37 ns per entry with every range checked; a mapped
pack costs address space, not memory). "Loading a scene pack" therefore means *prefetching* its entries and creating the scene's
assets, not opening files. The split gives locality (a scene's data is contiguous on disk), download and patch granularity (mobile
asset delivery, DLC), and a natural unit for prefetch and unload.

### 3.1 `data/index.json`

```json
{
  "format": "iron.packs",
  "version": 1,
  "identifier": "dev.iron.scenes",
  "build": "6f1c...32 hex: content hash of the pack ids in order",
  "codec": "lz4",
  "packs": [
    { "name": "global", "file": "global.irpk", "id": "32 hex", "size": 81234, "entries": 12, "role": "global", "plan": "32 hex" },
    { "name": "shared", "file": "shared.irpk", "id": "32 hex", "size": 4096, "entries": 2, "role": "shared", "plan": "32 hex" },
    { "name": "scene-level1", "file": "scene-level1.irpk", "id": "32 hex", "size": 30123, "entries": 5, "role": "scene",
      "scene": "scenes/level1.scene.json", "needs": ["global", "shared"], "plan": "32 hex" }
  ],
  "scenes": { "scenes/level1.scene.json": "scene-level1" }
}
```

Mount order is the array order. At start the app checks each pack's header `pack_id` against `id` (a mixed or half-copied `data/`
folder is an error naming the pack, not wrong assets). `plan` is the hash of the pack's planned entry list (incremental builds, 6.3).
`needs` is informative for prefetch and delivery (scene packs need the global and shared packs). Pack names are file stems: lowercase
`[a-z0-9-]`, the scene's file stem slugged (`scenes/Level 1.scene.json` -> `scene-level-1`; a collision gets `-2`).

### 3.2 The layered file system

`iron_pack::Vfs` (built): a stack of `AssetSource`s; lookups go from the last mounted down and stop at the first answer, a tombstone
answers "deleted". Sources: `DirSource` (a game folder; `.meta` GUIDs scanned at creation, `rescan()` after changes), `PackSource` (a
mounted pack). The same calls serve both:

```rust
pub trait AssetSource: Send + Sync {
    fn name(&self) -> &str;
    fn find(&self, path: &str, cooked: bool) -> Option<Found>;   // Found::Asset(AssetInfo) | Found::Deleted
    fn find_guid(&self, guid: &Guid) -> Option<Found>;
    fn read(&self, info: &AssetInfo) -> std::io::Result<Bytes>;
    fn read_range(&self, info: &AssetInfo, offset: u64, len: u64) -> std::io::Result<Bytes>;
    fn paths(&self) -> Vec<String>;
    fn root(&self) -> Option<&Path> { None }                    // the folder of a loose source
}

impl Vfs {
    pub fn dir(root: &Path) -> Vfs;                              // the dev loop
    pub fn mount(&mut self, s: Arc<dyn AssetSource>);  pub fn unmount(&mut self, name: &str) -> bool;
    pub fn read(&self, path: &str) -> io::Result<Bytes>;         // "data:..." decoded, "guid:<hex>" by GUID, else a game path
    pub fn read_to_string(&self, path: &str) -> io::Result<String>;
    pub fn read_cooked(&self, path: &str) -> Option<io::Result<(AssetInfo, Bytes)>>;
    pub fn read_range(&self, path: &str, offset: u64, len: u64) -> io::Result<Bytes>;
    pub fn info(&self, path: &str) -> Option<AssetInfo>;  pub fn exists(&self, path: &str) -> bool;
    pub fn guid_to_path(&self, guid: &Guid) -> Option<String>;  pub fn paths(&self) -> Vec<String>;
    pub fn root(&self) -> Option<&Path>;
}
```

Path rules are `iron_core::resolve_asset`'s (relative, no `..`, no schemes or drives), normalized (`./a\b` -> `a/b`), case-sensitive.
Dev loop: `Vfs::dir(game)`; the app keeps its `.iron/cooked` freshness check for loose files (`DirSource` never claims a cooked form).
Release: packs from `data/index.json`, patches, DLC, mods on top. Testing a release layout in the dev loop: `iron <game> --packs`
mounts `<game>/dist/data` instead of the folder.

## 4. Runtime seam

### 4.1 Where assets are read today (all through `iron_core::resolve_asset` + `std::fs`)

| # | Site | What | Change |
|---|---|---|---|
| 1 | `crates/iron_app/src/world_view.rs:656-682` (`texture_op`) | textures: cooked check (`cook::cooked_texture`), else `resolve_asset` + `fs::read` + `image` | `vfs.read_cooked(src)` first (packs), then the dev cooked check, then `vfs.read(src)` |
| 2 | `world_view.rs:482` (`load_model`) -> `cook::cooked_model`, `models::load` (`models.rs:111-115`), `models::read_uri` (`models.rs:73-87`, glTF external buffers/images) | models | `vfs.read_cooked`, then `models::load(&Vfs, src, ids)` with `read_uri` over the Vfs |
| 3 | `crates/iron_app/src/main.rs:2405-2427` (`load_sound`) | `audio.load` | `vfs.read(src)`; streams pass `Bytes` (no copy, 4.4) |
| 4 | `main.rs:2379-2403` (`load_font`), `main.rs:2526-2536` (`add_stylesheet` `@import` and `@font-face`) | fonts, runtime stylesheets | `vfs.read` / `vfs.read_to_string` |
| 5 | `main.rs:1157-1172` (`asset.readText` op) | text files for QuickJS games | `vfs.read_to_string` |
| 6 | `crates/iron_app/src/native.rs:252-275` (`load_requested_scenes`) | scene files (`SceneManager.loadSceneAsync`) | `vfs.read_to_string` |
| 7 | `crates/iron_app/src/cursor.rs:12-18` | cursor images | `vfs.read` |
| 8 | `crates/iron_render/src/ui/images.rs:166-173` (SVG), `:220-240` (UI images), root set by `ui/mod.rs:643 set_asset_root` | CSS `url()` / `<image>` | `set_assets(Vfs)`; `vfs.read(url)` |
| 9 | `crates/iron_app/src/asset_db.rs:12` (`scan`, `.meta` files) -> `__iron_info.assets` / `iron_engine_api::assets::set_assets` | GUIDs, importer settings | from `vfs` entries (`Vfs::entries()`, step 1) |
| 10 | `crates/iron_app/src/cook.rs:586-606` (`fresh`, `cooked_texture`, `cooked_model`) | dev cooked files | unchanged, given `vfs.root()` (dev only) |
| 11 | `crates/iron_app/src/reload.rs` (`AssetWatcher`) | dev hot reload | unchanged; off when no loose source is mounted |
| 12 | `crates/iron_app/src/info.rs:46,81` (`package.json`, icon), `main.rs:180,552,2442,2513` (bundle, `game.css`) | exe metadata | not assets: embedded by `iron-build package`; unchanged |
| 13 | `crates/iron_app/src/project.rs`, `editor.rs`, `file_drop.rs` | the editor's project files | not assets at run time: unchanged (the editor works on loose folders) |

`crates/iron_render_api` and `crates/iron_audio` read no files (they queue load requests: `RenderCommand::Texture`/`Model` with a
`src`, `AudioCommand::Load` with a path; the host reads); `crates/iron_scenes` reads none (`request_load` / `finish_load(text)`).
So the seam is the app plus one function in `iron_render`: about 15 call sites, each `resolve_asset(root, src)` + `fs::read(path)`
becoming `vfs.read(src)` (the `data:` branch disappears into `Vfs::read`).

### 4.2 Cooked forms from packs

A pack's texture entry is the cooked `IRTX` (flag cooked). The importer settings at cook time come from the `.meta`; a script may ask
for different ones (`Texture.load(src, { linear: true, mipmaps: false })`). Rules, so a pack never needs the source:

- colour space differs: use the same blocks, flip `srgb` (BC blocks are colour-space agnostic; only the view format changes);
- mipmaps asked off, cooked has mips: upload mip 0 only;
- mipmaps asked on, cooked has one level: `uncook_texture` then make mips on upload (what the dev fallback does today);
- filter, wrap, anisotropy: sampler settings, already taken from the script (`world_view.rs:662-667`).

Models: `IRMD` is used as is (`model_from_bytes`). UI images: the source form; when only a cooked form exists, `uncook_texture` mip 0
(BC loses quality: the build stores the source form for anything CSS or UI references, 6.2).

### 4.3 Streaming

- **Audio.** `audio.load(path, stream = true)`: today the whole file is read into a `Vec` and `StreamingSoundData::from_cursor(Cursor<Vec>)`
  decodes from it. With packs, `Cursor<iron_pack::Bytes>` over the mapped, stored (uncompressed) entry: no copy, the OS pages it in
  as the decoder reads (kira's `from_cursor` takes `T: AsRef<[u8]> + Send + Sync + 'static`). Positional-read mode: a `MediaSource`
  over `read_range` (symphonia is in the tree through kira).
- **Textures** (later, `IRTX` v2): mips stored smallest first with a table of `(offset u64, size u64)` per mip; the loader reads the
  tail (smallest mips, a few KiB) at once with `read_range`, uploads a low-resolution texture, then the rest by priority and a memory
  budget. v1 files keep loading (version check).
- **Models**: whole entries; large ones are chunked, so a loader thread reads them without blocking the frame.

### 4.4 Loading API (Rust side, later steps)

`iron_app::asset_jobs` (new): a small worker pool (2 threads by default, `IRON_ASSET_THREADS`) taking requests `(key, priority)` from a
priority queue (`High` = scene loading, `Normal` = script loads, `Low` = prefetch), doing `vfs.read` + decode (image decode, `IRTX`/`IRMD`
parse, static audio decode) off the main thread; finished results come back through a channel and the main thread uploads them with
a per-frame budget (default 8 ms of uploads or 64 MiB). A request returns a handle id; the existing per-asset `isLoaded` / `failed`
flags of `Texture`/`Model` stay the script-facing truth. **Lifetime**: an `AssetCache` keyed by `(path, form, settings)` with
refcounts; assets created by a scene's `assets` table are owned by that scene and released on `unloadScene`;
`Resources.unloadUnusedAssets()` frees refcount-0 assets; a texture memory budget (`QualitySettings.textureBudgetMB`) drops high mips
of least-recently-drawn streamed textures once v2 exists.

### 4.5 Script API

Existing calls keep working unchanged: they already name assets by path or `guid:<hex>` (`Assets.resolve`), and the host reads through
the Vfs: `Texture.load(path)`, `Model.load(path)`, `audio.load(path, stream)`, `loadFont`, `SceneManager.loadScene` /
`loadSceneAsync(path)`, `Cursor.setCursor(path)`, CSS `url()`, `Assets.guidToPath` / `pathToGuid` / `importer` / `count` / `pathAt`.
New (namespace `Assets`, `iron_engine_api::assets`, step 10):

```ts
Assets.exists(path: string): boolean
Assets.packOf(path: string): string                 // "" in the dev loop
Assets.preload(path: string, priority?: "high" | "normal" | "low"): number   // request id; reads and decodes ahead
Assets.preloadScene(scenePath: string): number      // every entry of the scene's pack
Assets.isDone(request: number): boolean
Assets.progress(request: number): number            // 0..1, bytes read / bytes planned
Assets.release(path: string): void                  // drop a refcount taken by preload
Resources.unloadUnusedAssets(): void
```

`Assets.load<T>(guid)` is spelled `T.load("guid:<hex>")` (`Texture.load`, `Model.load`, `audio.load`) since handle classes are not
generic over the boundary; the compiler may lower `Assets.load<Texture>(g)` to it later (one rule in `lower/`).

## 5. Scenes and prefabs

`iron.scene` v1 JSON stays the authoring and the runtime format (`crates/iron_scenes/src/data.rs` reads it with `serde_json`; a 12 KiB
scene parses in tens of microseconds, so a binary form buys nothing yet). The **cooked scene** in a pack is the same JSON:

1. minified (no indentation: the three example scenes shrink 2.7 to 3.1 times; LZ4 gives 5x on the pretty-printed files);
2. every asset reference carries its GUID: `assets` table entries (`{"type":"Texture","src":...}`) get `"guid"` when the file has a
   `.meta`; `{"$type":"Sound","src":...}` and the `"model"` extra (`iron_kinds::ObjectExtra`) likewise. The loader already prefers the
   GUID (`resources.rs:619-631`: `guid_to_path` then `src`), so renamed files keep loading;
3. nothing else changes (component kinds, `$ref`s, render settings).

A scene file is an entry of kind `scene` (cooked) in its own scene pack; `SceneManager.loadSceneAsync(path)` reads it through the Vfs
(site 6). Prefab files (`*.prefab.json`) are kind `prefab`, placed like assets (a prefab used by one scene lives in that scene's pack).

## 6. The build: `iron-build pack <game>`

### 6.1 Reference graph

Roots and edges, all deterministic (sorted sets):

| Source | Found how | Goes to |
|---|---|---|
| code string literals naming an existing asset (`referenced_assets` in `tools/iron_build/src/lib.rs:261`) | existing scan of the bundled source | global; `*.scene.json` / `*.prefab.json` literals are scene / prefab roots instead |
| `package.json` `iron.startScene`, `iron.scenes` (new optional list) | config | scene roots |
| scene / prefab JSON | walk every value: `assets` table entries (`src`, `guid`), `$type` objects with `src`/`clip`, `"model"` extras, and **any string** that names an existing asset file (the same test as code literals) | that scene; a string naming another `*.scene.json` is a scene root, not an asset |
| `game.css` `url()` copies (files over 32 KiB; smaller ones are inlined `data:` URLs already) and `@font-face` sources | `css.rs` output (`copied`) | global, source form |
| `resources/**` under the game folder (Unity's `Resources`) and `iron.pack.include` globs | config | global |
| glTF external files (`.bin`, images) | inputs of the cooked model | inside the model's payload, not separate entries |

Assets nothing references are left out, listed by `--report` (`unreferenced: N files, M bytes`).

### 6.2 Split and forms

- referenced by code, CSS, fonts or `resources/` -> **global**;
- else referenced by exactly one scene -> **that scene's pack**; by two or more -> **shared**;
- textures referenced from scenes, models or materials -> cooked; from CSS/UI -> source; from code literals -> both (a code literal may
  be `Texture.load` or a UI `<image src>`; the build cannot tell), unless the `.meta` importer says `"pack": "cooked" | "source" | "both"`;
- models -> cooked (`IRMD`); audio, fonts, svg, data -> source; scenes and prefabs -> cooked JSON (section 5);
- compression per entry: off for already-compressed kinds (png, jpg, mp3, ogg, opus, flac, woff2), on otherwise (the 1/16 rule still
  stores incompressible chunks raw);
- entry order inside a pack: scene file first, then by kind (textures, models, audio, fonts, other), then path: deterministic and
  roughly the order a scene load asks for them.

### 6.3 Incremental and deterministic

- Cooked payloads are cached in `<game>/.iron/cache/packs/<key>.bin` with `key = content_hash(source bytes, external glTF files' bytes,
  importer JSON, cooker version, form)`; a changed `.meta` or a cooker version bump recooks.
- Each pack's planned entry list (GUID, path, kind, flags, meta, payload key) hashes to `plan`; when `data/index.json` already lists the
  same `plan` and the pack file's header id matches, the pack is not rewritten (its mtime stays: the platform uploader sees no change).
- Packs are written with `PackWriter::write_file` (temporary file + rename) into `dist/data/`, `index.json` last (atomically), so a
  running app never sees half a build. Cooking runs on `std::thread::scope` workers (no new dependency); results are collected and
  added in plan order, so parallelism never changes the bytes.

### 6.4 Commands

```text
iron-build pack <game> [--codec lz4|deflate|none] [--out <dir>] [--verify] [--report] [--no-cook]
iron-build build <game> --release      # build + pack (dist/data/), no loose assets copied into dist/
iron-build package <game> --release    # (package agent's command) exe + data/ next to it
```

- `pack` needs cooked files: phase A runs `iron <game> --cook` (the engine's cooker, `find_iron`) and reads `.iron/cooked/`; phase B
  moves the cooker into `crates/iron_cook` so `iron-build` links it (plan step 15).
- `build` without `--release` keeps today's loose `dist/` copy (dev loop, hot reload).
- `package --release`: the executable holds the code, `game.css`, the icon and the metadata; `data/` holds every asset. The app looks for
  `data/index.json` next to the executable when it carries an embedded bundle.

### 6.5 Mobile

- **Android**: packs go into the APK/AAB under `assets/data/` **stored uncompressed** (`androidResources { noCompress += ["irpk", "json"] }`),
  opened with `AAssetManager_open` + `AAsset_openFileDescriptor64` (fd, offset, length) -> `Pack::open_at(file, offset, len, Map, ..)`
  (built and tested, `M9-API-1108`). Over 200 MB: Play Asset Delivery install-time packs (same API); scene packs as fast-follow or
  on-demand packs later (the manifest's `role`/`needs` say which). OBB is not needed.
- **iOS**: packs in the app bundle (`<bundle>/data/`), mapped; On-Demand Resources tags per scene pack later. Packs hold no code, so
  App Store rules about downloaded code do not apply; the code is AOT-compiled (no JIT on iOS) and lives in the executable.
- 32-bit targets: map only packs under 1 GiB, otherwise `OpenMode::Read`.

## 7. Measurements (2026-10-21, this machine: Windows 11, 24 logical cores; release build of `crates/iron_pack` through a probe)

Corpus: the examples' assets (`examples/*/assets`, scenes, the local Charm fonts), their cooked forms (`iron_app --cook` on copies), and
as a stand-in for real game textures (the examples' textures are tiny pixel art) the 252 renders in `tests/output` + `tests/golden`,
cooked the same way. Identical files across examples were counted once (20.9 MB of duplicates skipped). Ratios are raw bytes over
stored chunk bytes; read speeds include the CRC check and a copy into a `Vec`, warm OS cache.

| Category | Files | Raw bytes | LZ4 ratio | LZ4 write MB/s | LZ4 read MB/s | deflate-6 ratio | deflate-6 write MB/s | deflate-6 read MB/s | deflate-1 ratio |
|---|---|---|---|---|---|---|---|---|---|
| cooked model (`IRMD`) | 6 | 2,207,827 | 1.60 | 445 | 1181 | 2.26 | 31 | 439 | 2.06 |
| cooked texture, renders (`IRTX`) | 190 | 105,676,164 | 3.75 | 537 | 884 | 6.13 | 44 | 506 | 4.69 |
| cooked texture, examples | 6 | 12,236 | 3.05 | 300 | 1423 | 4.28 | 60 | 385 | 3.86 |
| glb (source) | 6 | 1,534,504 | 1.68 | 434 | 1185 | 2.56 | 34 | 534 | 2.28 |
| png | 201 | 25,134,555 | 1.00 | | (stored) | 1.00 | | | 1.00 |
| mp3 | 1 | 2,888,015 | 1.00 | | (stored) | 1.00 | | | 1.00 |
| wav | 1 | 6,218 | 1.00 | | | 1.11 | | | 1.10 |
| ttf | 2 | 270,068 | 1.45 | 302 | 887 | 1.90 | 34 | 361 | 1.67 |
| scene JSON (pretty) | 3 | 27,326 | 5.07 | 649 | 1938 | 9.97 | 91 | 763 | 6.45 |

Caveat: the renders are flat-shaded images with large uniform areas (and 40 of the 251 cooked as RGBA8 because their sizes are not
multiples of 4: those 40 hold 48 MB, 39% of the cooked bytes, at 5.33 bytes/pixel against BC1's 0.67), so their ratios are far higher
than photographic BC1/BC3 data will give (expect about 1.1 to 1.4 with LZ4, 1.2 to 1.6 with deflate). Models and fonts are typical.

**Cooked versus source** (bytes):

| Set | Source | Cooked | Cooked / source | Cooked + LZ4 |
|---|---|---|---|---|
| examples' textures (7 PNGs, pixel art) | 3,415 | 13,000 | 3.8 | ~4,300 |
| examples' models (6 unique glb) | 1,534,504 | 2,207,827 | 1.44 | 1,380,000 |
| renders (251 PNGs) | 27,533,593 | 121,951,472 | 4.4 | ~32,500,000 |

BC1 is a fixed 0.5 byte per pixel (plus a third for mips); PNG of flat or pixel-art images is far smaller. For photographic textures
(PNG about 2 to 3 bytes/pixel) BC1 with mips is smaller than the PNG. The point of cooking is load time (no decode, no mip generation,
direct upload), not disk size.

**Throughput** (a 512 MiB pack of 512 stored 1 MiB entries, warm cache; a cold read is bound by the disk instead):

| Mode | Open | Read + copy all, first pass | Read + copy all, best of 5 | 20k random 64 KiB reads + copy |
|---|---|---|---|---|
| mmap, verify off | 84 us | 2151 MB/s | 5627 MB/s | 22,004 MB/s (3.0 us each) |
| mmap, verify on | 83 us | 1704 MB/s | 4319 MB/s | 3,455 MB/s (19.0 us each: CRC of the whole 256 KiB chunk) |
| pread, verify off | 108 us | 1844 MB/s | 2040 MB/s | 921 MB/s (71 us each) |
| pread, verify on | 54 us | 1841 MB/s | 1411 MB/s | 550 MB/s (119 us each) |

Mapped and zero-copy, a stored entry costs nothing to "read" until its pages are touched (touching one byte per page of 512 MiB that is
already mapped takes 0.2 ms). Hashes: CRC-32 (`crc32fast`) 23 GB/s, SipHash-1-3-128 5.9 GB/s. Writing the 512 MiB stored pack with
fsync: 310 MB/s.

**Table of contents at 100,000 entries** (paths like `assets/level_042/props/prop_042123.png`, a 42-byte importer text each):
TOC 14.2 MB (142 bytes per entry with meta texts interned; 80 fixed, ~43 path, 24 chunk), write 386 ms, open 3.7 ms mapped (TOC CRC
plus every range checked) or 3.5 ms by positional read, `find_guid` 313 ns, `find_path` 807 ns hit / 966 ns miss (both include
building the `Entry` with its strings), a `Vfs` of ten 10k-entry packs answers `info(path)` in 2.7 us on average (it asks up to ten
packs). A scene that loads 10,000 assets spends about 27 ms in lookups: fine, and a merged index can cut it later (plan step 10).

**Codec choice.** LZ4 reads at 0.9 to 1.9 GB/s with this simple safe decoder (a tuned decoder such as `lz4_flex` reaches 3 to 4 GB/s
on the same format), writes at 300 to 650 MB/s, and gives 1.45 to 1.7 on models, glTF and fonts. Deflate gives 30 to 60 percent
smaller packs but reads 2 to 3 times slower and writes 10 times slower (31 to 91 MB/s; a 10 GB game is minutes per core). zstd would give
deflate's ratio at near-LZ4 read speed, but no zstd crate is in the dependency tree (the C `zstd` crate needs `cc` and a download;
`ruzstd` is pure Rust and decodes only). **Default: LZ4** (the format is standard LZ4 block, so `lz4_flex` can replace the in-crate
codec later without changing packs), deflate selectable (`--codec deflate`) for download-size-bound targets (mobile), zstd codec id 3
reserved for a user decision.

## 8. Risks

- **Mapped files and external truncation**: a pack truncated while mapped faults the process (SIGBUS / an access violation). Packs are
  read-only release files; the dev loop uses loose files. Positional-read mode avoids it where it matters.
- **The 1/16 rule and contiguity**: a large entry with a few compressible chunks becomes non-contiguous (copied on read). Audio is
  marked `compress: false` by kind so streams stay zero-copy.
- **Code literals**: dynamic paths (`"assets/level" + n + ".png"`) are invisible to the scan; `resources/` and `iron.pack.include` cover
  them, and a release run logs `asset not in any pack: <path>` once per path.
- **Two forms per code-literal texture** cost size; the `.meta` `pack` key is the escape hatch.
- **Cooker in the engine binary** (phase A): `iron-build pack` needs a built `iron_app`; phase B (`crates/iron_cook`) removes that.
- **RGBA8 fallback in the cooker** for sizes that are not multiples of 4 inflates packs (measured above); padding to the next multiple
  of 4 with a UV scale, or BC on the padded image, is a cooker follow-up.

## 9. Not in scope

Encryption or obfuscation of packs (they are not a security boundary: any key ships with the game; header flag bit 1 and the codec
byte are reserved should a platform require it); a binary scene format; texture transcoding to ASTC/ETC2 for mobile (cooker work, the
pack format carries any payload); delta patch generation beyond patch packs (Steam and the mobile stores diff files themselves, and
deterministic packs keep those diffs small).

## 10. Decisions for the user

1. **Codec**: LZ4 default with deflate as an option (recommended), or add zstd (needs a new crate: `zstd` (C, `cc`) or `ruzstd`
   (pure Rust, decoder only; the build would still need an encoder)).
2. **Encryption/obfuscation**: out of scope (recommended).
3. **DLC policy**: DLC as extra `index.json` + packs under `dlc/<id>/`, mounted after base and patches, installed-ness decided by the
   platform (Steam's DLC folders, store APIs); can DLC override base assets (recommended yes, as patches do) or only add?
4. **Mods**: allow loose-file and pack mods from `mods/` mounted last, per game (`iron.mods: true` in `package.json`), or never.
5. **Unreferenced assets**: leave them out with a report (recommended), with `resources/` and `iron.pack.include` to force them in.
6. **Code-literal textures**: store both forms (recommended, safe) or cooked only.
7. **Shared packs**: one `shared.irpk` (recommended to start) or one per set of scenes sharing assets (Unity's per-bundle dependencies;
   finer downloads, more files).
8. **Cooker RGBA8 fallback**: pad odd-sized textures to BC (smaller packs, a UV change) or keep RGBA8.
