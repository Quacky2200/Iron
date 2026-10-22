//! `Spline`: a path through knots (`iron_math::spline`), the script API of `js/engine/src/spline.ts`
//! and the compiled facade made native. A `Spline` is a handle to a spline kept here per thread, so
//! compiled scripts and QuickJS call the same code; [`reset`] forgets every spline (a reload).

use iron_api::{Handle, iron_api};
use iron_math::V3;
use iron_math::spline::{Knot, Spline, SplineHit, SplineNearest, Tangent};
use std::cell::RefCell;

macro_rules! numbered_handle {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name(pub u32);

        impl Handle for $name {
            fn to_raw(self) -> u64 {
                u64::from(self.0)
            }
            fn from_raw(raw: u64) -> Self {
                $name(raw as u32)
            }
        }
    };
}

numbered_handle!(
    /// A `Spline` by number (1-based: 0 is "none").
    SplineRef
);
#[derive(Default)]
struct State {
    splines: Vec<Option<Spline>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn st<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Forgets every spline (a reload, or a test).
pub fn reset() {
    st(|s| *s = State::default());
}

/// Runs `f` on a spline (`None`: no such spline).
pub fn with_spline<R>(r: SplineRef, f: impl FnOnce(&mut Spline) -> R) -> Option<R> {
    st(|st| {
        st.splines
            .get_mut((r.0 as usize).checked_sub(1)?)?
            .as_mut()
            .map(f)
    })
}

/// Keeps a spline and returns its handle.
pub fn add_spline(spline: Spline) -> SplineRef {
    st(|st| {
        st.splines.push(Some(spline));
        SplineRef(st.splines.len() as u32)
    })
}

#[iron_api(
    namespace = "Spline", quickjs, like,
    handles(Spline = SplineRef),
    values(Vector3 = V3 [x: f64, y: f64, z: f64]),
    enums(
        /// `auto`: smooth through the neighbours; `bezier`: the knot's own handles; `linear`: none.
        TangentMode = Tangent [Auto, Bezier, Linear]
    ),
    records(
        /// One knot of a `Spline` (a copy: edit a spline with its methods).
        SplineKnot = Knot [
            position: Vector3,
            /// The handle before the knot, relative to it (used in `bezier` mode).
            tangent_in: Vector3,
            /// The handle after the knot, relative to it (used in `bezier` mode).
            tangent_out: Vector3,
            mode: TangentMode
        ],
        /// Where `t` (0 to 1 by distance) falls on a spline: a segment and its own parameter.
        SplineHit = SplineHit [segment: f64, u: f64],
        /// The nearest point on a spline: its `t` (by distance), the point and how far.
        SplineNearest = SplineNearest [t: f64, point: Vector3, distance: f64]
    )
)]
pub mod spline_api {
    use super::*;

    /// A path through knots (Unity's Splines package, the core of it): smooth unless told
    /// otherwise, every segment a cubic Bezier. `evaluate(t)` goes by distance (t 0 to 1 along
    /// the whole length) so things move at a steady speed; `segmentPoint(i, u)` goes by each
    /// segment's own parameter.
    impl SplineRef {
        /// `new Spline(points?, { closed?, tension?, mode? })`: a knot at each point, all in `mode`.
        #[options(after = points)]
        #[optional(points, closed = false, tension = 0.5, mode)]
        pub fn create(points: &[V3], closed: bool, tension: f64, mode: Tangent) -> SplineRef {
            add_spline(Spline::new(points, closed, tension, mode))
        }

        /// Forgets the spline (a handle is a number, nothing frees it by itself).
        pub fn destroy(self) {
            st(|st| {
                if let Some(slot) = (self.0 as usize)
                    .checked_sub(1)
                    .and_then(|i| st.splines.get_mut(i))
                {
                    *slot = None;
                }
            });
        }

        /// A copy of every knot (edit with `setPosition`, `setTangents`, `setMode`, ...).
        #[property]
        pub fn knots(self) -> Vec<Knot> {
            with_spline(self, |s| s.knots().to_vec()).unwrap_or_default()
        }

        #[property]
        pub fn count(self) -> f64 {
            with_spline(self, |s| s.count() as f64).unwrap_or(0.0)
        }

        /// Whether the last knot joins back to the first.
        #[property]
        pub fn closed(self) -> bool {
            with_spline(self, |s| s.closed()).unwrap_or(false)
        }

        pub fn set_closed(self, on: bool) {
            with_spline(self, |s| s.set_closed(on));
        }

        /// How strongly `auto` knots bend: 0 is straight lines, 0.5 Catmull-Rom.
        #[property]
        pub fn tension(self) -> f64 {
            with_spline(self, |s| s.tension()).unwrap_or(0.5)
        }

