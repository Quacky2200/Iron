// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The game in the background (Unity's `Application.runInBackground`, focus and
//! pause events) and the fixed-tick clock that pausing has to stop cleanly.

use std::time::{Duration, Instant};

/// Focus, minimising and whether that pauses the game.
#[derive(Debug)]
pub struct Lifecycle {
    /// `Application.runInBackground`: keep running without focus.
    pub run_in_background: bool,
    /// Never pause (screenshot runs: their window may never get focus).
    always_run: bool,
    focused: bool,
    /// Occluded (macOS, Wayland) or sized to nothing (Windows minimises to 0 × 0).
    occluded: bool,
    zero_size: bool,
    /// A dialog of the host's is open over the game (`Application.showVideoSettings`).
    modal: bool,
    /// When the pause began.
    paused_at: Option<Instant>,
    /// The focus and pause state scripts were last told (they start focused, running).
    reported: (bool, bool),
}

/// What changed in an [`Lifecycle::update`].
#[derive(Debug, Default, PartialEq)]
pub struct Change {
    /// The new `(focused, paused)` for scripts (`__platypus_focus`), when either changed.
    pub report: Option<(bool, bool)>,
    /// The game just paused.
    pub paused: bool,
    /// The game just resumed, after this long paused.
    pub resumed: Option<Duration>,
}

impl Lifecycle {
    pub fn new(run_in_background: bool, always_run: bool) -> Self {
        Self {
            run_in_background,
            always_run,
            focused: true,
            occluded: false,
            zero_size: false,
            modal: false,
            paused_at: None,
            reported: (true, false),
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn set_occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.zero_size = width == 0 || height == 0;
    }

    /// A host dialog opened or closed over the game: it pauses the game, even with
    /// `runInBackground`.
    pub fn set_modal(&mut self, modal: bool) {
        self.modal = modal;
    }

    /// Whether the window is in the background: unfocused or minimised.
    fn background(&self) -> bool {
        !self.focused || self.occluded || self.zero_size
    }

    /// Whether the game should be paused now.
    pub fn should_pause(&self) -> bool {
        !self.always_run && (self.modal || (!self.run_in_background && self.background()))
    }

    pub fn paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// Applies the latest state: starts or ends the pause and says what to report.
    pub fn update(&mut self, now: Instant) -> Change {
        let mut change = Change::default();
        match (self.should_pause(), self.paused_at) {
            (true, None) => {
                self.paused_at = Some(now);
                change.paused = true;
            }
            (false, Some(at)) => {
                self.paused_at = None;
                change.resumed = Some(now.saturating_duration_since(at));
            }
            _ => {}
        }
        let state = (
            self.focused && !self.occluded && !self.zero_size,
            self.paused(),
        );
        if state != self.reported {
            self.reported = state;
            change.report = Some(state);
        }
        change
    }
}

/// The fixed-tick clock: frame times in, how many fixed ticks to run out.
#[derive(Debug, Default)]
pub struct FixedClock {
    last_frame: Option<Instant>,
    accumulator: f64,
}

impl FixedClock {
    /// A frame starts at `now`: its length (one tick for the first frame, or after a
    /// [`FixedClock::restart`]) and the fixed ticks it runs, at most `max_ticks`. A
    /// frame that hits the limit drops the rest instead of carrying it over.
    pub fn advance(&mut self, now: Instant, tick: f64, max_ticks: u32) -> (f64, u32) {
        let elapsed = self
            .last_frame
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(tick);
        self.last_frame = Some(now);
        self.accumulator += elapsed;
        let ticks = ((self.accumulator / tick) as u32).min(max_ticks);
        if ticks == max_ticks {
            self.accumulator = 0.0;
        } else {
            self.accumulator -= f64::from(ticks) * tick;
        }
        (elapsed, ticks)
    }

