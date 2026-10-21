//! `iron-build package <game>`: one executable for a game. A copy of the engine host
//! (`iron_app`) gets the game's icon and version info (Windows PE resources, `pe.rs`) and the
//! game's code (bundle, CSS, metadata, icon) as a payload at its end (`iron_core::payload`). The
//! assets are copied to a `data/` folder beside it, never embedded. See `docs/iron-build.md`.

use crate::{Options, build, icon, pe};
use iron_core::payload::{self, Archive};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// What the executable says about itself. Resolved from `package.json` (`iron` section, then the
/// top level) and the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub title: String,
    /// The file name without `.exe`.
    pub exe: String,
    pub company: String,
    pub identifier: String,
    pub version: String,
    pub copyright: String,
    pub description: String,
    /// The icon file (`.png` or `.ico`), relative to the game folder, or as given on the command line.
    pub icon: Option<String>,
}

impl Metadata {
    /// The Windows file name (`OriginalFilename`).
    pub fn file_name(&self) -> String {
        format!("{}.exe", self.exe)
    }
}

/// Command-line overrides of [`Metadata`].
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub title: Option<String>,
    pub exe: Option<String>,
    pub company: Option<String>,
    pub version: Option<String>,
    pub copyright: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}

/// A name that is safe as a file name on every system.
fn file_safe(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_ .".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let t = t.trim().trim_matches('.').to_string();
    if t.is_empty() { "game".into() } else { t }
}

/// Resolves the metadata. Order: command line, `iron.<field>`, the `package.json` field of the
/// same meaning, a default (title: `name`; exe: the title; version: `0.0.0`; copyright:
/// "Copyright (c) <company>"; description: the title).
pub fn resolve(pkg: &Value, folder: &str, ov: &Overrides) -> Metadata {
    let iron = &pkg["iron"];
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    let name = text(&pkg["name"]).unwrap_or_else(|| folder.to_owned());
    let title = ov
        .title
        .clone()
        .or_else(|| text(&iron["title"]))
        .unwrap_or_else(|| name.clone());
    let exe = ov
        .exe
        .clone()
        .or_else(|| text(&iron["exe"]))
        .map(|e| e.trim_end_matches(".exe").to_string())
        .map(|e| file_safe(&e))
        .unwrap_or_else(|| file_safe(&title));
    let company = ov
        .company
        .clone()
        .or_else(|| text(&iron["company"]))
        .or_else(|| text(&pkg["author"]))
        .unwrap_or_default();
    let copyright = ov
        .copyright
        .clone()
        .or_else(|| text(&iron["copyright"]))
        .unwrap_or_else(|| {
            if company.is_empty() {
                String::new()
            } else {
                format!("Copyright (c) {company}")
            }
        });
    Metadata {
        exe,
        company,
        copyright,
        identifier: text(&iron["identifier"]).unwrap_or_else(|| format!("dev.iron.{name}")),
        version: ov
            .version
            .clone()
            .or_else(|| text(&iron["version"]))
            .or_else(|| text(&pkg["version"]))
            .unwrap_or_else(|| "0.0.0".into()),
        description: ov
            .description
            .clone()
            .or_else(|| text(&iron["description"]))
            .or_else(|| text(&pkg["description"]))
            .unwrap_or_else(|| title.clone()),
        icon: ov.icon.clone().or_else(|| text(&iron["icon"])),
        title,
    }
}

/// The `package.json` that goes into the payload: the game's, with the resolved metadata, so the
/// running game (`Application`, the window title and icon, the settings folder) sees the same.
pub fn payload_package_json(
    pkg: &Value,
    meta: &Metadata,
    icon_entry: Option<&str>,
    data: &DataInfo,
) -> Value {
    let mut out = if pkg.is_object() {
        pkg.clone()
    } else {
        json!({})
    };
    out["version"] = json!(meta.version);
    if !out["iron"].is_object() {
        out["iron"] = json!({});
    }
    let iron = &mut out["iron"];
    iron["title"] = json!(meta.title);
    iron["company"] = json!(meta.company);
    iron["identifier"] = json!(meta.identifier);
    iron["exe"] = json!(meta.exe);
    iron["copyright"] = json!(meta.copyright);
    iron["description"] = json!(meta.description);
    // Where the running exe looks for the assets, and whether it may start without them.
    iron["dataDir"] = json!(data.dir);
    iron["dataRequired"] = json!(data.required);
    match icon_entry {
        Some(p) => iron["icon"] = json!(p),
        None => {
            if let Some(o) = iron.as_object_mut() {
                o.remove("icon");
            }
        }
    }
    out
}

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Every file under `dir` as `/`-separated paths relative to `base`, skipping dot names.
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| io(dir, e))?
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = e.path();
        if path.is_dir() {
            walk(base, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push(rel);
        }
    }
    Ok(())
}

/// The name of the data folder and whether the game needs it to run (`iron.dataDir`,
/// `iron.dataRequired` of the payload's `package.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataInfo {
    pub dir: String,
    pub required: bool,
}

/// The default name of the folder beside the exe that holds the assets.
pub const DEFAULT_DATA_DIR: &str = "data";

