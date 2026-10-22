//! The pack build of `iron-build package --packs`: the data files of a game written as `.irpk`
//! packs (`iron_pack`) instead of a loose `data/` folder.
//!
//! - `assets.dat`: everything the code may load at any time (named by the code, reached by a
//!   dynamic path, selected by `include`) and the assets two or more scenes share.
//! - `scene-<slug>.dat`: one scene file (the first entry) and the assets only that scene uses.
//! - `index.json`: the packs in mount order.
//!
//! The placement rule is section 2.5 of `docs/research/iron-virtual-fs-plan.md`. Files stay in
//! source form: textures and models are not cooked here (the cooker lives in `iron_app`).
//! The output depends only on the files, never on the time or the machine.

use crate::include::Reason;
use crate::package::DataFile;
use iron_core::payload;
use iron_pack::{AssetKind, Codec, EntrySpec, Guid, Pack, PackSummary, PackWriter, WriteOptions};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Why an entry is in the pack it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    Code,
    Pattern,
    Include,
    /// Used by two or more scenes only.
    Shared,
    /// Used by this scene only.
    SceneOnly,
    /// The scene file itself.
    SceneFile,
}

impl Placed {
    fn name(self) -> &'static str {
        match self {
            Placed::Code => "code",
            Placed::Pattern => "patterns",
            Placed::Include => "include",
            Placed::Shared => "shared",
            Placed::SceneOnly => "scene",
            Placed::SceneFile => "scene file",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEntry {
    /// The game path (the pack key, exact case).
    pub key: String,
    pub src: PathBuf,
    pub placed: Placed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPack {
    /// `assets` or `scene-<slug>`.
    pub name: String,
    pub file: String,
    /// `global` or `scene`.
    pub role: &'static str,
    /// The scene file of a scene pack.
    pub scene: Option<String>,
    pub entries: Vec<PlannedEntry>,
}

/// Which pack each file goes to.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PackPlan {
    /// `assets` first, then the scene packs by scene path.
    pub packs: Vec<PlannedPack>,
}

/// File names that are compressed already (the codec is not tried on them).
fn precompressed(key: &str) -> bool {
    let ext = key.rsplit_once('.').map_or("", |(_, e)| e);
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "mp3" | "ogg" | "opus" | "flac" | "woff2"
    )
}

/// The slug of a scene path: `scenes/Level 1.scene.json` is `level-1`.
fn slug(scene: &str) -> String {
    let name = scene.rsplit('/').next().unwrap_or(scene);
    let name = name.strip_suffix(".scene.json").unwrap_or(name);
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_owned();
    if out.is_empty() { "scene".into() } else { out }
}

fn collect_strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| collect_strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| collect_strings(x, out)),
        _ => {}
    }
}

/// The `guid` of a `.meta` text, if it has one.
fn meta_guid(text: &str) -> Option<Guid> {
    let v: Value = serde_json::from_str(text).ok()?;
    Guid::parse(v["guid"].as_str()?)
}

/// What a scene file (or a prefab) names: game files, by path or `guid:`. Prefabs and other
/// JSON files it names are followed. Scene files it names are returned separately.
struct Refs<'a> {
    game: &'a Path,
    files: &'a BTreeSet<String>,
    guids: HashMap<Guid, String>,
}

impl Refs<'_> {
    fn of(&self, root: &str) -> (BTreeSet<String>, BTreeSet<String>) {
        let (mut assets, mut scenes) = (BTreeSet::new(), BTreeSet::new());
        let mut seen = BTreeSet::new();
        let mut todo = vec![root.to_owned()];
        while let Some(file) = todo.pop() {
            if !seen.insert(file.clone()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(self.game.join(&file)) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let mut strings = Vec::new();
            collect_strings(&json, &mut strings);
            for s in strings {
                let s = s.trim();
                let key = match s.strip_prefix("guid:") {
                    Some(g) => Guid::parse(g).and_then(|g| self.guids.get(&g).cloned()),
                    None => {
                        let k = s.trim_start_matches("./");
                        self.files.contains(k).then(|| k.to_owned())
                    }
                };
                let Some(key) = key.filter(|k| k != root && !k.ends_with(".meta")) else {
                    continue;
                };
                if key.ends_with(".scene.json") {
                    scenes.insert(key);
                } else {
                    if key.ends_with(".json") {
                        todo.push(key.clone());
                    }
                    assets.insert(key);
                }
            }
        }
        (assets, scenes)
    }
}

