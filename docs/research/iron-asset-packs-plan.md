# Asset packs: the step list

The design is `docs/research/iron-asset-packs.md` (read sections 2 to 6 first). This list is for the implementer: one commit per step,
in order, each with every check of `CLAUDE.md` green. Test IDs: `M9-API-1100..1199` (Rust crates and the app), `M9-TS-2500..2599`
(`tools/iron_build`). Add every ID to `TESTS.md` in the step that adds the test. After each group (marked **docs**), a Sonnet subagent
updates `docs/` from the diff only (`docs/asset-packs.md` new; `docs/assets-and-models.md`, `docs/iron-build.md`, `docs/README.md`).

General rules:

- `crates/iron_pack` stays free of engine crates (only `iron_core`, `memmap2`, `crc32fast`, `siphasher`, `miniz_oxide`, and from step 1
  `serde_json`: all already in `Cargo.lock`). Run its checks with
  `cargo test --offline -p iron_pack` and `cargo clippy --offline -p iron_pack --all-targets -- -D warnings` while working, the full
  workspace before committing.
- `tools/iron_build` is in the `tools` workspace; it depends on `../../crates/iron_pack` by path, as it does on `iron_core`.
- Never add a crate that is not in a lock file without the user's OK (downloads need it). zstd, `lz4_flex`, `rayon` (tools) are out.
- Temporary directories in tests: `std::env::temp_dir().join(format!("iron_<test>_{}", std::process::id()))`, removed at the end.
- Edit with the Edit/Write tools or a Python script in the scratchpad (bash heredocs mangle apostrophes and backslashes here).

## Done: step 0, the container crate (commit with this plan)

`crates/iron_pack`: `format.rs` (layout, `Guid`, `AssetKind`, `Codec`, header/entry/chunk records), `hash.rs` (`content_hash`,
`path_hash`, `crc32`, `normalize_path`), `lz4.rs` (LZ4 block compress/decompress, safe), `writer.rs` (`PackWriter::new(W, opts)`,
`add(&EntrySpec)`, `finish() -> (W, PackSummary)`, `PackWriter::write_file(path, opts, |w| ..)`), `reader.rs` (`Pack::open`,
`open_with(path, OpenMode)`, `open_at(file, offset, len, mode, origin)`, `from_bytes`, `find_guid`, `find_path(path, cooked)`, `read`,
`read_range`, `verify`, `set_verify`, `Bytes`), `vfs.rs` (`AssetSource`, `DirSource`, `PackSource`, `Vfs`). Tests `M9-API-1100..1112`.

## Group A: the build side (no `iron_app` change)

### Step 1. Manifest, entry listing, merged lookups (`crates/iron_pack`)

- `src/manifest.rs` (new; add `serde_json = "1.0.154"` to `crates/iron_pack/Cargo.toml`, it is in the lock):
  ```rust
  pub struct PackRef { pub name: String, pub file: String, pub id: [u8; 16], pub size: u64, pub entries: usize,
                       pub role: Role, pub scene: Option<String>, pub needs: Vec<String>, pub plan: [u8; 16] }
  pub enum Role { Global, Shared, Scene, Patch, Dlc }
  pub struct Manifest { pub identifier: String, pub build: [u8; 16], pub codec: Codec, pub packs: Vec<PackRef>,
                        pub scenes: BTreeMap<String, String> }
  impl Manifest {
      pub fn load(dir: &Path) -> Result<Manifest>;                 // dir/index.json; format "iron.packs", version 1
      pub fn to_json(&self) -> String;                             // key order as in design 3.1, 2-space indent, trailing \n
      pub fn mount(&self, dir: &Path, vfs: &mut Vfs, mode: OpenMode) -> Result<()>;  // opens in order, checks header id == id
  }
  ```
  A mismatched id is `Error::Format("data/<file>: built by another build (id ..., index says ...)")`. Hex ids are 32 lowercase digits.
- `Vfs::entries(&self) -> Vec<AssetInfo>`: every visible asset (source forms first, then cooked forms of paths without a source),
  sorted by path; used for the release asset database.
- `AssetSource::entries(&self) -> Vec<AssetInfo>` with a default from `paths()` + `find`; `PackSource` overrides it from the TOC.
- Tests: `M9-API-1113` manifest round-trip (`to_json` -> `load` equal, unknown keys ignored, missing `packs` is an error);
  `M9-API-1114` `mount` mounts in order and refuses a pack whose id differs; `M9-API-1115` `Vfs::entries` hides tombstoned assets and
  lists GUID + meta from packs and from `DirSource` alike.
