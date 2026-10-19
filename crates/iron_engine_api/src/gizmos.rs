//! `Gizmos`: immediate-mode debug lines. A script clears, sets the colour, draws lines and calls
//! `present(camera)`; the host takes the finished frame with [`take_frame`] and draws it as thin
//! lines, so no mesh is built in script each frame.

use crate::camera::CameraRef;
use crate::color::Rgba;
use glam::Vec3;
use iron_api::iron_api;
use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq)]
pub struct GizmoLine {
    pub from: Vec3,
    pub to: Vec3,
    pub color: Rgba,
}

/// The lines of one `present`, for the camera it named.
#[derive(Clone, Debug, PartialEq)]
pub struct GizmoFrame {
    pub camera: CameraRef,
    pub lines: Vec<GizmoLine>,
}

struct State {
    color: Rgba,
    pending: Vec<GizmoLine>,
    presented: Option<GizmoFrame>,
}

const WHITE: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

thread_local! {
    static STATE: RefCell<State> = const {
        RefCell::new(State { color: WHITE, pending: Vec::new(), presented: None })
    };
}

/// The last presented frame, once (the host draws it until the script presents another).
pub fn take_frame() -> Option<GizmoFrame> {
    STATE.with(|s| s.borrow_mut().presented.take())
}

pub fn reset() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.color = WHITE;
        s.pending.clear();
        s.presented = None;
    });
}

#[iron_api(
    namespace = "Gizmos",
    quickjs,
    handles(Camera = CameraRef),
    values(
        Vector3 = Vec3 [x: f32, y: f32, z: f32],
        Color = Rgba [r: f32, g: f32, b: f32, a: f32]
    )
)]
pub mod gizmos_api {
    use super::{GizmoFrame, GizmoLine, STATE};
    use crate::camera::CameraRef;
    use crate::color::Rgba;
    use glam::Vec3;

    /// The colour of the lines drawn next.
    #[property]
    pub fn color() -> Rgba {
        STATE.with(|s| s.borrow().color)
    }

    pub fn set_color(c: Rgba) {
        STATE.with(|s| s.borrow_mut().color = c);
    }

    /// Forgets the lines drawn so far.
    pub fn clear() {
        STATE.with(|s| s.borrow_mut().pending.clear());
    }

    pub fn draw_line(from: Vec3, to: Vec3) {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let color = s.color;
            s.pending.push(GizmoLine { from, to, color });
        });
    }

    /// Hands the lines drawn since `clear` to the renderer for this camera.
    pub fn present(camera: CameraRef) {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let lines = std::mem::take(&mut s.pending);
            s.presented = Some(GizmoFrame { camera, lines });
        });
    }
}
