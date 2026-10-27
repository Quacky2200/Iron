// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Which component classes have number properties an animation may drive, registered by the code
//! that owns the component (the engine API, the host). A curve for `("Camera", "fieldOfView")` writes
//! through the setter registered under that pair; a value field is a property of its own
//! (`backgroundColor.r`).

use crate::pose::PropertySink;
use platypus_scene::ObjectId;
use std::cell::RefCell;

type Has = fn(ObjectId) -> bool;
type Setter = fn(ObjectId, f64);

#[derive(Default)]
struct Registry {
    types: Vec<(&'static str, Has)>,
    setters: Vec<(&'static str, &'static str, Setter)>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Makes a component class known: `has` says whether an object has one.
pub fn register_type(name: &'static str, has: Has) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        match r.types.iter_mut().find(|t| t.0 == name) {
            Some(t) => t.1 = has,
            None => r.types.push((name, has)),
        }
    });
}

/// Makes a number property of a class animatable.
pub fn register_setter(type_name: &'static str, property: &'static str, set: Setter) {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        match r
            .setters
            .iter_mut()
            .find(|s| s.0 == type_name && s.1 == property)
        {
            Some(s) => s.2 = set,
            None => r.setters.push((type_name, property, set)),
        }
    });
}

/// The registered components, and (for the rest, such as a game's own compiled components) an
/// optional second sink.
pub struct RegistrySink<'a> {
    pub other: Option<&'a mut dyn PropertySink>,
}

impl PropertySink for RegistrySink<'_> {
    fn has_component(&mut self, object: ObjectId, type_name: &str) -> bool {
        let known = REGISTRY.with(|r| {
            r.borrow()
                .types
                .iter()
                .find(|t| t.0 == type_name)
                .map(|t| t.1)
        });
        match known {
            Some(has) => has(object),
            None => self
                .other
                .as_mut()
                .is_some_and(|o| o.has_component(object, type_name)),
        }
    }

    fn set_number(&mut self, object: ObjectId, type_name: &str, property: &str, value: f64) {
        let setter = REGISTRY.with(|r| {
            r.borrow()
                .setters
                .iter()
                .find(|s| s.0 == type_name && s.1 == property)
                .map(|s| s.2)
        });
        match setter {
            Some(set) => set(object, value),
            None => {
                if let Some(o) = self.other.as_mut() {
                    o.set_number(object, type_name, property, value);
                }
            }
        }
    }
}
