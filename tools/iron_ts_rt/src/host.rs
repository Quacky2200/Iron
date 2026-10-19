//! The engine side of compiled components: a class table (from the generated bindings'
//! `classes()`), one rooted instance per component and the lifecycle the QuickJS-based
//! `ComponentHost` has, with no script engine in the loop.
//!
//! Order and rules match `iron_script::ComponentHost`: `onAwake` when spawned, `onStart` once
//! before a component's first other hook, `onUpdate` for all and then `onLateUpdate` for all,
//! disabled or inactive-object components skipped, a component that throws is reported and
//! disabled. After every hook the microtask queue runs (the event-loop contract of `event.rs`).

use super::*;

pub const HOOK_AWAKE: usize = 0;
pub const HOOK_START: usize = 1;
pub const HOOK_UPDATE: usize = 2;
pub const HOOK_LATE_UPDATE: usize = 3;
pub const HOOK_FIXED_UPDATE: usize = 4;
pub const HOOK_PRE_RENDER: usize = 5;
pub const HOOK_DESTROY: usize = 6;
pub const HOOK_APP_FOCUS: usize = 7;
pub const HOOK_APP_PAUSE: usize = 8;
const HOOK_NAMES: [&str; 9] = [
    "onAwake",
    "onStart",
    "onUpdate",
    "onLateUpdate",
    "onFixedUpdate",
    "onPreRender",
    "onDestroy",
    "onApplicationFocus",
    "onApplicationPause",
];

type Create = unsafe extern "C" fn(f64, u64) -> *mut u8;
type Hook = unsafe extern "C" fn(*mut u8, f64);

/// One registered class: its constructor thunk and the hooks it has (a missing hook costs nothing).
pub struct NativeClass {
    pub name: &'static str,
    /// The class struct id in the compiled program: what `getComponent(Class)` passes.
    pub sid: u64,
    /// The class and its ancestors (`getComponent(Base)` finds subclasses).
    pub bases: &'static [u64],
    pub create: Create,
    pub hooks: [Option<Hook>; 9],
    /// The `@property` table: name, byte offset in the object, type name (`PROPS_<Class>`).
    pub props: &'static [(&'static str, usize, &'static str)],
}

/// A `@property` value as the engine sees it (scene files, the editor).
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// `null` or `undefined`.
    Null,
    Num(f64),
    Bool(bool),
    Str(String),
    /// Another component, by id.
    Component(u32),
}

/// A component in the thread registry: what `getComponent` searches.
struct Live {
    id: u32,
    object: u64,
    sid: u64,
    bases: &'static [u64],
    ptr: *mut u8,
}

impl Live {
    /// Is this a component of class `sid`, or of a subclass?
    fn is(&self, sid: u64) -> bool {
        self.sid == sid || self.bases.contains(&sid)
    }
}

/// A component `addComponent` made from script, waiting for the host to run its `onAwake`.
struct Pending {
    id: u32,
    class: usize,
    object: u64,
    root: usize,
    ptr: *mut u8,
}

#[derive(Default)]
struct Registry {
    /// (sid, class index, constructor, bases) of the classes of the current host.
    creators: Vec<(u64, usize, Create, &'static [u64])>,
    live: Vec<Live>,
    pending: Vec<Pending>,
    /// Components script asked to remove (`removeComponent`, `destroy()`): their `onDestroy` runs
    /// when the calling hook returns.
    removals: Vec<u32>,
    next_id: u32,
}

thread_local! {
    static REG: std::cell::RefCell<Registry> = std::cell::RefCell::new(Registry { next_id: 1, ..Registry::default() });
}

/// `go.getComponent(Class)`: the first live component of that class on the object, or null.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_get(object: u64, sid: f64) -> *mut u8 {
    REG.with(|r| {
        r.borrow()
            .live
            .iter()
            .find(|l| l.object == object && l.is(sid as u64))
            .map_or(std::ptr::null_mut(), |l| l.ptr)
    })
}

/// How many components of class `sid` (or a subclass) are on the object.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_count(object: u64, sid: f64) -> f64 {
    REG.with(|r| {
        r.borrow()
            .live
            .iter()
            .filter(|l| l.object == object && l.is(sid as u64))
            .count() as f64
    })
}