/// Places `data` (the data files of `collect_files`) into packs. `.meta` files are not entries:
/// their GUID and importer settings go into the asset's entry. Scene files are read for the
/// assets they name; those assets are packed too even when `include` does not select them.
pub fn plan(game: &Path, data: &[DataFile]) -> Result<PackPlan, String> {
    // The files to pack and why (a key is a game path).
    let mut base: BTreeMap<String, (PathBuf, Reason)> = BTreeMap::new();
    for f in data.iter().filter(|f| !f.rel.ends_with(".meta")) {
        base.insert(f.rel.clone(), (f.src.clone(), f.reason));
    }
    // A `.meta` that says `"importer": { "pack": false }` keeps a file out (never one the code names).
    base.retain(|_, (src, reason)| {
        *reason == Reason::Code
            || std::fs::read_to_string(format!("{}.meta", src.display()))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .is_none_or(|v| v["importer"]["pack"].as_bool() != Some(false))
    });

    let files: BTreeSet<String> = crate::include::game_files(game).into_iter().collect();
    let mut guids = HashMap::new();
    for f in files.iter().filter(|f| f.ends_with(".meta")) {
        if let Some(g) = std::fs::read_to_string(game.join(f))
            .ok()
            .and_then(|t| meta_guid(&t))
        {
            guids.insert(g, f.trim_end_matches(".meta").to_owned());
        }
    }
    let refs = Refs {
        game,
        files: &files,
        guids,
    };

    // Scene roots: every scene file taken, then any scene a scene names.
    let mut roots: BTreeSet<String> = base
        .keys()
        .filter(|k| k.ends_with(".scene.json"))
        .cloned()
        .collect();
    let mut users: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut queue: Vec<String> = roots.iter().cloned().collect();
    let mut done = BTreeSet::new();
    while let Some(root) = queue.pop() {
        if !done.insert(root.clone()) {
            continue;
        }
        let (assets, scenes) = refs.of(&root);
        for a in assets {
            users.entry(a).or_default().insert(root.clone());
        }
        for s in scenes {
            if roots.insert(s.clone()) {
                queue.push(s);
            }
        }
    }
    for r in &roots {
        if !base.contains_key(r) {
            base.insert(r.clone(), (game.join(r), Reason::Include));
        }
    }

    // Placement (plan section 2.5).
    let mut global = Vec::new();
    let mut per_scene: BTreeMap<String, Vec<PlannedEntry>> = BTreeMap::new();
    let mut all_keys: BTreeSet<String> = base.keys().cloned().collect();
    all_keys.extend(users.keys().cloned());
    for key in all_keys {
        if roots.contains(&key) {
            continue;
        }
        let from_data = base.get(&key);
        let src = from_data
            .map(|(s, _)| s.clone())
            .unwrap_or_else(|| game.join(&key));
        let placed = match from_data.map(|(_, r)| *r) {
            Some(Reason::Code) => Some(Placed::Code),
            Some(Reason::Pattern) => Some(Placed::Pattern),
            Some(Reason::Include) => Some(Placed::Include),
            None => None,
        };
        let entry = |placed| PlannedEntry {
            key: key.clone(),
            src: src.clone(),
            placed,
        };
        match (placed, users.get(&key)) {
            (Some(p), _) => global.push(entry(p)),
            (None, Some(u)) if u.len() == 1 => {
                let scene = u.iter().next().cloned().unwrap_or_default();
                per_scene
                    .entry(scene)
                    .or_default()
                    .push(entry(Placed::SceneOnly));
            }
            (None, _) => global.push(entry(Placed::Shared)),
        }
    }

    let mut packs = vec![PlannedPack {
        name: "assets".into(),
        file: "assets.dat".into(),
        role: "global",
        scene: None,
        entries: global,
    }];
    let mut slugs: BTreeSet<String> = BTreeSet::new();
    for scene in &roots {
        let mut name = slug(scene);
        let mut n = 2;
        while !slugs.insert(name.clone()) {
            name = format!("{}-{n}", slug(scene));
            n += 1;
        }
        let (src, _) = &base[scene];
        let mut entries = vec![PlannedEntry {
            key: scene.clone(),
            src: src.clone(),
            placed: Placed::SceneFile,
        }];
        entries.extend(per_scene.remove(scene).unwrap_or_default());
        packs.push(PlannedPack {
            name: format!("scene-{name}"),
            file: format!("scene-{name}.dat"),
            role: "scene",
            scene: Some(scene.clone()),
            entries,
        });
    }

    // Lookups are case-sensitive, so two keys that differ only in case are a mistake.
    let mut lower: HashMap<String, String> = HashMap::new();
    for e in packs.iter().flat_map(|p| &p.entries) {
        if let Some(other) = lower.insert(e.key.to_lowercase(), e.key.clone())
            && other != e.key
        {
            return Err(format!(
                "paths differ only in case: {other:?} and {:?} (pack keys are case-sensitive)",
                e.key
            ));
        }
    }
    for p in &mut packs {
        let first = usize::from(p.role == "scene");
        p.entries[first..].sort_by(|a, b| a.key.cmp(&b.key));
        if p.entries.iter().any(|e| !payload::safe_path(&e.key)) {
            return Err(format!("{}: an entry has an unsafe path", p.name));
        }
    }
    Ok(PackPlan { packs })
}