        pub fn set_tension(self, t: f64) {
            with_spline(self, |s| s.set_tension(t));
        }

        #[property]
        pub fn segment_count(self) -> f64 {
            with_spline(self, |s| s.segment_count() as f64).unwrap_or(0.0)
        }

        /// Goes up on every change, so anything drawing the spline can tell it moved.
        #[property]
        pub fn version(self) -> f64 {
            with_spline(self, |s| s.version()).unwrap_or(0.0)
        }

        /// The whole length.
        #[property]
        pub fn length(self) -> f64 {
            with_spline(self, |s| s.length()).unwrap_or(0.0)
        }

        /// Adds a knot at the end; returns its index.
        #[optional(mode, tangent_in, tangent_out)]
        pub fn add_knot(self, position: V3, mode: Tangent, tangent_in: V3, tangent_out: V3) -> f64 {
            self.insert_knot(f64::INFINITY, position, mode, tangent_in, tangent_out)
        }

        /// Puts a knot before index `index` (the count: at the end); returns its index.
        #[optional(mode, tangent_in, tangent_out)]
        pub fn insert_knot(
            self,
            index: f64,
            position: V3,
            mode: Tangent,
            tangent_in: V3,
            tangent_out: V3,
        ) -> f64 {
            // A handle left out is NaN in every field.
            let given = |v: V3| (!v.x.is_nan()).then_some(v);
            with_spline(self, |s| {
                s.insert_knot(index, position, mode, given(tangent_in), given(tangent_out)) as f64
            })
            .unwrap_or(0.0)
        }

        pub fn remove_knot(self, index: f64) {
            with_spline(self, |s| s.remove_knot(index));
        }

        pub fn set_position(self, index: f64, position: V3) {
            with_spline(self, |s| s.set_position(index, position));
        }

        /// Sets a knot's handles (relative to it) and makes it a `bezier` knot.
        pub fn set_tangents(self, index: f64, tangent_in: V3, tangent_out: V3) {
            with_spline(self, |s| s.set_tangents(index, tangent_in, tangent_out));
        }

        /// Changes a knot's mode; a knot turning `bezier` keeps the handles it had (auto ones included).
        pub fn set_mode(self, index: f64, mode: Tangent) {
            with_spline(self, |s| s.set_mode(index, mode));
        }

        /// A knot's handle after it, relative to it, whatever its mode (zero when straight).
        pub fn handle_out(self, index: f64) -> V3 {
            with_spline(self, |s| s.handle_out(index)).unwrap_or(V3::ZERO)
        }

        /// A knot's handle before it, relative to it, whatever its mode.
        pub fn handle_in(self, index: f64) -> V3 {
            with_spline(self, |s| s.handle_in(index)).unwrap_or(V3::ZERO)
        }

        /// A point on segment `i` (0 to `segmentCount` - 1), `u` 0 to 1 by the segment's own parameter.
        pub fn segment_point(self, i: f64, u: f64) -> V3 {
            with_spline(self, |s| s.segment_point(i, u)).unwrap_or(V3::ZERO)
        }

        /// The direction (not normalized) segment `i` is going at `u`.
        pub fn segment_derivative(self, i: f64, u: f64) -> V3 {
            with_spline(self, |s| s.segment_derivative(i, u)).unwrap_or(V3::ZERO)
        }

        /// Where `t` (0 to 1 by distance) falls: a segment and its own parameter.
        pub fn locate(self, t: f64) -> SplineHit {
            with_spline(self, |s| s.locate(t)).unwrap_or(SplineHit {
                segment: 0.0,
                u: 0.0,
            })
        }

        /// The point `t` of the way along (0 to 1, by distance).
        pub fn evaluate(self, t: f64) -> V3 {
            with_spline(self, |s| s.evaluate(t)).unwrap_or(V3::ZERO)
        }

        /// The direction the spline goes at `t` (by distance), normalized.
        pub fn tangent(self, t: f64) -> V3 {
            with_spline(self, |s| s.tangent(t)).unwrap_or(V3::ZERO)
        }

        /// Points along the whole spline, `perSegment` to a segment; open ones include the last knot.
        #[optional(per_segment = 16)]
        pub fn sample(self, per_segment: f64) -> Vec<V3> {
            with_spline(self, |s| s.sample(per_segment)).unwrap_or_default()
        }

        /// The nearest point on the spline to `p`: its `t` (by distance), the point and how far.
        pub fn nearest(self, p: V3) -> SplineNearest {
            with_spline(self, |s| s.nearest(p)).unwrap_or(SplineNearest {
                t: 0.0,
                point: V3::ZERO,
                distance: 0.0,
            })
        }
    }
}