/// A new array of the object components of class `sid` (or a subclass), in the order they were
/// added. May collect first.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_all_gc(object: u64, sid: f64) -> *mut Array {
    let ptrs: Vec<u64> = REG.with(|r| {
        r.borrow()
            .live
            .iter()
            .filter(|l| l.object == object && l.is(sid as u64))
            .map(|l| l.ptr as u64)
            .collect()
    });
    component_array(&ptrs)
}

/// The first component of class `sid` (or a subclass) on any object, or null.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_find_any(sid: f64) -> *mut u8 {
    REG.with(|r| {
        r.borrow()
            .live
            .iter()
            .find(|l| l.is(sid as u64))
            .map_or(std::ptr::null_mut(), |l| l.ptr)
    })
}

/// `removeComponent` / `destroy()` on a component: it is gone for `getComponent` at once and its
/// `onDestroy` runs when the calling hook returns.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_remove(ptr: *mut u8) {
    REG.with(|r| {
        let mut r = r.borrow_mut();
        if let Some(i) = r.live.iter().position(|l| l.ptr == ptr) {
            let id = r.live[i].id;
            r.live.remove(i);
            // Added and removed in the same hook: it never wakes.
            if let Some(j) = r.pending.iter().position(|p| p.id == id) {
                let p = r.pending.remove(j);
                ts_rt_root_remove(p.root);
            } else {
                r.removals.push(id);
            }
        }
    });
}

/// An array holding these pointers (an array of objects).
fn component_array(ptrs: &[u64]) -> *mut Array {
    let arr = ts_rt_arr_new_gc(ptrs.len() as i64, 1);
    // The array is not reachable from anything yet; the pushes below do not allocate.
    let root = ts_rt_root_add(arr as *mut u8);
    for p in ptrs {
        // SAFETY: `arr` is a live array with room for every pointer.
        unsafe { ts_rt_arr_push(arr, *p) };
    }
    ts_rt_root_remove(root);
    arr
}

/// `go.addComponent(Class)`: makes the component now (fields default) and queues its `onAwake` for
/// the host, which runs it when the calling hook returns. May collect (it runs the constructor).
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_component_add_gc(object: u64, sid: f64) -> *mut u8 {
    let sid = sid as u64;
    let found = REG.with(|r| {
        let mut r = r.borrow_mut();
        let (_, class, create, bases) = *r.creators.iter().find(|c| c.0 == sid)?;
        let id = r.next_id;
        r.next_id += 1;
        Some((id, class, create, bases))
    });
    let Some((id, class, create, bases)) = found else {
        panic!("addComponent: the class is not registered with a NativeHost");
    };
    // SAFETY: the thunk was generated for this class; the registry is not borrowed while it runs.
    let ptr = unsafe { create(id as f64, object) };
    let root = ts_rt_root_add(ptr);
    REG.with(|r| {
        let mut r = r.borrow_mut();
        r.live.push(Live {
            id,
            object,
            sid,
            bases,
            ptr,
        });
        r.pending.push(Pending {
            id,
            class,
            object,
            root,
            ptr,
        });
    });
    ptr
}

struct Entry {
    id: u32,
    class: usize,
    /// The engine object (a raw handle) the component is on, if any.
    object: Option<u64>,
    root: usize,
    this: *mut u8,
    started: bool,
    enabled: bool,
}

/// A component whose hook threw: it is disabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub id: u32,
    pub class: &'static str,
    pub hook: &'static str,
}

pub struct NativeHost {
    classes: Vec<NativeClass>,
    entries: Vec<Entry>,
}

impl Drop for NativeHost {
    fn drop(&mut self) {
        for e in self.entries.drain(..) {
            ts_rt_root_remove(e.root);
        }
        REG.with(|r| {
            let mut r = r.borrow_mut();
            for p in r.pending.drain(..) {
                ts_rt_root_remove(p.root);
            }
            r.creators.clear();
            r.live.clear();
        });
    }
}