- Acceptance: the three tests pass; `cargo clippy -p iron_pack` clean.

### Step 2. The reference graph (`tools/iron_build/src/pack/graph.rs`)

- `tools/iron_build/Cargo.toml`: `iron_pack = { path = "../../crates/iron_pack" }`.
- `pub mod pack;` in `lib.rs`; `src/pack/mod.rs` re-exports.
  ```rust
  pub struct Graph {
      pub code: BTreeSet<String>,                       // assets named by code literals (not scenes/prefabs)
      pub css: BTreeSet<String>,                        // files css.rs copied (dist-relative names) and @font-face srcs
      pub include: BTreeSet<String>,                    // resources/** and iron.pack.include globs
      pub scenes: BTreeMap<String, BTreeSet<String>>,   // scene path -> assets it uses (prefabs expanded)
      pub prefabs: BTreeMap<String, BTreeSet<String>>,  // prefab path -> assets it uses
  }
  pub fn graph(root: &Path, source: &str, css: &CssOut, config: &GameConfig) -> Result<Graph, String>;
  pub fn scene_refs(root: &Path, json: &serde_json::Value) -> (BTreeSet<String>, BTreeSet<String>); // (assets, scenes and prefabs)
  ```
  Scene roots: `*.scene.json` code literals, `iron.startScene`, `iron.scenes` (array of paths; new, optional), and scene paths found
  inside scenes (transitively). `scene_refs` walks every JSON value; a string counts when `iron_core::resolve_asset(root, s)` is a file
  and its extension is an asset kind (`assets::kind`) or it ends in `.scene.json`/`.prefab.json`; objects with `src` or `clip` strings
  and `guid` strings resolve the GUID through the `.meta` files first (a renamed file is still found). Make `referenced_assets` in
  `lib.rs` `pub(crate)` and reuse it for code literals. `iron.pack.include` is an array of globs (`*` within a path part, `**` across
  parts; write the 20-line matcher, no crate).
- Tests (fixture games written into temp dirs): `M9-TS-2500` code literals go to `code`, a scene literal becomes a scene root;
  `M9-TS-2501` a scene's `assets` table, `$type` Sound `src`, a `"model"` extra and a plain string property are all found, a GUID-only
  reference resolves through `.meta`; `M9-TS-2502` a scene naming another scene adds a root, not an asset; prefabs expand into the
  scene that uses them; `M9-TS-2503` `resources/**` and `iron.pack.include` globs are included; CSS copies and font faces land in `css`.
- Acceptance: `graph` over `examples/scenes` (after `iron-build build`) lists `scenes/level1.scene.json`, `level2`, `bonus` with
  their assets; a test asserts the exact sets for that example.

### Step 3. The split and the entry plan (`tools/iron_build/src/pack/plan.rs`)

```rust
pub enum Form { Source, Cooked }
pub struct PlannedEntry { pub path: String, pub guid: iron_pack::Guid, pub kind: iron_pack::AssetKind, pub form: Form,
                          pub compress: bool, pub meta: String }
pub struct PackPlan { pub name: String, pub role: iron_pack::manifest::Role, pub scene: Option<String>,
                      pub entries: Vec<PlannedEntry> }
pub struct Plan { pub packs: Vec<PackPlan>, pub unreferenced: Vec<String> }
pub fn plan(root: &Path, graph: &Graph) -> Result<Plan, String>;
```

- Rules exactly as design 6.2: code/css/include -> `global`; one scene -> `scene-<slug>`; two or more -> `shared`. Packs ordered
  `global`, `shared`, then scene packs by scene path. An empty `shared` is still planned (stable manifest).
- Forms: kind texture: from scenes/models -> `Cooked`; from css -> `Source`; from code/include -> both, unless the `.meta`
  `importer.pack` is `"cooked"`, `"source"` or `"both"`. Model -> `Cooked`. Scene, prefab -> `Cooked` (cooked JSON). Others -> `Source`.
