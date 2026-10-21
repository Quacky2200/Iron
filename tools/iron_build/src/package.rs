//! `iron-build package <game>`: one executable for a game. A copy of the engine host
//! (`iron_app`) gets the game's icon and version info (Windows PE resources, `pe.rs`) and the
//! game's files as a payload at its end (`iron_core::payload`). See `docs/iron-build.md`.

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
pub fn payload_package_json(pkg: &Value, meta: &Metadata, icon_entry: Option<&str>) -> Value {
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

/// The files of the game, as payload entries (path relative to the game folder).
///
/// - `dist/`: `game.irbundle`, `game.css` and the files the CSS references. Not the AOT
///   products, the old `.exe` outputs or dot files; not the copies of assets (those come from the
///   game folder, next).
/// - Every asset the bundle names (`assets` in `game.irbundle`), the scene `iron.startScene`
///   and every path of `iron.include` (files or folders; default `["assets"]`), each with its `.meta`.
/// - `package.json`, rewritten (`payload_package_json`), and the icon as `iron-icon.<ext>`.
pub fn collect_files(
    game: &Path,
    pkg: &Value,
    meta: &Metadata,
    icon_bytes: Option<(&str, Vec<u8>)>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bundle_path = game.join("dist/game.irbundle");
    let bundle: Value = serde_json::from_str(
        &std::fs::read_to_string(&bundle_path).map_err(|e| io(&bundle_path, e))?,
    )
    .map_err(|e| format!("{}: {e}", bundle_path.display()))?;

    // What to take from the game folder itself.
    let mut wanted: Vec<String> = Vec::new();
    for a in bundle["assets"].as_array().into_iter().flatten() {
        wanted.extend(a.as_str().map(str::to_owned));
    }
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
        let skip = inner == "game.o"
            || inner == "game_bindings.rs"
            || inner.ends_with(".exe")
            || paths.iter().any(|p| p == inner);
        if !skip && !paths.contains(&rel) {
            paths.push(rel);
        }
    }

    let mut files = Vec::new();
    for rel in paths {
        let data = std::fs::read(game.join(&rel)).map_err(|e| io(&game.join(&rel), e))?;
        files.push((rel, data));
    }
    let icon_entry = icon_bytes.as_ref().map(|(name, _)| name.to_string());
    let json = payload_package_json(pkg, meta, icon_entry.as_deref());
    files.push((
        "package.json".into(),
        serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?,
    ));
    if let Some((name, data)) = icon_bytes {
        files.push((name.to_string(), data));
    }
    Ok(files)
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
}

#[derive(Debug)]
pub struct Packaged {
    pub path: PathBuf,
    pub size: u64,
    pub payload_size: u64,
    pub files: usize,
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

    let files = collect_files(
        game,
        &pkg,
        &meta,
        icon.as_ref().map(|(n, d)| (n.as_str(), d.clone())),
    )?;

    // The host without any payload of its own.
    let host_path = find_host(game, opts.host.as_deref())?;
    let host_len = payload::host_len(&host_path)?;
    let mut host = std::fs::read(&host_path).map_err(|e| io(&host_path, e))?;
    host.truncate(host_len as usize);
    let is_pe = host.starts_with(b"MZ");
    if is_pe {
        let ico = match &icon {
            Some((_, data)) => Some(icon::to_ico(data)?),
            None => None,
        };
        host = pe::set_resources(host, &meta, ico.as_deref())?;
    }
    let payload_size = payload::write_payload(&mut host, &files)?;

    let out = opts.out.clone().unwrap_or_else(|| {
        let name = if is_pe {
            meta.file_name()
        } else {
            meta.exe.clone()
        };
        game.join("dist").join(name)
    });
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
        meta,
        resources: is_pe,
    })
}

const USAGE: &str = "usage: iron-build package <game> [--host iron_app.exe] [--out file] [--no-build] [--no-tsc]
       [--exe name] [--title t] [--company c] [--version v] [--copyright c] [--description d] [--icon file.png|ico]
       [--sign-cmd \"signtool sign ... {exe}\"]";

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
                "{}: {} file(s), payload {} bytes, {} bytes total{}",
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
        let j = payload_package_json(&pkg(), &m, Some("iron-icon.png"));
        assert_eq!(j["version"], "9.9.9");
        assert_eq!(j["iron"]["icon"], "iron-icon.png");
        assert_eq!(j["iron"]["title"], "My Game");
        assert_eq!(j["name"], "my-game");
        assert!(
            payload_package_json(&pkg(), &m, None)["iron"]
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
            collect_files(&d, &p, &m, None)
                .unwrap_err()
                .contains("unsafe")
        );
        std::fs::write(
            d.join("dist/game.irbundle"),
            r#"{"format":1,"assets":["assets/a.png","scenes/s.scene.json"]}"#,
        )
        .unwrap();
        p["iron"]["include"] = json!(["extra"]);
        let files = collect_files(&d, &p, &m, Some(("iron-icon.png", vec![1]))).unwrap();
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        for want in [
            "dist/game.irbundle",
            "dist/game.css",
            "dist/font-1.ttf",
            "assets/a.png",
            "assets/a.png.meta",
            "scenes/s.scene.json",
            "extra/x.txt",
            "extra/sub/y.txt",
            "package.json",
            "iron-icon.png",
        ] {
            assert!(names.contains(&want), "{want} missing from {names:?}");
        }
        for not in [
            "dist/game.o",
            "dist/old.exe",
            "dist/.iron-reload",
            "dist/assets/a.png",
            "assets/unused.png",
        ] {
            assert!(!names.contains(&not), "{not} should be left out");
        }
        // The game folder's copy of an asset is the one packaged.
        let a = files.iter().find(|(n, _)| n == "assets/a.png").unwrap();
        assert_eq!(a.1, b"PNG");
        // Without `iron.include` the whole assets folder goes in.
        p["iron"].as_object_mut().unwrap().remove("include");
        let files = collect_files(&d, &p, &m, None).unwrap();
        assert!(files.iter().any(|(n, _)| n == "assets/unused.png"));
        assert!(!files.iter().any(|(n, _)| n == "extra/x.txt"));
        let _ = std::fs::remove_dir_all(&d);
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
        assert!(p.files >= 6);
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
}