impl NativeHost {
    pub fn new(classes: Vec<NativeClass>) -> Self {
        REG.with(|r| {
            let mut r = r.borrow_mut();
            r.creators = classes
                .iter()
                .enumerate()
                .map(|(i, c)| (c.sid, i, c.create, c.bases))
                .collect();
            r.live.clear();
            r.pending.clear();
        });
        NativeHost {
            classes,
            entries: Vec::new(),
        }
    }

    pub fn class_names(&self) -> Vec<&'static str> {
        self.classes.iter().map(|c| c.name).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The ids of the live components, in creation order.
    pub fn ids(&self) -> Vec<u32> {
        self.entries.iter().map(|e| e.id).collect()
    }

    pub fn contains(&self, id: u32) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    pub fn is_enabled(&self, id: u32) -> bool {
        self.entries.iter().any(|e| e.id == id && e.enabled)
    }

    /// Creates a component of `class` on `object` (a raw handle, or none) and runs its `onAwake`.
    /// An unknown class is an `Err`; a throwing `onAwake` is a fault and the component is dropped.
    pub fn spawn(&mut self, class: &str, object: Option<u64>) -> Result<u32, String> {
        let Some(ci) = self.classes.iter().position(|c| c.name == class) else {
            return Err(format!("no component class `{class}` is registered"));
        };
        let id = REG.with(|r| {
            let mut r = r.borrow_mut();
            let id = r.next_id;
            r.next_id += 1;
            id
        });
        // SAFETY: the thunk was generated for this class.
        let this = unsafe { (self.classes[ci].create)(id as f64, object.unwrap_or(0)) };
        let root = ts_rt_root_add(this);
        REG.with(|r| {
            r.borrow_mut().live.push(Live {
                id,
                object: object.unwrap_or(0),
                sid: self.classes[ci].sid,
                bases: self.classes[ci].bases,
                ptr: this,
            })
        });
        self.entries.push(Entry {
            id,
            class: ci,
            object,
            root,
            this,
            started: false,
            enabled: true,
        });
        let i = self.entries.len() - 1;
        if let Some(f) = self.call(i, HOOK_AWAKE, 0.0) {
            self.drop_entry(i);
            return Err(format!("{}.{} threw", f.class, f.hook));
        }
        self.adopt();
        Ok(id)
    }

    /// Runs what script asked for in a hook that has returned: components added get their `onAwake`
    /// and components removed their `onDestroy`. The host does this after every hook it runs; call
    /// it after calling compiled code directly.
    pub fn settle(&mut self) -> Vec<Fault> {
        self.adopt()
    }

    /// Takes the components `addComponent` made from script and runs their `onAwake`. A component
    /// made by an `onAwake` is taken in turn.
    fn adopt(&mut self) -> Vec<Fault> {
        let mut faults = Vec::new();
        // Components script removed: `onDestroy` runs, then they are forgotten.
        while let Some(id) = REG.with(|r| {
            let mut r = r.borrow_mut();
            (!r.removals.is_empty()).then(|| r.removals.remove(0))
        }) {
            self.destroy(id);
        }
        loop {
            let next = REG.with(|r| {
                let mut r = r.borrow_mut();
                (!r.pending.is_empty()).then(|| r.pending.remove(0))
            });
            let Some(p) = next else {
                return faults;
            };
            self.entries.push(Entry {
                id: p.id,
                class: p.class,
                object: (p.object != 0).then_some(p.object),
                root: p.root,
                this: p.ptr,
                started: false,
                enabled: true,
            });
            let i = self.entries.len() - 1;
            if let Some(f) = self.call(i, HOOK_AWAKE, 0.0) {
                faults.push(f);
            }
        }
    }

    /// One frame: `onStart` for new components, `onUpdate` for all, then `onLateUpdate` for all.
    /// `active` says whether an object is active in the scene.
    pub fn update(&mut self, dt: f64, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        let mut faults = self.run(HOOK_UPDATE, dt, active);
        faults.extend(self.run(HOOK_LATE_UPDATE, dt, active));
        faults
    }

    /// One fixed tick.
    pub fn fixed_update(&mut self, dt: f64, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        self.run(HOOK_FIXED_UPDATE, dt, active)
    }

    /// Just before the frame is drawn.
    pub fn pre_render(&mut self, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        self.run(HOOK_PRE_RENDER, 0.0, active)
    }

