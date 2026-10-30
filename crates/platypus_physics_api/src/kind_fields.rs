// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! What the physics components' fields are, for the editor's Inspector
//! (`platypus_kinds::register_fields`): the names `kinds.rs` saves, their types, enum choices and
//! what a reference points at, with Unity's meaning as the doc.

use platypus_kinds::{FieldDoc, register_fields};
use serde_json::{Value, json};

fn f(name: &str, ty: &str, default: Value, doc: &str) -> FieldDoc {
    FieldDoc {
        name: name.into(),
        ty: ty.into(),
        default,
        doc: doc.into(),
    }
}

const DETECTION: &str = "enum:discrete|continuous";

fn collider_3d(extra: Vec<FieldDoc>) -> Vec<FieldDoc> {
    let mut out = vec![
        f(
            "center",
            "v3",
            Value::Null,
            "The collider's centre in the object's space.",
        ),
        f(
            "isTrigger",
            "bool",
            json!(false),
            "Reports overlaps instead of colliding.",
        ),
    ];
    out.extend(extra);
    out
}

fn collider_2d(extra: Vec<FieldDoc>) -> Vec<FieldDoc> {
    let mut out = vec![
        f(
            "offset",
            "v2",
            Value::Null,
            "The collider's centre in the object's space.",
        ),
        f(
            "isTrigger",
            "bool",
            json!(false),
            "Reports overlaps instead of colliding.",
        ),
    ];
    out.extend(extra);
    out
}

fn joint(extra: Vec<FieldDoc>) -> Vec<FieldDoc> {
    let mut out = vec![
        f(
            "connectedBody",
            "component:Rigidbody",
            Value::Null,
            "The body it is joined to (none: the world).",
        ),
        f(
            "anchor",
            "v3",
            Value::Null,
            "Where the joint is, in the object's space.",
        ),
        f(
            "connectedAnchor",
            "v3",
            Value::Null,
            "Where the joint is on the connected body.",
        ),
        f(
            "autoConfigureConnectedAnchor",
            "bool",
            json!(true),
            "Works out the connected anchor from the anchor.",
        ),
    ];
    out.extend(extra);
    out
}

/// Registers the schemas (the host calls it with the kinds).
pub fn register() {
    register_fields(
        "Rigidbody",
        vec![
            f("mass", "f64", json!(1.0), "In kilograms."),
            f("drag", "f64", json!(0.0), "Slows its movement."),
            f("angularDrag", "f64", json!(0.05), "Slows its turning."),
            f("useGravity", "bool", json!(true), "Gravity pulls it."),
            f(
                "isKinematic",
                "bool",
                json!(false),
                "Moved by its transform only, not by forces.",
            ),
            f(
                "constraints",
                "mask",
                json!(0),
                "Frozen axes, as Unity's RigidbodyConstraints flags.",
            ),
            f(
                "collisionDetection",
                DETECTION,
                json!("discrete"),
                "Continuous stops fast bodies passing through.",
            ),
        ],
    );
    register_fields(
        "Rigidbody2D",
        vec![
            f("mass", "f64", json!(1.0), "In kilograms."),
            f("drag", "f64", json!(0.0), "Slows its movement."),
            f("angularDrag", "f64", json!(0.05), "Slows its turning."),
            f(
                "gravityScale",
                "f64",
                json!(1.0),
                "How strongly gravity pulls it.",
            ),
            f(
                "bodyType",
                "enum:dynamic|kinematic|static",
                json!("dynamic"),
                "Moved by forces, by its transform, or not at all.",
            ),
            f("freezeRotation", "bool", json!(false), "Never turns."),
            f(
                "collisionDetectionMode",
                DETECTION,
                json!("discrete"),
                "Continuous stops fast bodies passing through.",
            ),
        ],
    );
    register_fields(
        "BoxCollider",
        collider_3d(vec![f("size", "v3", Value::Null, "The box's size.")]),
    );
    register_fields(
        "SphereCollider",
        collider_3d(vec![f("radius", "f64", json!(0.5), "The sphere's radius.")]),
    );
    register_fields(
        "CapsuleCollider",
        collider_3d(vec![
            f("radius", "f64", json!(0.5), "The capsule's radius."),
            f("height", "f64", json!(2.0), "Its height, ends included."),
        ]),
    );
    register_fields(
        "MeshCollider",
        collider_3d(vec![f(
            "convex",
            "bool",
            json!(false),
            "Uses the mesh's convex hull.",
        )]),
    );
    register_fields(
        "CharacterController",
        collider_3d(vec![
            f("radius", "f64", json!(0.5), "The capsule's radius."),
            f("height", "f64", json!(2.0), "Its height, ends included."),
            f(
                "slopeLimit",
                "f64",
                json!(45.0),
                "The steepest slope it walks up, in degrees.",
            ),
            f(
                "stepOffset",
                "f64",
                json!(0.3),
                "The highest step it climbs.",
            ),
        ]),
    );
    register_fields(
        "BoxCollider2D",
        collider_2d(vec![f("size", "v2", Value::Null, "The box's size.")]),
    );
    register_fields(
        "CircleCollider2D",
        collider_2d(vec![f("radius", "f64", json!(0.5), "The circle's radius.")]),
    );
    register_fields(
        "CapsuleCollider2D",
        collider_2d(vec![
            f("size", "v2", Value::Null, "The capsule's width and height."),
            f(
                "direction",
                "enum:vertical|horizontal",
                json!("vertical"),
                "Which way it is long.",
            ),
        ]),
    );
    register_fields("PolygonCollider2D", collider_2d(Vec::new()));
    register_fields("EdgeCollider2D", collider_2d(Vec::new()));
    register_fields("FixedJoint", joint(Vec::new()));
    register_fields("CharacterJoint", joint(Vec::new()));
    register_fields(
        "HingeJoint",
        joint(vec![
            f("axis", "v3", Value::Null, "The axis it turns about."),
            f(
                "useLimits",
                "bool",
                json!(false),
                "Keeps the angle within the limits.",
            ),
            f(
                "useMotor",
                "bool",
                json!(false),
                "Drives it at the motor's speed.",
            ),
        ]),
    );
    register_fields(
        "SpringJoint",
        joint(vec![
            f("spring", "f64", json!(10.0), "How strongly it pulls back."),
            f(
                "damper",
                "f64",
                json!(0.2),
                "How quickly the bouncing stops.",
            ),
        ]),
    );
    register_fields(
        "RopeJoint",
        joint(vec![f(
            "maxDistance",
            "f64",
            json!(1.0),
            "How far apart the bodies may get.",
        )]),
    );
    register_fields(
        "Rope",
        vec![
            f("length", "f64", json!(1.0), "In metres."),
            f("segments", "i32", json!(10), "How many links."),
            f("radius", "f64", json!(0.02), "The rope's thickness."),
            f("segmentMass", "f64", json!(0.1), "Each link's mass."),
            f(
                "end",
                "component:Rigidbody",
                Value::Null,
                "The body its far end holds (none: free).",
            ),
        ],
    );
}
