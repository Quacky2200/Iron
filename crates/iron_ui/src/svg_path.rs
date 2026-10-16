//! SVG path data (`path("M 0 0 L 10 0 ...")` in `clip-path`), flattened to
//! polygons: curves and arcs become short line segments.

/// Line segments per Bézier curve.
const CURVE_SEGMENTS: usize = 16;

/// The contours of `d`, each a list of points (closed implicitly), in the path's
/// own units. `None` for malformed data.
pub fn flatten(d: &str) -> Option<Vec<Vec<(f32, f32)>>> {
    let mut p = Parser {
        chars: d.chars().collect(),
        pos: 0,
    };
    let mut contours: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut contour: Vec<(f32, f32)> = Vec::new();
    let (mut cur, mut start) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32));
    // The previous command's second control point, for `S`/`T` reflection.
    let mut last_cubic: Option<(f32, f32)> = None;
    let mut last_quad: Option<(f32, f32)> = None;
    let mut command: Option<char> = None;
    let finish = |contour: &mut Vec<(f32, f32)>, contours: &mut Vec<Vec<(f32, f32)>>| {
        if contour.len() > 1 {
            contours.push(std::mem::take(contour));
        } else {
            contour.clear();
        }
    };
    loop {
        p.skip_separators();
        if p.done() {
            break;
        }
        if let Some(c) = p.command() {
            // A path starts with a move-to.
            if contours.is_empty() && contour.is_empty() && !c.eq_ignore_ascii_case(&'M') {
                return None;
            }
            command = Some(c);
        } else if command.is_none() {
            return None;
        }
        let c = command?;
        let rel = c.is_ascii_lowercase();
        let base = if rel { cur } else { (0.0, 0.0) };
        let point = |p: &mut Parser| -> Option<(f32, f32)> {
            let x = p.number()?;
            let y = p.number()?;
            Some((base.0 + x, base.1 + y))
        };
        let (mut cubic, mut quad) = (None, None);
        match c.to_ascii_uppercase() {
            'M' => {
                finish(&mut contour, &mut contours);
                cur = point(&mut p)?;
                start = cur;
                contour.push(cur);
                // Further pairs are line-tos.
                command = Some(if rel { 'l' } else { 'L' });
            }
            'L' => {
                cur = point(&mut p)?;
                contour.push(cur);
            }
            'H' => {
                let x = p.number()?;
                cur.0 = if rel { cur.0 + x } else { x };
                contour.push(cur);
            }
            'V' => {
                let y = p.number()?;
                cur.1 = if rel { cur.1 + y } else { y };
                contour.push(cur);
            }
            'C' | 'S' => {
                let c1 = if c.eq_ignore_ascii_case(&'C') {
                    point(&mut p)?
                } else {
                    reflect(last_cubic, cur)
                };
                let c2 = point(&mut p)?;
                let end = point(&mut p)?;
                for i in 1..=CURVE_SEGMENTS {
                    let t = i as f32 / CURVE_SEGMENTS as f32;
                    contour.push(cubic_at(cur, c1, c2, end, t));
                }
                cubic = Some(c2);
                cur = end;
            }
            'Q' | 'T' => {
                let c1 = if c.eq_ignore_ascii_case(&'Q') {
                    point(&mut p)?
                } else {
                    reflect(last_quad, cur)
                };
                let end = point(&mut p)?;
                for i in 1..=CURVE_SEGMENTS {
                    let t = i as f32 / CURVE_SEGMENTS as f32;
                    contour.push(quad_at(cur, c1, end, t));
                }
                quad = Some(c1);
                cur = end;
            }
            'A' => {
                let rx = p.number()?;
                let ry = p.number()?;
                let rotation = p.number()?;
                let large = p.flag()?;
                let sweep = p.flag()?;
                let end = point(&mut p)?;
                arc(&mut contour, cur, end, (rx, ry), rotation, large, sweep);
                cur = end;
            }
            'Z' => {
                finish(&mut contour, &mut contours);
                cur = start;
                contour.push(cur);
                // `Z` takes no arguments: the next number needs a new command.
                command = None;
            }
            _ => return None,
        }
        last_cubic = cubic;
        last_quad = quad;
    }
    finish(&mut contour, &mut contours);
    Some(contours)
}

fn reflect(control: Option<(f32, f32)>, cur: (f32, f32)) -> (f32, f32) {
    match control {
        Some((x, y)) => (2.0 * cur.0 - x, 2.0 * cur.1 - y),
        None => cur,
    }
}

fn cubic_at(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        w0 * a.0 + w1 * b.0 + w2 * c.0 + w3 * d.0,
        w0 * a.1 + w1 * b.1 + w2 * c.1 + w3 * d.1,
    )
}

fn quad_at(a: (f32, f32), b: (f32, f32), c: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (w0, w1, w2) = (u * u, 2.0 * u * t, t * t);
    (
        w0 * a.0 + w1 * b.0 + w2 * c.0,
        w0 * a.1 + w1 * b.1 + w2 * c.1,
    )
}