/// The name of a codec on the command line and in `index.json`.
pub fn codec_name(c: Codec) -> &'static str {
    match c {
        Codec::None => "none",
        Codec::Lz4 => "lz4",
        Codec::Deflate => "deflate",
        Codec::Zstd => "zstd",
    }
}

/// `--codec lz4|zstd|deflate|none`.
pub fn parse_codec(s: &str) -> Result<Codec, String> {
    [Codec::Lz4, Codec::Zstd, Codec::Deflate, Codec::None]
        .into_iter()
        .find(|c| codec_name(*c) == s)
        .ok_or_else(|| format!("--codec {s}: expected lz4, zstd, deflate or none"))
}

/// One pack that was written.
#[derive(Debug, Clone)]
pub struct WrittenPack {
    pub plan: PlannedPack,
    pub path: PathBuf,
    pub summary: PackSummary,
}

#[derive(Debug, Clone, Default)]
pub struct PackReport {
    pub packs: Vec<WrittenPack>,
    /// Things to know (textures and models stay in source form).
    pub notes: Vec<String>,
}

impl PackReport {
    pub fn entries(&self) -> usize {
        self.packs.iter().map(|p| p.summary.entries).sum()
    }

    pub fn bytes(&self) -> u64 {
        self.packs.iter().map(|p| p.summary.file_size).sum()
    }

    /// What went where: a header per pack with the reasons, and the files (all of them when
    /// `all` or a pack has at most `limit` entries, else the first three of each reason).
    pub fn listing(&self, all: bool, limit: usize) -> String {
        let mut out = String::new();
        for w in &self.packs {
            let mut counts: Vec<(Placed, usize)> = Vec::new();
            for e in &w.plan.entries {
                match counts.iter_mut().find(|c| c.0 == e.placed) {
                    Some(c) => c.1 += 1,
                    None => counts.push((e.placed, 1)),
                }
            }
            counts.sort_by_key(|c| c.0.name());
            let why: Vec<String> = counts
                .iter()
                .map(|(p, n)| format!("{} {n}", p.name()))
                .collect();
            out.push_str(&format!(
                "{}: {} entries ({}), {} bytes raw, {} bytes stored, file {} bytes\n",
                w.plan.file,
                w.summary.entries,
                why.join(", "),
                w.summary.raw_bytes,
                w.summary.stored_bytes,
                w.summary.file_size
            ));
            // Every file when asked (or when few), else a few per reason.
            let mut per_reason: Vec<(Placed, usize)> = Vec::new();
            for e in &w.plan.entries {
                let n = match per_reason.iter_mut().find(|c| c.0 == e.placed) {
                    Some(c) => {
                        c.1 += 1;
                        c.1
                    }
                    None => {
                        per_reason.push((e.placed, 1));
                        1
                    }
                };
                if all || w.plan.entries.len() <= limit || n <= 3 {
                    out.push_str(&format!("  {:<10} {}\n", e.placed.name(), e.key));
                }
            }
            if !all && w.plan.entries.len() > limit {
                out.push_str("  ... (--report lists every file)\n");
            }
        }
        out
    }
}