- `compress = false` for extensions png, jpg, jpeg, mp3, ogg, opus, flac, woff2; true otherwise.
- `meta` = the `.meta` file's `importer` object serialized compactly (`serde_json::to_string`), `""` without a `.meta`.
- GUID from `.meta` (`assets::read_guid` + `Guid::parse`), else `Guid::from_path`. The cooked and the source entry of one path need two
  GUIDs (a pack's GUIDs are unique): the source keeps the `.meta` GUID; the cooked form gets
  `Guid(content_hash(b"iron.cooked:" ++ guid bytes))`; the VFS finds cooked forms by path, and `Assets.pathToGuid` answers the
  source GUID. (Write this rule into `docs/asset-packs.md`.)
- Entry order in a pack: scenes first, then textures, models, audio, fonts, other kinds; by path inside each.
- `slug(scene_path)`: file stem without `.scene`, lowercase, runs of non-`[a-z0-9]` -> `-`, trimmed; collisions get `-2`, `-3` in path order.
- Tests: `M9-TS-2504` an asset of one scene goes to its pack, of two scenes to `shared`, of code and a scene to `global`;
  `M9-TS-2505` forms per the rules incl. the `pack` importer key; `M9-TS-2506` compress flags and meta text; `M9-TS-2507` slugs and
  collisions; the plan is identical across two runs (`assert_eq!` on a `Debug` dump).

### Step 4. Writing packs: `iron-build pack` (`tools/iron_build/src/pack/write.rs`, `main.rs`)

```rust
pub struct PackOptions { pub codec: iron_pack::Codec, pub out: Option<PathBuf>, pub verify: bool, pub report: bool, pub no_cook: bool }
pub struct PackReport { pub packs: Vec<(String, iron_pack::PackSummary)>, pub unreferenced: Vec<String>, pub warnings: Vec<String>,
                        pub time: Duration }
pub fn pack(root: &Path, opts: &PackOptions) -> Result<PackReport, String>;
pub fn cook_scene(text: &str, root: &Path) -> Result<String, String>;   // minify + add "guid" to asset references (design 5)
```

- `pack` runs `build(root, Options { no_compile: false, .. })` first (it needs the bundle source and the CSS copies), then
  `graph`, `plan`, then payloads: `Source` -> the file's bytes (CSS copies from `dist/`); `Cooked` texture/model -> the fresh
  `.iron/cooked/<path>.tex|.model` (`cook::cooked_path` rule: `<root>/.iron/cooked/<path>.<ext>`, fresh = not older than the source);
  if any is missing or stale and not `--no-cook`, run `find_iron(root)` + `--cook` once and look again; still missing -> error
  `"<path>: not cooked (iron --cook failed?)"`. Cooked scene/prefab -> `cook_scene`.
- Writes `dist/data/<name>.irpk` with `PackWriter::write_file` (`WriteOptions { codec, ..Default }`), then `dist/data/index.json`
  (`Manifest::to_json`, through `write_atomic`); removes `*.irpk` in `dist/data/` that the plan no longer has. `build` = content hash of
  the concatenated pack ids.
- `--verify` opens every pack and runs `Pack::verify()`. `--report` prints a table: pack, entries, raw MB, stored MB, ratio; then
  unreferenced files and their total.
- `main.rs`: `pack <game> [--codec lz4|deflate|none] [--out dir] [--verify] [--report] [--no-cook]`; `build <game> --release` = `build`
  then `pack` and no loose asset copy into `dist/` (delete the `dist/assets` copies a dev build left).
- Tests: `M9-TS-2508` a fixture game (two scenes, a shared texture, a code-loaded sound, a CSS image over 32 KiB, a font) packs into
  global/shared/two scene packs with the expected entries (open them with `Pack::open`), `--verify` passes; `M9-TS-2509` two runs give
  byte-identical packs and index; `M9-TS-2510` `cook_scene` output parses, has GUIDs on asset references, is smaller than the input,
  and `iron_scenes` loads it (`iron_build` already depends on `iron_scenes`: `lib.rs` `manifests()`); `M9-TS-2511` a missing cooked file with `--no-cook` is an error naming the path; `M9-TS-2512` stale packs are removed.
  Use textures that need no cooker in unit tests by writing fake `.iron/cooked/<path>.tex` files newer than their sources.
- Acceptance: `cargo run -p iron_build -- pack ../examples/scenes --verify --report` and the same for `world`, `classics`,
  `platformer`, `arcade` succeed (needs `target/debug/iron_app` for cooking: `cargo build -p iron_app` first); record the report
  tables in `docs/asset-packs.md`.

### Step 5. Incremental packs (`tools/iron_build/src/pack/cache.rs`)

- Cooked payload cache `<root>/.iron/cache/packs/<32 hex>.bin`, key = `content_hash(form byte ++ cooker version ("irtx1"/"irmd1"/
  "scene1") ++ importer JSON ++ source bytes ++ for glTF: every external uri's bytes)`. Phase A fills it from `.iron/cooked/` files.
- Plan hash per pack: `content_hash` over each entry's `(guid, path, kind, form, compress, meta, payload key)` in order, plus the codec
  and `WriteOptions`. Skip writing a pack whose `plan` equals the old `index.json`'s and whose file header id equals the old `id`.
- Tests: `M9-TS-2513` an unchanged game repacks without rewriting any `.irpk` (mtimes equal) and in under 25% of the first run's time;
  `M9-TS-2514` touching one scene's texture rewrites only that scene's pack and the index; `M9-TS-2515` a changed `.meta` importer
  recooks (key changes).
- **docs** (Group A): `docs/asset-packs.md` (format summary, the build, the manifest), `docs/iron-build.md` (`pack`, `--release`).

## Group B: the runtime seam (`iron_app`, `iron_render`, `iron_audio`)

### Step 6. Loads through a `Vfs`, no behaviour change

- `crates/iron_app/Cargo.toml`: `iron_pack = { path = "../iron_pack" }`; `crates/iron_render/Cargo.toml` likewise.
- `App` (main.rs) gets `assets: iron_pack::Vfs` beside `asset_root` (`Vfs::dir(&asset_root)` at start and on reload); `Native`
  (native.rs) gets the same instead of `root` for scene reads.
- Replace the reads at the sites of design 4.1 rows 1 to 8 with `vfs.read(src)` / `read_to_string`; the `data:` branches go (the Vfs
  decodes them). Signatures: `world_view.texture_op(name, &Vfs, args)`, `world_view.model_op(&Vfs, args)`,
  `models::load(vfs: &Vfs, src, ids)`, `models::read_uri(vfs, model_dir, uri)`, `load_font(ui, vfs, ..)`, `load_sound(mixer, vfs, args)`,
  `cursor::read(vfs, url)`, `add_stylesheet(ui, vfs, ..)`, `new_panel_tree(vfs, ..)`, `render_sync::sync(views, scene, &Vfs)`.
  `cook::cooked_texture/cooked_model` keep a `root: &Path` and are called with `vfs.root()` when it is `Some`.
- `iron_render::ui`: `set_asset_root(root)` becomes `set_assets(vfs: iron_pack::Vfs)`; `images.rs` reads `self.assets.read(url)`
  (decode with `image::load_from_memory` instead of `image::open`). Keep `set_asset_root(root)` as a one-line wrapper
  (`set_assets(Vfs::dir(root))`) for the tests and the editor.
- Tests: `M9-API-1116` (`world_view.rs` tests): `texture_op` and `model_op` with a `Vfs` holding only a `PackSource` (pack made in
  the test from `examples/world/assets/coin.png` and `house.glb`) queue the same uploads as with the folder (no GPU: compare
  `texture_uploads` sizes and `mesh_uploads` counts); `M9-API-1117` (`main.rs` tests) `load_sound` into `OfflineBackend` and `load_font`
  from a pack-only Vfs; existing tests unchanged.
- Acceptance: full `cargo test --workspace`; screenshots of `examples/world`, `examples/css`, `examples/text` with
  `--screenshot --frames 30` are pixel-identical to the same runs before the change (keep the before files in `tests/output/`, compare
  with a 10-line Python script reading both PNGs).

### Step 7. Release mode: mounting `data/index.json`

- `crates/iron_app/src/data.rs` (new): `pub fn release_vfs(dir: &Path) -> anyhow::Result<Vfs>` (`Manifest::load` + `mount`, `OpenMode::Map`
  on 64-bit, `Read` on 32-bit) and `pub fn find_data_dir(args, exe) -> Option<PathBuf>`: `--data <dir>`; else `<game>/dist/data` with
  `--packs`; else `<exe dir>/data` when the executable carries an embedded bundle (the package agent's `embedded_bundle()`; until it
  lands, only the flags).
- Args: `--data <dir>`, `--packs` (`parse_args_from` in main.rs; tests in its `mod tests`).
- In release mode: `asset_db::scan` is replaced by `Vfs::entries()` mapped to `{ path, guid, importer }` (importer = the entry `meta`
  parsed, `{}` when empty); `AssetWatcher` is not started; `cook::cooked_*` are skipped (no `vfs.root()`).
- Cooked forms from packs (design 4.2): `texture_op` tries `vfs.read_cooked(src)` first -> `cook::texture_from_bytes`; adapt `srgb`
  (overwrite the flag), `mipmaps` off (keep `mips[0]`), mipmaps on with one level (`uncook_texture` -> normal upload path).
  `load_model` tries `vfs.read_cooked(src)` -> `cook::model_from_bytes(bytes, ids)`. UI images (`images.rs`): source first, else
  `read_cooked` + `uncook_texture` mip 0 (add `uncook_texture` to `iron_render` or keep it in the app and pass a decoder closure: put
  it in `iron_render::world::texture` as `CookedTexture::level0_rgba(&self)`, moving the BC decode from `cook.rs` there).
- Log once per path: `asset not in any pack: <path>` (a `HashSet` in the app).
- Tests: `M9-API-1118` `find_data_dir` precedence; `M9-API-1119` a cooked texture from a pack uploads with the script's colour space and
  mipmap choice (the three cases); `M9-API-1120` the release asset database answers `Assets.pathToGuid` / `importer` from pack metas;
  `M9-API-1121` a manifest with a wrong pack id fails start with the pack's name in the error.
- Acceptance: for `examples/scenes`, `world`, `classics`, `materials`, `platformer`: `iron-build pack <game>` then
  `iron <game> --packs --screenshot out.png --frames 30` equals the loose-file screenshot (pixel diff 0, or the BC difference only for
  textures that were not cooked before: list them); mounting all packs of an example takes under 2 ms (log the time at `info`).

### Step 8. Streaming audio straight from the map (`crates/iron_audio`)

- `Mixer::load(&mut self, id, bytes: SoundBytes, stream)` with `pub type SoundBytes = std::sync::Arc<dyn AsRef<[u8]> + Send + Sync>`;
  wrap it in a newtype implementing `AsRef<[u8]>` for kira's `from_cursor` (`StaticSoundData::from_cursor` and
  `StreamingSoundData::from_cursor` take `Cursor<T: AsRef<[u8]> + Send + Sync + 'static>`). `iron_audio` does not depend on
  `iron_pack`: the app passes `Arc::new(bytes)` (`iron_pack::Bytes` is `AsRef<[u8]>`). Update `OfflineBackend` and the device backend.
