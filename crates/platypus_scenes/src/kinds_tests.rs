// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Scene files, prefabs and copies of the animation components.

use crate::manager;
use platypus_animation::{CullingMode, CullingType};
use platypus_engine_api::animation::{AnimationRef, ClipRef, CurveRef};
use platypus_engine_api::animator::{AnimatorRef, ControllerRef, NodeRef};
use platypus_scene::ObjectId;
use serde_json::Value;

fn named(scene: u32, name: &str) -> ObjectId {
    manager::roots(scene)
        .into_iter()
        .find(|&o| platypus_scene::api::with_scene(|s| s.name(o) == Some(name)).unwrap_or(false))
        .expect("loaded")
}

/// Removes these keys anywhere in a JSON value.
fn strip(v: &mut Value, keys: &[&str]) {
    match v {
        Value::Object(m) => {
            for k in keys {
                m.remove(*k);
            }
            m.values_mut().for_each(|x| strip(x, keys));
        }
        Value::Array(a) => a.iter_mut().for_each(|x| strip(x, keys)),
        _ => {}
    }
}

// M9-REN-03
#[test]
fn scene_and_prefab_files_from_before_the_rename_still_load() {
    manager::reset();
    let mut scene = platypus_scene::Scene::new();
    let _scope = platypus_scene::api::enter(&mut scene);
    let s = manager::load_scene(
        r#"{"format":"iron.scene","version":1,"name":"Old","objects":[{"name":"o"}]}"#,
        false,
    )
    .unwrap();
    named(s, "o");
    let p = manager::prefab_from_data(
        r#"{"format":"iron.prefab","version":1,"name":"P","root":{"id":1,"name":"Made"}}"#,
        None,
    );
    assert!(p.is_ok(), "{p:?}");
    assert!(manager::load_scene(r#"{"format":"other","version":1}"#, false).is_err());
}

// M5-ANI-16
#[test]
fn animation_components_keep_their_settings_through_files_prefabs_and_copies() {
    platypus_engine_api::animation::reset();
    manager::reset();
    let mut scene = platypus_scene::Scene::new();
    let _scope = platypus_scene::api::enter(&mut scene);
    let clip = ClipRef::create("swing");
    clip.set_curve(
        "",
        "Transform",
        "localPosition.x",
        CurveRef::linear(0.0, 0.0, 1.0, 1.0),
    );
    let other = ClipRef::create("sway");
    let controller = ControllerRef::create();
    controller.add_node(NodeRef::create("Swing", clip));
    let (figure, lamp) =
        platypus_scene::api::with_scene(|s| (s.create("Figure"), s.create("Lamp"))).unwrap();
    let animator = AnimatorRef::attach(figure);
    animator.set_controller(controller);
    animator.set_speed(1.5);
    animator.set_culling_mode(CullingMode::CullUpdateTransforms);
    animator.set_enabled(false);
    let animation = AnimationRef::attach(lamp);
    animation.set_clip(clip);
    animation.add_clip(other, "");
    animation.set_play_automatically(false);
    animation.set_culling_type(CullingType::BasedOnRenderers);
    let level = manager::create_scene("level");
    manager::move_to_scene(figure, level);
    manager::move_to_scene(lamp, level);
    let text = manager::serialize(level).unwrap();
    assert!(
        text.contains(r#""cullingMode":"cullUpdateTransforms""#),
        "{text}"
    );
    assert!(text.contains(r#""cullingType":"basedOnRenderers""#));

    let check_animator = |a: AnimatorRef, same_controller: bool| {
        assert_ne!(a.0, 0);
        assert_eq!(a.culling_mode(), CullingMode::CullUpdateTransforms);
        assert_eq!(a.speed(), 1.5);
        assert!(!a.enabled());
        assert_eq!(a.controller().node_count(), 1.0);
        assert_eq!(a.controller() == controller, same_controller);
    };
    let check_animation = |a: AnimationRef, same_clip: bool| {
        assert_eq!(a.culling_type(), CullingType::BasedOnRenderers);
        assert!(!a.play_automatically());
        assert_eq!(a.clip().name(), "swing");
        assert_eq!(a.clip_count(), 1.0);
        assert_eq!(a.clip_at(0.0).name(), "sway");
        assert_eq!(a.clip() == clip, same_clip);
    };
    // From the file: the controller and clips are made again (one controller, shared).
    let loaded = manager::load_scene(&text, true).unwrap();
    check_animator(AnimatorRef::find_on(named(loaded, "Figure")), false);
    check_animation(AnimationRef::find_on(named(loaded, "Lamp")), false);
    // A prefab and a copy keep the live controller and clips.
    let prefab = manager::prefab_from_object(figure);
    let placed = manager::instantiate_prefab(prefab, None, None, None).unwrap();
    check_animator(AnimatorRef::find_on(placed), true);
    let copy = manager::instantiate(lamp, None, None, None).unwrap();
    check_animation(AnimationRef::find_on(copy), true);
    assert!(crate::take_warnings().is_empty());

    // A file from before these were saved loads with the defaults, and quietly.
    let mut old: Value = serde_json::from_str(&text).unwrap();
    strip(
        &mut old,
        &[
            "controller",
            "speed",
            "cullingMode",
            "enabled",
            "clip",
            "clips",
            "playAutomatically",
            "cullingType",
        ],
    );
    let old = manager::load_scene(&old.to_string(), true).unwrap();
    let a = AnimatorRef::find_on(named(old, "Figure"));
    assert_eq!(
        (a.culling_mode(), a.speed(), a.enabled(), a.controller().0),
        (CullingMode::AlwaysAnimate, 1.0, true, 0)
    );
    let b = AnimationRef::find_on(named(old, "Lamp"));
    assert_eq!(
        (b.culling_type(), b.play_automatically(), b.clip_count()),
        (CullingType::AlwaysAnimate, true, 0.0)
    );
    assert!(crate::take_warnings().is_empty());
    platypus_engine_api::animation::reset();
    manager::reset();
}

// M9-API-212
#[test]
fn tags_survive_scene_files_and_copies() {
    manager::reset();
    let mut scene = iron_scene::Scene::new();
    let _scope = iron_scene::api::enter(&mut scene);
    let (boss, plain) =
        iron_scene::api::with_scene(|s| (s.create("Boss"), s.create("Plain"))).unwrap();
    iron_scene::api::with_scene(|s| s.set_tag_name(boss, "Enemy"));
    let level = manager::create_scene("tagged");
    manager::move_to_scene(boss, level);
    manager::move_to_scene(plain, level);
    let text = manager::serialize(level).unwrap();
    assert!(text.contains(r#""tag":"Enemy""#), "{text}");
    // Untagged objects add nothing to the file.
    assert_eq!(text.matches(r#""tag""#).count(), 1, "{text}");
    let loaded = manager::load_scene(&text, true).unwrap();
    let tag = |o| iron_scene::api::with_scene(|s| s.tag_name(o).to_owned()).unwrap();
    assert_eq!(tag(named(loaded, "Boss")), "Enemy");
    assert_eq!(tag(named(loaded, "Plain")), "Untagged");
    let copy = manager::instantiate(boss, None, None, None).unwrap();
    assert_eq!(tag(copy), "Enemy");
}
