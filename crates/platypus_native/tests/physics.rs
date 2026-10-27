// SPDX-License-Identifier: Apache-2.0

//! The physics API from QuickJS: the same worlds as the compiled-code tests
//! (`platypus_physics_api/src/tests.rs`). IDs are in TESTS.md (M9-API-*).
mod common;
use common::{js, throws, with_world};

// M9-API-126
//
// A Rigidbody with a BoxCollider falls under gravity, rests on a floor and carries its object (the
// compiled test `a_rigidbody_with_a_box_rests_on_the_floor` gets y = 1).
#[test]
fn a_rigidbody_with_a_box_rests_on_the_floor() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            Physics.addBox(new Vector3(10, 0.5, 10));
            const go = GameObject.create();
            go.transform.position = new Vector3(0, 4, 0);
            const rb = Rigidbody.attach(go);
            const box = BoxCollider.attach(go);
            box.bounciness = 0;
            for (let i = 0; i < 240; i++) Physics.step();
            const y = go.transform.position.y;
            JSON.stringify([Math.abs(y - 1) < 0.05, rb.mass, rb.useGravity, rb.isKinematic,
                Rigidbody.findOn(go) !== null, box.attachedRigidbody !== null, rb.velocity.y < 0.01,
                PhysicsComponents.rigidbodyCount()])
            "#,
        );
        assert_eq!(out, "[true,1,true,false,true,true,true,1]");
        // The enum-like parameters: ForceMode and RigidbodyConstraints.
        let out: String = js(
            ctx,
            r#"
            const go2 = GameObject.create();
            const rb2 = Rigidbody.attach(go2);
            BoxCollider.attach(go2);
            rb2.useGravity = false;
            rb2.addForce(new Vector3(0, 0, 10), ForceMode.Impulse);
            Physics.step();
            Physics.step();
            rb2.constraints = RigidbodyConstraints.FreezePositionX | RigidbodyConstraints.FreezeRotation;
            JSON.stringify([ForceMode.Impulse, rb2.velocity.z > 5, rb2.constraints, rb2.collisionDetection])
            "#,
        );
        assert_eq!(out, "[1,true,114,\"discrete\"]");
        // One error case: an object handle must be a GameObject.
        let e = throws(ctx, "Rigidbody.attach(5)");
        assert_eq!(e, "TypeError: expected a GameObject or null");
        assert!(throws(ctx, "rb2.addForce(5)").starts_with("TypeError"));
    });
}

// M9-API-127
//
// An immediate raycast answers the hit's collider, distance, point and normal; a miss is null.
#[test]
fn a_raycast_reports_distance_point_and_normal() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            Physics.addBox(new Vector3(10, 0.5, 10));
            Physics.step();
            const hit = Physics.raycast(new Vector3(2, 5, 1), new Vector3(0, -1, 0), 20);
            const miss = Physics.raycast(new Vector3(50, 5, 0), new Vector3(0, -1, 0), 20);
            const short = Physics.raycast(new Vector3(0, 5, 0), new Vector3(0, -1, 0), 1);
            const p = hit.point, n = hit.normal;
            JSON.stringify([Math.round(hit.distance * 1000) / 1000, p.x, p.y, p.z, n.x, n.y, n.z,
                hit.collider !== null, miss, short, String(hit), JSON.stringify(hit)])
            "#,
        );
        assert_eq!(
            out,
            r#"[4.5,2,0.5,1,0,1,0,true,null,null,"[object RaycastHit]","{}"]"#
        );
        // Left-out maxDistance (optional) still casts.
        let far: bool = js(
            ctx,
            "Physics.raycast(new Vector3(0, 5, 0), new Vector3(0, -1, 0)) !== null",
        );
        assert!(far);
        assert!(throws(ctx, "Physics.raycast(1, 2)").starts_with("TypeError"));
    });
}

// M9-API-128
//
// Bodies made by script: a dynamic ball falls to a floor and stops, a rope joint holds a body, and
// the setters and getters pair up.
#[test]
fn bodies_colliders_and_joints_made_from_script() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            Physics.gravity = new Vector3(0, -20, 0);
            const g = Physics.gravity;
            Physics.addBox(new Vector3(20, 0.5, 20)).setFriction(0.5);
            const ball = Physics.createBody('dynamic');
            ball.position = new Vector3(0, 3, 0);
            const col = ball.addBall(0.5);
            col.setRestitution(0);
            for (let i = 0; i < 240; i++) Physics.step();
            const rest = ball.position.y;
            const anchor = Physics.createBody('fixed');
            anchor.position = new Vector3(0, 10, 0);
            const swing = Physics.createBody('dynamic');
            swing.position = new Vector3(0, 8, 0);
            swing.addBall(0.2);
            const joint = Physics.createJoint('rope', anchor, swing, new Vector3(0, 0, 0), new Vector3(0, 0, 0), [2]);
            for (let i = 0; i < 120; i++) Physics.step();
            const dist = Math.abs(swing.position.y - 10);
            joint.remove();
            JSON.stringify([g.y, Math.abs(rest - 1) < 0.05, col.body !== null, dist <= 2.1, ball.kind,
                String(joint), String(col), ball.grounded])
            "#,
        );
        assert_eq!(
            out,
            r#"[-20,true,true,true,"dynamic","[object PhysicsJoint]","[object PhysicsCollider]",false]"#
        );
        assert!(throws(ctx, "Physics.createBody(5)").starts_with("TypeError"));
        assert!(
            throws(
                ctx,
                "Physics.createJoint('rope', null, null, {x:0,y:0,z:0}, {x:0,y:0,z:0}, 'no')"
            )
            .starts_with("TypeError")
        );
    });
}