/// Writes the packs of `plan` and `index.json` into `dir`. Files are read one at a time.
pub fn write(plan: &PackPlan, dir: &Path, codec: Codec) -> Result<PackReport, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // Old output of an earlier run must not linger beside the new packs.
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n == "index.json"
                || n == "assets.dat"
                || (n.starts_with("scene-") && n.ends_with(".dat"))
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let mut report = PackReport::default();
    let mut index = Vec::new();
    for pack in &plan.packs {
        let path = dir.join(&pack.file);
        let summary = PackWriter::write_file(
            &path,
            WriteOptions {
                codec,
                ..WriteOptions::default()
            },
            |w| {
                for e in &pack.entries {
                    let data = std::fs::read(&e.src).map_err(|err| {
                        iron_pack::Error::Invalid(format!("{}: {err}", e.src.display()))
                    })?;
                    let meta_text =
                        std::fs::read_to_string(format!("{}.meta", e.src.display())).ok();
                    let meta_json = meta_text
                        .as_deref()
                        .and_then(|t| serde_json::from_str::<Value>(t).ok());
                    let importer = meta_json
                        .as_ref()
                        .map(|v| v["importer"].clone())
                        .filter(|v| !v.is_null())
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    let mut spec = EntrySpec::new(&e.key, &data);
                    if let Some(g) = meta_text.as_deref().and_then(meta_guid) {
                        spec.guid = g;
                    }
                    spec.meta = &importer;
                    spec.compress = !precompressed(&e.key);
                    w.add(&spec)?;
                }
                Ok(())
            },
        )
        .map_err(|err| format!("{}: {err}", pack.file))?;
        let mut item = serde_json::json!({
            "name": pack.name,
            "file": pack.file,
            "role": pack.role,
            "id": summary.pack_id.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "entries": summary.entries,
            "bytes": summary.file_size,
        });
        if let Some(s) = &pack.scene {
            item["scene"] = s.clone().into();
        }
        index.push(item);
        report.packs.push(WrittenPack {
            plan: pack.clone(),
            path,
            summary,
        });
    }
    let json = serde_json::json!({ "format": 1, "codec": codec_name(codec), "packs": index });
    std::fs::write(
        dir.join("index.json"),
        serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("{}: {e}", dir.display()))?;
    if report.packs.iter().flat_map(|p| &p.plan.entries).any(|e| {
        matches!(
            AssetKind::of_path(&e.key),
            AssetKind::Texture | AssetKind::Model
        )
    }) {
        report.notes.push(
            "textures and models are stored in source form (the pack build has no cooker yet)"
                .into(),
        );
    }
    Ok(report)
}