- The reload path (`main.rs:1297-1306`) passes the new bytes the same way.
- Tests: `M9-API-1122` a streamed WAV loaded from a mapped pack plays in `OfflineBackend` and the bytes passed were shared
  (`Bytes::is_shared()` before the call); `M9-API-1123` the existing audio tests with `Vec` bytes still pass (`Arc::new(vec)`).
- Acceptance: `examples/raindrop-demo` (mp3, 2.9 MB, streamed) from `--packs` plays in a screenshot run with `IRON_LOG_FILE` showing no
  load error.
- **docs** (Group B): `docs/asset-packs.md` (runtime: `Vfs`, release mode, flags), `docs/assets-and-models.md`, `docs/physics-and-audio.md`.

## Group C: loading for real games

### Step 9. Scenes from packs and scene prefetch

- `native.rs::load_requested_scenes` and `main.rs` `asset.readText` read through the Vfs (done in step 6) and cooked scenes load
  (`cook_scene` output: covered by `iron_scenes` already).
- `Vfs::prefetch(&self, pack_name: &str) -> PrefetchHandle`: a thread that reads every entry of that pack (`Pack::read`, results
  dropped: pages come into the OS cache; compressed entries are decompressed and dropped) and reports bytes done; `PrefetchHandle`
  has `progress() -> f32`, `done() -> bool`, `cancel()`. In `iron_pack/src/prefetch.rs`.
