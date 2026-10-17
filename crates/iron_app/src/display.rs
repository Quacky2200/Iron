//! The window's mode and size as scripts and saved settings see them: `screen.set`
//! ops, the manifest's `iron.display` defaults, the player's saved choice
//! (`iron.display` in prefs) and the state scripts read (`__iron_info.display`).

use iron_platform::Window;
use iron_platform::display::{self, DisplaySettings, FullScreenMode};
use serde_json::{Value, json};

/// `base` with whatever `v` (`{ mode, width, height, refreshRate, windowWidth,
/// windowHeight }`) sets. Exclusive fullscreen without a size takes the monitor's current
/// resolution.
pub fn from_json(v: Option<&Value>, base: DisplaySettings) -> DisplaySettings {
    let Some(v) = v else { return base };
    let num = |k: &str| v.get(k).and_then(Value::as_f64);
    let size = |k: &str, default: u32| num(k).filter(|w| *w >= 1.0).map_or(default, |w| w as u32);
    let mode = v
        .get("mode")
        .and_then(Value::as_str)
        .and_then(FullScreenMode::parse)
        .unwrap_or(base.mode);
    let sized = num("width").is_some_and(|w| w >= 1.0);
    let (width, height) = if mode == FullScreenMode::ExclusiveFullScreen && !sized {
        (0, 0)
    } else {
        (size("width", base.width), size("height", base.height))
    };
    // The window size: given, else the windowed size given, else the base's.
    let windowed = mode == FullScreenMode::Windowed && sized;
    DisplaySettings {
        mode,
        width,
        height,
        refresh_rate: num("refreshRate").map_or(base.refresh_rate, |r| r.max(0.0) as f32),
        window_width: size(
            "windowWidth",
            if windowed { width } else { base.window_width },
        ),
        window_height: size(
            "windowHeight",
            if windowed { height } else { base.window_height },
        ),
    }
}

/// What's saved in prefs.
pub fn to_json(s: &DisplaySettings) -> Value {
    json!({
        "mode": s.mode.name(),
        "width": s.width,
        "height": s.height,
        "refreshRate": s.refresh_rate,
        "windowWidth": s.window_width,
        "windowHeight": s.window_height,
    })
}

/// What scripts read: the settings plus what the platform and monitor offer (empty
/// until the window exists).
pub fn state(window: Option<&Window>, s: &DisplaySettings) -> Value {
    let resolution = |r: display::Resolution| json!({ "width": r.width, "height": r.height, "refreshRate": r.refresh_rate });
    let (supported, resolutions, current) = match window {
        Some(w) => (
            display::supported_modes(w),
            display::resolutions(w),
            display::current_resolution(w),
        ),
        None => (FullScreenMode::ALL.to_vec(), Vec::new(), None),
    };
    json!({
        "mode": s.mode.name(),
        "width": s.width,
        "height": s.height,
        "refreshRate": s.refresh_rate,
        "windowWidth": s.window_width,
        "windowHeight": s.window_height,
        "supportedModes": supported.iter().map(|m| m.name()).collect::<Vec<_>>(),
        "resolutions": resolutions.into_iter().map(resolution).collect::<Vec<_>>(),
        "currentResolution": current.map(resolution),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-DSP-02
    #[test]
    fn saved_display_settings_override_the_manifest_and_bad_values_are_ignored() {
        let manifest = json!({ "mode": "FullScreenWindow", "width": 1600, "height": 900 });
        let base = from_json(Some(&manifest), DisplaySettings::default());
        assert_eq!(base.mode, FullScreenMode::FullScreenWindow);
        assert_eq!((base.width, base.height), (1600, 900));
        let saved = json!({ "mode": "Windowed", "width": 1920, "height": -5, "refreshRate": 144 });
        let s = from_json(Some(&saved), base);
        assert_eq!(s.mode, FullScreenMode::Windowed);
        assert_eq!((s.width, s.height, s.refresh_rate), (1920, 900, 144.0));
        assert_eq!(from_json(Some(&to_json(&s)), base), s);
        // Exclusive without a size: the monitor's own (0 until applied); the window size
        // stays for later.
        let full = from_json(Some(&json!({ "mode": "ExclusiveFullScreen" })), s);
        assert_eq!((full.width, full.height), (0, 0));
        assert_eq!((full.window_width, full.window_height), (1920, 900));
        let odd = json!({ "mode": "Sideways" });
        assert_eq!(
            from_json(Some(&odd), base).mode,
            FullScreenMode::FullScreenWindow
        );
    }
}