/// `iron-build pack-info <file>`: header, codecs and entries of a pack.
pub fn info(path: &Path) -> Result<String, String> {
    let pack = Pack::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let h = pack.header();
    let mut out = format!(
        "{}\n  version {}, id {}, {} entries, file {} bytes, chunk {} KiB, align {} bytes\n",
        path.display(),
        h.version,
        h.pack_id
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        pack.len(),
        h.file_size,
        (1u64 << h.chunk_log2) / 1024,
        1u64 << h.align_log2
    );
    let mut codecs: BTreeMap<String, (usize, u64, u64)> = BTreeMap::new();
    let mut rows = Vec::new();
    for e in pack.entries() {
        let chunks = pack.chunks_of(&e);
        let stored: u64 = chunks.iter().map(|c| u64::from(c.1)).sum();
        let mut names: Vec<String> = chunks
            .iter()
            .map(|c| match c.0 {
                Some(Codec::None) => "raw".to_owned(),
                Some(Codec::Lz4) => "lz4".to_owned(),
                Some(Codec::Deflate) => "deflate".to_owned(),
                Some(Codec::Zstd) => "zstd".to_owned(),
                None => "?".to_owned(),
            })
            .collect();
        names.sort();
        names.dedup();
        let codec = names.join("+");
        let c = codecs.entry(codec.clone()).or_default();
        c.0 += 1;
        c.1 += e.size;
        c.2 += stored;
        rows.push((e.path.clone(), e, stored, codec));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let sums: Vec<String> = codecs
        .iter()
        .map(|(k, v)| format!("{k}: {} entries, {} -> {} bytes", v.0, v.1, v.2))
        .collect();
    out.push_str(&format!("  codecs: {}\n", sums.join("; ")));
    out.push_str(&format!(
        "  {:<8} {:>10} {:>10} {:<8} {:<34} path\n",
        "kind", "size", "stored", "codec", "guid"
    ));
    for (path, e, stored, codec) in rows {
        out.push_str(&format!(
            "  {:<8} {:>10} {:>10} {:<8} {} {}{}\n",
            format!("{:?}", e.kind).to_lowercase(),
            e.size,
            stored,
            codec,
            e.guid,
            path,
            if e.is_cooked() { " (cooked)" } else { "" }
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(tag: &str, files: &[(&str, &str)]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_packs_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (p, c) in files {
            let f = d.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, c).unwrap();
        }
        d
    }

    fn data(d: &Path, rels: &[(&str, Reason)]) -> Vec<DataFile> {
        rels.iter()
            .map(|(r, reason)| DataFile {
                rel: (*r).to_owned(),
                src: d.join(r),
                size: 1,
                reason: *reason,
            })
            .collect()
    }

    fn keys(p: &PlannedPack) -> Vec<&str> {
        p.entries.iter().map(|e| e.key.as_str()).collect()
    }

    // M9-TS-2554
    #[test]
    fn assets_go_to_the_scene_that_uses_them_alone_else_to_assets_dat() {
        let d = game(
            "place",
            &[
                (
                    "scenes/a.scene.json",
                    r#"{"assets":{"x":{"src":"assets/a_only.png"},"y":{"src":"./assets/shared.wav"},"p":"assets/boss.prefab.json","c":"assets/code.png","i":"assets/inc.png"}}"#,
                ),
                (
                    "scenes/b.scene.json",
                    r#"{"objects":[{"sound":"assets/shared.wav","t":"assets/b_only.png"}]}"#,
                ),
                ("assets/boss.prefab.json", r#"{"mesh":"assets/boss.glb"}"#),
                ("assets/a_only.png", "1"),
                ("assets/b_only.png", "2"),
                ("assets/shared.wav", "3"),
                ("assets/boss.glb", "4"),
                ("assets/code.png", "5"),
                ("assets/inc.png", "6"),
                ("assets/free.png", "7"),
            ],
        );
        // Not selected by include: a_only, b_only, shared and the prefab chain come from the scenes.
        let files = data(
            &d,
            &[
                ("scenes/a.scene.json", Reason::Code),
                ("scenes/b.scene.json", Reason::Code),
                ("assets/code.png", Reason::Code),
                ("assets/inc.png", Reason::Include),
                ("assets/free.png", Reason::Include),
            ],
        );
        let plan = plan(&d, &files).unwrap();
        let names: Vec<&str> = plan.packs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["assets", "scene-a", "scene-b"]);
        assert_eq!(
            keys(&plan.packs[0]),
            [
                "assets/code.png",
                "assets/free.png",
                "assets/inc.png",
                "assets/shared.wav"
            ]
        );
        // The scene file is first; its own assets (and the prefab's) follow, sorted.
        assert_eq!(
            keys(&plan.packs[1]),
            [
                "scenes/a.scene.json",
                "assets/a_only.png",
                "assets/boss.glb",
                "assets/boss.prefab.json"
            ]
        );
        assert_eq!(
            keys(&plan.packs[2]),
            ["scenes/b.scene.json", "assets/b_only.png"]
        );
        let why = |p: usize, k: &str| {
            plan.packs[p]
                .entries
                .iter()
                .find(|e| e.key == k)
                .unwrap()
                .placed
        };
        assert_eq!(why(0, "assets/shared.wav"), Placed::Shared);
        assert_eq!(why(0, "assets/code.png"), Placed::Code);
        assert_eq!(why(1, "assets/a_only.png"), Placed::SceneOnly);
        assert_eq!(why(1, "scenes/a.scene.json"), Placed::SceneFile);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2554
    #[test]
    fn a_scene_file_has_its_own_pack_even_when_included_and_slugs_stay_distinct() {
        let d = game(
            "slug",
            &[
                ("a/Level 1.scene.json", "{}"),
                ("b/level-1.scene.json", "{}"),
                ("assets/x.png", "1"),
            ],
        );
        let files = data(
            &d,
            &[
                ("a/Level 1.scene.json", Reason::Include),
                ("b/level-1.scene.json", Reason::Include),
                ("assets/x.png", Reason::Include),
            ],
        );
        let plan = plan(&d, &files).unwrap();
        let names: Vec<&str> = plan.packs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["assets", "scene-level-1", "scene-level-1-2"]);
        assert_eq!(keys(&plan.packs[0]), ["assets/x.png"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2556
    #[test]
    fn two_keys_that_differ_only_in_case_are_an_error() {
        let d = game("case", &[("assets/a.png", "1")]);
        let files = vec![
            DataFile {
                rel: "assets/a.png".into(),
                src: d.join("assets/a.png"),
                size: 1,
                reason: Reason::Include,
            },
            DataFile {
                rel: "assets/A.png".into(),
                src: d.join("assets/a.png"),
                size: 1,
                reason: Reason::Include,
            },
        ];
        let e = plan(&d, &files).unwrap_err();
        assert!(e.contains("only in case"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2558
    #[test]
    fn a_written_pack_reads_back_byte_identical_with_guids_meta_and_the_vfs() {
        let big: String = (0..5000)
            .map(|i| format!("line {i} of some text\n"))
            .collect();
        let d = game(
            "read",
            &[
                ("assets/a.svg", "<svg/>"),
                (
                    "assets/a.svg.meta",
                    r#"{"guid":"0123456789abcdef0123456789abcdef","importer":{"type":"texture"}}"#,
                ),
                ("assets/big.txt", &big),
                ("assets/s.mp3", "ID3-not-really"),
                ("scenes/l.scene.json", r#"{"t":"assets/only.png"}"#),
                ("assets/only.png", "PNGDATA"),
            ],
        );
        let files = data(
            &d,
            &[
                ("assets/a.svg", Reason::Include),
                ("assets/a.svg.meta", Reason::Include),
                ("assets/big.txt", Reason::Include),
                ("assets/s.mp3", Reason::Include),
                ("scenes/l.scene.json", Reason::Code),
            ],
        );
        let out = d.join("out");
        let report = write(&plan(&d, &files).unwrap(), &out, Codec::Lz4).unwrap();
        assert_eq!(report.packs.len(), 2);
        assert!(out.join("index.json").is_file());

        let assets = Pack::open(&out.join("assets.dat")).unwrap();
        let scene = Pack::open(&out.join("scene-l.dat")).unwrap();
        for (pack, key) in [
            (&assets, "assets/a.svg"),
            (&assets, "assets/big.txt"),
            (&assets, "assets/s.mp3"),
            (&scene, "scenes/l.scene.json"),
            (&scene, "assets/only.png"),
        ] {
            let e = pack
                .find_path(key, false)
                .unwrap_or_else(|| panic!("{key}"));
            assert_eq!(
                &*pack.read(&e).unwrap(),
                &std::fs::read(d.join(key)).unwrap()[..],
                "{key}"
            );
        }
        let e = assets.find_path("assets/a.svg", false).unwrap();
        assert_eq!(e.guid.to_hex(), "0123456789abcdef0123456789abcdef");
        assert!(e.meta.contains("texture"));
        // The big text is compressed, the mp3 is stored.
        let big_entry = assets.find_path("assets/big.txt", false).unwrap();
        assert!(
            assets
                .chunks_of(&big_entry)
                .iter()
                .all(|c| c.0 == Some(Codec::Lz4))
        );
        let mp3 = assets.find_path("assets/s.mp3", false).unwrap();
        assert!(
            assets
                .chunks_of(&mp3)
                .iter()
                .all(|c| c.0 == Some(Codec::None))
        );
        assert_eq!(assets.verify().unwrap(), assets.len());

        // Through the Vfs the way a runtime mounts them: assets at the bottom, scenes above.
        let mut vfs = iron_pack::Vfs::new();
        vfs.mount(std::sync::Arc::new(iron_pack::PackSource::new(
            "assets", assets,
        )));
        vfs.mount(std::sync::Arc::new(iron_pack::PackSource::new(
            "scene-l", scene,
        )));
        assert_eq!(&*vfs.read("assets/only.png").unwrap(), b"PNGDATA");
        assert_eq!(&*vfs.read("assets/a.svg").unwrap(), b"<svg/>");
        assert_eq!(vfs.read_to_string("assets/big.txt").unwrap(), big);

        // pack-info lists them.
        let text = info(&out.join("assets.dat")).unwrap();
        assert!(
            text.contains("assets/big.txt") && text.contains("lz4") && text.contains("raw"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2556
    #[test]
    fn two_builds_write_the_same_bytes_and_an_old_pack_does_not_linger() {
        let d = game(
            "same",
            &[
                ("assets/a.png", "A"),
                ("assets/b.wav", "BBBBBBBBBBBBBBBBBBBBBBBB"),
                ("s/x.scene.json", "{}"),
            ],
        );
        let files = data(
            &d,
            &[
                ("assets/a.png", Reason::Include),
                ("assets/b.wav", Reason::Include),
                ("s/x.scene.json", Reason::Code),
            ],
        );
        let p = plan(&d, &files).unwrap();
        let (o1, o2) = (d.join("o1"), d.join("o2"));
        std::fs::create_dir_all(&o2).unwrap();
        std::fs::write(o2.join("scene-gone.dat"), b"old").unwrap();
        write(&p, &o1, Codec::Lz4).unwrap();
        write(&p, &o2, Codec::Lz4).unwrap();
        assert!(!o2.join("scene-gone.dat").exists());
        for f in ["assets.dat", "scene-x.dat", "index.json"] {
            assert_eq!(
                std::fs::read(o1.join(f)).unwrap(),
                std::fs::read(o2.join(f)).unwrap(),
                "{f}"
            );
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2560
    #[test]
    fn the_zstd_codec_is_chosen_by_name_written_read_back_and_shown_by_pack_info() {
        assert_eq!(parse_codec("zstd").unwrap(), Codec::Zstd);
        assert_eq!(parse_codec("lz4").unwrap(), Codec::Lz4);
        assert!(parse_codec("rar").is_err());
        let big: String = (0..20000)
            .map(|i| {
                format!(
                    "line {i} of some text
"
                )
            })
            .collect();
        let d = game(
            "zstd",
            &[("assets/big.txt", &big), ("assets/s.mp3", "ID3-not-really")],
        );
        let files = data(
            &d,
            &[
                ("assets/big.txt", Reason::Include),
                ("assets/s.mp3", Reason::Include),
            ],
        );
        let out = d.join("out");
        write(&plan(&d, &files).unwrap(), &out, Codec::Zstd).unwrap();
        let index = std::fs::read_to_string(out.join("index.json")).unwrap();
        assert!(index.contains("\"codec\": \"zstd\""), "{index}");
        let pack = Pack::open(&out.join("assets.dat")).unwrap();
        let big_entry = pack.find_path("assets/big.txt", false).unwrap();
        assert!(
            pack.chunks_of(&big_entry)
                .iter()
                .all(|c| c.0 == Some(Codec::Zstd))
        );
        assert_eq!(&*pack.read(&big_entry).unwrap(), big.as_bytes());
        // A range across a chunk boundary (chunks are 256 KiB).
        let at = (1u64 << 18) - 5;
        let got = pack.read_range(&big_entry, at, 10).unwrap();
        assert_eq!(&*got, &big.as_bytes()[at as usize..at as usize + 10]);
        assert_eq!(pack.verify().unwrap(), pack.len());
        let text = info(&out.join("assets.dat")).unwrap();
        assert!(text.contains("zstd") && text.contains("raw"), "{text}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
