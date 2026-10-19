//! `Screen` and `QualitySettings`: the few display settings scripts read and set. The host reads
//! what scripts asked for with [`state`] / [`quality`] and writes the window size with [`set_size`].

use iron_api::iron_api;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenState {
    pub width: u32,
    pub height: u32,
    pub full_screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualityState {
    pub v_sync_count: i32,
}

thread_local! {
    static SCREEN: Cell<ScreenState> = const {
        Cell::new(ScreenState { width: 1280, height: 720, full_screen: false })
    };
    static QUALITY: Cell<QualityState> = const { Cell::new(QualityState { v_sync_count: 1 }) };
}

pub fn state() -> ScreenState {
    SCREEN.with(Cell::get)
}

pub fn quality() -> QualityState {
    QUALITY.with(Cell::get)
}

/// The window's size changed.
pub fn set_size(width: u32, height: u32) {
    SCREEN.with(|s| {
        let mut v = s.get();
        v.width = width;
        v.height = height;
        s.set(v);
    });
}

#[iron_api(namespace = "Screen", quickjs)]
pub mod screen_api {
    use super::SCREEN;

    /// The window's width in pixels.
    #[property]
    pub fn width() -> u32 {
        SCREEN.with(|s| s.get().width)
    }

    #[property]
    pub fn height() -> u32 {
        SCREEN.with(|s| s.get().height)
    }

    /// Fullscreen on or off; the app applies it at the end of the frame.
    #[property]
    pub fn full_screen() -> bool {
        SCREEN.with(|s| s.get().full_screen)
    }

    pub fn set_full_screen(on: bool) {
        SCREEN.with(|s| {
            let mut v = s.get();
            v.full_screen = on;
            s.set(v);
        });
    }
}

#[iron_api(namespace = "QualitySettings", quickjs)]
pub mod quality_api {
    use super::QUALITY;

    /// Vertical syncs per frame: 0 off, 1 on.
    #[property]
    pub fn v_sync_count() -> i32 {
        QUALITY.with(|q| q.get().v_sync_count)
    }

    pub fn set_v_sync_count(count: i32) {
        QUALITY.with(|q| {
            let mut v = q.get();
            v.v_sync_count = count;
            q.set(v);
        });
    }
}
