//! CSS transforms: parsing `transform` and the individual `translate` / `rotate` /
//! `scale` properties into 4×4 matrices, and the 3×3 homographies the draw list uses.
//!
//! Every element is a flat plane. A 4×4 transform (with `perspective`) maps that
//! plane into its parent's space; dropping z ("flattening", CSS's default
//! `transform-style: flat`) leaves a 3×3 homography from local (x, y) to the parent's
//! (x, y), which maps straight lines to straight lines and inverts exactly. So
//! drawing and hit testing only ever need homographies; `preserve-3d` just delays
//! the flattening (the 4×4s multiply first).
//!
//! Coordinates are window pixels, y down, as layout produces them; matrices act on
//! column vectors (`m[row][col]`).

use crate::color::parse_angle_degrees;
use crate::paint::{Len, parse_background_position, split_top_level};

/// A 4×4 matrix, `m[row][col]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    pub fn translate(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[0][3] = x;
        m.0[1][3] = y;
        m.0[2][3] = z;
        m
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[0][0] = x;
        m.0[1][1] = y;
        m.0[2][2] = z;
        m
    }

    /// `perspective(d)`: things further away (negative z) shrink towards the origin.
    pub fn perspective(d: f32) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[3][2] = -1.0 / d.max(1.0);
        m
    }

    /// Rotation by `deg` about the axis (x, y, z) (CSS `rotate3d`; positive angles
    /// turn clockwise when looking along the axis towards the viewer, as in CSS).
    pub fn rotate(x: f32, y: f32, z: f32, deg: f32) -> Mat4 {
        let len = (x * x + y * y + z * z).sqrt();
        if len == 0.0 {
            return Self::IDENTITY;
        }
        let (x, y, z) = (x / len, y / len, z / len);
        let (s, c) = deg.to_radians().sin_cos();
        let t = 1.0 - c;
        Mat4([
            [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
            [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
            [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub fn skew(ax_deg: f32, ay_deg: f32) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[0][1] = ax_deg.to_radians().tan();
        m.0[1][0] = ay_deg.to_radians().tan();
        m
    }

    /// `self` then `o` applied to a point is `self * o` (`o` acts first).
    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [[0.0; 4]; 4];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..4).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        Mat4(r)
    }

    pub fn inverse(&self) -> Option<Mat4> {
        // Gauss-Jordan with partial pivoting.
        let mut a = self.0;
        let mut inv = Self::IDENTITY.0;
        for col in 0..4 {
            let pivot = (col..4).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
            if a[pivot][col].abs() < 1e-12 {
                return None;
            }
            a.swap(col, pivot);
            inv.swap(col, pivot);
            let p = a[col][col];
            for k in 0..4 {
                a[col][k] /= p;
                inv[col][k] /= p;
            }
            for row in 0..4 {
                if row != col {
                    let f = a[row][col];
                    for k in 0..4 {
                        a[row][k] -= f * a[col][k];
                        inv[row][k] -= f * inv[col][k];
                    }
                }
            }
        }
        Some(Mat4(inv))
    }

    /// The homography of the z = 0 plane: rows and columns x, y, w.
    pub fn flatten(&self) -> Homography {
        let m = &self.0;
        let idx = [0, 1, 3];
        Homography(idx.map(|r| idx.map(|c| m[r][c])))
    }

    /// Whether the plane faces away from the viewer (CSS: the inverse's m33 is
    /// negative), for `backface-visibility: hidden`.
    pub fn is_backface(&self) -> bool {
        self.inverse().is_some_and(|i| i.0[2][2] < 0.0)
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
}

/// A 3×3 projective map of the plane, `m[row][col]` on (x, y, 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homography(pub [[f32; 3]; 3]);

impl Homography {
    pub const IDENTITY: Homography =
        Homography([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    pub fn mul(&self, o: &Homography) -> Homography {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        Homography(r)
    }

    /// `(s, tx, ty)` when this only scales evenly by `s > 0` and moves by (tx, ty): no
    /// rotation, skew, uneven scale or perspective. Text under such a map can be drawn
    /// as plain text at the new size.
    pub fn as_scale_translate(&self) -> Option<(f32, f32, f32)> {
        let m = &self.0;
        let w = m[2][2];
        if w.abs() < 1e-6 || m[2][0].abs() > 1e-6 || m[2][1].abs() > 1e-6 {
            return None;
        }
        let (sx, sy) = (m[0][0] / w, m[1][1] / w);
        let even = (sx - sy).abs() <= 1e-4 * sx.abs().max(1.0);
        let axis_aligned = (m[0][1] / w).abs() < 1e-5 && (m[1][0] / w).abs() < 1e-5;
        (sx > 1e-4 && even && axis_aligned).then_some((sx, m[0][2] / w, m[1][2] / w))
    }

    pub fn inverse(&self) -> Option<Homography> {
        let m = &self.0;
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = m[1][2] * m[2][0] - m[1][0] * m[2][2];
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
        if det.abs() < 1e-12 {
            return None;
        }
        let d = 1.0 / det;
        Some(Homography([
            [
                c00 * d,
                (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * d,
                (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * d,
            ],
            [
                c01 * d,
                (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * d,
                (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * d,
            ],
            [
                c02 * d,
                (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * d,
                (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * d,
            ],
        ]))
    }

    /// Maps a point; `None` when it lands at or behind the viewer (w ≤ 0).
    pub fn apply(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        let m = &self.0;
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        (w > 1e-6).then(|| {
            (
                (m[0][0] * x + m[0][1] * y + m[0][2]) / w,
                (m[1][0] * x + m[1][1] * y + m[1][2]) / w,
            )
        })
    }

    /// The axis-aligned bounds of `r` mapped (for culling and scissors). A corner
    /// behind the viewer makes the bounds unlimited.
    pub fn map_rect(&self, r: crate::Rect) -> crate::Rect {
        let corners = [
            (r.x, r.y),
            (r.x + r.width, r.y),
            (r.x, r.y + r.height),
            (r.x + r.width, r.y + r.height),
        ];
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for (x, y) in corners {
            let Some((x, y)) = self.apply(x, y) else {
                return crate::Rect::new(-1e6, -1e6, 2e6, 2e6);
            };
            lo = (lo.0.min(x), lo.1.min(y));
            hi = (hi.0.max(x), hi.1.max(y));
        }
        crate::Rect::new(lo.0, lo.1, hi.0 - lo.0, hi.1 - lo.1)
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
}

/// One `transform` function, with lengths still unresolved (percentages need the
/// element's size).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformFn {
    /// A 4×4 matrix with no unresolved lengths (`matrix`, `matrix3d`, rotations,
    /// scales, skews, `perspective`, `translateZ`).
    Fixed(Mat4),
    /// `translate*`: x and y may be percentages of the border box; z is px.
    Translate(Len, Len, f32),
}

/// What an element's transform properties say, before its size is known.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransformSpec {
    /// `transform`, in order (empty: `none`).
    pub functions: Vec<TransformFn>,
    /// The individual properties, applied before `transform`: translate, rotate,
    /// scale, in that order.
    pub translate: Option<(Len, Len, f32)>,
    pub rotate: Option<Mat4>,
    pub scale: Option<Mat4>,
}

impl TransformSpec {
    pub fn is_none(&self) -> bool {
        self.functions.is_empty()
            && self.translate.is_none()
            && self.rotate.is_none()
            && self.scale.is_none()
    }

    /// The element's own 4×4 transform for its border box `rect`, about `origin`
    /// (x, y as lengths of the box, z in px). `None` for no transform.
    pub fn matrix(&self, rect: crate::Rect, origin: (Len, Len, f32)) -> Option<Mat4> {
        if self.is_none() {
            return None;
        }
        let translate = |x: Len, y: Len, z: f32| {
            Mat4::translate(x.resolve(rect.width), y.resolve(rect.height), z)
        };
        let (ox, oy, oz) = (
            rect.x + origin.0.resolve(rect.width),
            rect.y + origin.1.resolve(rect.height),
            origin.2,
        );
        let mut m = Mat4::translate(ox, oy, oz);
        if let Some((x, y, z)) = self.translate {
            m = m.mul(&translate(x, y, z));
        }
        if let Some(r) = &self.rotate {
            m = m.mul(r);
        }
        if let Some(s) = &self.scale {
            m = m.mul(s);
        }
        for f in &self.functions {
            m = m.mul(&match *f {
                TransformFn::Fixed(f) => f,
                TransformFn::Translate(x, y, z) => translate(x, y, z),
            });
        }
        Some(m.mul(&Mat4::translate(-ox, -oy, -oz)))
    }
}

/// All of an element's transform-related properties (shared, as most elements have
/// none: `Visual::transform` is `None` then).
#[derive(Clone, Debug, PartialEq)]
pub struct Transforms {
    pub spec: TransformSpec,
    /// `transform-origin`: x, y lengths of the border box, z in px.
    pub origin: (Len, Len, f32),
    /// `perspective` (px) for this element's transformed children; `None` for none.
    pub perspective: Option<f32>,
    pub perspective_origin: (Len, Len),
    /// `transform-style: preserve-3d`.
    pub preserve_3d: bool,
    /// `backface-visibility: hidden`.
    pub backface_hidden: bool,
}

impl Default for Transforms {
    fn default() -> Self {
        Self {
            spec: TransformSpec::default(),
            origin: (Len::Percent(0.5), Len::Percent(0.5), 0.0),
            perspective: None,
            perspective_origin: (Len::Percent(0.5), Len::Percent(0.5)),
            preserve_3d: false,
            backface_hidden: false,
        }
    }
}

/// Where drawing (or hit testing) is, transform-wise: `world` maps the current flat
/// plane to the window, and `pending` is 3D transform not flattened yet (inside a
/// `preserve-3d` element).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformContext {
    pub world: Homography,
    pub pending: Mat4,
}

impl TransformContext {
    pub const ROOT: TransformContext = TransformContext {
        world: Homography::IDENTITY,
        pending: Mat4::IDENTITY,
    };
}

/// A plain number argument.
fn number(s: &str) -> Option<f32> {
    s.trim().parse().ok()
}

/// An angle (`10deg`, `0.5turn`, `1rad`, `100grad`; a bare `0` is allowed).
fn angle(s: &str) -> Option<f32> {
    let s = s.trim();
    if s == "0" {
        return Some(0.0);
    }
    s.ends_with(|c: char| c.is_ascii_alphabetic())
        .then(|| parse_angle_degrees(s))
        .flatten()
}

/// A length in px (`translateZ`, `perspective`, `transform-origin` z): no percentages.
fn px_len(s: &str) -> Option<f32> {
    match Len::parse(s)? {
        Len::Px(px) => Some(px),
        Len::Percent(_) => None,
    }
}

/// A scale factor: a number or a percentage.
fn factor(s: &str) -> Option<f32> {
    let s = s.trim();
    match s.strip_suffix('%') {
        Some(p) => Some(number(p)? / 100.0),
        None => number(s),
    }
}

/// `transform`: `none`, or transform functions separated by spaces.
pub fn parse_transform(s: &str) -> Option<Vec<TransformFn>> {
    if s.trim() == "none" {
        return Some(Vec::new());
    }
    split_top_level(s, ' ')
        .into_iter()
        .map(parse_function)
        .collect()
}

fn parse_function(call: &str) -> Option<TransformFn> {
    let (name, args) = call.strip_suffix(')')?.split_once('(')?;
    let args: Vec<&str> = split_top_level(args, ',');
    let fixed = |m: Mat4| Some(TransformFn::Fixed(m));
    let n = args.len();
    match (name.trim().to_ascii_lowercase().as_str(), n) {
        ("matrix", 6) => {
            let v: Vec<f32> = args.iter().map(|a| number(a)).collect::<Option<_>>()?;
            let mut m = Mat4::IDENTITY;
            m.0[0][0] = v[0];
            m.0[1][0] = v[1];
            m.0[0][1] = v[2];
            m.0[1][1] = v[3];
            m.0[0][3] = v[4];
            m.0[1][3] = v[5];
            fixed(m)
        }
        ("matrix3d", 16) => {
            let v: Vec<f32> = args.iter().map(|a| number(a)).collect::<Option<_>>()?;
            // Column-major, as CSS writes it.
            let mut m = [[0.0; 4]; 4];
            for (i, value) in v.iter().enumerate() {
                m[i % 4][i / 4] = *value;
            }
            fixed(Mat4(m))
        }
        ("perspective", 1) => match args[0].trim() {
            "none" => fixed(Mat4::IDENTITY),
            d => {
                let d = px_len(d).filter(|d| *d >= 0.0)?;
                fixed(Mat4::perspective(d))
            }
        },
        ("rotate" | "rotatez", 1) => fixed(Mat4::rotate(0.0, 0.0, 1.0, angle(args[0])?)),
        ("rotatex", 1) => fixed(Mat4::rotate(1.0, 0.0, 0.0, angle(args[0])?)),
        ("rotatey", 1) => fixed(Mat4::rotate(0.0, 1.0, 0.0, angle(args[0])?)),
        ("rotate3d", 4) => fixed(Mat4::rotate(
            number(args[0])?,
            number(args[1])?,
            number(args[2])?,
            angle(args[3])?,
        )),
        ("translate", 1 | 2) => Some(TransformFn::Translate(
            Len::parse(args[0])?,
            args.get(1).map_or(Some(Len::Px(0.0)), |a| Len::parse(a))?,
            0.0,
        )),
        ("translate3d", 3) => Some(TransformFn::Translate(
            Len::parse(args[0])?,
            Len::parse(args[1])?,
            px_len(args[2])?,
        )),
        ("translatex", 1) => Some(TransformFn::Translate(
            Len::parse(args[0])?,
            Len::Px(0.0),
            0.0,
        )),
        ("translatey", 1) => Some(TransformFn::Translate(
            Len::Px(0.0),
            Len::parse(args[0])?,
            0.0,
        )),
        ("translatez", 1) => fixed(Mat4::translate(0.0, 0.0, px_len(args[0])?)),
        ("scale", 1 | 2) => {
            let x = factor(args[0])?;
            let y = args.get(1).map_or(Some(x), |a| factor(a))?;
            fixed(Mat4::scale(x, y, 1.0))
        }
        ("scale3d", 3) => fixed(Mat4::scale(
            factor(args[0])?,
            factor(args[1])?,
            factor(args[2])?,
        )),
        ("scalex", 1) => fixed(Mat4::scale(factor(args[0])?, 1.0, 1.0)),
        ("scaley", 1) => fixed(Mat4::scale(1.0, factor(args[0])?, 1.0)),
        ("scalez", 1) => fixed(Mat4::scale(1.0, 1.0, factor(args[0])?)),
        ("skew", 1 | 2) => fixed(Mat4::skew(
            angle(args[0])?,
            args.get(1).map_or(Some(0.0), |a| angle(a))?,
        )),
        ("skewx", 1) => fixed(Mat4::skew(angle(args[0])?, 0.0)),
        ("skewy", 1) => fixed(Mat4::skew(0.0, angle(args[0])?)),
        _ => None,
    }
}

/// The `translate` property: `none`, or `x [y [z]]`.
pub fn parse_translate(s: &str) -> Option<Option<(Len, Len, f32)>> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    Some(match parts[..] {
        ["none"] => None,
        [x] => Some((Len::parse(x)?, Len::Px(0.0), 0.0)),
        [x, y] => Some((Len::parse(x)?, Len::parse(y)?, 0.0)),
        [x, y, z] => Some((Len::parse(x)?, Len::parse(y)?, px_len(z)?)),
        _ => return None,
    })
}

/// The `rotate` property: `none`, `angle`, `x|y|z angle` or `vx vy vz angle`.
pub fn parse_rotate(s: &str) -> Option<Option<Mat4>> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    Some(match parts[..] {
        ["none"] => None,
        [a] => Some(Mat4::rotate(0.0, 0.0, 1.0, angle(a)?)),
        [axis, a] | [a, axis] if angle(a).is_some() => {
            let (x, y, z) = match axis {
                "x" => (1.0, 0.0, 0.0),
                "y" => (0.0, 1.0, 0.0),
                "z" => (0.0, 0.0, 1.0),
                _ => return None,
            };
            Some(Mat4::rotate(x, y, z, angle(a)?))
        }
        [x, y, z, a] => Some(Mat4::rotate(number(x)?, number(y)?, number(z)?, angle(a)?)),
        _ => return None,
    })
}

/// The `scale` property: `none`, or `x [y [z]]` (numbers or percentages).
pub fn parse_scale(s: &str) -> Option<Option<Mat4>> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    Some(match parts[..] {
        ["none"] => None,
        [x] => {
            let x = factor(x)?;
            Some(Mat4::scale(x, x, 1.0))
        }
        [x, y] => Some(Mat4::scale(factor(x)?, factor(y)?, 1.0)),
        [x, y, z] => Some(Mat4::scale(factor(x)?, factor(y)?, factor(z)?)),
        _ => return None,
    })
}

/// `transform-origin`: a position (keywords, lengths, percentages) and an optional
/// z length. The default is the centre.
pub fn parse_origin(s: &str) -> Option<(Len, Len, f32)> {
    let parts = split_top_level(s, ' ');
    // A third value is the z offset.
    let (xy, z) = match parts.len() {
        3 => (parts[..2].join(" "), px_len(parts[2])?),
        1 | 2 => (parts.join(" "), 0.0),
        _ => return None,
    };
    // A lone length or percentage is x, with y centred (as for background-position).
    let p = parse_background_position(&xy)?;
    Some((p.x, p.y, z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rect;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    // M3-XF-03
    #[test]
    fn scale_and_move_maps_are_recognised_and_others_not() {
        let h = |m: [[f32; 3]; 3]| Homography(m);
        assert_eq!(
            h([[1.5, 0.0, 10.0], [0.0, 1.5, -4.0], [0.0, 0.0, 1.0]]).as_scale_translate(),
            Some((1.5, 10.0, -4.0))
        );
        // Normalised by w.
        assert_eq!(
            h([[3.0, 0.0, 20.0], [0.0, 3.0, 8.0], [0.0, 0.0, 2.0]]).as_scale_translate(),
            Some((1.5, 10.0, 4.0))
        );
        let rotated = h([[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
        let uneven = h([[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        let perspective = h([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.001, 0.0, 1.0]]);
        let mirrored = h([[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]]);
        for m in [rotated, uneven, perspective, mirrored] {
            assert_eq!(m.as_scale_translate(), None, "{m:?}");
        }
    }

    /// Where the point (x, y) of a 100×100 box at the origin goes under `transform`
    /// about its centre.
    fn map(transform: &str, x: f32, y: f32) -> (f32, f32) {
        let spec = TransformSpec {
            functions: parse_transform(transform).unwrap(),
            ..Default::default()
        };
        let m = spec
            .matrix(
                Rect::new(0.0, 0.0, 100.0, 100.0),
                (Len::Percent(0.5), Len::Percent(0.5), 0.0),
            )
            .unwrap();
        m.flatten().apply(x, y).unwrap()
    }

    // M3-XF-01
    #[test]
    fn two_d_functions_map_points_like_css() {
        assert!(close(map("translate(12px, 50%)", 0.0, 0.0), (12.0, 50.0)));
        assert!(close(
            map("translateX(2px) translateY(3px)", 0.0, 0.0),
            (2.0, 3.0)
        ));
        // Rotations turn clockwise on screen (y down), about the centre.
        assert!(close(map("rotate(90deg)", 100.0, 50.0), (50.0, 100.0)));
        assert!(close(map("rotate(0.25turn)", 100.0, 50.0), (50.0, 100.0)));
        assert!(close(map("rotateZ(90deg)", 100.0, 50.0), (50.0, 100.0)));
        assert!(close(map("scale(2, 0.5)", 100.0, 100.0), (150.0, 75.0)));
        assert!(close(map("scaleX(2)", 0.0, 0.0), (-50.0, 0.0)));
        assert!(close(map("scaleY(50%)", 0.0, 0.0), (0.0, 25.0)));
        assert!(close(map("skewX(45deg)", 50.0, 100.0), (100.0, 100.0)));
        assert!(close(map("skew(0, 45deg)", 100.0, 50.0), (100.0, 100.0)));
        assert!(close(map("skewY(1.07rad)", 50.0, 50.0), (50.0, 50.0)));
        // matrix(a, b, c, d, e, f): x' = ax + cy + e, y' = bx + dy + f (about the centre).
        assert!(close(map("matrix(1, 0, 0, 1, 5, 6)", 0.0, 0.0), (5.0, 6.0)));
        // Functions apply right to left: translate then rotate differs from the reverse.
        assert!(close(
            map("rotate(90deg) translateX(10px)", 50.0, 50.0),
            (50.0, 60.0)
        ));
        assert!(close(
            map("translateX(10px) rotate(90deg)", 50.0, 50.0),
            (60.0, 50.0)
        ));
        assert_eq!(parse_transform("none"), Some(vec![]));
    }

    // M3-XF-01
    #[test]
    fn three_d_functions_project_onto_the_plane() {
        // Without perspective, rotateY(60deg) squashes x by cos 60° about the centre.
        assert!(close(map("rotateY(60deg)", 100.0, 50.0), (75.0, 50.0)));
        assert!(close(map("rotateX(60deg)", 50.0, 100.0), (50.0, 75.0)));
        // translateZ alone does nothing visible; with perspective it scales.
        assert!(close(map("translateZ(2px)", 0.0, 0.0), (0.0, 0.0)));
        let near = map("perspective(100px) translateZ(50px)", 100.0, 50.0);
        assert!(
            (near.0 - 150.0).abs() < 1e-2,
            "{near:?}: twice as far from the centre"
        );
        // With perspective, the near edge of a turned card grows.
        let left = map("perspective(200px) rotateY(45deg)", 0.0, 0.0);
        let right = map("perspective(200px) rotateY(45deg)", 100.0, 0.0);
        assert!(left.1 < 0.0 && right.1 > 0.0, "{left:?} {right:?}");
        assert!(close(
            map("rotate3d(0, 0, 1, 90deg)", 100.0, 50.0),
            (50.0, 100.0)
        ));
        assert!(close(
            map("scale3d(2.5, 1.2, 0.3)", 100.0, 100.0),
            (175.0, 110.0)
        ));
        assert!(close(map("scaleZ(0.3)", 10.0, 10.0), (10.0, 10.0)));
        let id = "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)";
        assert!(close(map(id, 7.0, 8.0), (7.0, 8.0)));
        let moved = "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 5, 6, 0, 1)";
        assert!(close(map(moved, 0.0, 0.0), (5.0, 6.0)), "column-major");
    }

    // M3-XF-01
    #[test]
    fn homographies_invert_and_backfaces_are_detected() {
        let spec = TransformSpec {
            functions: parse_transform("perspective(300px) rotateY(30deg) skewX(10deg)").unwrap(),
            ..Default::default()
        };
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let m = spec
            .matrix(rect, (Len::Percent(0.5), Len::Percent(0.5), 0.0))
            .unwrap();
        let h = m.flatten();
        let (x, y) = h.apply(30.0, 40.0).unwrap();
        let back = h.inverse().unwrap().apply(x, y).unwrap();
        assert!(close(back, (30.0, 40.0)), "{back:?}");
        assert!(!m.is_backface());
        let flipped = TransformSpec {
            functions: parse_transform("rotateY(180deg)").unwrap(),
            ..Default::default()
        };
        assert!(
            flipped
                .matrix(rect, (Len::Px(0.0), Len::Px(0.0), 0.0))
                .unwrap()
                .is_backface()
        );
    }

    // M3-XF-01
    #[test]
    fn individual_properties_and_bad_values() {
        assert_eq!(
            parse_translate("10px 50% 3px"),
            Some(Some((Len::Px(10.0), Len::Percent(0.5), 3.0)))
        );
        assert_eq!(parse_translate("none"), Some(None));
        assert!(parse_rotate("x 30deg").unwrap().is_some());
        assert!(parse_rotate("30deg y").unwrap().is_some());
        assert!(parse_rotate("1 1 0 45deg").unwrap().is_some());
        assert!(parse_scale("2 50%").unwrap().is_some());
        assert_eq!(
            parse_origin("left top 4px"),
            Some((Len::Percent(0.0), Len::Percent(0.0), 4.0))
        );
        assert_eq!(
            parse_origin("10px"),
            Some((Len::Px(10.0), Len::Percent(0.5), 0.0))
        );
        for bad in [
            "rotate(10)",
            "translate(1px, 2px, 3px)",
            "translateZ(10%)",
            "matrix(1, 2, 3)",
            "scale(two)",
            "perspective(-5px)",
            "spin(1turn)",
        ] {
            assert_eq!(parse_transform(bad), None, "{bad}");
        }
        assert_eq!(parse_rotate("w 30deg"), None);
    }
}
