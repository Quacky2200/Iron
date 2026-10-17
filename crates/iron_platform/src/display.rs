//! Window modes and resolutions (Unity's `Screen.fullScreenMode`, `Screen.SetResolution`,
//! `Screen.resolutions`). What each platform allows differs: Wayland has no exclusive
//! fullscreen (compositors own the display mode), and phones and tablets only run
//! fullscreen; asking for something unsupported falls back to the nearest mode.

use winit::dpi::PhysicalSize;
use winit::monitor::{MonitorHandle, VideoModeHandle};
use winit::window::{Fullscreen, Window};

/// How the game's window covers the screen (Unity's `FullScreenMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FullScreenMode {
    /// The display switches to the game's resolution and refresh rate.
    ExclusiveFullScreen,
    /// A borderless window covering the monitor at its own resolution ("borderless").
    FullScreenWindow,
    /// A normal window, maximized.
    MaximizedWindow,
    /// A normal window of the chosen size.
    Windowed,
}

impl FullScreenMode {
    pub const ALL: [FullScreenMode; 4] = [
        Self::ExclusiveFullScreen,
        Self::FullScreenWindow,
        Self::MaximizedWindow,
        Self::Windowed,
    ];

    /// Unity's name, which scripts use.
    pub fn name(self) -> &'static str {
        match self {
            Self::ExclusiveFullScreen => "ExclusiveFullScreen",
            Self::FullScreenWindow => "FullScreenWindow",
            Self::MaximizedWindow => "MaximizedWindow",
            Self::Windowed => "Windowed",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name() == name)
    }

    pub fn is_fullscreen(self) -> bool {
        matches!(self, Self::ExclusiveFullScreen | Self::FullScreenWindow)
    }
}

/// A display mode: size in physical pixels and refresh rate in Hz (0 when unknown).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
}

/// The window's mode and, for windowed and exclusive modes, its size and refresh rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplaySettings {
    pub mode: FullScreenMode,
    pub width: u32,
    pub height: u32,
    /// For exclusive fullscreen (0: the monitor's current rate).
    pub refresh_rate: f32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            mode: FullScreenMode::Windowed,
            width: 1280,
            height: 720,
            refresh_rate: 0.0,
        }
    }
}

/// The modes this platform offers, most fullscreen first.
pub fn supported_modes(window: &Window) -> Vec<FullScreenMode> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return vec![FullScreenMode::FullScreenWindow];
    }
    let mut modes = FullScreenMode::ALL.to_vec();
    if is_wayland(window) {
        modes.retain(|m| *m != FullScreenMode::ExclusiveFullScreen);
    }
    modes
}

fn is_wayland(window: &Window) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    matches!(
        window.window_handle().map(|h| h.as_raw()),
        Ok(RawWindowHandle::Wayland(_))
    )
}

/// The monitor's display modes (for exclusive fullscreen and sizing windows), largest
/// and fastest first, without repeats.
pub fn resolutions(window: &Window) -> Vec<Resolution> {
    let Some(monitor) = window.current_monitor() else {
        return Vec::new();
    };
    let mut list: Vec<Resolution> = monitor.video_modes().map(|m| to_resolution(&m)).collect();
    list.sort_by(|a, b| {
        (b.width * b.height, b.width)
            .cmp(&(a.width * a.height, a.width))
            .then(b.refresh_rate.total_cmp(&a.refresh_rate))
    });
    list.dedup();
    list
}

/// What the monitor runs at now.
pub fn current_resolution(window: &Window) -> Option<Resolution> {
    let monitor = window.current_monitor()?;
    let size = monitor.size();
    Some(Resolution {
        width: size.width,
        height: size.height,
        refresh_rate: monitor
            .refresh_rate_millihertz()
            .map_or(0.0, |mhz| mhz as f32 / 1000.0),
    })
}

fn to_resolution(mode: &VideoModeHandle) -> Resolution {
    let size = mode.size();
    Resolution {
        width: size.width,
        height: size.height,
        refresh_rate: mode.refresh_rate_millihertz() as f32 / 1000.0,
    }
}