- `SceneManager.loadSceneAsync(path)` in a release run: look up `Manifest.scenes[path]`, start a prefetch of that pack and of its
  `needs` that are not done, finish the load (`finish_load`) when the prefetch is done; the scene's `progress` (iron_scenes
  `request_load` state) follows the prefetch.
- Tests: `M9-API-1124` prefetch of a 3-entry pack reports 1.0 and `done`; cancel stops it; `M9-API-1125` a release-mode
  `loadSceneAsync` of `examples/scenes` level2 finishes and the scene has its objects (no GPU; `Native` test harness as in
  `native.rs` tests).

### Step 10. Async loads, priorities, handles, script API

- `crates/iron_app/src/asset_jobs.rs`: `AssetJobs::new(vfs, threads)`, `request(key: AssetKey, priority: Priority) -> u32`,
  `poll() -> Vec<(u32, Result<Decoded, String>)>`, `cancel(id)`; `enum Decoded { Texture(TextureData), Cooked(CookedTexture),
  Model(Model), Sound(Arc<..>), Text(String) }`; a `BinaryHeap<(Priority, Reverse<seq>)>` queue behind a `Mutex` + `Condvar`;
  `threads` from `IRON_ASSET_THREADS` (default 2). The main thread uploads finished results with a budget per frame (8 ms measured
  with `Instant`, or 64 MiB), the rest next frame.
