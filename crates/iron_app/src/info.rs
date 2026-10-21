//! What scripts learn about the game and the machine: the game's manifest (its
//! `project.json`) for `Application`, and the OS, hardware, GPU and screen for
//! `SystemInfo`. It reaches scripts as `globalThis.__iron_info` before the bundle runs.

use std::path::Path;

use iron_core::project::{LoadOptions, Platform, Project, SystemUser};
use serde_json::{Map, Value, json};

/// The game's details, from `project.json` in its folder (else the `iron` section of
/// `package.json`): `name`, `title`, `version`, `company`, `identifier` and `icon` (a PNG path).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Manifest {
    pub name: String,
    /// Shown in the window's title bar; the name if not given.
    pub title: String,
    pub version: String,
    pub company: String,
    /// Reverse-DNS id (`com.example.game`).
    pub identifier: String,
    /// The window and taskbar icon, a path in the game folder.
    pub icon: Option<String>,
    /// `iron.quality`: the default quality level and per-level overrides, passed to
    /// scripts' `QualitySettings` as is.
    pub quality: Value,
    /// `iron.layers`: layer names by number, passed to scripts' `LayerMask` as is.
    pub layers: Value,
    /// `iron.startScene`: a scene file scripts load at startup.
    pub start_scene: Option<String>,
    /// `iron.display`: the window's default mode and size (`{ mode, width, height }`);
    /// `resizable: false` keeps the window at that size.
    pub display: Value,
    /// `iron.runInBackground`: keep running while the window is in the background
    /// (Unity's `Application.runInBackground`, off by default).
    pub run_in_background: bool,
    /// `iron.fps`: `false` turns the FPS counter off.
    pub fps: Option<bool>,
    /// `iron.lightUnits`: `physical` for real-world units (the sun in lux, lamps in lumens,
    /// glowing surfaces in nits, as Unity 7 and HDRP), or `legacy` (the default: Iron's own).
    pub physical_units: bool,
}

impl Manifest {
    /// Reads the project of `root` (`project.json`, else the `iron` section of `package.json`,
    /// see `iron_core::project`) for this platform and user. A broken file is logged and the
    /// defaults named after the folder are used.
    pub fn load(root: &Path) -> Self {
        let user = SystemUser;
        let opts = LoadOptions::new(Platform::current(), &user);
        match Project::load(root, &opts) {
            Ok(p) => Self::from_project(p),
            Err(e) => {
                tracing::error!("{e}");
                // `.` (the current folder) has no name of its own until it's made absolute.
                let absolute = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
                let folder = absolute
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "game".into());
                Self::from_json(&Value::Null, &folder)
            }
        }
    }

    /// From a `package.json` value (its `iron` section), without templates.
    pub fn from_json(json: &Value, folder: &str) -> Self {
        Self::from_project(Project::from_package_json(json, folder))
    }

    pub fn from_project(p: Project) -> Self {
        let physical_units = light_units(&p.extra);
        Self {
            name: p.name,
            title: p.title,
            version: p.version,
            company: p.company,
            identifier: p.identifier,
            icon: p.icon,
            quality: p.quality,
            layers: p.layers,
            start_scene: p.start_scene,
            display: p.display,
            run_in_background: p.run_in_background,
            fps: p.fps,
            physical_units,
        }
    }
}

/// `iron.lightUnits`, warning about the old pipeline settings: Iron has one render pipeline,
/// as Unity 7 (its URP), whose settings come with the quality levels.
fn light_units(iron: &serde_json::Map<String, Value>) -> bool {
    if iron.contains_key("renderPipeline") || iron.contains_key("renderPipelineSettings") {
        tracing::warn!(
            "iron.renderPipeline: Iron has one render pipeline (as Unity 7): set its features \
             per quality level (iron.quality.levels: volumetricFog, hdr, additionalLights, \
             additionalLightShadows), and real-world light units with iron.lightUnits"
        );
    }
    match iron.get("lightUnits").and_then(Value::as_str) {
        None | Some("legacy") => false,
        Some("physical") => true,
        Some(other) => {
            tracing::warn!("iron.lightUnits `{other}`: not physical or legacy");
            false
        }
    }
}

