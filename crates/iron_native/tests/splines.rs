//! `Spline` and `SplineAnimate` from QuickJS: the same native code compiled games call (the numbers
//! are compared with the old TypeScript in `tools/iron_ts_tests/tests/all/outlines.rs`). IDs are in
//! TESTS.md (M9-API-16xx).
mod common;
use common::{js, throws, with_world};

// M9-API-1603
#[test]
fn splines_pass_through_knots_go_by_distance_and_close() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const v = (x, y, z = 0) => new Vector3(x, y, z);
            const near = (a, b, d = 1e-4) => Math.abs(a.x - b.x) < d && Math.abs(a.y - b.y) < d && Math.abs(a.z - b.z) < d;
            const pts = [v(0, 0), v(1, 2), v(3, 1), v(4, 3)];
            const s = new Spline(pts);
            // The curve through the same points as GDI's cardinal curve.
            const curve = Shapes.curve(pts.map((p) => new Vector2(p.x, p.y)), 0.5, 4);
            let onCurve = true;
            for (let i = 0; i < 3; i++) for (let k = 0; k < 4; k++) {
              const c = curve[i * 4 + k];
              onCurve = onCurve && near(s.segmentPoint(i, k / 4), v(c.x, c.y));
            }
            const through = pts.every((p, i) => near(s.segmentPoint(Math.min(i, 2), i === 3 ? 1 : 0), p));
            const line = new Spline([v(0, 0), v(2, 0), v(2, 2)], { mode: 'linear' });
            const b = new Spline();
            b.addKnot(v(0, 0), 'bezier', v(0, 0), v(0, 1));
            b.addKnot(v(2, 0), 'bezier', v(0, 1), v(0, -1));
            const uneven = new Spline([v(0, 0), v(1, 0), v(4, 0)], { mode: 'linear' });
            const square = new Spline([v(0, 0), v(1, 0), v(1, 1), v(0, 1)], { closed: true, mode: 'linear' });
            const open = new Spline([v(0, 0), v(1, 0), v(1, 1)]);
            JSON.stringify([
              s.segmentCount, onCurve, through,
              near(line.segmentPoint(0, 0.5), v(1, 0)), near(line.segmentPoint(1, 0.25), v(2, 0.5)),
              near(b.segmentPoint(0, 0.5), v(1, 0.75)),
              Math.abs(uneven.length - 4) < 1e-4, near(uneven.evaluate(0.25), v(1, 0)), near(uneven.evaluate(0.5), v(2, 0)),
              near(uneven.evaluate(1), v(4, 0)), near(uneven.tangent(0.7), v(1, 0)),
              square.segmentCount, Math.abs(square.length - 4) < 1e-4, near(square.evaluate(0.875), v(0, 0.5)),
              square.sample(2).length, open.sample(2).length,
              s instanceof Spline, String(s), s.sample(2)[0] instanceof Vector3,
            ])
            "#,
        );
        assert_eq!(
            out,
            r#"[3,true,true,true,true,true,true,true,true,true,true,4,true,true,8,5,true,"[object Spline]",true]"#
        );
    });
}

// M9-API-1604
#[test]
fn splines_find_the_nearest_point_and_edits_change_knots_and_version() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const v = (x, y, z = 0) => new Vector3(x, y, z);
            const s = new Spline([v(0, 0), v(4, 0)], { mode: 'linear' });
            const near = s.nearest(v(1, 3));
            const before = s.version;
            s.insertKnot(1, v(2, 2));
            const grew = s.count === 3 && s.version > before;
            s.setPosition(1, v(2, 1));
            const moved = s.knots[1].position.y;
            // Turning an auto knot to bezier keeps the shape it had.
            const auto = new Spline([v(0, 0), v(1, 1), v(2, 0)]);
            const mid = auto.segmentPoint(0, 0.5);
            auto.setMode(1, 'bezier');
            const kept = auto.segmentPoint(0, 0.5).distance(mid) < 1e-9;
            auto.setTangents(1, v(-1, 0), v(1, 0));
            const mode = auto.knots[1].mode;
            s.removeKnot(0);
            // Knots are copies: editing one changes nothing.
            const knots = s.knots;
            knots[0].position.x = 99;
            const hit = s.locate(0.5);
            JSON.stringify([Math.round(near.t * 1000) / 1000, Math.round(near.distance * 1000) / 1000, grew, moved,
              kept, mode, s.count, s.knots[0].position.x, hit.segment, typeof hit.u, Object.keys(near)])
            "#,
        );
        assert_eq!(
            out,
            r#"[0.25,3,true,1,true,"bezier",2,2,0,"number",["t","point","distance"]]"#
        );
    });
}