- `texture_op` / `model_op` / sound loads become requests (the script-facing `isLoaded`/`failed` flags already exist: `texture_loaded`,
  `model_loaded`); the QuickJS ops reply when done instead of at once (`js/engine/src/textures.ts` and `models.ts` already wait
  through `whenLoaded`).
- `iron_engine_api::assets` gains the functions of design 4.5 (`exists`, `packOf`, `preload`, `preloadScene`, `isDone`, `progress`,
  `release`) as host requests (`take_requests()` / `report(id, done, progress)`, the pattern `iron_scenes::request_load` uses), and
  `Resources.unloadUnusedAssets()` in a new `resources` module.
- A merged path index for many packs: `Vfs::freeze(&mut self)` builds one `HashMap<(u64 path hash, bool), (layer, entry index)>`
  for pack layers (dir layers still searched live), dropped on `mount`/`unmount`.
- Tests: `M9-API-1126` priorities: with one worker, a `High` request queued after ten `Low` ones finishes before them; `M9-API-1127`
  the upload budget splits 100 queued textures over several frames; `M9-API-1128` `freeze` gives the same answers as the unfrozen
  stack over 10k random paths and is at least 3x faster (measured in the test with a loose bound, `#[ignore]` the timing part on CI);
  `M9-TS-2520..2522` a compiled script uses `Assets.preload` / `isDone` / `progress` (iron_ts tests with the engine manifests, the
  host simulated as in the existing `M9-API-806` style tests).
- Acceptance: a test scene of 200 textures (1024x1024 RGBA, made in the test) loads with no frame over 16 ms of main-thread work
  (log max frame work) in a screenshot run.

### Step 11. Lifetimes, unloading, memory budget

- `AssetCache` in the app: `(path, form, settings) -> (engine id, refcount, bytes, last_used_frame)`; scene loads take refs through
  `iron_scenes`' asset table (`asset_cache` in `data.rs` maps entries to handles: add `iron_scenes::scene_assets(scene) -> Vec<u64>`
  and release them on `unloadScene`); `Resources.unloadUnusedAssets()` frees refcount-0 entries (`RenderCommand` removals, `Mixer`
  unload).
- `QualitySettings.textureBudgetMB` (new setting, default 0 = no limit) logged when exceeded; dropping mips waits for step 12.
- Tests: `M9-API-1129` loading then unloading a scene frees its textures and keeps the ones the next scene uses; `M9-API-1130`
  `unloadUnusedAssets` frees only refcount-0 assets; `M9-API-1131` refcounts survive `instantiate` of a prefab using the same texture.

### Step 12. Texture mip streaming (`IRTX` v2)

(Partly blocked by step 17: store each mip with a 256-byte row pitch and 512-byte-aligned offsets now, see
`iron-directstorage-and-codecs.md` 5.5; chunk-boundary padding of large mips for DirectStorage waits for step 17.)

- `cook.rs`: `VERSION_V2 = 2`: header as v1 plus `mip_count u16` followed by `(offset u64, size u64)` per mip relative to the payload
  start, mips stored **smallest first**; `texture_from_bytes` reads v1 and v2; the cooker writes v2. Move `IRTX` read/write to
  `iron_render::world::texture` (or `crates/iron_cook`, step 15) so the pack builder and the app share it.
- Loader: request the tail first (`vfs.read_range(path, 0, header + small mips)`), upload a texture with the small mips (`set_cooked_texture`
  with a mip offset: add `WorldRenderer::set_cooked_texture_levels(id, levels: Range<u32>, data)`), then the rest at `Low` priority;
  the budget of step 11 drops the top mips of textures not drawn for 300 frames.