    /// Forgets the last frame, so time spent paused isn't caught up on.
    pub fn restart(&mut self) {
        *self = Self::default();
    }
}

/// `Application.targetFrameRate` as a time between frames. Like Unity, the cap only
/// counts with vsync off (vsync paces frames itself); screenshot runs aren't capped.
#[cfg(test)]
pub fn frame_cap(target_fps: i32, vsync: bool, screenshot: bool) -> Option<Duration> {
    frame_cap_with(target_fps, vsync, screenshot, false)
}

/// The slowest frame rate while the main game is paused but another event domain (an isolated
/// window) is live: its windows stay responsive without the paused game spinning the GPU.
pub const PAUSED_FRAME_CAP: Duration = Duration::from_micros(33_333);

/// [`frame_cap`] that also caps frames at 30 Hz when `main_paused_others_live`.
pub fn frame_cap_with(
    target_fps: i32,
    vsync: bool,
    screenshot: bool,
    main_paused_others_live: bool,
) -> Option<Duration> {
    let cap = (!screenshot && !vsync && target_fps > 0)
        .then(|| Duration::from_secs_f64(1.0 / f64::from(target_fps)));
    if main_paused_others_live && !screenshot {
        return Some(cap.map_or(PAUSED_FRAME_CAP, |c| c.max(PAUSED_FRAME_CAP)));
    }
    cap
}

/// The app pauses only when every event domain is paused: the main one and each isolated one.
pub fn app_paused(main_paused: bool, isolated_paused: impl IntoIterator<Item = bool>) -> bool {
    main_paused && isolated_paused.into_iter().all(|p| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK: f64 = 1.0 / 60.0;

    // M5-APP-07
    #[test]
    fn losing_focus_pauses_unless_running_in_the_background() {
        let t0 = Instant::now();
        let mut l = Lifecycle::new(false, false);
        assert_eq!(l.update(t0), Change::default());

        l.set_focused(false);
        let c = l.update(t0);
        assert!(c.paused && l.paused());
        assert_eq!(c.report, Some((false, true)));
        // Nothing new, nothing to report.
        assert_eq!(l.update(t0), Change::default());

        l.set_focused(true);
        let c = l.update(t0 + Duration::from_secs(3));
        assert_eq!(c.resumed, Some(Duration::from_secs(3)));
        assert_eq!(c.report, Some((true, false)));

        // Minimised: Windows reports a 0 × 0 size, others occlusion.
        l.set_size(0, 0);
        assert!(l.update(t0).paused);
        // Restored but covered: still paused, nothing new to say.
        l.set_size(800, 600);
        l.set_occluded(true);
        assert_eq!(l.update(t0), Change::default());
        assert!(l.paused());
        l.set_occluded(false);
        assert!(l.update(t0).resumed.is_some());

        // `runInBackground`: focus is still reported, but nothing pauses.
        l.run_in_background = true;
        l.set_focused(false);
        let c = l.update(t0);
        assert!(!c.paused && !l.paused());
        assert_eq!(c.report, Some((false, false)));
        // Turning it off while in the background pauses at once.
        l.run_in_background = false;
        assert!(l.update(t0).paused);

        // Screenshot runs never pause.
        let mut shot = Lifecycle::new(false, true);
        shot.set_focused(false);
        assert!(!shot.update(t0).paused);
    }

    // M9-VSP-14
    #[test]
    fn a_host_dialog_pauses_the_game_even_with_run_in_background() {
        let t0 = Instant::now();
        let mut l = Lifecycle::new(true, false);
        l.set_modal(true);
        let c = l.update(t0);
        assert!(c.paused && l.paused());
        assert_eq!(c.report, Some((true, true)));
        l.set_modal(false);
        assert_eq!(
            l.update(t0 + Duration::from_secs(2)).resumed,
            Some(Duration::from_secs(2))
        );
        // Screenshot runs never pause.
        let mut shot = Lifecycle::new(false, true);
        shot.set_modal(true);
        assert!(!shot.update(t0).paused);
    }

    // M5-APP-07
    #[test]
    fn the_fixed_clock_caps_catch_up_and_restarts_without_a_burst() {
        let t0 = Instant::now();
        let mut c = FixedClock::default();
        assert_eq!(c.advance(t0, TICK, 5), (TICK, 1));
        let (_, ticks) = c.advance(t0 + Duration::from_secs_f64(TICK * 2.5), TICK, 5);
        assert_eq!(ticks, 2);
        // A long hitch runs at most five ticks and drops the rest.
        let t1 = t0 + Duration::from_secs(1);
        assert_eq!(c.advance(t1, TICK, 5).1, 5);
        let (_, ticks) = c.advance(t1 + Duration::from_secs_f64(TICK * 1.5), TICK, 5);
        assert_eq!(ticks, 1);

        // After a pause the first frame counts as one tick, however long it was.
        c.restart();
        let (dt, ticks) = c.advance(t1 + Duration::from_secs(60), TICK, 5);
        assert_eq!((dt, ticks), (TICK, 1));
    }

    // M9-TS-2836
    #[test]
    fn the_app_pauses_only_when_every_domain_is_paused_and_caps_frames_while_one_runs() {
        assert!(app_paused(true, []));
        assert!(app_paused(true, [true, true]));
        assert!(
            !app_paused(true, [true, false]),
            "a live inspector keeps the app running"
        );
        assert!(!app_paused(false, [true]));
        // Main paused, another domain live: frames at most 30 Hz, even with no other cap.
        assert_eq!(
            frame_cap_with(-1, true, false, true),
            Some(PAUSED_FRAME_CAP)
        );
        // A slower target rate wins; a faster one is raised to 30 Hz.
        assert_eq!(
            frame_cap_with(10, false, false, true),
            Some(Duration::from_millis(100))
        );
        assert_eq!(
            frame_cap_with(120, false, false, true),
            Some(PAUSED_FRAME_CAP)
        );
        // Otherwise as before; screenshot runs are never capped.
        assert_eq!(frame_cap_with(-1, true, false, false), None);
        assert_eq!(frame_cap_with(50, false, true, true), None);
    }

    // M5-APP-08
    #[test]
    fn the_frame_rate_cap_only_counts_with_vsync_off() {
        assert_eq!(frame_cap(50, false, false), Some(Duration::from_millis(20)));
        assert_eq!(frame_cap(50, true, false), None);
        assert_eq!(frame_cap(-1, false, false), None);
        assert_eq!(frame_cap(50, false, true), None);
    }
}