// M9-API-1605
#[test]
fn enums_are_strings_with_name_and_parse_and_unknown_names_throw() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const s = new Spline([new Vector3(0, 0, 0), new Vector3(1, 0, 0)]);
            JSON.stringify([TangentMode.name(0), TangentMode.name(2), TangentMode.name(3) === undefined,
              TangentMode.parse('bezier'), TangentMode.parse('nope'),
              s.knots[0].mode, new Spline([new Vector3(0, 0, 0)], { mode: 'linear' }).knots[0].mode,
              SplineLoopMode.parse('pingPong'), SplineAlignment.name(1)])
            "#,
        );
        assert_eq!(
            out,
            r#"["auto","linear",true,1,-1,"auto","linear",1,"path"]"#
        );
        assert!(throws(ctx, "new Spline([], { mode: 'wavy' })").starts_with("TypeError"));
        assert!(
            throws(ctx, "new Spline().addKnot(new Vector3(0, 0, 0), 'wavy')")
                .starts_with("TypeError")
        );
        assert!(throws(ctx, "new Spline().setMode(0, 5)").starts_with("TypeError"));
    });
}

// M9-API-1606
#[test]
fn the_script_helpers_of_spline_animate_give_the_path_progress_and_facing() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const path = new Spline([new Vector3(0, 0, 0), new Vector3(4, 0, 0), new Vector3(4, 4, 0)], { mode: 'linear' });
            const a = SplineAnimate.pose(path, 1, 4, 'loop');
            const back = SplineAnimate.pose(path, 5, 4, 'pingPong');
            const f = SplineAnimate.facing(new Vector3(0, 1, 0));
            const r = f.rotate(Vector3.right);
            const r3 = (n) => Math.round(n * 1000) / 1000;
            JSON.stringify([
              [a.position.x, a.position.y].map(r3), [a.direction.x, a.direction.y].map(r3),
              [back.position.x, back.position.y].map(r3), [back.direction.x, back.direction.y].map(r3),
              SplineAnimate.progress(9, 4, 'once'), SplineAnimate.progress(5, 4, 'pingPong'), SplineAnimate.progress(4.5, 4, 'loop'),
              [r.x, r.y].map(r3),
            ])
            "#,
        );
        assert_eq!(out, "[[2,0],[1,0],[4,2],[0,-1],1,0.75,0.125,[0,1]]");
    });
}

// M9-API-1607
//
// A SplineAnimate saved in a scene file: the spline goes inline, the loop mode and alignment as
// names, the container as an object reference; loading makes them again.
#[test]
fn a_spline_animate_and_its_spline_survive_a_scene_file() {
    with_world(|ctx| {
        iron_scenes::register_engine_kinds();
        let out: String = js(
            ctx,
            r#"
            const track = GameObject.create();
            track.name = 'track';
            const car = GameObject.create();
            car.name = 'car';
            const a = SplineAnimate.attach(car);
            a.spline = new Spline([new Vector3(0, 0, 0), new Vector3(4, 0, 0)], { closed: false, tension: 0.25, mode: 'linear' });
            a.container = track;
            a.duration = 3;
            a.loopMode = 'pingPong';
            a.alignment = 'none';
            a.playing = false;
            a.elapsed = 1.5;
            const level = SceneManager.createScene('level');
            SceneManager.moveGameObjectToScene(track, level);
            SceneManager.moveGameObjectToScene(car, level);
            const text = level.toJson();
            const loaded = SceneManager.loadScene(text, 'additive');
            let copy = null;
            for (let i = 0; i < loaded.rootCount; i++) if (loaded.rootAt(i).name === 'car') copy = loaded.rootAt(i);
            const b = SplineAnimate.findOn(copy);
            JSON.stringify([text.includes('"SplineAnimate"'), text.includes('"pingPong"'), text.includes('"Spline"'),
              b !== null, b.duration, b.loopMode, b.alignment, b.playing, b.elapsed,
              b.spline.count, b.spline.tension, b.spline.knots[0].mode, b.spline === a.spline,
              b.container !== null && b.container.name])
            "#,
        );
        assert_eq!(
            out,
            r#"[true,true,true,true,3,"pingPong","none",false,1.5,2,0.25,"linear",false,"track"]"#
        );
    });
}