- Tests: `M9-API-1132` v2 round-trip and v1 still read; `M9-API-1133` a tail read of a 2048^2 BC1 texture reads under 64 KiB;
  `M9-API-1134` (GPU, `shared_gpu()`) a texture uploaded low-mips-first then full equals a full upload (golden compare).
- **docs** (Group C).

## Group D: shipping

### Step 13. Patches, DLC, mods  (**needs the user's decisions 3 and 4**)

- `iron-build pack --patch-from <old data dir> --patch-name patch-0001`: compares the old manifest's packs (open them, map path+form ->
  content hash) with the new plan; writes one patch pack with new/changed entries and tombstones for removed ones, role `Patch`,
  appended to the old manifest (written as the new `index.json` next to the old packs). Mount order: base, patches by name order.
- DLC: `dlc/<id>/index.json` with role `Dlc` packs, mounted after patches, sorted by id, when the folder exists.
- Mods (if allowed): `mods/<name>/` loose (`DirSource`) or with an `index.json`, mounted last in name order, only when
  `package.json` `iron.mods` is true.
- Tests: `M9-TS-2530` a patch replaces one texture and deletes one sound; the patched Vfs answers new bytes and NotFound;
  `M9-TS-2531` a DLC pack adds a scene that `Manifest.scenes` lists; `M9-TS-2532` mods are ignored unless enabled.

### Step 14. `iron-build package --release` (with the package agent's command)

- `package --release` = `build --release` (step 4) then the package step, copying `dist/data/` to `<out>/data/` (only `index.json`
  and the `*.irpk` it lists). The exe embeds no assets; `find_data_dir` finds `<exe dir>/data`.
- Tests: `M9-TS-2540` the packaged folder has the exe and `data/` with exactly the manifest's files; `M9-TS-2541` no file with an
  asset extension lies next to the exe outside `data/`.
- Acceptance: `iron-build package examples/arcade --release`, then the packaged exe with `--screenshot` renders the same as the dev run.

### Step 15. `crates/iron_cook`: the cooker without the engine binary (phase B)

- Move from `crates/iron_app/src/cook.rs`: BC1/BC3 encode/decode, `IRTX`/`IRMD` read/write, `cook_texture`, `cook_texture_file`; from
  `models.rs`: glTF -> meshes/textures/description (`load`, `read_uri`, `EngineIds`). The plain data types `CookedTexture`, `TexelFormat`,
  `TextureData`, `TextureFilter`, `TextureWrap`, `MeshData` move to `iron_cook` and `iron_render` re-exports them (`pub use
  iron_cook::{..}`), so no caller changes. `iron_cook` depends on `image`, `gltf`, `serde_json`, `iron_core`, `iron_pack` (for `Vfs`)
  only.
- `iron-build pack` calls `iron_cook` directly (in parallel on `std::thread::scope`, results added in plan order); `--cook` of the
  engine stays for the dev loop and calls the same code.
- Tests: the moved tests keep their IDs (`M5-AST-06` etc.); `M9-TS-2542` `pack` with no engine binary present cooks textures and models.

### Step 16. Mobile packaging (when the Android/iOS targets exist; M9.c)

- Android: `Pack::open_at` over `AAsset_openFileDescriptor64`; the Gradle template stores `irpk` and `json` uncompressed; a startup
  check that `AAsset_getLength64` equals the manifest `size`. iOS: `data/` in the bundle's resources. No new pack format work.

## Group E: codecs and DirectStorage

### Step 17. Codec decision after the DirectStorage research (`CODEC-1`; **needs the user's OK for crates and a D3D12 probe**)

Research: `docs/research/iron-directstorage-and-codecs.md` (2026-10-21). Decided there: LZ4 stays the default, xz/LZMA and Brotli are
out, zstd is the recommended second codec. Open: whether Windows builds get a GPU-decompressed codec (GDeflate, or DirectStorage 1.4
zstd once it is GA) for texture and mesh chunks.

- Now (no dependency, any time): `Codec::GDeflate = 4` reserved in `format.rs` next to zstd's 3 (a reader refuses it with
  `Error::Format("codec 4 (GDeflate) not supported by this build")`); test `M9-API-1135` a chunk with codec 3 or 4 fails `read` with
  that error and `open` still succeeds.
