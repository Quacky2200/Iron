//! `iron-build`: builds a game written in TypeScript into `dist/` for `iron_app`, with no bun and no
//! `node_modules` (see `docs/iron-build.md`).
//!
//! A build reads `project.json` (else the `iron` section of `package.json`) and `tsconfig.json`, resolves the module graph from the
//! entry file (`iron_ts::modules`), runs `tsc --noEmit -p .` when `tsc` can be found, compiles the
//! bundle with all engine manifests (so errors are found now, not when the game starts), and writes
//! `dist/game.irbundle` (the resolved source, the CSS and asset lists), `dist/game.css`, and the assets
//! the program names.

pub mod assets;
pub mod css;
pub mod dev;
pub mod icon;
pub mod init;
pub mod package;
pub mod pe;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What `project.json` (or `package.json`) and `tsconfig.json` say about a game.
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub root: PathBuf,
    pub name: String,
    pub title: String,
    pub company: String,
    pub identifier: String,
    /// The entry file, relative to the game folder.
    pub entry: String,
    /// `tsconfig.json` `include` (default `["src"]`).
    pub include: Vec<String>,
    pub jsx: Option<String>,
    pub has_tsconfig: bool,
}

impl GameConfig {
    pub fn load(root: &Path) -> Result<GameConfig, String> {
        let user = iron_core::project::SystemUser;
        let project = iron_core::project::Project::load(
            root,
            &iron_core::project::LoadOptions::new(iron_core::project::Platform::current(), &user),
        )
        .map_err(|e| e.to_string())?;
        let name = project.name.clone();
        let ts_path = root.join("tsconfig.json");
        let ts: serde_json::Value = std::fs::read_to_string(&ts_path)
            .ok()
            .and_then(|t| serde_json::from_str(&strip_json_comments(&t)).ok())
            .unwrap_or(serde_json::Value::Null);
        let include = ts["include"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["src".to_string()]);
        let entry = match project.main.clone() {
            Some(m) => m,
            None => [
                "src/main.tsx",
                "src/main.ts",
                "src/index.tsx",
                "src/index.ts",
                "main.tsx",
                "main.ts",
            ]
            .iter()
            .find(|c| root.join(c).exists())
            .map(|c| c.to_string())
            .ok_or_else(|| {
                format!(
                    "{}: no entry file (src/main.tsx, src/main.ts, ...)",
                    root.display()
                )
            })?,
        };
        Ok(GameConfig {
            root: root.to_path_buf(),
            title: project.title,
            company: project.company,
            identifier: project.identifier,
            name,
            entry,
            include,
            jsx: ts["compilerOptions"]["jsx"].as_str().map(String::from),
            has_tsconfig: ts_path.exists(),
        })
    }
}

/// Removes `//` and `/* */` comments (outside strings) and trailing commas, so JSONC parses.
pub fn strip_json_comments(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            out.push(c);
            if c == '\\' && i + 1 < b.len() {
                out.push(b[i + 1]);
                i += 1;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        } else if c == '/' && b.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        } else {
            out.push(c);
        }
        i += 1;
    }
    // Trailing commas before } or ].
    let chars: Vec<char> = out.chars().collect();
    let mut res = String::new();
    let mut in_str = false;
    for (i, &c) in chars.iter().enumerate() {
        if c == '"' && (i == 0 || chars[i - 1] != '\\') {
            in_str = !in_str;
        }
        if !in_str && c == ',' {
            let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some('}') | Some(']')) {
                continue;
            }
        }
        res.push(c);
    }
    res
}

/// Every engine manifest the compiler checks calls against, plus the `Ui` namespace: the same lists
/// as `iron_native::manifests()` (that crate links QuickJS, which a build tool does not need).
pub fn manifests() -> Vec<String> {
    let mut all = iron_engine_api::manifests();
    all.push(iron_scene::api::scene_api::MANIFEST.to_json());
    all.extend(iron_scenes::manifests());
    all.extend(iron_physics_api::manifests());
    all.extend(iron_render_api::manifests());
    all.push(iron_ts_ui::manifest_json());
    all
}

