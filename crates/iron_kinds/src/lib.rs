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
    static COMPONENTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Owners call this when they add or remove a component (any kind): what is worked out from
/// which components are on which objects is kept against [`components_version`].
pub fn components_changed() {
    COMPONENTS.with(|c| c.set(c.get() + 1));
}

/// A number that changes whenever a component is added or removed on this thread.
pub fn components_version() -> u64 {
    COMPONENTS.with(std::cell::Cell::get)
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
    ASSET_KINDS.with(|k| k.borrow_mut().clear());
    OBJECT_EXTRAS.with(|k| k.borrow_mut().clear());
}

/// An asset kind: a value that lives outside components and is shared by them (`Texture`, `Mesh`,
/// `Material`). A component kind saves one as the placeholder `{"$handle":"Texture","id":N}` (null
/// for none) and loads it the same way; the scene loader and saver translate:
///
/// - a **shared** kind goes into the scene file's `assets` table (`{"$asset":"texture1"}` in the
///   property, the table entry `{"type":"Texture", ...}`); one handle is one entry however many
///   components use it;
/// - an **inline** kind is written in the property itself as `{"$type":"Material", ...}`.
///
/// The `def` is the JSON the file holds (for a shared kind the entry, whose `type` is the kind's
/// name); a definition may itself contain `$handle` placeholders for other assets.
/// Makes an asset from its definition (the handle).
pub type AssetCreateFn = Box<dyn Fn(&Value) -> Result<u64, String>>;
/// The definition of an asset handle (`None`: it cannot be saved).
pub type AssetDescribeFn = Box<dyn Fn(u64) -> Option<Value>>;

pub struct AssetKind {
    pub name: String,
    /// Other `type` names in definitions that this kind makes too (`RenderTexture`).
    pub also: Vec<String>,
    /// Shared (assets table) or inline (`$type`).
    pub shared: bool,
    /// Makes the asset from its definition.
    pub create: AssetCreateFn,
    /// The definition of an asset, or `None` when it cannot be saved (a texture made from pixels).
    pub describe: AssetDescribeFn,
}

thread_local! {
    static ASSET_KINDS: RefCell<Vec<Rc<AssetKind>>> = const { RefCell::new(Vec::new()) };
}

/// Registers an asset kind, replacing one of the same name.
pub fn register_asset(kind: AssetKind) {
    ASSET_KINDS.with(|k| {
        let mut k = k.borrow_mut();
        let kind = Rc::new(kind);
        match k.iter().position(|x| x.name == kind.name) {
            Some(i) => k[i] = kind,
            None => k.push(kind),
        }
    });
}

/// The asset kind of that name.
pub fn get_asset(name: &str) -> Option<Rc<AssetKind>> {
    ASSET_KINDS.with(|k| {
        k.borrow()
            .iter()
            .find(|x| x.name == name || x.also.iter().any(|a| a == name))
            .cloned()
    })
}

/// Something an object itself carries in a scene file besides its transform and components
/// (an instantiated model: `"model": { "src": .. }`). The saver asks `save` for each object and
/// leaves out objects `hidden` says are parts of something else (a model's nodes); the loader
/// calls `load` when an object's data has the key.
/// Applies an object's saved extra to it.
pub type ExtraLoadFn = Box<dyn Fn(u64, &Value)>;

pub struct ObjectExtra {
    pub key: String,
    pub save: Box<dyn Fn(u64) -> Option<Value>>,
    pub load: ExtraLoadFn,
    pub hidden: Box<dyn Fn(u64) -> bool>,
}

thread_local! {
    static OBJECT_EXTRAS: RefCell<Vec<Rc<ObjectExtra>>> = const { RefCell::new(Vec::new()) };
}

/// Registers an object extra, replacing one with the same key.
pub fn register_object_extra(extra: ObjectExtra) {
    OBJECT_EXTRAS.with(|k| {
        let mut k = k.borrow_mut();
        let extra = Rc::new(extra);
        match k.iter().position(|x| x.key == extra.key) {
            Some(i) => k[i] = extra,
            None => k.push(extra),
        }
    });
}

/// Every object extra, in registration order.
pub fn object_extras() -> Vec<Rc<ObjectExtra>> {
    OBJECT_EXTRAS.with(|k| k.borrow().clone())
}

/// A placeholder for an asset of a kind (`null` for no asset, handle 0).
pub fn asset_ref(kind: &str, handle: u64) -> Value {
    if handle == 0 {
        Value::Null
    } else {
        serde_json::json!({ "$handle": kind, "id": handle })
    }
}

/// The handle in an asset placeholder of that kind (`Some(0)` for null).
pub fn asset_handle(value: &Value, kind: &str) -> Option<u64> {
    match value {
        Value::Null => Some(0),
        Value::Object(o) if o.get("$handle").and_then(Value::as_str) == Some(kind) => {
            o.get("id").and_then(Value::as_u64)
        }
        _ => None,
    }
}

/// A placeholder for a GameObject.
pub fn object_ref(object: u64) -> Value {
    serde_json::json!({ "$ref": "GameObject", "object": object })
}

/// A placeholder for a component.
pub fn component_ref(kind: &str, handle: u64) -> Value {
    serde_json::json!({ "$ref": "Component", "kind": kind, "handle": handle })
}