- The prototype of that note's section 6 (`ds_probe`, headless D3D12, cold cache, LZ4 vs stored-through-DirectStorage vs GDeflate vs
  zstd into wgpu buffers, 2 GB photographic BC textures + 500 MB meshes).
- **Acceptance** (the decision rule, copied from the note): a GPU codec mode is added only if it is at least 1.5x faster in wall time
  than CPU LZ4 on 8 threads, or needs at most half the CPU time at no more than 1.1x the wall time, with a pack no larger than 0.8x the
  LZ4 pack, and the wgpu interop correctness test passes (buffer bytes equal the raw entry; a texture copied from it equals a direct
  upload). zstd is preferred over GDeflate when both pass and DirectStorage 1.4 is GA. DirectStorage for I/O only (stored/LZ4 chunks
  through the custom decompression queue) is kept only if at least 1.2x faster than today's path. The numbers go into the note, the
  decision into `iron-asset-packs.md` section 10.
- If a GPU codec is chosen, follow-ups (separate steps then): its CPU decoder in `iron_pack` (Windows), `iron-build pack --target` with
  per-kind codec tables and `data-<target>/` folders (note 4.1), a `DirectStorage` loader in `iron_app` behind the `Vfs` for the DX12
  backend (and the question of making DX12 the Windows default).

**Blocked by step 17:** only the GPU-codec work itself. Steps 1 to 11, 13, 15, 16 are not blocked (LZ4 assumed). Partly blocked:
step 4 (`--codec` accepts `lz4|deflate|none`; `zstd`/`gdeflate` values wait), step 12 (the 256-byte row pitch and 512-byte mip offsets
of the note's 5.5 go in now; padding large mips to chunk boundaries for DirectStorage texture destinations waits), step 14 (one `data/`
per target only if step 17 picks a per-target codec).

## One-line summary of the steps

0. (done) `crates/iron_pack`: container, LZ4, writer, reader, VFS (`M9-API-1100..1112`).
1. Manifest (`data/index.json`), `Vfs::entries`, mount with id checks (`M9-API-1113..1115`).
2. Reference graph from code literals, scenes, prefabs, CSS, `resources/` (`M9-TS-2500..2503`).
3. Split into global/shared/scene packs, forms, GUIDs, order (`M9-TS-2504..2507`).
4. `iron-build pack` writes packs + index, cooked scenes, `--verify`/`--report`, `build --release` (`M9-TS-2508..2512`).
5. Incremental cache and plan hashes (`M9-TS-2513..2515`).
6. `iron_app`/`iron_render` load through a `Vfs`, no behaviour change (`M9-API-1116..1117`).
7. Release mode: mount `data/`, cooked forms from packs, asset DB from packs (`M9-API-1118..1121`).
8. Zero-copy streamed audio from the map (`M9-API-1122..1123`).
9. Scenes from packs and scene-pack prefetch (`M9-API-1124..1125`).
10. Async loads, priorities, upload budget, merged index, `Assets.preload` API (`M9-API-1126..1128`, `M9-TS-2520..2522`).
11. Refcounts, unload, `unloadUnusedAssets`, texture budget (`M9-API-1129..1131`).
12. `IRTX` v2 and mip streaming (`M9-API-1132..1134`).
13. Patches, DLC, mods (decisions 3, 4) (`M9-TS-2530..2532`).
14. `package --release`: exe + `data/` (`M9-TS-2540..2541`).
15. `crates/iron_cook`, cooking inside `iron-build` (`M9-TS-2542`).
16. Android/iOS packaging notes into code when those targets exist.
17. Codec decision after the DirectStorage research: reserve GDeflate (codec 4), then the `CODEC-1` prototype decides a Windows GPU
    codec (`M9-API-1135`; blocks only the GPU-codec work, parts of 4, 12, 14).

## Needs the user

- Decision 1 (codec) before step 4 ships its default: LZ4 assumed (the research confirms it, `iron-directstorage-and-codecs.md`);
  zstd as the second codec needs the OK for a crate; step 17's prototype needs the OK for DirectStorage, `gdeflate` and `zstd`
  downloads and a D3D12 probe on this machine.
- Decisions 3 and 4 (DLC, mods) before step 13.
- Decision 5 (unreferenced assets), 6 (both forms for code-literal textures), 7 (one shared pack) before step 3: the plan assumes the
  recommended answers.
- Decision 8 (cooker RGBA8 fallback) any time; independent.
- Encryption: out of scope unless the user says otherwise.