/// The monitor mode closest to the request: size first, then refresh rate.
fn closest_mode(monitor: &MonitorHandle, s: &DisplaySettings) -> Option<VideoModeHandle> {
    let want = s.refresh_rate;
    monitor.video_modes().min_by(|a, b| {
        let key = |m: &VideoModeHandle| {
            let r = to_resolution(m);
            let size = r.width.abs_diff(s.width) + r.height.abs_diff(s.height);
            // No rate asked for: the fastest.
            let rate = if want > 0.0 {
                (r.refresh_rate - want).abs()
            } else {
                -r.refresh_rate
            };
            (size, rate)
        };
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
    })
}

/// The mode to use for a request on this platform: the request if it's supported,
/// else the nearest one that is.
pub fn effective_mode(requested: FullScreenMode, supported: &[FullScreenMode]) -> FullScreenMode {
    if supported.contains(&requested) {
        return requested;
    }
    let fallbacks: &[FullScreenMode] = if requested.is_fullscreen() {
        &[
            FullScreenMode::FullScreenWindow,
            FullScreenMode::MaximizedWindow,
        ]
    } else {
        &[
            FullScreenMode::MaximizedWindow,
            FullScreenMode::FullScreenWindow,
        ]
    };
    fallbacks
        .iter()
        .copied()
        .find(|m| supported.contains(m))
        .or_else(|| supported.first().copied())
        .unwrap_or(FullScreenMode::Windowed)
}

/// Puts the window in a mode. Returns what took effect (an unsupported mode falls
/// back; an exclusive mode gets the monitor's closest size and rate).
pub fn apply(window: &Window, s: &DisplaySettings) -> DisplaySettings {
    let mode = effective_mode(s.mode, &supported_modes(window));
    let mut out = DisplaySettings { mode, ..*s };
    match mode {
        FullScreenMode::ExclusiveFullScreen => {
            let video = window.current_monitor().and_then(|m| closest_mode(&m, s));
            match video {
                Some(video) => {
                    let r = to_resolution(&video);
                    (out.width, out.height, out.refresh_rate) = (r.width, r.height, r.refresh_rate);
                    window.set_fullscreen(Some(Fullscreen::Exclusive(video)));
                }
                None => {
                    // No modes reported: borderless instead.
                    out.mode = FullScreenMode::FullScreenWindow;
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
            }
        }
        FullScreenMode::FullScreenWindow => {
            window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
        }
        FullScreenMode::MaximizedWindow => {
            window.set_fullscreen(None);
            window.set_maximized(true);
        }
        FullScreenMode::Windowed => {
            window.set_fullscreen(None);
            window.set_maximized(false);
            // No bigger than the monitor.
            let (mw, mh) = window
                .current_monitor()
                .map_or((u32::MAX, u32::MAX), |m| (m.size().width, m.size().height));
            out.width = s.width.clamp(320, mw.max(320));
            out.height = s.height.clamp(240, mh.max(240));
            let _ = window.request_inner_size(PhysicalSize::new(out.width, out.height));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // M5-DSP-01
    #[test]
    fn unsupported_modes_fall_back_to_the_nearest_supported_one() {
        use FullScreenMode::*;
        let desktop = FullScreenMode::ALL.to_vec();
        let wayland = vec![FullScreenWindow, MaximizedWindow, Windowed];
        let phone = vec![FullScreenWindow];
        assert_eq!(
            effective_mode(ExclusiveFullScreen, &desktop),
            ExclusiveFullScreen
        );
        assert_eq!(
            effective_mode(ExclusiveFullScreen, &wayland),
            FullScreenWindow
        );
        assert_eq!(effective_mode(Windowed, &phone), FullScreenWindow);
        assert_eq!(effective_mode(MaximizedWindow, &phone), FullScreenWindow);
        for m in FullScreenMode::ALL {
            assert_eq!(FullScreenMode::parse(m.name()), Some(m));
        }
        assert_eq!(FullScreenMode::parse("Borderless"), None);
    }
}
