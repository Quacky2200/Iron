// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The system's light/dark preference, for `prefers-color-scheme` (docs/platform.md, "System
//! theme").
//!
//! Windows and macOS: winit's `Window::theme()`. Linux: winit reports none (Wayland gives back
//! only the theme the app set, X11 nothing), so the XDG Settings portal's
//! `org.freedesktop.appearance` `color-scheme` is read at start and followed by its
//! `SettingChanged` signal on a background thread, which wakes the event loop; without a portal,
//! GNOME's `gsettings` answer at start. The mapping from the answers is plain code, tested on
//! every platform.

use std::sync::atomic::{AtomicU8, Ordering};

use winit::window::{Theme, Window};

/// The portal's `color-scheme`: 0 no preference, 1 dark, 2 light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    NoPreference,
    Dark,
    Light,
}

impl ColorScheme {
    /// The portal's number; values it doesn't define mean no preference (as the spec asks).
    pub fn from_portal(n: u32) -> Self {
        match n {
            1 => ColorScheme::Dark,
            2 => ColorScheme::Light,
            _ => ColorScheme::NoPreference,
        }
    }

    /// `gsettings get org.gnome.desktop.interface color-scheme` output (`'prefer-dark'`).
    pub fn from_gsettings(stdout: &str) -> Option<Self> {
        match stdout.trim().trim_matches('\'') {
            "prefer-dark" => Some(ColorScheme::Dark),
            "prefer-light" => Some(ColorScheme::Light),
            "default" => Some(ColorScheme::NoPreference),
            _ => None,
        }
    }

    /// As winit's theme: `None` for no preference.
    pub fn theme(self) -> Option<Theme> {
        match self {
            ColorScheme::NoPreference => None,
            ColorScheme::Dark => Some(Theme::Dark),
            ColorScheme::Light => Some(Theme::Light),
        }
    }
}

/// A portal reply reduced to what the preference needs. `ReadOne` answers a variant holding the
/// `u32`; the older `Read` wraps that variant in one more.
#[derive(Clone, Debug, PartialEq)]
pub enum PortalValue {
    U32(u32),
    Variant(Box<PortalValue>),
    Other,
}

impl PortalValue {
    /// The scheme inside however many variants wrap it; `None` for a value of another type.
    pub fn scheme(&self) -> Option<ColorScheme> {
        let mut v = self;
        loop {
            match v {
                PortalValue::U32(n) => return Some(ColorScheme::from_portal(*n)),
                PortalValue::Variant(inner) => v = inner,
                PortalValue::Other => return None,
            }
        }
    }
}

/// The system's answer: 0 none yet (winit's is used), else 1 + the `ColorScheme`.
static SYSTEM: AtomicU8 = AtomicU8::new(0);

/// Stores the system's scheme; true when it changed.
pub fn set_system_scheme(scheme: ColorScheme) -> bool {
    let code = match scheme {
        ColorScheme::NoPreference => 1,
        ColorScheme::Dark => 2,
        ColorScheme::Light => 3,
    };
    SYSTEM.swap(code, Ordering::Relaxed) != code
}

/// The system's scheme, when one was read (Linux).
pub fn system_scheme() -> Option<ColorScheme> {
    match SYSTEM.load(Ordering::Relaxed) {
        1 => Some(ColorScheme::NoPreference),
        2 => Some(ColorScheme::Dark),
        3 => Some(ColorScheme::Light),
        _ => None,
    }
}

/// The theme to follow: the system's, when it was read, else `window`'s (winit).
pub fn system_theme(window: Option<&Window>) -> Option<Theme> {
    match system_scheme() {
        Some(scheme) => scheme.theme(),
        None => window.and_then(Window::theme),
    }
}

/// Whether the system prefers dark (see [`system_theme`]).
pub fn prefers_dark(window: Option<&Window>) -> bool {
    system_theme(window) == Some(Theme::Dark)
}

