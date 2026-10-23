// SPDX-License-Identifier: Apache-2.0

//! The math types as JavaScript classes in QuickJS (`iron_math::register_quickjs`): constructors
//! through `create`, overloads by argument count, statics, getters and methods. IDs are in
//! TESTS.md (M9-API-46..).

fn with_js<R>(f: impl FnOnce(&rquickjs::Ctx<'_>) -> R) -> R {
    let rt = rquickjs::Runtime::new().unwrap();
    let ctx = rquickjs::Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        iron_math::register_quickjs(&ctx).unwrap();
        f(&ctx)
    })
}

// M9-API-46
//
// `new Vector3(x, y, z)` goes through `create` (a missing component is 0), results of methods are
// instances of the class, and statics and getters read like the TypeScript's.
#[test]
fn vectors_are_classes_with_constructors_methods_and_statics() {
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                const a = new Vector3(1, 2, 2);
                const b = a.add(new Vector3(1, 1, 1)).scale(2);
                JSON.stringify([a.magnitude, b.x, b.y, b.z, b instanceof Vector3, new Vector3(5).y,
                  Vector3.up.y, Vector3.forward.z, a.cross(Vector3.right).z, a.toString(),
                  new Vector2(3, 4).magnitude, Vector2Int.round(new Vector2(-0.5, 2.5)).toString(),
                  Mathf.PI === Math.PI, Mathf.lerp(0, 10, 2), Mathf.smoothDamp(0, 10, 0, 0.4, 0.05).x]);
                "#,
            )
            .unwrap();
        assert_eq!(
            s,
            r#"[3,4,6,6,true,0,1,-1,-2,"(1, 2, 2)",5,"(0, 3)",true,10,0.26230904996652704]"#
        );
    });
}

// M9-API-47
//
// A name with several versions (`Quaternion.euler(x, y, z)` and `euler(vector)`, `lookRotation`
// with and without `up`, `Matrix4x4.trs` with and without scale) picks one by argument count.
#[test]
fn overloaded_functions_are_picked_by_argument_count() {
    with_js(|ctx| {
        let s: String = ctx
            .eval(
                r#"
                const e1 = Quaternion.euler(0, 90, 0), e2 = Quaternion.euler(new Vector3(0, 90, 0));
                const l1 = Quaternion.lookRotation(new Vector3(1, 0, 0));
                const l2 = Quaternion.lookRotation(new Vector3(1, 0, 0), Vector3.up);
                const m1 = Matrix4x4.trs(new Vector3(1, 2, 3), Quaternion.identity);
                const m2 = Matrix4x4.trs(new Vector3(1, 2, 3), Quaternion.identity, new Vector3(2, 2, 2));
                JSON.stringify([e1.y === e2.y, l1.w === l2.w, m1.get(0, 0), m2.get(0, 0), m2.get(0, 3),
                  e1.rotate(new Vector3(0, 0, 1)).x.toFixed(6), e1.eulerAngles.y.toFixed(6),
                  new Ray(new Vector3(0, 0, 0), new Vector3(0, 5, 0)).direction.y,
                  new Plane(new Vector3(0, 1, 0), -2).raycast(new Ray(new Vector3(0, 5, 0), Vector3.down)),
                  new Bounds(new Vector3(0, 0, 0), new Vector3(2, 2, 2)).max.x,
                  new Pose(new Vector3(1, 0, 0), Quaternion.identity).transformPoint(new Vector3(0, 1, 0)).x]);
                "#,
            )
            .unwrap();
        assert_eq!(s, r#"[true,true,1,2,1,"1.000000","90.000000",1,3,1,1]"#);
    });
}