/// Reads a PNG icon from the game folder as RGBA (`None` if it can't).
pub fn load_icon(root: &Path, path: &str) -> Option<(Vec<u8>, u32, u32)> {
    let file = iron_core::resolve_asset(root, path)?;
    match image::open(&file) {
        Ok(image) => {
            let image = image.to_rgba8();
            let (w, h) = image.dimensions();
            Some((image.into_raw(), w, h))
        }
        Err(e) => {
            tracing::warn!("icon `{path}`: {e}");
            None
        }
    }
}

/// Everything scripts get, as one JSON object. Starts with the manifest and the
/// machine; the GPU and screen join once the window and renderer exist.
#[derive(Clone, Debug)]
pub struct Info(pub Map<String, Value>);

impl Info {
    pub fn new(manifest: &Manifest) -> Self {
        let mut map = Map::new();
        map.insert(
            "app".into(),
            json!({
                "name": manifest.name,
                "productName": manifest.title,
                "version": manifest.version,
                "companyName": manifest.company,
                "identifier": manifest.identifier,
                "engineVersion": env!("CARGO_PKG_VERSION"),
                "quality": manifest.quality,
                "layers": manifest.layers,
                "startScene": manifest.start_scene,
                "runInBackground": manifest.run_in_background,
                "lightUnits": if manifest.physical_units { "physical" } else { "legacy" },
                "targetFrameRate": -1,
                "isEditor": super::editor::ENABLED,
                "project": super::editor::project().map(|p| p.to_string_lossy().into_owned()),
            }),
        );
        map.insert("system".into(), system());
        Self(map)
    }

    /// Sets a top-level section (`prefs`, `display`).
    pub fn set(&mut self, key: &str, value: Value) {
        self.0.insert(key.to_owned(), value);
    }

    /// Sets a field of the `app` section, so scripts reloaded later see what the
    /// old ones chose (`runInBackground`, `targetFrameRate`).
    pub fn set_app(&mut self, key: &str, value: Value) {
        if let Some(Value::Object(app)) = self.0.get_mut("app") {
            app.insert(key.to_owned(), value);
        }
    }

    /// Adds (or replaces) fields of the `system` section.
    pub fn set_system(&mut self, fields: Value) {
        if let (Some(Value::Object(system)), Value::Object(fields)) =
            (self.0.get_mut("system"), fields)
        {
            system.extend(fields);
        }
    }

    /// A script that installs it as `globalThis.__iron_info`.
    pub fn script(&self) -> String {
        format!(
            "globalThis.__iron_info = {};",
            Value::Object(self.0.clone())
        )
    }
}

