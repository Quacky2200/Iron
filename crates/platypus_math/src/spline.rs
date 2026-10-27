// SPDX-License-Identifier: Apache-2.0

//! Splines (Unity's Splines package, the core of it): a path through knots, smooth unless told
//! otherwise. A knot's tangents are automatic (a cardinal spline through its neighbours; `tension`
//! 0.5 is Catmull-Rom, like GDI's curves), set by hand as Bezier handles, or left out (straight
//! segments). Every segment is a cubic Bezier.
//!
//! `evaluate(t)` goes by distance (t 0 to 1 along the whole length), so things move along a spline
//! at a steady speed; `segment_point(i, u)` goes by each segment's own parameter. Ported from
//! `js/engine/src/spline.ts` and the compiled facade, in the same order of operations, so the
//! numbers are bit for bit what they were. Plain maths: `platypus_engine_api::spline` makes it
//! the script `Spline`.

use crate::jsnum;
use crate::math::{V3, v3};

/// `auto`: smooth through the neighbours; `bezier`: the knot's own handles; `linear`: none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tangent {
    #[default]
    Auto,
    Bezier,
    Linear,
}

/// One knot: its position and its handles (relative to it; used in `bezier` mode).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knot {
    pub position: V3,
    pub tangent_in: V3,
    pub tangent_out: V3,
    pub mode: Tangent,
}

/// A knot as scripts see it (`SplineKnot`).
pub type KnotView = Knot;

/// Where `t` (0 to 1 by distance) falls: a segment and its own parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineHit {
    pub segment: f64,
    pub u: f64,
}

/// The nearest point on a spline to a position: its `t` (by distance), the point and how far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplineNearest {
    pub t: f64,
    pub point: V3,
    pub distance: f64,
}

/// Length-table samples per segment (more: closer to true distances).
const STEPS: usize = 32;

#[derive(Clone, Debug)]
pub struct Spline {
    knots: Vec<Knot>,
    closed: bool,
    tension: f64,
    /// Distance along the spline at each table sample (`STEPS` per segment, plus the start).
    table: Option<Vec<f64>>,
    /// Every segment's four Bezier points as plain numbers (12 each).
    ctrl: Option<Vec<f64>>,
    /// Goes up on every change, so anything drawing the spline can tell it moved.
    version: f64,
}

impl Default for Spline {
    fn default() -> Self {
        Spline {
            knots: Vec::new(),
            closed: false,
            tension: 0.5,
            table: None,
            ctrl: None,
            version: 0.0,
        }
    }
}

fn copy(v: V3) -> V3 {
    v3(v.x, v.y, v.z)
}

/// The knot index a script number names, if it is one.
fn index_of(index: f64, len: usize) -> Option<usize> {
    (index >= 0.0 && index < len as f64 && index.fract() == 0.0).then_some(index as usize)
}

impl Spline {
    /// A spline through `points` (every knot in `mode`).
    pub fn new(points: &[V3], closed: bool, tension: f64, mode: Tangent) -> Spline {
        let mut s = Spline {
            closed,
            tension,
            ..Spline::default()
        };
        for p in points {
            s.add_knot(*p, mode, None, None);
        }
        s
    }

    pub fn knots(&self) -> &[Knot] {
        &self.knots
    }

    pub fn count(&self) -> usize {
        self.knots.len()
    }

    pub fn version(&self) -> f64 {
        self.version
    }

    pub fn closed(&self) -> bool {
        self.closed
    }

    pub fn set_closed(&mut self, on: bool) {
        self.closed = on;
        self.changed();
    }

    pub fn tension(&self) -> f64 {
        self.tension
    }

    pub fn set_tension(&mut self, t: f64) {
        self.tension = t;
        self.changed();
    }

    pub fn segment_count(&self) -> usize {
        let n = self.knots.len();
        if n < 2 {
            0
        } else if self.closed {
            n
        } else {
            n - 1
        }
    }

    /// Adds a knot at the end; returns its index.
    pub fn add_knot(
        &mut self,
        position: V3,
        mode: Tangent,
        tangent_in: Option<V3>,
        tangent_out: Option<V3>,
    ) -> usize {
        self.insert_knot(
            self.knots.len() as f64,
            position,
            mode,
            tangent_in,
            tangent_out,
        )
    }

    /// Puts a knot before index `index` (the count: at the end); returns its index.
    pub fn insert_knot(
        &mut self,
        index: f64,
        position: V3,
        mode: Tangent,
        tangent_in: Option<V3>,
        tangent_out: Option<V3>,
    ) -> usize {
        let i = jsnum::max(0.0, jsnum::min(self.knots.len() as f64, index.trunc()));
        let i = if i.is_nan() { 0 } else { i as usize };
        let out = tangent_out.map_or(v3(0.0, 0.0, 0.0), copy);
        let tin = tangent_in.map_or_else(|| out.scale(-1.0), copy);
        self.knots.insert(
            i,
            Knot {
                position: copy(position),
                tangent_in: tin,
                tangent_out: out,
                mode,
            },
        );
        self.changed();
        i
    }

