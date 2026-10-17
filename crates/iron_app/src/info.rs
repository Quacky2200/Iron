//! What scripts learn about the game and the machine: the game's manifest (its
//! `package.json`) for `Application`, and the OS, hardware, GPU and screen for
//! `SystemInfo`. It reaches scripts as `globalThis.__iron_info` before the bundle runs.

use std::path::Path;

use serde_json::{Map, Value, json};

/// The game's details, from `package.json` in its folder: `name`, `version`, and an
/// `iron` section with `title`, `company`, `identifier` and `icon` (a PNG path).
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
    /// `iron.display`: the window's default mode and size (`{ mode, width, height }`).
    pub display: Value,
    /// `iron.runInBackground`: keep running while the window is in the background
    /// (Unity's `Application.runInBackground`, off by default).
    pub run_in_background: bool,
}

impl Manifest {
    /// Reads `package.json` in `root`; a missing or broken one gives defaults named
    /// after the folder.
    pub fn load(root: &Path) -> Self {
        // `.` (the current folder) has no name of its own until it's made absolute.
        let absolute = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let folder = absolute
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "game".into());
        let text = std::fs::read_to_string(root.join("package.json")).ok();
        let parsed = text.as_deref().and_then(|t| match serde_json::from_str(t) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("package.json: {e}");
                None
            }
        });
        Self::from_json(parsed.as_ref().unwrap_or(&Value::Null), &folder)
    }

    pub fn from_json(json: &Value, folder: &str) -> Self {
        let text = |v: &Value| v.as_str().map(str::to_owned);
        let iron = &json["iron"];
        let name = text(&json["name"]).unwrap_or_else(|| folder.to_owned());
        Self {
            title: text(&iron["title"]).unwrap_or_else(|| name.clone()),
            version: text(&json["version"]).unwrap_or_else(|| "0.0.0".into()),
            company: text(&iron["company"])
                .or_else(|| text(&json["author"]))
                .unwrap_or_default(),
            identifier: text(&iron["identifier"]).unwrap_or_default(),
            icon: text(&iron["icon"]),
            quality: iron["quality"].clone(),
            layers: iron["layers"].clone(),
            start_scene: text(&iron["startScene"]),
            display: iron["display"].clone(),
            run_in_background: iron["runInBackground"].as_bool().unwrap_or(false),
            name,
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
                "targetFrameRate": -1,
                "isEditor": super::editor::ENABLED,
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