// M9-API-129
//
// A character controller drops onto the floor, reports grounded and carries its object (the
// compiled test `a_character_walks_on_the_floor`); a Rope hangs from its object.
#[test]
fn character_and_rope_components() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            Physics.addBox(new Vector3(10, 0.5, 10));
            const hero = GameObject.create();
            hero.transform.position = new Vector3(0, 3, 0);
            const cc = CharacterController.attach(hero);
            for (let i = 0; i < 90; i++) {
                cc.move(new Vector3(0.02, -0.1, 0));
                Physics.step();
            }
            const p = hero.transform.position;
            const ropeObject = GameObject.create();
            ropeObject.transform.position = new Vector3(5, 5, 0);
            const rope = Rope.attach(ropeObject);
            rope.length = 2;
            rope.segments = 4;
            for (let i = 0; i < 60; i++) Physics.step();
            const first = rope.pointAt(0), last = rope.pointAt(rope.pointCount - 1);
            JSON.stringify([cc.isGrounded, Math.abs(p.y - 1.5) < 0.1, p.x > 1, rope.pointCount,
                Math.abs(first.y - 5) < 0.05, Math.abs(last.y - 3) < 0.3, rope.linkCount > 0,
                PhysicsJoints.jointCount() >= 0])
            "#,
        );
        assert_eq!(out, "[true,true,true,5,true,true,true,true]");
        assert!(throws(ctx, "cc.move()").starts_with("TypeError"));
    });
}

// M9-API-130
//
// 2D: a body falls in the XY plane (z stays) and rests on a 2D floor; the 2D ray names the object.
#[test]
fn two_d_bodies_keep_their_z_and_rays_name_the_object() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const ground = GameObject.create();
            ground.name = 'ground';
            BoxCollider2D.attach(ground).size = new Vector2(20, 1);
            const ball = GameObject.create();
            ball.transform.position = new Vector3(0, 5, 0.7);
            Rigidbody2D.attach(ball);
            CircleCollider2D.attach(ball).radius = 0.5;
            for (let i = 0; i < 240; i++) Physics.step();
            const p = ball.transform.position;
            const hit = Physics2D.raycast(new Vector2(5, 5), new Vector2(0, -1), 100);
            JSON.stringify([Math.abs(p.y - 1) < 0.06, Math.abs(p.z - 0.7) < 1e-3,
                hit !== null && hit.colliderObject.name, Physics2D.rigidbody2dCount(),
                Physics2D.gravity.y])
            "#,
        );
        assert_eq!(out, r#"[true,true,"ground",1,-9.81]"#);
        assert!(throws(ctx, "Physics2D.raycast(1, 2)").starts_with("TypeError"));
    });
}
// M9-API-131
//
// Joint components hold bodies (the compiled test `joints_hold_bodies`), and a MeshCollider takes
// arrays of vectors and numbers.
#[test]
fn joint_components_and_mesh_colliders() {
    with_world(|ctx| {
        let out: String = js(
            ctx,
            r#"
            const anchor = GameObject.create();
            anchor.transform.position = new Vector3(0, 5, 0);
            const arb = Rigidbody.attach(anchor);
            arb.isKinematic = true;
            BoxCollider.attach(anchor);
            const held = GameObject.create();
            held.transform.position = new Vector3(0, 3.5, 0);
            Rigidbody.attach(held);
            SphereCollider.attach(held).radius = 0.2;
            const joint = FixedJoint.attach(held);
            joint.connectedBody = arb;
            const hinge = HingeJoint.attach(GameObject.create());
            hinge.axis = new Vector3(0, 0, 1);
            hinge.useLimits = true;
            hinge.limitsMin = -30;
            hinge.limitsMax = 30;
            for (let i = 0; i < 180; i++) Physics.step();
            const mesh = GameObject.create();
            const mc = MeshCollider.attach(mesh);
            mc.setMesh([new Vector3(0, 0, 0), new Vector3(1, 0, 0), new Vector3(0, 0, 1)], [0, 1, 2]);
            JSON.stringify([Math.abs(held.transform.position.y - 3.5) < 0.05, joint.connectedBody !== null,
                hinge.limitsMax, hinge.useLimits, hinge.axis.z, mc.pointCount, PhysicsJoints.jointCount() >= 2])
            "#,
        );
        assert_eq!(out, "[true,true,30,true,1,3,true]");
        assert!(throws(ctx, "mc.setMesh('no')").starts_with("TypeError"));
    });
}
