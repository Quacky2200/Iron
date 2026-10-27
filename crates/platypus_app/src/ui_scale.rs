// SPDX-License-Identifier: Apache-2.0

//! UI scale (s&box's ScreenPanel scaling): screen UI is laid out in "UI pixels" and
//! drawn bigger or smaller, so a menu designed for 1080p fills a 4K window the same.
//! Set from scripts with `setUiScale` in `@iron/engine`.

use iron_core::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum UiScale {
    /// One UI pixel per logical (DPI-scaled) pixel.
    #[default]
    None,
    /// A fixed zoom.
    Fixed(f32),
    /// Scale so the window is this many UI pixels high (s&box targets 1080).
    Height(f32),
    /// Scale so a `width` × `height` design fits inside the window.
    Fit(f32, f32),
}

/// Never shrink UI below a quarter or grow it past 8×: a tiny window shouldn't make
/// text unreadable or a huge factor explode the glyph atlas.
const LIMITS: (f32, f32) = (0.25, 8.0);

impl UiScale {
    /// UI pixels → logical pixels, for a window `width` × `height` logical pixels.
    pub fn factor(self, width: f32, height: f32) -> f32 {
        let f = match self {
            UiScale::None => 1.0,
            UiScale::Fixed(f) => f,
            UiScale::Height(h) => height / h,
            UiScale::Fit(w, h) => (width / w).min(height / h),
        };
        if f.is_finite() && f > 0.0 {
            f.clamp(LIMITS.0, LIMITS.1)
        } else {
            1.0
        }
    }

    /// From the `uiScale` op: `(mode, a, b)` — `"none"`, `"fixed", f`, `"height", h` or
    /// `"fit", w, h`. `None` for anything else.
    pub fn from_op(args: &[Value]) -> Option<Self> {
        let num = |i: usize| {
            args.get(i)
                .and_then(Value::as_f64)
                .map(|n| n as f32)
                .filter(|n| n.is_finite() && *n > 0.0)
        };
        Some(match args.first()?.as_str()? {
            "none" => UiScale::None,
            "fixed" => UiScale::Fixed(num(1)?),
            "height" => UiScale::Height(num(1)?),
            "fit" => UiScale::Fit(num(1)?, num(2)?),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M3-GAME-02
    #[test]
    fn modes_scale_to_the_window() {
        assert_eq!(UiScale::None.factor(1920.0, 1080.0), 1.0);
        assert_eq!(UiScale::Fixed(1.5).factor(800.0, 600.0), 1.5);
        assert_eq!(UiScale::Height(1080.0).factor(3840.0, 2160.0), 2.0);
        assert_eq!(
            UiScale::Height(1080.0).factor(1280.0, 720.0),
            720.0 / 1080.0
        );
        // A wide window fits by height, a tall one by width.
        assert_eq!(UiScale::Fit(1920.0, 1080.0).factor(3840.0, 1080.0), 1.0);
        assert_eq!(UiScale::Fit(1920.0, 1080.0).factor(960.0, 1080.0), 0.5);
        assert_eq!(UiScale::Height(1080.0).factor(100.0, 10.0), 0.25, "clamped");
        assert_eq!(UiScale::Height(1080.0).factor(100.0, 0.0), 1.0, "minimised");
    }

    // M3-GAME-02
    #[test]
    fn op_arguments() {
        let s = |v: &str| Value::from(v);
        assert_eq!(UiScale::from_op(&[s("none")]), Some(UiScale::None));
        assert_eq!(
            UiScale::from_op(&[s("height"), Value::Number(1080.0)]),
            Some(UiScale::Height(1080.0))
        );
        assert_eq!(
            UiScale::from_op(&[s("fit"), Value::Number(1280.0), Value::Number(720.0)]),
            Some(UiScale::Fit(1280.0, 720.0))
        );
        assert_eq!(UiScale::from_op(&[s("fixed"), Value::Number(-1.0)]), None);
        assert_eq!(UiScale::from_op(&[s("zoom")]), None);
    }
}