/// An elliptical arc from `from` to `to` (SVG's endpoint form), as line segments.
fn arc(
    out: &mut Vec<(f32, f32)>,
    from: (f32, f32),
    to: (f32, f32),
    radii: (f32, f32),
    rotation_deg: f32,
    large: bool,
    sweep: bool,
) {
    let (mut rx, mut ry) = (radii.0.abs(), radii.1.abs());
    if rx == 0.0 || ry == 0.0 || from == to {
        out.push(to);
        return;
    }
    // SVG 1.1 appendix F.6.5: endpoint to centre parameterisation.
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let coef = sign * (num / den).max(0.0).sqrt();
    let (cxp, cyp) = (coef * rx * y1 / ry, -coef * ry * x1 / rx);
    let cx = cos * cxp - sin * cyp + (from.0 + to.0) / 2.0;
    let cy = sin * cxp + cos * cyp + (from.1 + to.1) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let start = angle(1.0, 0.0, (x1 - cxp) / rx, (y1 - cyp) / ry);
    let mut delta = angle(
        (x1 - cxp) / rx,
        (y1 - cyp) / ry,
        (-x1 - cxp) / rx,
        (-y1 - cyp) / ry,
    );
    let tau = std::f32::consts::TAU;
    if !sweep && delta > 0.0 {
        delta -= tau;
    } else if sweep && delta < 0.0 {
        delta += tau;
    }
    let steps = ((delta.abs() / (tau / 32.0)).ceil() as usize).max(1);
    for i in 1..steps {
        let t = start + delta * i as f32 / steps as f32;
        let (st, ct) = t.sin_cos();
        out.push((
            cx + rx * cos * ct - ry * sin * st,
            cy + rx * sin * ct + ry * cos * st,
        ));
    }
    out.push(to);
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn done(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn skip_separators(&mut self) {
        while self
            .chars
            .get(self.pos)
            .is_some_and(|c| c.is_whitespace() || *c == ',')
        {
            self.pos += 1;
        }
    }

    fn command(&mut self) -> Option<char> {
        let c = *self.chars.get(self.pos)?;
        ("MmLlHhVvCcSsQqTtAaZz".contains(c)).then(|| {
            self.pos += 1;
            c
        })
    }

    /// A number: optional sign, digits, a decimal part and an exponent. `10-5`
    /// and `.5.5` split into two numbers, as SVG allows.
    fn number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.pos;
        let at = |p: &Self, i: usize| p.chars.get(i).copied();
        if matches!(at(self, self.pos), Some('+' | '-')) {
            self.pos += 1;
        }
        let mut seen_dot = false;
        while let Some(c) = at(self, self.pos) {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !seen_dot {
                seen_dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if matches!(at(self, self.pos), Some('e' | 'E')) {
            let save = self.pos;
            self.pos += 1;
            if matches!(at(self, self.pos), Some('+' | '-')) {
                self.pos += 1;
            }
            let digits = self.pos;
            while at(self, self.pos).is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
            if self.pos == digits {
                self.pos = save;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse().ok()
    }

    /// An arc flag: a single `0` or `1`, which may run into the next number.
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let c = *self.chars.get(self.pos)?;
        self.pos += 1;
        match c {
            '0' => Some(false),
            '1' => Some(true),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    // M3-CLIP-01
    #[test]
    fn lines_and_relative_commands() {
        let c = flatten("M10 10 h20 v10 L 10 20 z m 5 0 l1-1").unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(
            c[0],
            vec![(10.0, 10.0), (30.0, 10.0), (30.0, 20.0), (10.0, 20.0)]
        );
        // After `z` the current point is the subpath start; `m` is relative to it.
        assert_eq!(c[1], vec![(15.0, 10.0), (16.0, 9.0)]);
        assert_eq!(
            flatten("M0,0L.5.5").unwrap()[0],
            vec![(0.0, 0.0), (0.5, 0.5)],
            "numbers run together"
        );
    }

    // M3-CLIP-01
    #[test]
    fn curves_and_arcs_end_where_they_should() {
        let c = flatten("M0 0 C 0 10 10 10 10 0 S 20 -10 20 0 Q 25 5 30 0 T 40 0").unwrap();
        let pts = &c[0];
        assert!(pts.contains(&(10.0, 0.0)) && close(*pts.last().unwrap(), (40.0, 0.0)));
        // Bulges down (y grows downwards) halfway along the first curve.
        assert!(close(pts[8], (5.0, 7.5)), "{:?}", pts[8]);

        // A half circle of radius 10 from (0,0) to (20,0), sweeping through (10, 10).
        let arc = flatten("M0 0 A 10 10 0 0 1 20 0").unwrap();
        let pts = &arc[0];
        assert!(close(*pts.last().unwrap(), (20.0, 0.0)));
        let lowest = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let highest = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        assert!((lowest - 10.0).abs() < 0.1 || (highest + 10.0).abs() < 0.1);
        // Arc flags written without separators.
        assert!(flatten("M0 0a5 5 0 1020 0").is_some());
    }

    // M3-CLIP-01
    #[test]
    fn malformed_paths_are_rejected() {
        for bad in ["L 10 10", "M 10", "M 0 0 X 1 1", "M 0 0 A 1 1 0 2 0 5 5"] {
            assert_eq!(flatten(bad), None, "{bad}");
        }
    }
}