    /// Runs `onDestroy` and forgets the component.
    pub fn destroy(&mut self, id: u32) {
        if let Some(i) = self.entries.iter().position(|e| e.id == id) {
            if self.entries[i].enabled {
                let _ = self.call(i, HOOK_DESTROY, 0.0);
            }
            self.drop_entry(i);
            self.adopt();
        }
    }

    /// The window gained or lost focus: `onApplicationFocus(hasFocus)` on the components of active
    /// objects.
    pub fn application_focus(&mut self, focused: bool, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        self.flag_hook(HOOK_APP_FOCUS, focused, active)
    }

    /// The game paused in the background or resumed: `onApplicationPause(paused)`.
    pub fn application_pause(&mut self, paused: bool, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        self.flag_hook(HOOK_APP_PAUSE, paused, active)
    }

    fn flag_hook(&mut self, hook: usize, flag: bool, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        let mut faults = Vec::new();
        for i in 0..self.entries.len() {
            let e = &self.entries[i];
            if !e.enabled || e.object.is_some_and(|o| !active(o)) {
                continue;
            }
            if let Some(f) = self.call(i, hook, f64::from(u8::from(flag))) {
                faults.push(f);
            }
        }
        faults.extend(self.adopt());
        faults
    }

    /// Destroys every component on `objects` (the engine destroyed them).
    pub fn destroy_on_objects(&mut self, objects: &[u64]) {
        let ids: Vec<u32> = self
            .entries
            .iter()
            .filter(|e| e.object.is_some_and(|o| objects.contains(&o)))
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.destroy(id);
        }
    }

    fn run(&mut self, hook: usize, dt: f64, active: &dyn Fn(u64) -> bool) -> Vec<Fault> {
        let mut faults = Vec::new();
        // Components spawned by a hook join the next pass, so the list is walked by index.
        let n = self.entries.len();
        for i in 0..n {
            let e = &self.entries[i];
            if !e.enabled || e.object.is_some_and(|o| !active(o)) {
                continue;
            }
            if !e.started {
                self.entries[i].started = true;
                let f = self.call(i, HOOK_START, 0.0);
                faults.extend(self.adopt());
                if let Some(f) = f {
                    faults.push(f);
                    continue;
                }
            }
            if let Some(f) = self.call(i, hook, dt) {
                faults.push(f);
            }
            faults.extend(self.adopt());
        }
        faults
    }

    /// Calls a hook if the class has it. A pending exception disables the component.
    fn call(&mut self, i: usize, hook: usize, dt: f64) -> Option<Fault> {
        let e = &self.entries[i];
        let class = &self.classes[e.class];
        let f = class.hooks[hook]?;
        // SAFETY: `this` is rooted and has the class the thunk was generated for.
        unsafe { f(e.this, dt) };
        ts_rt_microtasks_run();
        if ts_rt_exc_pending() {
            ts_rt_exc_take();
            let id = e.id;
            self.entries[i].enabled = false;
            return Some(Fault {
                id,
                class: class.name,
                hook: HOOK_NAMES[hook],
            });
        }
        None
    }

    fn drop_entry(&mut self, i: usize) {
        let e = self.entries.remove(i);
        REG.with(|r| r.borrow_mut().live.retain(|l| l.id != e.id));
        ts_rt_root_remove(e.root);
    }

    fn entry(&self, id: u32) -> Result<&Entry, String> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| format!("no component {id}"))
    }

    /// The `@property` fields of a component: name and type name.
    pub fn properties(&self, id: u32) -> Result<Vec<(&'static str, &'static str)>, String> {
        let e = self.entry(id)?;
        Ok(self.classes[e.class]
            .props
            .iter()
            .map(|p| (p.0, p.2))
            .collect())
    }

    /// Reads a `@property` through the reflection table, with no script running.
    pub fn get_property(&self, id: u32, name: &str) -> Result<PropValue, String> {
        let e = self.entry(id)?;
        let class = &self.classes[e.class];
        let &(_, off, kind) = class
            .props
            .iter()
            .find(|p| p.0 == name)
            .ok_or_else(|| format!("{} has no property `{name}`", class.name))?;
        let (base, nullable) = split_kind(kind);
        // SAFETY: the table offsets were generated for this class and its fields are 8 bytes.
        let word = unsafe { *(e.this.add(off) as *const u64) };
        match base {
            "number" => Ok(if nullable && word == NULL_NUM_BITS {
                PropValue::Null
            } else {
                PropValue::Num(f64::from_bits(word))
            }),
            "boolean" => Ok(match (nullable, word) {
                (true, 0) => PropValue::Null,
                (true, w) => PropValue::Bool(w == 2),
                (false, w) => PropValue::Bool(w != 0),
            }),
            "string" => Ok(if word == 0 {
                PropValue::Null
            } else {
                // SAFETY: a string field holds a live string or null.
                PropValue::Str(unsafe { Str::from_raw(word as *const Str) }.to_string_lossy())
            }),
            b if b.starts_with("object:") => {
                if word == 0 {
                    return Ok(PropValue::Null);
                }
                REG.with(|r| {
                    r.borrow()
                        .live
                        .iter()
                        .find(|l| l.ptr as u64 == word)
                        .map(|l| PropValue::Component(l.id))
                })
                .ok_or_else(|| format!("property `{name}` holds an object that is not a component"))
            }
            _ => Err(format!(
                "property `{name}` has the unsupported type `{kind}`"
            )),
        }
    }

    /// Writes a `@property` through the reflection table.
    pub fn set_property(&mut self, id: u32, name: &str, v: PropValue) -> Result<(), String> {
        let e = self.entry(id)?;
        let class = &self.classes[e.class];
        let &(_, off, kind) = class
            .props
            .iter()
            .find(|p| p.0 == name)
            .ok_or_else(|| format!("{} has no property `{name}`", class.name))?;
        let (base, nullable) = split_kind(kind);
        let word: u64 = match (base, &v) {
            ("number", PropValue::Null) if nullable => NULL_NUM_BITS,
            (_, PropValue::Null) if nullable => 0,
            ("number", PropValue::Num(n)) => n.to_bits(),
            ("boolean", PropValue::Bool(b)) => match (nullable, b) {
                (true, true) => 2,
                (true, false) => 1,
                (false, b) => *b as u64,
            },
            ("string", PropValue::Str(s)) => {
                // SAFETY: the bytes are valid UTF-8, hence WTF-8; this never collects.
                unsafe { ts_rt_str_new(s.as_ptr(), s.len() as i64) as u64 }
            }
            (b, PropValue::Component(c)) if b.starts_with("object:") => {
                let other = self.entry(*c)?;
                if self.classes[other.class].name != &b["object:".len()..] {
                    return Err(format!(
                        "property `{name}` is `{kind}`, but component {c} is a {}",
                        self.classes[other.class].name
                    ));
                }
                other.this as u64
            }
            _ => return Err(format!("property `{name}` is `{kind}`, not {v:?}")),
        };
        // SAFETY: as in `get_property`.
        unsafe { *(e.this.add(off) as *mut u64) = word };
        Ok(())
    }

    /// Every supported `@property` of a component: what a scene file stores.
    pub fn save_properties(&self, id: u32) -> Result<Vec<(&'static str, PropValue)>, String> {
        let mut out = Vec::new();
        for (name, kind) in self.properties(id)? {
            let base = split_kind(kind).0;
            if matches!(base, "number" | "boolean" | "string") || base.starts_with("object:") {
                out.push((name, self.get_property(id, name)?));
            }
        }
        Ok(out)
    }

    /// Applies saved values; a value that does not fit is an error in the returned list and the
    /// rest are still applied.
    pub fn load_properties(&mut self, id: u32, values: &[(&str, PropValue)]) -> Vec<String> {
        let mut errors = Vec::new();
        for (name, v) in values {
            if let Err(e) = self.set_property(id, name, v.clone()) {
                errors.push(e);
            }
        }
        errors
    }
}

/// `"number?"` is (`"number"`, nullable).
fn split_kind(kind: &str) -> (&str, bool) {
    match kind.strip_suffix('?') {
        Some(b) => (b, true),
        None => (kind, false),
    }
}

const NULL_NUM_BITS: u64 = 0x7FF8_DEAD_BEEF_0001;