    pub fn remove_knot(&mut self, index: f64) {
        if let Some(i) = index_of(index, self.knots.len()) {
            self.knots.remove(i);
            self.changed();
        }
    }

    pub fn set_position(&mut self, index: f64, position: V3) {
        if let Some(i) = index_of(index, self.knots.len()) {
            self.knots[i].position = copy(position);
            self.changed();
        }
    }

    /// Sets a knot's handles (relative to it) and makes it a `bezier` knot.
    pub fn set_tangents(&mut self, index: f64, tangent_in: V3, tangent_out: V3) {
        if let Some(i) = index_of(index, self.knots.len()) {
            let k = &mut self.knots[i];
            k.tangent_in = copy(tangent_in);
            k.tangent_out = copy(tangent_out);
            k.mode = Tangent::Bezier;
            self.changed();
        }
    }

    /// Changes a knot's mode; a knot turning `bezier` keeps the handles it had (auto ones included).
    pub fn set_mode(&mut self, index: f64, mode: Tangent) {
        let Some(i) = index_of(index, self.knots.len()) else {
            return;
        };
        if self.knots[i].mode == mode {
            return;
        }
        if mode == Tangent::Bezier && self.knots[i].mode != Tangent::Bezier {
            let (hin, hout) = (self.handle_in(i as f64), self.handle_out(i as f64));
            self.knots[i].tangent_in = hin;
            self.knots[i].tangent_out = hout;
        }
        self.knots[i].mode = mode;
        self.changed();
    }

    /// A knot's handle after it, relative to it, whatever its mode (zero when straight).
    pub fn handle_out(&self, index: f64) -> V3 {
        let Some(i) = index_of(index, self.knots.len()) else {
            return v3(0.0, 0.0, 0.0);
        };
        let k = &self.knots[i];
        match k.mode {
            Tangent::Bezier => k.tangent_out,
            // Straight: a third of the way to the next knot, so the segment keeps an even speed.
            Tangent::Linear => self.neighbour(i, 1).sub(k.position).scale(1.0 / 3.0),
            // GDI+ cardinal handles: a third of `tension` times the chord across the knot.
            Tangent::Auto => self
                .neighbour(i, 1)
                .sub(self.neighbour(i, -1))
                .scale(self.tension / 3.0),
        }
    }

    /// A knot's handle before it, relative to it, whatever its mode.
    pub fn handle_in(&self, index: f64) -> V3 {
        let Some(i) = index_of(index, self.knots.len()) else {
            return v3(0.0, 0.0, 0.0);
        };
        let k = &self.knots[i];
        match k.mode {
            Tangent::Bezier => k.tangent_in,
            Tangent::Linear => self.neighbour(i, -1).sub(k.position).scale(1.0 / 3.0),
            Tangent::Auto => self.handle_out(i as f64).scale(-1.0),
        }
    }

    /// The knot `step` along from `index` (wrapping when closed, else the end knot itself).
    fn neighbour(&self, index: usize, step: i64) -> V3 {
        let n = self.knots.len() as i64;
        let i = index as i64 + step;
        if self.closed {
            self.knots[(((i % n) + n) % n) as usize].position
        } else {
            self.knots[i.clamp(0, n - 1) as usize].position
        }
    }

    fn changed(&mut self) {
        self.table = None;
        self.ctrl = None;
        self.version += 1.0;
    }

    /// Every segment's four Bezier points as plain numbers (12 each), cached until a change.
    fn controls(&mut self) -> &[f64] {
        if self.ctrl.is_none() {
            let n = self.knots.len();
            let segments = self.segment_count();
            let mut c = vec![0.0; segments * 12];
            for i in 0..segments {
                let j = (i + 1) % n;
                let p0 = self.knots[i].position;
                let p3 = self.knots[j].position;
                let h1 = self.handle_out(i as f64);
                let h2 = self.handle_in(j as f64);
                let o = i * 12;
                c[o] = p0.x;
                c[o + 1] = p0.y;
                c[o + 2] = p0.z;
                c[o + 3] = p0.x + h1.x;
                c[o + 4] = p0.y + h1.y;
                c[o + 5] = p0.z + h1.z;
                c[o + 6] = p3.x + h2.x;
                c[o + 7] = p3.y + h2.y;
                c[o + 8] = p3.z + h2.z;
                c[o + 9] = p3.x;
                c[o + 10] = p3.y;
                c[o + 11] = p3.z;
            }
            self.ctrl = Some(c);
        }
        self.ctrl.as_deref().unwrap_or_default()
    }

