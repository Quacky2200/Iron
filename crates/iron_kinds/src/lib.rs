//! The registry of component kinds: how the scene loader, `instantiate` and the editor make,
//! save and load components without knowing their classes. A kind is a name (`Camera`, or a
//! class a game registered) and a set of functions over the component's handle; whoever owns
//! the components (the engine API, a compiled program's `NativeHost`) registers one per class.
//!
//! Everything is per thread, like the registries it fronts. A kind works on raw `u64`s so this
//! crate needs no engine types: `object` is a GameObject's raw handle, a component handle is
//! whatever number the kind uses (a component id).
//!
//! **References.** A property that points at another object or component is a placeholder value:
//! `{"$ref":"GameObject","object":RAW}` or `{"$ref":"Component","kind":"Spinner","handle":N}`.
//! The scene saver turns placeholders into file ids (`{"$ref":..,"id":N}`) and the loader turns
//! them back, so a kind never sees file ids.

use serde_json::{Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Saved property values by name.
pub type Props = Map<String, Value>;
/// Applies saved values to a component; returns warnings.
pub type LoadFn = Box<dyn Fn(u64, &Props) -> Vec<String>>;
/// Wakes a batch of components.
pub type AwakeFn = Box<dyn Fn(&[u64])>;

/// A component kind. Each function is called with the kind's own component handles.
pub struct Kind {
    pub name: String,
    /// Adds a component to an object, not awake yet.
    pub create: Box<dyn Fn(u64) -> Result<u64, String>>,
    /// The kind's components on an object, in the order they were added.
    pub on: Box<dyn Fn(u64) -> Vec<u64>>,
    /// The `@property` values, as JSON (references as placeholders). Values the kind cannot
    /// save are left out.
    pub save: Box<dyn Fn(u64) -> Map<String, Value>>,
    /// Applies saved values; returns warnings for the ones that did not fit.
    pub load: LoadFn,
    /// Wakes components made by `create` and loaded (their `onAwake`), in order.
    pub awake: AwakeFn,
    /// The object of a component.
    pub object: Box<dyn Fn(u64) -> Option<u64>>,
    /// Removes a component (its `onDestroy` runs when the host next settles).
    pub destroy: Box<dyn Fn(u64)>,
}

thread_local! {
    static KINDS: RefCell<Vec<Rc<Kind>>> = const { RefCell::new(Vec::new()) };
}

/// Registers a kind, replacing one of the same name (a reload registers its classes again).
pub fn register(kind: Kind) {
    KINDS.with(|k| {
        let mut k = k.borrow_mut();
        let kind = Rc::new(kind);
        match k.iter().position(|x| x.name == kind.name) {
            Some(i) => k[i] = kind,
            None => k.push(kind),
        }
    });
}

/// Removes a kind.
pub fn unregister(name: &str) {
    KINDS.with(|k| k.borrow_mut().retain(|x| x.name != name));
}

/// The kind of that name.
pub fn get(name: &str) -> Option<Rc<Kind>> {
    KINDS.with(|k| k.borrow().iter().find(|x| x.name == name).cloned())
}

/// Every kind, in registration order (the order a saved object lists its components in).
pub fn all() -> Vec<Rc<Kind>> {
    KINDS.with(|k| k.borrow().clone())
}

/// Forgets every kind (tests).
pub fn clear() {
    KINDS.with(|k| k.borrow_mut().clear());
}

/// A placeholder for a GameObject.
pub fn object_ref(object: u64) -> Value {
    serde_json::json!({ "$ref": "GameObject", "object": object })
}

/// A placeholder for a component.
pub fn component_ref(kind: &str, handle: u64) -> Value {
    serde_json::json!({ "$ref": "Component", "kind": kind, "handle": handle })
}