/// The most the embedded essentials may add up to: past this, none of them are embedded.
pub const ESSENTIALS_LIMIT: u64 = 256 * 1024;

/// An asset file that goes to the data folder (not read into memory here: games have gigabytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFile {
    /// Relative to the game folder; the same path under `data/`.
    pub rel: String,
    pub src: PathBuf,
    pub size: u64,
}

/// What a game is made of: the small payload and the files for the data folder.
#[derive(Debug)]
pub struct Collected {
    /// The payload entries (path relative to the game folder).
    pub payload: Vec<(String, Vec<u8>)>,
    /// Everything for `data/`, including the files also embedded as essentials.
    pub data: Vec<DataFile>,
    pub info: DataInfo,
}

/// `dir` is one plain folder name (no separators, not hidden).
pub fn valid_data_dir(dir: &str) -> bool {
    !dir.is_empty()
        && !dir.starts_with('.')
        && !dir.contains(['/', '\\', ':'])
        && payload::safe_path(dir)
}

/// An absolute, `.`/`..`-free form of `p` for comparing (no file system access).
fn abs(p: &Path) -> PathBuf {
    let p = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// Copies the data files under `dir` (same relative paths). Returns the bytes written.
pub fn copy_data(files: &[DataFile], dir: &Path) -> Result<u64, String> {
    let mut total = 0;
    for f in files {
        let to = dir.join(&f.rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        total += std::fs::copy(&f.src, &to).map_err(|e| io(&to, e))?;
    }
    Ok(total)
}

/// The files of the game.
///
/// The **payload** holds only `dist/game.irbundle`, `dist/game.css`, the rewritten `package.json`
/// (`payload_package_json`), the icon as `iron-icon.<ext>` and the *essentials*: the assets the
/// bundle itself names (`assets` in `game.irbundle`, no `.meta`), but only when together they are
/// at most [`ESSENTIALS_LIMIT`] (256 KiB); past it none is embedded, a large file never is.
///
/// The **data** files are everything the game may read: every asset the bundle names, the scene
/// `iron.startScene`, every path of `iron.include` (files or folders; default `["assets"]`), each
/// with its `.meta`, and the other files of `dist/` (what the CSS references). Not `game.o`,
/// `game_bindings.rs`, `*.exe`, `game.js`, source maps, dot files, copies of assets in `dist/` or
/// anything under `exclude` (the output's own data folder).
///
/// `info.required` is false when the game has no data files, or when the essentials are all of
/// them (apart from `.meta`): such a game starts without a data folder.
pub fn collect_files(
    game: &Path,
    pkg: &Value,
    meta: &Metadata,
    icon_bytes: Option<(&str, Vec<u8>)>,
    data_dir: &str,
    exclude: &[PathBuf],
) -> Result<Collected, String> {
    let bundle_path = game.join("dist/game.irbundle");
    let bundle: Value = serde_json::from_str(
        &std::fs::read_to_string(&bundle_path).map_err(|e| io(&bundle_path, e))?,
    )
    .map_err(|e| format!("{}: {e}", bundle_path.display()))?;

    // What to take from the game folder itself.
    let mut named: Vec<String> = Vec::new();
    for a in bundle["assets"].as_array().into_iter().flatten() {
        named.extend(a.as_str().map(str::to_owned));
    }
    let mut wanted = named.clone();
    wanted.extend(pkg["iron"]["startScene"].as_str().map(str::to_owned));
    // The build finds assets by the string literals of the program; a path built at run time
    // (`"assets/" + name`) or inside CSS text is missed, so the `assets` folder goes in whole
    // unless `iron.include` says what to take instead.
    let include = pkg["iron"]["include"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![json!("assets")]);
    for inc in &include {
        let Some(rel) = inc.as_str() else { continue };
        let rel = rel.trim_end_matches('/');
        if !payload::safe_path(rel) {
            return Err(format!("iron.include: unsafe path {rel:?}"));
        }
        let p = game.join(rel);
        if p.is_dir() {
            walk(game, &p, &mut wanted)?;
        } else {
            wanted.push(rel.to_owned());
        }
    }
    let mut paths: Vec<String> = Vec::new();
    for rel in wanted {
        if !payload::safe_path(&rel) {
            return Err(format!("unsafe asset path {rel:?}"));
        }
        for r in [rel.clone(), format!("{rel}.meta")] {
            if !paths.contains(&r) && game.join(&r).is_file() {
                paths.push(r);
            }
        }
    }
    // dist/: all but the copies of assets already taken, build products and executables.
    let mut dist = Vec::new();
    if game.join("dist").is_dir() {
        walk(game, &game.join("dist"), &mut dist)?;
    }
    for rel in dist {
        let inner = rel.strip_prefix("dist/").unwrap_or(&rel);
        let skip = matches!(
            inner,
            "game.irbundle" | "game.css" | "game.o" | "game_bindings.rs" | "game.js" | "vendor.js"
        ) || inner.ends_with(".map")
            || inner.ends_with(".exe")
            || paths.iter().any(|p| p == inner)
            || exclude.iter().any(|x| abs(&game.join(&rel)).starts_with(x));
        if !skip && !paths.contains(&rel) {
            paths.push(rel);
        }
    }

    let mut data = Vec::new();
    for rel in paths {
        let src = game.join(&rel);
        let size = std::fs::metadata(&src).map_err(|e| io(&src, e))?.len();
        data.push(DataFile { rel, src, size });
    }

    let mut files = Vec::new();
    for rel in ["dist/game.irbundle", "dist/game.css"] {
        let p = game.join(rel);
        match std::fs::read(&p) {
            Ok(bytes) => files.push((rel.to_string(), bytes)),
            Err(e) if rel == "dist/game.irbundle" => return Err(io(&p, e)),
            Err(_) => {}
        }
    }
    // The essentials: the assets the bundle names, if all of them are tiny together.
    let essential: Vec<&DataFile> = data
        .iter()
        .filter(|f| !f.rel.ends_with(".meta") && named.contains(&f.rel))
        .collect();
    let embed = essential.iter().map(|f| f.size).sum::<u64>() <= ESSENTIALS_LIMIT;
    let embedded: Vec<&str> = if embed {
        essential.iter().map(|f| f.rel.as_str()).collect()
    } else {
        Vec::new()
    };
    for f in &data {
        if embedded.contains(&f.rel.as_str()) {
            let bytes = std::fs::read(&f.src).map_err(|e| io(&f.src, e))?;
            files.push((f.rel.clone(), bytes));
        }
    }
    let info = DataInfo {
        dir: data_dir.to_string(),
        required: data
            .iter()
            .any(|f| !f.rel.ends_with(".meta") && !embedded.contains(&f.rel.as_str())),
    };

    let icon_entry = icon_bytes.as_ref().map(|(name, _)| name.to_string());
    let json = payload_package_json(pkg, meta, icon_entry.as_deref(), &info);
    files.push((
        "package.json".into(),
        serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?,
    ));
    if let Some((name, bytes)) = icon_bytes {
        files.push((name.to_string(), bytes));
    }
    Ok(Collected {
        payload: files,
        data,
        info,
    })
}

/// How to package.
#[derive(Debug, Clone, Default)]
pub struct PackageOptions {
    /// The engine host to copy (default: `find_host`).
    pub host: Option<PathBuf>,
    /// Where to write (default `<game>/dist/<exe>.exe`).
    pub out: Option<PathBuf>,
    pub overrides: Overrides,
    /// Do not build first: use the `dist/` that is there.
    pub no_build: bool,
    pub no_tsc: bool,
    /// A command to sign the finished file; `{exe}` is replaced by its (quoted) path.
    pub sign_cmd: Option<String>,
    /// The folder beside the exe that gets the assets (default `data`).
    pub data_dir: Option<String>,
    /// Do not copy the assets (the exe still looks for the data folder).
    pub no_data: bool,
}

#[derive(Debug)]
pub struct Packaged {
    pub path: PathBuf,
    pub size: u64,
    pub payload_size: u64,
    pub files: usize,
    /// The data folder, when assets were copied.
    pub data_path: Option<PathBuf>,
    pub data_files: usize,
    pub data_bytes: u64,
    pub info: DataInfo,
    /// The names of the embedded files.
    pub embedded: Vec<String>,
    pub meta: Metadata,
    /// Windows resources were written (the host is a PE file).
    pub resources: bool,
}

fn exe_name(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// The engine host: `--host`, `$IRON_HOST`, `iron_app` next to this program, or the newest
/// `target/{release,debug}/iron_app` found going up from the game folder and the current folder.
pub fn find_host(game: &Path, explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Ok(p.to_path_buf())
        } else {
            Err(format!("host {} not found", p.display()))
        };
    }
    if let Some(p) = std::env::var_os("IRON_HOST").map(PathBuf::from) {
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("$IRON_HOST {} not found", p.display()))
        };
    }
    let mut found: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        found.push(dir.join(exe_name("iron_app")));
    }
    let mut starts = vec![game.to_path_buf()];
    starts.extend(std::env::current_dir().ok());
    for s in starts {
        let s = s.canonicalize().unwrap_or(s);
        for a in s.ancestors() {
            for profile in ["release", "debug"] {
                found.push(a.join("target").join(profile).join(exe_name("iron_app")));
            }
        }
    }
    let mtime = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    found
        .into_iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| (p.to_string_lossy().contains("release"), mtime(p)))
        .ok_or_else(|| "no iron_app host found (build it, or pass --host <path>)".to_string())
}