    /// Segment `i`'s point at `u` (x, y, z).
    fn point_into(&mut self, i: f64, u: f64) -> [f64; 3] {
        let last = self.segment_count() as f64 - 1.0;
        let i = jsnum::max(0.0, jsnum::min(last, i));
        let c = self.controls();
        if i.is_nan() {
            return [f64::NAN; 3];
        }
        let o = i as usize * 12;
        let v = 1.0 - u;
        let a = v * v * v;
        let b = 3.0 * v * v * u;
        let d = 3.0 * v * u * u;
        let e = u * u * u;
        [
            a * c[o] + b * c[o + 3] + d * c[o + 6] + e * c[o + 9],
            a * c[o + 1] + b * c[o + 4] + d * c[o + 7] + e * c[o + 10],
            a * c[o + 2] + b * c[o + 5] + d * c[o + 8] + e * c[o + 11],
        ]
    }

    /// A point on segment `i` (0 to `segment_count` - 1), `u` 0 to 1 by the segment's own parameter.
    pub fn segment_point(&mut self, i: f64, u: f64) -> V3 {
        if self.segment_count() == 0 {
            return self.knots.first().map_or(v3(0.0, 0.0, 0.0), |k| k.position);
        }
        let p = self.point_into(i, u);
        v3(p[0], p[1], p[2])
    }

    fn derivative_axis(c: &[f64], o: usize, axis: usize, u: f64) -> f64 {
        let v = 1.0 - u;
        let a = 3.0 * v * v;
        let b = 6.0 * v * u;
        let e = 3.0 * u * u;
        a * (c[o + 3 + axis] - c[o + axis])
            + b * (c[o + 6 + axis] - c[o + 3 + axis])
            + e * (c[o + 9 + axis] - c[o + 6 + axis])
    }

    /// The direction (not normalized) segment `i` is going at `u`.
    pub fn segment_derivative(&mut self, i: f64, u: f64) -> V3 {
        if self.segment_count() == 0 {
            return v3(0.0, 0.0, 0.0);
        }
        let last = self.segment_count() as f64 - 1.0;
        let i = jsnum::max(0.0, jsnum::min(last, i));
        let c = self.controls();
        if i.is_nan() {
            return v3(f64::NAN, f64::NAN, f64::NAN);
        }
        let o = i as usize * 12;
        let x = Self::derivative_axis(c, o, 0, u);
        let y = Self::derivative_axis(c, o, 1, u);
        let z = Self::derivative_axis(c, o, 2, u);
        // Where the handles sit on the knots, the direction comes from the chord.
        if x * x + y * y + z * z > 1e-18 {
            return v3(x, y, z);
        }
        v3(c[o + 9] - c[o], c[o + 10] - c[o + 1], c[o + 11] - c[o + 2])
    }

    fn lengths(&mut self) -> &[f64] {
        if self.table.is_none() {
            let mut table = vec![0.0];
            let mut total = 0.0;
            for i in 0..self.segment_count() {
                let p = self.point_into(i as f64, 0.0);
                let (mut px, mut py, mut pz) = (p[0], p[1], p[2]);
                for s in 1..=STEPS {
                    let q = self.point_into(i as f64, s as f64 / STEPS as f64);
                    let dx = q[0] - px;
                    let dy = q[1] - py;
                    let dz = q[2] - pz;
                    total += (dx * dx + dy * dy + dz * dz).sqrt();
                    table.push(total);
                    (px, py, pz) = (q[0], q[1], q[2]);
                }
            }
            self.table = Some(table);
        }
        self.table.as_deref().unwrap_or_default()
    }

    /// The whole length.
    pub fn length(&mut self) -> f64 {
        self.lengths().last().copied().unwrap_or(0.0)
    }