/// The OS and hardware, as far as the platform tells.
fn system() -> Value {
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );
    let family = match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "MacOSX",
        "linux" | "freebsd" | "openbsd" | "netbsd" => "Linux",
        "android" => "Android",
        "ios" => "iOS",
        _ => "Other",
    };
    let device_type = match std::env::consts::OS {
        "android" | "ios" => "Handheld",
        _ => "Desktop",
    };
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    json!({
        "operatingSystem": System::long_os_version().unwrap_or_else(|| family.to_owned()),
        "operatingSystemFamily": family,
        "operatingSystemVersion": System::os_version().unwrap_or_default(),
        "deviceType": device_type,
        "deviceName": System::host_name().unwrap_or_default(),
        "processorType": sys.cpus().first().map(|c| c.brand().trim().to_owned()).unwrap_or_default(),
        "processorCount": cpus,
        "systemMemorySize": sys.total_memory() / (1024 * 1024),
        "architecture": std::env::consts::ARCH,
        "language": sys_locale::get_locale().unwrap_or_else(|| "en-US".into()),
        "touchSupported": device_type == "Handheld",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-APP-01
    #[test]
    fn manifests_read_package_json_with_fallbacks() {
        let m = Manifest::from_json(
            &json!({
                "name": "fox-run",
                "version": "1.2.0",
                "author": "Someone",
                "iron": { "title": "Fox Run", "icon": "assets/icon.png" }
            }),
            "platformer",
        );
        assert_eq!(m.title, "Fox Run");
        assert_eq!(m.version, "1.2.0");
        assert_eq!(m.company, "Someone");
        assert_eq!(m.icon.as_deref(), Some("assets/icon.png"));
        let bare = Manifest::from_json(&Value::Null, "platformer");
        assert_eq!(
            (bare.name.as_str(), bare.title.as_str()),
            ("platformer", "platformer")
        );

        // The script carries the app and system sections, and later additions.
        let mut info = Info::new(&m);
        info.set_system(json!({ "graphicsDeviceName": "Test GPU" }));
        let script = info.script();
        assert!(script.starts_with("globalThis.__iron_info = {"));
        assert!(script.contains("\"productName\":\"Fox Run\""));
        assert!(script.contains("\"graphicsDeviceName\":\"Test GPU\""));
        assert!(script.contains("\"processorCount\":"));
    }

    // M9-API-1320
    #[test]
    fn a_folder_with_project_json_uses_it_for_title_identifier_and_start_scene() {
        let d = std::env::temp_dir().join(format!("iron_app_manifest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("package.json"),
            r#"{ "name": "npm-name", "iron": { "title": "FROM PACKAGE" } }"#,
        )
        .unwrap();
        std::fs::write(
            d.join("project.json"),
            r#"{ "name": "fox", "title": "Fox ${env:IRON_NO_SUCH_VAR_1320:-Run}", "identifier": "dev.iron.fox",
                 "startScene": "scenes/a.scene.json", "layers": { "8": "Detail" },
                 "windows": { "fps": false }, "macos": { "fps": false }, "linux": { "fps": false } }"#,
        )
        .unwrap();
        let m = Manifest::load(&d);
        assert_eq!(m.title, "Fox Run");
        // package.json is not read when a project.json exists.
        assert_eq!(m.name, "fox");
        assert_eq!(m.identifier, "dev.iron.fox");
        assert_eq!(m.start_scene.as_deref(), Some("scenes/a.scene.json"));
        assert_eq!(m.layers["8"], "Detail");
        assert_eq!(m.fps, Some(false));
        // A broken template leaves the defaults and does not stop the game.
        std::fs::write(
            d.join("project.json"),
            r#"{ "name": "fox", "title": "${nope}" }"#,
        )
        .unwrap();
        assert_eq!(
            Manifest::load(&d).title,
            d.file_name().unwrap().to_string_lossy()
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    // M9-API-910
    #[test]
    fn fps_comes_from_the_manifest() {
        assert_eq!(
            Manifest::from_json(&json!({ "iron": { "fps": false } }), "g").fps,
            Some(false)
        );
        assert_eq!(Manifest::from_json(&Value::Null, "g").fps, None);
    }

    // M7-RP-01
    #[test]
    fn light_units_come_from_the_manifest_and_old_pipelines_are_gone() {
        let units = |v: Value| Manifest::from_json(&json!({ "iron": v }), "g").physical_units;
        assert!(units(json!({ "lightUnits": "physical" })));
        assert!(!units(json!({ "lightUnits": "legacy" })));
        assert!(!units(json!({})));
        assert!(!units(json!({ "lightUnits": "candles" })));
        // The old pipeline choice no longer picks anything (it only warns).
        assert!(!units(json!({ "renderPipeline": "hd" })));
        let script = Info::new(&Manifest::from_json(
            &json!({ "iron": { "lightUnits": "physical" } }),
            "g",
        ))
        .script();
        assert!(script.contains("\"lightUnits\":\"physical\""));
        assert!(!script.contains("renderPipeline"));
    }

    // M5-APP-09
    #[test]
    fn run_in_background_comes_from_the_manifest_and_follows_scripts() {
        let m = Manifest::from_json(&json!({ "iron": { "runInBackground": true } }), "g");
        assert!(m.run_in_background);
        assert!(!Manifest::from_json(&Value::Null, "g").run_in_background);
        let mut info = Info::new(&m);
        assert!(info.script().contains("\"runInBackground\":true"));
        info.set_app("runInBackground", json!(false));
        info.set_app("targetFrameRate", json!(30));
        let script = info.script();
        assert!(script.contains("\"runInBackground\":false"));
        assert!(script.contains("\"targetFrameRate\":30"));
    }
}
