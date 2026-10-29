// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Components of kinds the program doesn't register survive a load and a save.

use crate::manager;
use serde_json::{Value, json};

const SCENE: &str = r#"{"format":"platypus.scene","version":1,"name":"Kept","objects":[
  {"id":1,"name":"Turntable","components":[
    {"id":2,"type":"Spinner","properties":{"speed":3,"axis":[0,1,0],"target":{"$ref":"GameObject","id":4}}},
    {"id":3,"type":"Camera","properties":{"fieldOfView":50}}
  ]},
  {"id":4,"name":"Target","components":[
    {"id":5,"type":"Follower","enabled":false,"properties":{"spinner":{"$ref":"Component","id":2}}}
  ]}
]}"#;

fn object<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == name)
        .unwrap()
}

// M5-SCN-10
#[test]
fn unknown_components_are_kept_and_saved_back_in_place() {
    manager::reset();
    let mut scene = platypus_scene::Scene::new();
    let _scope = platypus_scene::api::enter(&mut scene);
    let s = manager::load_scene(SCENE, false).unwrap();
    let warnings = crate::take_warnings();
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Spinner") && w.contains("kept")),
        "{warnings:?}"
    );
    let saved: Value = serde_json::from_str(&manager::serialize(s).unwrap()).unwrap();

    let turntable = object(&saved, "Turntable");
    let components = turntable["components"].as_array().unwrap();
    let types: Vec<_> = components.iter().map(|c| c["type"].clone()).collect();
    assert_eq!(
        types,
        [json!("Spinner"), json!("Camera")],
        "file order kept"
    );
    let spinner = &components[0];
    assert_eq!(spinner["properties"]["speed"], json!(3));
    assert_eq!(spinner["properties"]["axis"], json!([0, 1, 0]));
    // References to and from kept components point at the right things after renumbering.
    let target = object(&saved, "Target");
    assert_eq!(spinner["properties"]["target"]["id"], target["id"]);
    let follower = &target["components"][0];
    assert_eq!(follower["type"], json!("Follower"));
    assert_eq!(follower["enabled"], json!(false), "other keys kept");
    assert_eq!(follower["properties"]["spinner"]["id"], spinner["id"]);

    // A second round trip gives the same data.
    let s2 = manager::load_scene(&saved.to_string(), false).unwrap();
    let again: Value = serde_json::from_str(&manager::serialize(s2).unwrap()).unwrap();
    assert_eq!(again, saved);

    // Copies carry them too.
    let root = manager::roots(s2)[0];
    let copy = manager::instantiate(root, None, None, None).unwrap();
    let kept = crate::opaque_components(copy);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].ty, "Spinner");
}
