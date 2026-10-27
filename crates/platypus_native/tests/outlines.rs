// SPDX-License-Identifier: Apache-2.0

//! `Shapes` from QuickJS: the same native functions compiled code calls (their numbers are compared
//! with the old TypeScript in `tools/platypus_ts_tests/tests/engine_api/outlines.rs`). IDs are in
//! TESTS.md (M9-API-16xx).
mod common;
use common::{js, throws, with_world};

// M9-API-1600
#[test]
fn shapes_return_vector2_instances_with_the_old_defaults() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const r = Shapes.rectangle(1, 2, 3, 4);
            const c = Shapes.circle(new Vector2(1, 1), 2);
            const rr = Shapes.roundedRectangle(0, 0, 4, 2, 5, 4);
            const e = Shapes.ellipse(Vector2.zero, 3, 1, 4);
            JSON.stringify([
              r.every((p) => p instanceof Vector2), r.map((p) => [p.x, p.y]),
              c.length, c.every((p) => Math.abs(p.distance(new Vector2(1, 1)) - 2) < 1e-9),
              rr.length, rr.every((p) => p.x > -1e-6 && p.x < 4 + 1e-6 && p.y > -1e-6 && p.y < 2 + 1e-6),
              e.length, Math.round(e[0].x), Math.round(e[1].y),
            ])
            "#,
        );
        assert_eq!(
            out,
            "[true,[[1,2],[4,2],[4,6],[1,6]],64,true,20,true,4,3,1]"
        );
    });
}

// M9-API-1601
#[test]
fn arcs_pies_polygons_beziers_and_curves() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const near = (p, x, y) => Math.abs(p.x - x) < 1e-6 && Math.abs(p.y - y) < 1e-6;
            const a = Shapes.arc(Vector2.zero, 1, 1, 0, 90);
            const back = Shapes.arc(Vector2.zero, 1, 1, 0, -90, 2);
            const pie = Shapes.pie(new Vector2(5, 5), 1, 1, 0, 180);
            const t = Shapes.regularPolygon(Vector2.zero, 2, 3);
            const tri = Shapes.triangle({ x: 0, y: 0 }, { x: 1, y: 0 }, [0, 1]);
            const b = Shapes.bezier({ x: 0, y: 0 }, { x: 0, y: 2 }, { x: 2, y: 2 }, { x: 2, y: 0 }, 4);
            const pts = [new Vector2(0, 0), new Vector2(1, 2), new Vector2(3, 1), new Vector2(4, 3)];
            const c = Shapes.curve(pts, 0.5, 4);
            const straight = Shapes.curve(pts, 0, 2);
            const closed = Shapes.closedCurve(pts, 0.5, 4);
            JSON.stringify([
              a.length, near(a[0], 1, 0), near(a[a.length - 1], 0, 1),
              back.length, near(back[2], 0, -1),
              near(pie[0], 5, 5), near(pie[pie.length - 1], 4, 5),
              t.length, near(t[0], 0, 2),
              tri.map((p) => [p.x, p.y]),
              b.length, near(b[0], 0, 0), near(b[2], 1, 1.5), near(b[4], 2, 0),
              c.length, pts.every((p, i) => near(c[i * 4], p.x, p.y)), near(straight[1], 0.5, 1),
              closed.length, pts.every((p, i) => near(closed[i * 4], p.x, p.y)),
            ])
            "#,
        );
        assert_eq!(
            out,
            "[17,true,true,3,true,true,true,3,true,[[0,0],[1,0],[0,1]],5,true,true,true,13,true,true,16,true]"
        );
    });
}

// M9-API-1602
#[test]
fn polygon_takes_points_as_arguments_spread_included_and_bad_calls_throw() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const pts = [new Vector2(1, 2), { x: 3, y: 4 }];
            JSON.stringify([
              Shapes.polygon().length,
              Shapes.polygon(...pts).map((p) => [p.x, p.y]),
              Shapes.polygon({ x: 5, y: 6 }).length,
              Shapes.circle([2, 3], 1, 4).map((p) => [Math.round(p.x), Math.round(p.y)]),
            ])
            "#,
        );
        assert_eq!(out, "[0,[[1,2],[3,4]],1,[[3,3],[2,4],[1,3],[2,2]]]");
        assert!(throws(ctx, "Shapes.circle('x', 1)").starts_with("TypeError"));
    });
}