/// Starts following the system's preference where winit can't: reads it (waiting a moment, so
/// the first frame has it) and watches it on a thread that calls `wake` after each change.
#[cfg(target_os = "linux")]
pub(crate) fn watch(wake: impl Fn() + Send + 'static) {
    portal::watch(wake);
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn watch(_wake: impl Fn() + Send + 'static) {}

#[cfg(target_os = "linux")]
mod portal {
    use std::sync::mpsc;
    use std::time::Duration;

    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedValue, Value};

    use super::{ColorScheme, PortalValue, set_system_scheme};

    const DEST: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const IFACE: &str = "org.freedesktop.portal.Settings";
    const NAMESPACE: &str = "org.freedesktop.appearance";
    const KEY: &str = "color-scheme";
    /// How long the start waits for the first answer (a portal being started can take longer;
    /// its answer then arrives as a change).
    const STARTUP_WAIT: Duration = Duration::from_millis(300);

    fn reduce(v: &Value) -> PortalValue {
        match v {
            Value::U32(n) => PortalValue::U32(*n),
            Value::Value(inner) => PortalValue::Variant(Box::new(reduce(inner))),
            _ => PortalValue::Other,
        }
    }

    /// `ReadOne`, else the older `Read`.
    fn read(proxy: &Proxy) -> Option<ColorScheme> {
        let reply = proxy
            .call::<_, _, OwnedValue>("ReadOne", &(NAMESPACE, KEY))
            .or_else(|_| proxy.call::<_, _, OwnedValue>("Read", &(NAMESPACE, KEY)));
        match reply {
            Ok(v) => reduce(&v).scheme(),
            Err(e) => {
                tracing::debug!("settings portal: no {NAMESPACE} {KEY}: {e}");
                None
            }
        }
    }

    fn gsettings() -> Option<ColorScheme> {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| ColorScheme::from_gsettings(&String::from_utf8_lossy(&out.stdout)))
            .flatten()
    }

    fn store(scheme: Option<ColorScheme>, wake: &impl Fn()) {
        if let Some(s) = scheme
            && set_system_scheme(s)
        {
            tracing::debug!("system colour scheme: {s:?}");
            wake();
        }
    }

    pub(super) fn watch(wake: impl Fn() + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("platypus-theme".into())
            .spawn(move || {
                let proxy =
                    Connection::session().and_then(|conn| Proxy::new(&conn, DEST, PATH, IFACE));
                let proxy = match proxy {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::debug!("settings portal: no session bus: {e}");
                        store(gsettings(), &wake);
                        let _ = tx.send(());
                        return;
                    }
                };
                // Subscribed before reading, so a change in between isn't lost.
                let changes =
                    proxy.receive_signal_with_args("SettingChanged", &[(0, NAMESPACE), (1, KEY)]);
                store(read(&proxy).or_else(gsettings), &wake);
                let _ = tx.send(());
                let changes = match changes {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!("settings portal: no SettingChanged: {e}");
                        return;
                    }
                };
                for msg in changes {
                    let Ok((ns, key, value)) =
                        msg.body().deserialize::<(String, String, OwnedValue)>()
                    else {
                        continue;
                    };
                    if ns == NAMESPACE && key == KEY {
                        store(reduce(&value).scheme(), &wake);
                    }
                }
            });
        if thread.is_ok() {
            let _ = rx.recv_timeout(STARTUP_WAIT);
        }
    }
}

// M2-THEME-01
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_numbers_map_to_schemes_and_unknown_ones_to_no_preference() {
        assert_eq!(ColorScheme::from_portal(0), ColorScheme::NoPreference);
        assert_eq!(ColorScheme::from_portal(1), ColorScheme::Dark);
        assert_eq!(ColorScheme::from_portal(2), ColorScheme::Light);
        assert_eq!(ColorScheme::from_portal(7), ColorScheme::NoPreference);
        assert_eq!(ColorScheme::Dark.theme(), Some(Theme::Dark));
        assert_eq!(ColorScheme::Light.theme(), Some(Theme::Light));
        assert_eq!(ColorScheme::NoPreference.theme(), None);
    }

    #[test]
    fn read_one_and_the_double_wrapped_read_reply_give_the_same_scheme() {
        let one = PortalValue::U32(1);
        assert_eq!(one.scheme(), Some(ColorScheme::Dark));
        let read = PortalValue::Variant(Box::new(PortalValue::U32(1)));
        assert_eq!(read.scheme(), Some(ColorScheme::Dark));
        let twice = PortalValue::Variant(Box::new(PortalValue::Variant(Box::new(
            PortalValue::U32(2),
        ))));
        assert_eq!(twice.scheme(), Some(ColorScheme::Light));
        assert_eq!(PortalValue::Other.scheme(), None);
        assert_eq!(
            PortalValue::Variant(Box::new(PortalValue::Other)).scheme(),
            None
        );
    }

    #[test]
    fn gsettings_answers_map_and_garbage_is_ignored() {
        assert_eq!(
            ColorScheme::from_gsettings("'prefer-dark'\n"),
            Some(ColorScheme::Dark)
        );
        assert_eq!(
            ColorScheme::from_gsettings("'prefer-light'\n"),
            Some(ColorScheme::Light)
        );
        assert_eq!(
            ColorScheme::from_gsettings("'default'\n"),
            Some(ColorScheme::NoPreference)
        );
        assert_eq!(ColorScheme::from_gsettings("No such key\n"), None);
    }

    // M2-THEME-02
    #[test]
    fn a_stored_system_scheme_wins_over_the_window_and_reports_changes() {
        assert!(set_system_scheme(ColorScheme::Dark));
        assert!(!set_system_scheme(ColorScheme::Dark), "no change");
        assert_eq!(system_theme(None), Some(Theme::Dark));
        assert!(prefers_dark(None));
        assert!(set_system_scheme(ColorScheme::NoPreference));
        assert_eq!(system_theme(None), None);
        assert!(set_system_scheme(ColorScheme::Light));
        assert!(!prefers_dark(None));
    }
}