fn run_sign(cmd: &str, exe: &Path) -> Result<(), String> {
    let line = cmd.replace("{exe}", &format!("\"{}\"", exe.display()));
    let mut c;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c = std::process::Command::new("cmd");
        c.args(["/S", "/C"]).raw_arg(format!("\"{line}\""));
    }
    #[cfg(not(windows))]
    {
        c = std::process::Command::new("sh");
        c.args(["-c", &line]);
    }
    let status = c.status().map_err(|e| format!("sign command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("sign command failed ({status}): {line}"))
    }
}

/// Packages the game in `game`.
pub fn package(game: &Path, opts: &PackageOptions) -> Result<Packaged, String> {
    if !opts.no_build {
        build(
            game,
            &Options {
                no_tsc: opts.no_tsc,
                ..Default::default()
            },
        )?;
    }
    let pkg_path = game.join("package.json");
    let pkg: Value = match std::fs::read_to_string(&pkg_path) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| format!("{}: {e}", pkg_path.display()))?,
        Err(_) => json!({}),
    };
    let folder = game
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "game".into());
    let meta = resolve(&pkg, &folder, &opts.overrides);

    // The icon: relative to the game folder (from package.json) or a path given on the command line.
    let icon_path = meta.icon.as_deref().map(|i| {
        let p = PathBuf::from(i);
        if opts.overrides.icon.is_some() || p.is_absolute() {
            p
        } else {
            game.join(i)
        }
    });
    let icon = match &icon_path {
        Some(p) => {
            let data = std::fs::read(p).map_err(|e| io(p, e))?;
            // Fail now, not on the player's machine, if it is not an image.
            icon::to_ico(&data).map_err(|e| format!("{}: {e}", p.display()))?;
            let ext = if icon::is_ico(&data) { "ico" } else { "png" };
            Some((format!("iron-icon.{ext}"), data))
        }
        None => None,
    };

    let data_dir = opts
        .data_dir
        .clone()
        .unwrap_or_else(|| DEFAULT_DATA_DIR.to_string());
    if !valid_data_dir(&data_dir) {
        return Err(format!(
            "--data-dir {data_dir:?} is not a plain folder name"
        ));
    }

    // The host without any payload of its own.
    let host_path = find_host(game, opts.host.as_deref())?;
    let host_len = payload::host_len(&host_path)?;
    let mut host = std::fs::read(&host_path).map_err(|e| io(&host_path, e))?;
    host.truncate(host_len as usize);
    let is_pe = host.starts_with(b"MZ");
    let out = opts.out.clone().unwrap_or_else(|| {
        let name = if is_pe {
            meta.file_name()
        } else {
            meta.exe.clone()
        };
        game.join("dist").join(name)
    });
    let data_path = out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
        .join(&data_dir);
    let collected = collect_files(
        game,
        &pkg,
        &meta,
        icon.as_ref().map(|(n, d)| (n.as_str(), d.clone())),
        &data_dir,
        &[abs(&data_path)],
    )?;
    if is_pe {
        let ico = match &icon {
            Some((_, data)) => Some(icon::to_ico(data)?),
            None => None,
        };
        host = pe::set_resources(host, &meta, ico.as_deref())?;
    }
    let payload_size = payload::write_payload(&mut host, &collected.payload)?;

    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    }
    let tmp = out.with_extension("tmp");
    std::fs::write(&tmp, &host).map_err(|e| io(&tmp, e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    std::fs::rename(&tmp, &out).map_err(|e| io(&out, e))?;
    // The assets stay files beside the exe.
    let data_bytes = if opts.no_data || collected.data.is_empty() {
        0
    } else {
        copy_data(&collected.data, &data_path)?
    };
    let copied = !(opts.no_data || collected.data.is_empty());
    if let Some(cmd) = &opts.sign_cmd {
        run_sign(cmd, &out)?;
    }
    // Read it back: a packaged file that cannot be opened is an error now.
    let check = Archive::open(&out)?.ok_or("the payload was not found in the output")?;
    let size = std::fs::metadata(&out).map_err(|e| io(&out, e))?.len();
    Ok(Packaged {
        path: out,
        size,
        payload_size,
        files: check.entries.len(),
        data_path: copied.then_some(data_path),
        data_files: if copied { collected.data.len() } else { 0 },
        data_bytes,
        info: collected.info,
        embedded: check.entries.iter().map(|e| e.path.clone()).collect(),
        meta,
        resources: is_pe,
    })
}

const USAGE: &str = "usage: iron-build package <game> [--host iron_app.exe] [--out file] [--no-build] [--no-tsc]
       [--exe name] [--title t] [--company c] [--version v] [--copyright c] [--description d] [--icon file.png|ico]
       [--sign-cmd \"signtool sign ... {exe}\"] [--data-dir name] [--no-data]";

/// The `package` command: `args` are the ones after the word `package`. Returns the exit code.
pub fn run_cli(args: &[String]) -> i32 {
    let mut opts = PackageOptions::default();
    let mut game = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let value = |it: &mut std::slice::Iter<'_, String>| it.next().cloned();
        match a.as_str() {
            "--no-build" => opts.no_build = true,
            "--no-tsc" => opts.no_tsc = true,
            "--host" => opts.host = value(&mut it).map(PathBuf::from),
            "--out" => opts.out = value(&mut it).map(PathBuf::from),
            "--exe" => opts.overrides.exe = value(&mut it),
            "--title" => opts.overrides.title = value(&mut it),
            "--company" => opts.overrides.company = value(&mut it),
            "--version" => opts.overrides.version = value(&mut it),
            "--copyright" => opts.overrides.copyright = value(&mut it),
            "--description" => opts.overrides.description = value(&mut it),
            "--icon" => opts.overrides.icon = value(&mut it),
            "--sign-cmd" => opts.sign_cmd = value(&mut it),
            "--data-dir" => opts.data_dir = value(&mut it),
            "--no-data" => opts.no_data = true,
            s if !s.starts_with("--") && game.is_none() => game = Some(PathBuf::from(s)),
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let game = game.unwrap_or_else(|| PathBuf::from("."));
    match package(&game, &opts) {
        Ok(p) => {
            println!(
                "{}: {} file(s) embedded ({} bytes), {} bytes total{}",
                p.path.display(),
                p.files,
                p.payload_size,
                p.size,
                if p.resources {
                    ", icon and version info set"
                } else {
                    ""
                }
            );
            match &p.data_path {
                Some(d) => println!(
                    "{}: {} asset file(s), {} bytes copied{}",
                    d.display(),
                    p.data_files,
                    p.data_bytes,
                    if p.info.required {
                        " (the exe needs this folder beside it)"
                    } else {
                        ""
                    }
                ),
                None if p.info.required => println!(
                    "assets not copied (--no-data): the exe needs a `{}` folder beside it, --data <dir> or $IRON_DATA",
                    p.info.dir
                ),
                None => {}
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg() -> Value {
        json!({
            "name": "my-game", "version": "1.2.3", "author": "Ann", "description": "Pkg description",
            "iron": { "title": "My Game", "company": "Acme", "identifier": "com.acme.mygame", "icon": "assets/icon.png" }
        })
    }

    // M9-TS-2400
    #[test]
    fn metadata_defaults_come_from_package_json() {
        let m = resolve(&pkg(), "folder", &Overrides::default());
        assert_eq!(m.title, "My Game");
        assert_eq!(m.exe, "My Game");
        assert_eq!(m.company, "Acme");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.copyright, "Copyright (c) Acme");
        assert_eq!(m.description, "Pkg description");
        assert_eq!(m.identifier, "com.acme.mygame");
        assert_eq!(m.icon.as_deref(), Some("assets/icon.png"));
        // Nothing at all: the folder names it.
        let m = resolve(&json!({}), "cool", &Overrides::default());
        assert_eq!(
            (m.title.as_str(), m.exe.as_str(), m.version.as_str()),
            ("cool", "cool", "0.0.0")
        );
        assert_eq!(m.copyright, "");
        assert_eq!(m.description, "cool");
        assert_eq!(m.identifier, "dev.iron.cool");
    }

    // M9-TS-2401
    #[test]
    fn iron_fields_and_flags_override_in_order() {
        let mut p = pkg();
        p["iron"]["version"] = json!("2.0.0");
        p["iron"]["exe"] = json!("mygame.exe");
        p["iron"]["copyright"] = json!("(c) 2026 Acme Inc.");
        let m = resolve(&p, "f", &Overrides::default());
        assert_eq!((m.version.as_str(), m.exe.as_str()), ("2.0.0", "mygame"));
        assert_eq!(m.copyright, "(c) 2026 Acme Inc.");
        let ov = Overrides {
            version: Some("3.1".into()),
            exe: Some("bad/na:me".into()),
            icon: Some("x.ico".into()),
            description: Some("d".into()),
            ..Default::default()
        };
        let m = resolve(&p, "f", &ov);
        assert_eq!((m.version.as_str(), m.exe.as_str()), ("3.1", "bad_na_me"));
        assert_eq!(
            (m.icon.as_deref(), m.description.as_str()),
            (Some("x.ico"), "d")
        );
    }

    // M9-TS-2402
    #[test]
    fn the_payload_package_json_carries_the_resolved_metadata() {
        let m = resolve(
            &pkg(),
            "f",
            &Overrides {
                version: Some("9.9.9".into()),
                ..Default::default()
            },
        );
        let data = DataInfo {
            dir: "data".into(),
            required: true,
        };
        let j = payload_package_json(&pkg(), &m, Some("iron-icon.png"), &data);
        assert_eq!(j["version"], "9.9.9");
        assert_eq!(j["iron"]["icon"], "iron-icon.png");
        assert_eq!(j["iron"]["title"], "My Game");
        assert_eq!(j["iron"]["dataDir"], "data");
        assert_eq!(j["iron"]["dataRequired"], true);
        assert_eq!(j["name"], "my-game");
        assert!(
            payload_package_json(&pkg(), &m, None, &data)["iron"]
                .get("icon")
                .is_none()
        );
    }

    fn game_folder(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_pkg_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (p, c) in [
            (
                "dist/game.irbundle",
                r#"{"format":1,"assets":["assets/a.png","scenes/s.scene.json","../evil"]}"#,
            ),
            ("dist/game.css", "body{}"),
            ("dist/font-1.ttf", "FONT"),
            ("dist/assets/a.png", "COPY"),
            ("dist/game.o", "OBJ"),
            ("dist/old.exe", "EXE"),
            ("dist/.iron-reload", "{}"),
            ("assets/a.png", "PNG"),
            ("assets/a.png.meta", "META"),
            ("assets/unused.png", "UNUSED"),
            ("scenes/s.scene.json", "{}"),
            ("extra/x.txt", "X"),
            ("extra/sub/y.txt", "Y"),
        ] {
            let f = d.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, c).unwrap();
        }
        d
    }

    // M9-TS-2410
    #[test]
    fn collect_takes_dist_the_named_assets_and_includes_but_not_the_rest() {
        let d = game_folder("collect");
        // The unsafe `../evil` in the asset list is refused.
        let mut p = pkg();
        let m = resolve(&p, "f", &Overrides::default());
        assert!(
            collect_files(&d, &p, &m, None, "data", &[])
                .unwrap_err()
                .contains("unsafe")
        );
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png","scenes/s.scene.json"]}"#,
        )
        .unwrap();
        p["iron"]["include"] = json!(["extra"]);
        let c = collect_files(&d, &p, &m, Some(("iron-icon.png", vec![1])), "data", &[]).unwrap();
        let data: Vec<&str> = c.data.iter().map(|f| f.rel.as_str()).collect();
        for want in [
            "dist/font-1.ttf",
            "assets/a.png",
            "assets/a.png.meta",
            "scenes/s.scene.json",
            "extra/x.txt",
            "extra/sub/y.txt",
        ] {
            assert!(data.contains(&want), "{want} missing from {data:?}");
        }
        for not in [
            "dist/game.irbundle",
            "dist/game.css",
            "dist/game.o",
            "dist/old.exe",
            "dist/.iron-reload",
            "dist/assets/a.png",
            "assets/unused.png",
        ] {
            assert!(!data.contains(&not), "{not} should be left out");
        }
        // The game folder's copy of an asset is the one copied.
        let a = c.data.iter().find(|f| f.rel == "assets/a.png").unwrap();
        assert_eq!(std::fs::read(&a.src).unwrap(), b"PNG");
        // Without `iron.include` the whole assets folder goes to data.
        p["iron"].as_object_mut().unwrap().remove("include");
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        assert!(c.data.iter().any(|f| f.rel == "assets/unused.png"));
        assert!(!c.data.iter().any(|f| f.rel == "extra/x.txt"));
        // Anything under `exclude` (the output's own data folder) is not collected.
        let c = collect_files(&d, &p, &m, None, "data", &[d.join("dist")]).unwrap();
        assert!(!c.data.iter().any(|f| f.rel.starts_with("dist/")));
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2600
    #[test]
    fn the_payload_holds_only_code_and_tiny_essentials() {
        let d = game_folder("payload");
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png","scenes/s.scene.json"]}"#,
        )
        .unwrap();
        let p = pkg();
        let m = resolve(&p, "f", &Overrides::default());
        let c = collect_files(&d, &p, &m, Some(("iron-icon.png", vec![1])), "data", &[]).unwrap();
        let names: Vec<&str> = c.payload.iter().map(|(n, _)| n.as_str()).collect();
        // Small named assets are essentials: embedded, with the code.
        for want in [
            "dist/game.irbundle",
            "dist/game.css",
            "package.json",
            "iron-icon.png",
            "assets/a.png",
            "scenes/s.scene.json",
        ] {
            assert!(names.contains(&want), "{want} missing from {names:?}");
        }
        // Not the other assets, their `.meta`, the fonts or anything else.
        for not in ["assets/a.png.meta", "assets/unused.png", "dist/font-1.ttf"] {
            assert!(!names.contains(&not), "{not} must not be embedded");
        }
        // The unused asset and the font still need the data folder.
        assert!(c.info.required);
        let j: Value = serde_json::from_slice(
            &c.payload
                .iter()
                .find(|(n, _)| n == "package.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(j["iron"]["dataDir"], "data");
        assert_eq!(j["iron"]["dataRequired"], true);

        // Exactly at the limit (the scene is 2 bytes): embedded. One byte over: nothing is.
        std::fs::write(d.join("scenes/s.scene.json"), b"{}").unwrap();
        std::fs::write(
            d.join("assets/a.png"),
            vec![7u8; ESSENTIALS_LIMIT as usize - 2],
        )
        .unwrap();
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        assert!(c.payload.iter().any(|(n, _)| n == "assets/a.png"));
        std::fs::write(
            d.join("assets/a.png"),
            vec![7u8; ESSENTIALS_LIMIT as usize - 1],
        )
        .unwrap();
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        let names: Vec<&str> = c.payload.iter().map(|(n, _)| n.as_str()).collect();
        assert!(!names.contains(&"assets/a.png") && !names.contains(&"scenes/s.scene.json"));
        assert_eq!(names.len(), 3, "{names:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2601
    #[test]
    fn a_game_whose_assets_are_all_essentials_starts_without_a_data_folder() {
        let d = std::env::temp_dir().join(format!("iron_pkg_small_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("dist")).unwrap();
        std::fs::write(d.join("dist/game.irbundle"), r#"{"format":1,"assets":[]}"#).unwrap();
        let p = json!({});
        let m = resolve(&p, "f", &Overrides::default());
        // No assets at all (hello): nothing to need.
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        assert!(c.data.is_empty() && !c.info.required);
        // Tiny named assets only (the `.meta` is not needed): complete.
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/t.png"), b"T").unwrap();
        std::fs::write(d.join("assets/t.png.meta"), b"M").unwrap();
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/t.png"]}"#,
        )
        .unwrap();
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        assert!(!c.info.required);
        assert!(c.payload.iter().any(|(n, _)| n == "assets/t.png"));
        // A file the bundle does not name (the whole folder is included): required.
        std::fs::write(d.join("assets/other.bin"), b"O").unwrap();
        let c = collect_files(&d, &p, &m, None, "data", &[]).unwrap();
        assert!(c.info.required);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2602
    #[test]
    fn the_data_folder_name_must_be_plain() {
        for ok in ["data", "assets-pack", "Game Data"] {
            assert!(valid_data_dir(ok), "{ok}");
        }
        for bad in ["", ".", "..", "a/b", "a\\b", "C:", "..\\x", ".hidden"] {
            assert!(!valid_data_dir(bad), "{bad}");
        }
    }

    // M9-TS-2411
    #[test]
    fn version_numbers_become_four_parts() {
        assert_eq!(pe::version_quad("1.2.3"), [1, 2, 3, 0]);
        assert_eq!(pe::version_quad("2.0.0-beta.1+x"), [2, 0, 0, 0]);
        assert_eq!(pe::version_quad("1.2.3.4"), [1, 2, 3, 4]);
        assert_eq!(pe::version_quad("junk"), [0, 0, 0, 0]);
    }

    fn png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(32, 32, image::Rgba([1, 2, 3, 255]));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    // M9-TS-2412
    #[test]
    fn package_writes_one_file_with_payload_resources_and_a_working_sign_hook() {
        let d = game_folder("package");
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png"]}"#,
        )
        .unwrap();
        std::fs::write(d.join("package.json"), pkg().to_string()).unwrap();
        std::fs::write(d.join("assets/icon.png"), png_bytes()).unwrap();
        // This test program stands in for the host (a PE file on Windows).
        let host = std::env::current_exe().unwrap();
        let out = d.join("out/my game.exe");
        let sign_marker = d.join("signed.txt");
        let opts = PackageOptions {
            host: Some(host),
            out: Some(out.clone()),
            no_build: true,
            overrides: Overrides {
                version: Some("4.5.6".into()),
                ..Default::default()
            },
            sign_cmd: Some(format!(
                "echo signed > \"{}\" & rem {{exe}}",
                sign_marker.display()
            )),
            ..Default::default()
        };
        let p = package(&d, &opts).unwrap();
        assert_eq!(p.path, out);
        assert!(p.files >= 5);
        let mut a = Archive::open(&out).unwrap().unwrap();
        let j: Value = serde_json::from_slice(&a.read("package.json").unwrap()).unwrap();
        assert_eq!(j["version"], "4.5.6");
        assert_eq!(j["iron"]["icon"], "iron-icon.png");
        assert_eq!(a.read("iron-icon.png").unwrap(), png_bytes());
        assert_eq!(a.read("assets/a.png").unwrap(), b"PNG");
        if cfg!(windows) {
            assert!(sign_marker.exists(), "the sign command did not run");
            let bytes = std::fs::read(&out).unwrap();
            let r = pe::read_resources(&bytes).unwrap();
            assert!(
                r.strings
                    .contains(&("ProductVersion".into(), "4.5.6".into()))
            );
            assert_eq!(r.icon_sizes.len(), 4);
        }
        // A second package on top of a packaged file replaces its payload.
        let again = package(
            &d,
            &PackageOptions {
                host: Some(out.clone()),
                out: Some(d.join("out/b.exe")),
                no_build: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(again.files, p.files);
        // Nothing of `out/data` (the first run's assets) is collected into the second.
        assert!(d.join("out/data/assets/a.png").is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    fn package_with(d: &Path, out: &str, data_dir: Option<&str>, no_data: bool) -> Packaged {
        package(
            d,
            &PackageOptions {
                host: Some(std::env::current_exe().unwrap()),
                out: Some(d.join(out)),
                no_build: true,
                data_dir: data_dir.map(str::to_owned),
                no_data,
                ..Default::default()
            },
        )
        .unwrap()
    }

    // M9-TS-2603
    #[test]
    fn assets_are_copied_to_a_data_folder_beside_the_exe_and_not_embedded() {
        let d = game_folder("data");
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png"]}"#,
        )
        .unwrap();
        std::fs::write(d.join("package.json"), pkg().to_string()).unwrap();
        std::fs::write(d.join("assets/icon.png"), png_bytes()).unwrap();
        // A big asset: not embedded, copied byte for byte.
        let big = vec![9u8; 300 * 1024];
        std::fs::write(d.join("assets/big.bin"), &big).unwrap();
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png","assets/big.bin"]}"#,
        )
        .unwrap();

        let p = package_with(&d, "out/g.exe", None, false);
        let data = d.join("out/data");
        assert_eq!(p.data_path.as_deref(), Some(data.as_path()));
        assert_eq!(std::fs::read(data.join("assets/big.bin")).unwrap(), big);
        for rel in [
            "assets/a.png",
            "assets/a.png.meta",
            "assets/unused.png",
            "dist/font-1.ttf",
        ] {
            assert!(data.join(rel).is_file(), "{rel} not in data/");
        }
        assert!(!data.join("dist/game.irbundle").exists());
        assert!(p.data_files >= 6);
        assert!(p.data_bytes >= big.len() as u64);
        // The exe's table of contents has the code and no asset beyond the tiny essentials.
        let a = Archive::open(&p.path).unwrap().unwrap();
        let names: Vec<&str> = a.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(names.contains(&"dist/game.irbundle") && names.contains(&"package.json"));
        for not in [
            "assets/big.bin",
            "assets/unused.png",
            "assets/a.png.meta",
            "dist/font-1.ttf",
        ] {
            assert!(!names.contains(&not), "{not} embedded in {names:?}");
        }
        assert!(p.info.required);

        // --data-dir renames the folder (and the exe is told).
        let p = package_with(&d, "out2/g.exe", Some("game-data"), false);
        assert!(d.join("out2/game-data/assets/big.bin").is_file());
        assert!(!d.join("out2/data").exists());
        let mut a = Archive::open(&p.path).unwrap().unwrap();
        let j: Value = serde_json::from_slice(&a.read("package.json").unwrap()).unwrap();
        assert_eq!(j["iron"]["dataDir"], "game-data");

        // --no-data writes no folder, but the exe still looks for it.
        let p = package_with(&d, "out3/g.exe", None, true);
        assert!(!d.join("out3/data").exists());
        assert_eq!(p.data_path, None);
        assert!(p.info.required);
        let err = package(
            &d,
            &PackageOptions {
                host: Some(std::env::current_exe().unwrap()),
                out: Some(d.join("out4/g.exe")),
                no_build: true,
                data_dir: Some("../x".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.contains("--data-dir"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2440: needs the engine host and a GPU, so it runs on request:
    // `cargo test -p iron_build -- --ignored standalone` (IRON_HOST or a built target/debug/iron_app).
    #[test]
    #[ignore = "needs iron_app built and a GPU"]
    fn standalone_exe_runs_from_another_folder_without_the_game_folder() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello");
        let d = std::env::temp_dir().join(format!("iron_pkg_run_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let game = d.join("game");
        for f in ["package.json", "tsconfig.json", "src/main.ts"] {
            let to = game.join(f);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(src.join(f), to).unwrap();
        }
        let opts = PackageOptions {
            no_tsc: true,
            out: Some(d.join("bin/hello.exe")),
            ..Default::default()
        };
        let p = package(&game, &opts).unwrap();
        // The game folder is gone; the program runs from yet another folder.
        std::fs::remove_dir_all(&game).unwrap();
        let cwd = d.join("elsewhere");
        std::fs::create_dir_all(&cwd).unwrap();
        let shot = cwd.join("out.png");
        let status = std::process::Command::new(&p.path)
            .args(["--screenshot", "out.png", "--frames", "5"])
            .current_dir(&cwd)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(std::fs::metadata(&shot).unwrap().len() > 100);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2610: `cargo test -p iron_build -- --ignored standalone_data`. A game with an
    // asset bigger than the essentials limit runs from `data/` beside the exe, with the game
    // folder gone, and says where it looked when `data/` is gone too.
    #[test]
    #[ignore = "needs iron_app built and a GPU"]
    fn standalone_data_folder_is_read_and_its_absence_is_a_clear_error() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/boxes");
        let d = std::env::temp_dir().join(format!("iron_pkg_data_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let game = d.join("game");
        for f in [
            "package.json",
            "tsconfig.json",
            "src/main.tsx",
            "assets/stripes.png",
        ] {
            let (from, to) = (src.join(f), game.join(f));
            if from.is_file() {
                std::fs::create_dir_all(to.parent().unwrap()).unwrap();
                std::fs::copy(from, to).unwrap();
            }
        }
        // Pushes the game past the 256 KiB essentials limit.
        std::fs::write(game.join("assets/padding.bin"), vec![0u8; 300 * 1024]).unwrap();
        let opts = PackageOptions {
            no_tsc: true,
            out: Some(d.join("bin/boxes.exe")),
            ..Default::default()
        };
        let p = package(&game, &opts).unwrap();
        assert!(!p.embedded.iter().any(|n| n.starts_with("assets/")));
        assert!(d.join("bin/data/assets/stripes.png").is_file());
        std::fs::remove_dir_all(&game).unwrap();
        let cwd = d.join("elsewhere");
        std::fs::create_dir_all(&cwd).unwrap();
        let run = |name: &str| {
            std::process::Command::new(&p.path)
                .args(["--screenshot", name, "--frames", "5"])
                .current_dir(&cwd)
                .output()
                .unwrap()
        };
        let ok = run("out.png");
        assert!(
            ok.status.success(),
            "{}",
            String::from_utf8_lossy(&ok.stderr)
        );
        assert!(std::fs::metadata(cwd.join("out.png")).unwrap().len() > 100);
        std::fs::remove_dir_all(d.join("bin/data")).unwrap();
        let bad = run("none.png");
        assert!(!bad.status.success());
        let err = String::from_utf8_lossy(&bad.stderr);
        assert!(err.contains("data folder was not found"), "{err}");
        assert!(!cwd.join("none.png").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