/// The text of `types/iron.d.ts` (the shipped ambient and TypeScript-side declarations included).
pub fn types_dts() -> Result<String, String> {
    let ms: Vec<String> = manifests()
        .into_iter()
        .filter(|m| !m.contains("\"namespace\":\"Ui\""))
        .collect();
    iron_ts::types_dts::generate_default(&ms)
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Skip the `tsc` check.
    pub no_tsc: bool,
    /// Also write the AOT object `dist/game.o` (release builds).
    pub aot: bool,
    /// Run `iron <game> --cook` afterwards (needs the `iron` binary; see `find_iron`).
    pub cook: bool,
    /// Skip compiling the bundle (module graph, CSS and assets only).
    pub no_compile: bool,
}

/// How `tsc` went.
#[derive(Debug, Clone, PartialEq)]
pub enum Tsc {
    Skipped,
    Missing,
    Ok,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Report {
    pub config: GameConfig,
    pub files: Vec<String>,
    pub css_files: Vec<String>,
    pub assets: Vec<String>,
    pub tsc: Tsc,
    pub warnings: Vec<String>,
    pub object_bytes: usize,
    pub time: Duration,
}

fn find_tsc(root: &Path) -> Option<PathBuf> {
    let mut dirs = vec![root.to_path_buf()];
    let mut p = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    while p.pop() {
        dirs.push(p.clone());
    }
    // The checkout this tool was built from (a game outside it still finds its TypeScript).
    let mut own = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while own.pop() {
        dirs.push(own.clone());
    }
    let names: &[&str] = if cfg!(windows) {
        &["tsc.cmd", "tsc.exe"]
    } else {
        &["tsc"]
    };
    for d in dirs {
        for n in names {
            let c = d.join("node_modules/.bin").join(n);
            if c.exists() {
                return Some(c);
            }
        }
    }
    for n in names {
        if let Ok(o) = std::process::Command::new(n).arg("--version").output()
            && o.status.success()
        {
            return Some(PathBuf::from(n));
        }
    }
    None
}

/// The `tsconfig.json` text `tsc` is run with: the game's own file extended, its `include` kept (as
/// paths from `dist/.tsc/`), plus the generated declarations (`iron.d.ts` next to it). Every file is a
/// module (`moduleDetection: force`, so top-level names of a game never merge with the engine's
/// globals), and JSX is only checked (the game needs no `react/jsx-runtime`).
pub fn tsc_config(config: &GameConfig) -> String {
    let include: Vec<String> = config
        .include
        .iter()
        .map(|i| format!("../../{}", i.trim_start_matches("./")))
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "extends": "../../tsconfig.json",
        "compilerOptions": {
            "noEmit": true,
            "types": [],
            "jsx": "preserve",
            "moduleDetection": "force",
        },
        "include": include,
        "files": ["iron.d.ts"],
        "exclude": ["../../node_modules"],
    }))
    .unwrap_or_default()
}

/// Writes `dist/.tsc/{tsconfig.json,iron.d.ts}` and returns the config path.
fn write_tsc_files(root: &Path, config: &GameConfig) -> Result<PathBuf, String> {
    let dir = root.join("dist/.tsc");
    let dts = types_dts()?;
    write_atomic(&dir.join("iron.d.ts"), dts.as_bytes()).map_err(|e| e.to_string())?;
    let cfg = dir.join("tsconfig.json");
    write_atomic(&cfg, tsc_config(config).as_bytes()).map_err(|e| e.to_string())?;
    Ok(cfg)
}

