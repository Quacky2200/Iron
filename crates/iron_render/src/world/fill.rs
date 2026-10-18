//! Filled shapes for [`WorldView::fills`](super::WorldView::fills): a closed polygon
//! tessellated into triangles (lyon), with GDI's two fill rules, so self-crossing outlines
//! (a star, a figure eight) fill the way WinForms fills them.

use lyon_tessellation::geom::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

/// Which parts of a self-crossing outline are inside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    /// Inside where a ray crosses the outline an odd number of times: a star's middle is a
    /// hole. GDI's default (`FillMode.Alternate`).
    #[default]
    EvenOdd,
    /// Inside wherever the outline winds around at all: a star is solid (`FillMode.Winding`).
    NonZero,
}

/// Triangles covering the closed polygon through `points` (three corners each, in order).
/// Points that aren't finite are skipped; fewer than three leave nothing to fill.
pub fn tessellate_fill(points: &[[f32; 2]], rule: FillRule) -> Vec<[f32; 2]> {
    let points: Vec<[f32; 2]> = points
        .iter()
        .copied()
        .filter(|p| p[0].is_finite() && p[1].is_finite())
        .collect();
    let [first, rest @ ..] = points.as_slice() else {
        return Vec::new();
    };
    if rest.len() < 2 {
        return Vec::new();
    }
    let mut path = Path::builder();
    path.begin(point(first[0], first[1]));
    for p in rest {
        path.line_to(point(p[0], p[1]));
    }
    path.end(true);
    let path = path.build();
    let options = FillOptions::default().with_fill_rule(match rule {
        FillRule::EvenOdd => lyon_tessellation::FillRule::EvenOdd,
        FillRule::NonZero => lyon_tessellation::FillRule::NonZero,
    });
    let mut out: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let made = FillTessellator::new().tessellate_path(
        &path,
        &options,
        &mut BuffersBuilder::new(&mut out, |v: FillVertex| v.position().to_array()),
    );
    if made.is_err() {
        return Vec::new();
    }
    out.indices
        .iter()
        .map(|&i| out.vertices[i as usize])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(triangles: &[[f32; 2]]) -> f32 {
        triangles
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&[a, b, c]| {
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum()
    }

    /// A five-pointed star drawn in one stroke (each point joined to the one two along),
    /// so its outline crosses itself and a pentagon sits in the middle.
    fn pentagram() -> Vec<[f32; 2]> {
        (0..5)
            .map(|i| {
                let a = (90.0 + 144.0 * i as f32).to_radians();
                [a.cos(), a.sin()]
            })
            .collect()
    }

    // M5-RND-25
    #[test]
    fn a_square_fills_with_two_triangles_covering_its_area() {
        let square = [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]];
        let t = tessellate_fill(&square, FillRule::EvenOdd);
        assert_eq!(t.len(), 6);
        assert!((area(&t) - 2.0).abs() < 1e-4);
    }

    // M5-RND-25
    #[test]
    fn a_crossing_star_has_a_hole_with_even_odd_and_none_with_non_zero() {
        let star = pentagram();
        let even_odd = area(&tessellate_fill(&star, FillRule::EvenOdd));
        let non_zero = area(&tessellate_fill(&star, FillRule::NonZero));
        // The middle pentagon (radius cos 72° / cos 36° of the points') is the difference.
        let r = 72f32.to_radians().cos() / 36f32.to_radians().cos();
        let pentagon = 2.5 * r * r * 72f32.to_radians().sin();
        assert!(
            (non_zero - even_odd - pentagon).abs() < 1e-3,
            "{non_zero} - {even_odd} vs {pentagon}"
        );
    }

    // M5-RND-25
    #[test]
    fn too_few_or_broken_points_fill_nothing() {
        assert!(tessellate_fill(&[[0.0, 0.0], [1.0, 0.0]], FillRule::EvenOdd).is_empty());
        let with_nan = [[0.0, 0.0], [f32::NAN, 1.0], [1.0, 0.0], [1.0, 1.0]];
        assert_eq!(tessellate_fill(&with_nan, FillRule::EvenOdd).len(), 3);
    }
}