    /// Where `t` (0 to 1 by distance) falls: a segment and its own parameter.
    pub fn locate(&mut self, t: f64) -> SplineHit {
        let segments = self.segment_count();
        if segments == 0 {
            return SplineHit {
                segment: 0.0,
                u: 0.0,
            };
        }
        let table = self.lengths();
        let total = table[table.len() - 1];
        let want = jsnum::max(0.0, jsnum::min(1.0, t)) * total;
        // The first sample at or past `want` (binary search over the increasing distances).
        let mut lo = 0usize;
        let mut hi = table.len() - 1;
        while lo < hi {
            let mid = (lo + hi) >> 1;
            if table[mid] < want {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let j = jsnum::max(1.0, lo as f64) as usize;
        let a = table[j - 1];
        let b = table[j];
        let frac = if b > a { (want - a) / (b - a) } else { 0.0 };
        let step = (j - 1) as f64 + frac;
        let segment = jsnum::min((segments - 1) as f64, (step / STEPS as f64).floor());
        SplineHit {
            segment,
            u: (step - segment * STEPS as f64) / STEPS as f64,
        }
    }

    /// The point `t` of the way along (0 to 1, by distance).
    pub fn evaluate(&mut self, t: f64) -> V3 {
        let h = self.locate(t);
        self.segment_point(h.segment, h.u)
    }

    /// The direction the spline goes at `t` (by distance), normalized.
    pub fn tangent(&mut self, t: f64) -> V3 {
        let h = self.locate(t);
        self.segment_derivative(h.segment, h.u).normalized()
    }

    /// Points along the whole spline, `per_segment` to a segment; open ones include the last knot.
    pub fn sample(&mut self, per_segment: f64) -> Vec<V3> {
        let segments = self.segment_count();
        if segments == 0 {
            return self.knots.iter().map(|k| k.position).collect();
        }
        let mut out = Vec::new();
        for i in 0..segments {
            let mut s = 0.0;
            while s < per_segment {
                out.push(self.segment_point(i as f64, s / per_segment));
                s += 1.0;
            }
        }
        if !self.closed {
            out.push(self.knots[self.knots.len() - 1].position);
        }
        out
    }

    fn distance_at(&mut self, t: f64, target: V3) -> f64 {
        let h = self.locate(t);
        let p = self.point_into(h.segment, h.u);
        let dx = p[0] - target.x;
        let dy = p[1] - target.y;
        let dz = p[2] - target.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// The nearest point on the spline to `p`: its `t` (by distance), the point and how far.
    pub fn nearest(&mut self, p: V3) -> SplineNearest {
        let target = copy(p);
        let total = self.length();
        if self.segment_count() == 0 || total == 0.0 {
            let point = self.knots.first().map_or(v3(0.0, 0.0, 0.0), |k| k.position);
            return SplineNearest {
                t: 0.0,
                point,
                distance: jsnum::hypot(&[
                    point.x - target.x,
                    point.y - target.y,
                    point.z - target.z,
                ]),
            };
        }
        // Coarse: every table sample; then narrow down around the best.
        let samples = (self.segment_count() * STEPS) as f64;
        let mut best = 0.0;
        let mut best_d = f64::INFINITY;
        let mut s = 0.0;
        while s <= samples {
            let d = self.distance_at(s / samples, target);
            if d < best_d {
                best_d = d;
                best = s / samples;
            }
            s += 1.0;
        }
        let mut step = 1.0 / samples;
        for _ in 0..20 {
            step /= 2.0;
            for side in 0..2 {
                let t = if side == 0 { best - step } else { best + step };
                let c = if self.closed {
                    ((t % 1.0) + 1.0) % 1.0
                } else {
                    jsnum::max(0.0, jsnum::min(1.0, t))
                };
                let d = self.distance_at(c, target);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
        }
        let point = self.evaluate(best);
        SplineNearest {
            t: best,
            point,
            distance: jsnum::hypot(&[point.x - target.x, point.y - target.y, point.z - target.z]),
        }
    }
}

/// How `SplineAnimate` runs along the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LoopMode {
    #[default]
    Loop,
    PingPong,
    Once,
}

/// How `SplineAnimate` turns its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Alignment {
    None,
    #[default]
    Path,
}

/// How far along the path a `SplineAnimate` is after `elapsed` seconds of a `duration`-second run
/// (0 to 1, by distance), and whether it is going back (the return leg of a ping-pong).
pub fn animate_t(elapsed: f64, duration: f64, mode: LoopMode) -> (f64, bool) {
    let d = jsnum::max(1e-6, duration);
    let x = elapsed / d;
    match mode {
        LoopMode::Once => (jsnum::max(0.0, jsnum::min(1.0, x)), false),
        LoopMode::PingPong => {
            let m = ((x % 2.0) + 2.0) % 2.0;
            (if m > 1.0 { 2.0 - m } else { m }, m > 1.0)
        }
        LoopMode::Loop => (((x % 1.0) + 1.0) % 1.0, false),
    }
}

/// Where a `SplineAnimate` puts its object: the point and the direction along the path (turned
/// round on the return leg of a ping-pong), both in the spline's space.
pub fn animate_pose(spline: &mut Spline, elapsed: f64, duration: f64, mode: LoopMode) -> (V3, V3) {
    let (t, back) = animate_t(elapsed, duration, mode);
    let at = spline.evaluate(t);
    let mut dir = spline.tangent(t);
    // Going back along a ping-pong, face the other way.
    if mode == LoopMode::PingPong && back {
        dir = dir.scale(-1.0);
    }
    (at, dir)
}