fn run_tsc(root: &Path, config: &GameConfig) -> Tsc {
    let Some(tsc) = find_tsc(root) else {
        return Tsc::Missing;
    };
    let cfg = match write_tsc_files(root, config).map(|_| "dist/.tsc/tsconfig.json") {
        Ok(c) => c,
        Err(e) => return Tsc::Failed(e),
    };
    match std::process::Command::new(&tsc)
        .arg("--noEmit")
        .arg("-p")
        .arg(cfg)
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => Tsc::Ok,
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            Tsc::Failed(text.trim().to_string())
        }
        Err(e) => Tsc::Failed(format!("cannot run {}: {e}", tsc.display())),
    }
}

/// String literals of the bundled source that name an existing file under the game folder.
fn referenced_assets(source: &str, root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let b = source.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let q = b[i];
        if q == b'"' || q == b'\'' || q == b'`' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != q && b[j] != b'\n' {
                if b[j] == b'\\' {
                    j += 1;
                }
                j += 1;
            }
            if j < b.len() && b[j] == q {
                let lit = &source[start..j.min(source.len())];
                let ext_ok = lit.rsplit_once('.').is_some_and(|(_, e)| {
                    assets::kind(&e.to_lowercase()).is_some() || lit.ends_with(".scene.json")
                });
                if ext_ok
                    && !lit.contains(['{', '$'])
                    && let Some(p) = iron_core::resolve_asset(root, lit)
                    && p.is_file()
                {
                    let rel = lit.trim_start_matches("./").to_string();
                    if !found.contains(&rel) {
                        found.push(rel);
                    }
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    found.sort();
    found
}

/// Writes `path` by a temporary file and a rename, so a watcher never reads half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Builds the game in `root` into `root/dist`.
pub fn build(root: &Path, opts: &Options) -> Result<Report, String> {
    let t0 = Instant::now();
    let config = GameConfig::load(root)?;
    let dist = root.join("dist");
    let entry = root.join(&config.entry);
    let bundle = iron_ts::modules::bundle(&entry)?;
    let mut warnings = Vec::new();

    let tsc = if opts.no_tsc || !config.has_tsconfig {
        Tsc::Skipped
    } else {
        run_tsc(root, &config)
    };
    if let Tsc::Failed(text) = &tsc {
        return Err(format!("tsc failed:\n{text}"));
    }

    let mut object_bytes = 0;
    if !opts.no_compile {
        let out =
            iron_ts::compile_with_manifests(&bundle.source, "speed", "game_", 0, &manifests())
                .map_err(|e| format!("{}: {}", config.name, bundle.locate_in(&e)))?;
        object_bytes = out.object.len();
        if opts.aot {
            write_atomic(&dist.join("game.o"), &out.object).map_err(|e| e.to_string())?;
            write_atomic(&dist.join("game_bindings.rs"), out.bindings.as_bytes())
                .map_err(|e| e.to_string())?;
        }
    }

    // Stylesheets, in import order.
    let css_paths: Vec<PathBuf> = bundle.css.iter().map(PathBuf::from).collect();
    let mut css_name = None;
    let mut copied = Vec::new();
    if !css_paths.is_empty() {
        let out = css::bundle(&css_paths, &dist).map_err(|e| e.to_string())?;
        warnings.extend(out.warnings);
        copied = out.copied;
        write_atomic(&dist.join("game.css"), out.css.as_bytes()).map_err(|e| e.to_string())?;
        css_name = Some("game.css".to_string());
    } else {
        let _ = std::fs::remove_file(dist.join("game.css"));
    }

    // Assets: .meta files, then what the program names, copied into dist/ with the same relative path.
    assets::update(root, false).map_err(|e| e.to_string())?;
    let names = referenced_assets(&bundle.source, root);
    for a in &names {
        for src_rel in [a.clone(), format!("{a}.meta")] {
            let src = root.join(&src_rel);
            if src.is_file() {
                let to = dist.join(&src_rel);
                if let Some(d) = to.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                std::fs::copy(&src, &to).map_err(|e| format!("{}: {e}", src.display()))?;
            }
        }
    }

    let irbundle = serde_json::json!({
        "format": 1,
        "title": config.title,
        "company": config.company,
        "identifier": config.identifier,
        "entry": config.entry,
        "source": bundle.source,
        "files": bundle.files,
        "css": css_name,
        "assets": names,
    });
    let text = serde_json::to_string_pretty(&irbundle).map_err(|e| e.to_string())?;
    write_atomic(&dist.join("game.irbundle"), text.as_bytes()).map_err(|e| e.to_string())?;

    if opts.cook {
        match find_iron(root) {
            Some(iron) => {
                let st = std::process::Command::new(&iron)
                    .arg(root)
                    .arg("--cook")
                    .status();
                if !matches!(st, Ok(s) if s.success()) {
                    warnings.push(format!("{} --cook failed", iron.display()));
                }
            }
            None => warnings
                .push("--cook: the `iron` binary was not found (cargo build -p iron_app)".into()),
        }
    }
    let _ = copied;
    Ok(Report {
        config,
        files: bundle.files,
        css_files: bundle.css,
        assets: names,
        tsc,
        warnings,
        object_bytes,
        time: t0.elapsed(),
    })
}

/// The engine binary, for `--cook`: next to the repo's `target/`, or on the PATH.
pub fn find_iron(root: &Path) -> Option<PathBuf> {
    let mut p = root.canonicalize().ok()?;
    while p.pop() {
        for rel in [
            "target/release/iron_app.exe",
            "target/debug/iron_app.exe",
            "target/release/iron_app",
            "target/debug/iron_app",
        ] {
            let c = p.join(rel);
            if c.exists() {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("iron_build_{name}_{}", assets::new_guid()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // M9-TS-1850
    #[test]
    fn jsonc_comments_and_trailing_commas_are_stripped() {
        let t = strip_json_comments("{ // c\n \"a\": \"x//y\", /* b */ \"b\": [1, 2,], }");
        let v: serde_json::Value = serde_json::from_str(&t).unwrap();
        assert_eq!(v["a"], "x//y");
        assert_eq!(v["b"][1], 2);
    }

    // M9-TS-1851
    #[test]
    fn config_reads_the_iron_section_and_tsconfig() {
        let d = temp("cfg");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/main.ts"), "export const a = 1;").unwrap();
        std::fs::write(
            d.join("package.json"),
            r#"{"name":"g","iron":{"title":"T","company":"C","identifier":"dev.x.g"}}"#,
        )
        .unwrap();
        std::fs::write(d.join("tsconfig.json"), "{\n // x\n \"compilerOptions\": {\"jsx\": \"react-jsx\"}, \"include\": [\"src\", \"types\"] }").unwrap();
        let c = GameConfig::load(&d).unwrap();
        assert_eq!(
            (c.title.as_str(), c.identifier.as_str(), c.entry.as_str()),
            ("T", "dev.x.g", "src/main.ts")
        );
        assert_eq!(c.include, ["src", "types"]);
        assert_eq!(c.jsx.as_deref(), Some("react-jsx"));
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2720
    #[test]
    fn config_reads_project_json_before_the_iron_section() {
        let d = temp("projectjson");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/app.ts"), "").unwrap();
        std::fs::write(
            d.join("package.json"),
            r#"{"name":"g","iron":{"title":"FROM PACKAGE","main":"src/other.ts"}}"#,
        )
        .unwrap();
        std::fs::write(
            d.join("project.json"),
            r#"{"title":"T ${env:IRON_NO_SUCH_VAR_2720:-ok}","identifier":"dev.x.g","main":"src/app.ts"}"#,
        )
        .unwrap();
        let c = GameConfig::load(&d).unwrap();
        assert_eq!(
            (c.title.as_str(), c.identifier.as_str(), c.entry.as_str()),
            ("T ok", "dev.x.g", "src/app.ts")
        );
        // A typo in a template stops the build and names the file and line.
        std::fs::write(d.join("project.json"), "{\n  \"title\": \"${titel}\"\n}").unwrap();
        let e = GameConfig::load(&d).unwrap_err();
        assert!(
            e.contains("project.json:2:") && e.contains("unknown variable"),
            "{e}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-1852
    #[test]
    fn a_build_writes_the_bundle_css_and_the_assets_the_code_names() {
        let d = temp("build");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::create_dir_all(d.join("assets")).unwrap();
        std::fs::write(d.join("assets/hit.wav"), b"RIFF").unwrap();
        std::fs::write(d.join("assets/unused.png"), b"x").unwrap();
        std::fs::write(d.join("package.json"), r#"{"name":"g"}"#).unwrap();
        std::fs::write(d.join("src/ui.css"), ".a { color: red }").unwrap();
        std::fs::write(
            d.join("src/util.ts"),
            "export function twice(n: number): number { return n * 2; }",
        )
        .unwrap();
        std::fs::write(
            d.join("src/main.ts"),
            "import \"./ui.css\";\nimport { twice } from \"./util\";\nexport function f(): number { const s = \"assets/hit.wav\"; return twice(s.length); }",
        )
        .unwrap();
        let r = build(
            &d,
            &Options {
                no_tsc: true,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(r.assets, ["assets/hit.wav"]);
        assert!(
            d.join("dist/assets/hit.wav").exists() && !d.join("dist/assets/unused.png").exists()
        );
        assert!(d.join("dist/assets/hit.wav.meta").exists());
        assert_eq!(
            std::fs::read_to_string(d.join("dist/game.css"))
                .unwrap()
                .trim(),
            ".a { color: red }"
        );
        let b: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.join("dist/game.irbundle")).unwrap())
                .unwrap();
        assert_eq!(b["format"], 1);
        assert_eq!(b["css"], "game.css");
        assert!(b["source"].as_str().unwrap().contains("function twice"));
        assert!(r.object_bytes > 0);
        // A type error stops the build.
        std::fs::write(
            d.join("src/util.ts"),
            "export function twice(n: number): number { return n + undefinedName; }",
        )
        .unwrap();
        assert!(
            build(
                &d,
                &Options {
                    no_tsc: true,
                    ..Options::default()
                }
            )
            .is_err()
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // ---- types/iron.d.ts as globals (M9-TS-2300..) ----------------------------------------------

    fn globals_game(name: &str, main: &str) -> PathBuf {
        let d = temp(name);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("package.json"), r#"{"name":"g"}"#).unwrap();
        std::fs::write(
            d.join("tsconfig.json"),
            r#"{"compilerOptions":{"target":"ES2022","lib":["ES2022"],"types":[],"module":"ESNext","moduleResolution":"bundler","strict":true,"noImplicitOverride":true,"verbatimModuleSyntax":true,"jsx":"react-jsx"},"include":["src"]}"#,
        )
        .unwrap();
        std::fs::write(d.join("src/main.tsx"), main).unwrap();
        d
    }

    const NO_IMPORTS: &str = "class Hello extends Component {\n  override onStart() {\n    const v = new Vector3(1, 2, 3);\n    console.log(v.x + Time.deltaTime);\n  }\n}\nregister({ Hello });\n";
    const WITH_IMPORTS: &str = "import { Component, register, Vector3, Time } from \"@iron/engine\";\n\nclass Hello extends Component {\n  override onStart() {\n    const v = new Vector3(1, 2, 3);\n    console.log(v.x + Time.deltaTime);\n  }\n}\nregister({ Hello });\n";

    fn no_tsc() -> Options {
        Options {
            no_tsc: true,
            ..Options::default()
        }
    }

    // M9-TS-2300
    #[test]
    fn the_declarations_are_deterministic_and_name_the_engine_as_globals() {
        let a = types_dts().unwrap();
        assert_eq!(a, types_dts().unwrap());
        for want in [
            "\ndeclare class Vector3 {",
            "\ndeclare class Quaternion {",
            "\ndeclare class Color {",
            "\ndeclare class GameObject {",
            "\ndeclare abstract class Component {",
            "\ndeclare function register(",
            "\ndeclare namespace Physics {",
            "\ndeclare namespace Time {",
            "\ndeclare namespace input {",
            "\ndeclare class Rigidbody {",
            "\ndeclare class BoxCollider {",
            "\ndeclare class Camera {",
            "\ndeclare class Light {",
            "\ndeclare namespace Screen {",
            "\ndeclare namespace QualitySettings {",
            "\ndeclare namespace PlayerPrefs {",
            "\ndeclare class Prefab {",
            "\ndeclare function useState<",
            "\ndeclare function useEffect(",
            "\ndeclare function createRoot(",
            "\ndeclare function Text(",
            "\ndeclare function Dialog(",
            "\ntype ReactNode =",
            "\ninterface IronStyle {",
            "\ninterface IronKeyboardEvent ",
            "\ntype FullScreenMode =",
            "\ninterface Gamepad {",
            "\ninterface Collision {",
            "\ntype Collision2D =",
            "\ndeclare module \"@iron/engine\" {",
            "  export import Vector3 = globalThis.Vector3;",
            "\ndeclare module \"@iron/react\" {",
            "  export import createRoot = globalThis.createRoot;",
            "\ndeclare module \"react\" {",
            "  export import useState = globalThis.useState;",
        ] {
            assert!(a.contains(want), "missing {want:?}");
        }
        // Nothing is written as `export` outside the three modules.
        assert!(!a.contains("\nexport "), "a top-level export");
    }

    // M9-TS-2301
    #[test]
    fn calls_have_an_options_object_form_and_creatable_classes_a_constructor() {
        let a = types_dts().unwrap();
        // `audio.load(path, { stream: true })` and `new GameObject(name, { position })`.
        assert!(a.contains("function load(path: string, options?: { stream?: boolean }): Sound;"));
        assert!(a.contains("  constructor(name?: string, options?: GameObjectOptions);"));
        assert!(a.contains("  constructor(color?: Color, emission?: Color, smoothness?: number"));
        assert!(a.contains("  static create(options: { "));
        // A required parameter never follows an optional one.
        assert!(!a.contains("?: number, options: {"));
    }

    // M9-TS-2806
    #[test]
    fn iron_build_types_declare_window_and_window_options() {
        let a = types_dts().unwrap();
        for want in [
            "\ndeclare class Window {",
            "\ninterface WindowOptions {",
            "\ntype WindowStyle =",
            "  static readonly main: Window;",
            "  static readonly current: Window;",
            "  static readonly count: number;",
            "  const focused: Window | undefined;",
            "  function create(options?: WindowOptions): Window;",
            "  style: WindowStyle;",
            "  readonly parent: Window | undefined;",
            "declare function createRoot(target?: WorldPanel | Window | number)",
        ] {
            assert!(a.contains(want), "missing {want:?}");
        }
        // The manifest's untyped `create` and `focused` are replaced by the supplement's.
        assert!(!a.contains("static create(title?: string"));
        assert!(!a.contains("static readonly focused: Window;"));
    }

    // M9-TS-2807
    #[test]
    fn tsc_checks_a_program_that_opens_windows() {
        let main = "Window.main.title = \"Game\";\nconst w = Window.create({ title: \"P\", style: \"tool\", parent: Window.main, width: 200 });\nconst f: Window | undefined = Window.focused;\nif (w.parent) console.log(w.parent.title, w.isChild, f === undefined, Window.count, Window.at(0)?.handle);\nw.onResized((width, height) => console.log(width + height));\nw.onCloseRequested(() => w.cancelClose());\nw.style = \"dialog\";\ncreateRoot(w).render(<box><text>{w.title}</text></box>);\ncreateRoot(Window.main).render(<box />);\nw.postMessage({ kind: \"select\", ids: [1, 2] });\nw.onMessage<{ kind: string }>((from, m) => console.log(from.title + m.kind));\nw.onRequest<number, string>((from, n, reply) => reply(\"x\" + n));\nWindow.onMessage<{ n: number }>((from, m) => console.log(m.n));\nWindow.broadcast({ n: 1 });\nconst reply: Promise<{ n: number }> = w.request<{ n: number }>({ kind: \"stats\" }, 500);\n";
        let d = globals_game("tscwin", main);
        let Some(_) = find_tsc(&d) else {
            eprintln!("M9-TS-2807 skipped: tsc not found (install TypeScript)");
            let _ = std::fs::remove_dir_all(&d);
            return;
        };
        let ok = |d: &Path| {
            build(
                d,
                &Options {
                    no_compile: true,
                    ..Options::default()
                },
            )
        };
        assert_eq!(ok(&d).unwrap().tsc, Tsc::Ok);
        // The option types are real.
        std::fs::write(
            d.join("src/main.tsx"),
            "Window.create({ style: \"floating\" });\n",
        )
        .unwrap();
        assert!(ok(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2302
    #[test]
    fn a_program_without_imports_compiles() {
        let d = globals_game("noimp", NO_IMPORTS);
        let r = build(&d, &no_tsc()).unwrap();
        assert!(r.object_bytes > 0 && r.files.len() == 1 && r.files[0].ends_with("src/main.tsx"));
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2303
    #[test]
    fn the_old_import_forms_still_compile() {
        let d = globals_game("oldimp", WITH_IMPORTS);
        let r = build(&d, &no_tsc()).unwrap();
        assert!(r.object_bytes > 0);
        let src = std::fs::read_to_string(d.join("dist/game.irbundle")).unwrap();
        assert!(!src.contains("@iron/engine\\\""), "the import was kept");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2304
    #[test]
    fn the_tsc_config_extends_the_games_and_adds_the_declarations() {
        let d = globals_game("cfg2", NO_IMPORTS);
        let c = GameConfig::load(&d).unwrap();
        let v: serde_json::Value = serde_json::from_str(&tsc_config(&c)).unwrap();
        assert_eq!(v["extends"], "../../tsconfig.json");
        assert_eq!(v["include"][0], "../../src");
        assert_eq!(v["files"][0], "iron.d.ts");
        assert_eq!(v["compilerOptions"]["moduleDetection"], "force");
        assert_eq!(v["compilerOptions"]["jsx"], "preserve");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2305
    #[test]
    fn tsc_checks_a_program_without_imports_and_the_old_forms() {
        let d = globals_game("tscnoimp", NO_IMPORTS);
        let Some(_) = find_tsc(&d) else {
            eprintln!("M9-TS-2305 skipped: tsc not found (install TypeScript)");
            let _ = std::fs::remove_dir_all(&d);
            return;
        };
        let ok = |d: &Path| {
            build(
                d,
                &Options {
                    no_compile: true,
                    ..Options::default()
                },
            )
        };
        assert_eq!(ok(&d).unwrap().tsc, Tsc::Ok);
        // The declarations are real: a wrong call fails.
        std::fs::write(
            d.join("src/main.tsx"),
            "class Hello extends Component {\n  override onStart() {\n    const v = new Vector3(1, 2);\n    const n: number = \"x\";\n  }\n}\n",
        )
        .unwrap();
        let Err(e) = ok(&d) else {
            panic!("a wrong call type-checked")
        };
        assert!(e.contains("tsc failed") && e.contains("TS2554"), "{e}");
        // The old imports.
        std::fs::write(d.join("src/main.tsx"), WITH_IMPORTS).unwrap();
        assert_eq!(ok(&d).unwrap().tsc, Tsc::Ok);
        std::fs::write(
            d.join("src/main.tsx"),
            "import { useState } from \"react\";\nimport { createRoot, Text, type IronStyle } from \"@iron/react\";\nimport { type Gamepad, Color } from \"@iron/engine\";\nconst style: IronStyle = { color: \"red\" };\nfunction App() {\n  const [n, setN] = useState(0);\n  const g: Gamepad | null = null;\n  return <box style={style} onClick={() => setN(n + 1)}><Text>{n}{String(g)}{Color.white.r}</Text></box>;\n}\ncreateRoot().render(<App />);\n",
        )
        .unwrap();
        assert_eq!(ok(&d).unwrap().tsc, Tsc::Ok);
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2306
    #[test]
    fn the_generated_declarations_have_no_errors_of_their_own() {
        let d = globals_game("tscdts", NO_IMPORTS);
        let Some(tsc) = find_tsc(&d) else {
            eprintln!("M9-TS-2306 skipped: tsc not found (install TypeScript)");
            let _ = std::fs::remove_dir_all(&d);
            return;
        };
        std::fs::write(d.join("iron.d.ts"), types_dts().unwrap()).unwrap();
        // skipLibCheck off: declaration errors are reported too.
        let o = std::process::Command::new(tsc)
            .args([
                "--noEmit",
                "--strict",
                "--skipLibCheck",
                "false",
                "--lib",
                "ES2022",
                "--target",
                "ES2022",
                "iron.d.ts",
            ])
            .current_dir(&d)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&o.stdout);
        assert!(o.status.success(), "{text}");
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-TS-2721
    #[test]
    fn every_example_has_a_project_json_and_package_json_has_no_iron_section() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let user = iron_core::project::FixedUser {
            username: "tester".into(),
            full_name: "Test Person".into(),
            ..Default::default()
        };
        let mut games = 0;
        for e in std::fs::read_dir(&examples).unwrap().flatten() {
            let p = e.path();
            if !p.join("package.json").exists() {
                continue;
            }
            games += 1;
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            let pkg: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(p.join("package.json")).unwrap())
                    .unwrap();
            assert!(
                pkg.get("iron").is_none(),
                "{name}: package.json still has `iron`"
            );
            let proj = iron_core::project::Project::load(
                &p,
                &iron_core::project::LoadOptions::new(iron_core::project::Platform::Windows, &user),
            )
            .unwrap();
            assert_eq!(
                proj.source,
                iron_core::project::Source::ProjectJson,
                "{name}"
            );
            assert_eq!(proj.identifier, format!("dev.iron.{name}"));
            assert_eq!(proj.company, "Test's Company");
            assert_eq!(proj.license, "private");
            assert!(!proj.title.is_empty());
        }
        assert_eq!(games, 20);
    }

    // M9-TS-2307
    #[test]
    fn all_the_examples_type_check_against_the_generated_declarations() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let Some(_) = find_tsc(&examples) else {
            eprintln!("M9-TS-2307 skipped: tsc not found (install TypeScript)");
            return;
        };
        let mut games = 0;
        for e in std::fs::read_dir(&examples).unwrap().flatten() {
            let p = e.path();
            if !p.join("package.json").exists() {
                continue;
            }
            games += 1;
            let r = build(
                &p,
                &Options {
                    no_compile: true,
                    ..Options::default()
                },
            );
            assert!(
                matches!(&r, Ok(r) if r.tsc == Tsc::Ok),
                "{}: {:?}",
                p.display(),
                r.map(|r| r.tsc)
            );
        }
        assert!(games >= 19, "{games} examples");
    }

    // M9-TS-2308
    #[test]
    fn the_examples_import_nothing_from_the_engine() {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        for e in std::fs::read_dir(&examples).unwrap().flatten() {
            let src = e.path().join("src");
            let Ok(rd) = std::fs::read_dir(&src) else {
                continue;
            };
            for f in rd.flatten() {
                let Ok(t) = std::fs::read_to_string(f.path()) else {
                    continue;
                };
                for bad in ["\"@iron/engine\"", "\"@iron/react\"", "from \"react\""] {
                    assert!(!t.contains(bad), "{}: {bad}", f.path().display());
                }
            }
        }
    }
}
