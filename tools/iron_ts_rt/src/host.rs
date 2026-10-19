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
const HOOK_NAMES: [&str; 7] = [
    "onAwake",
    "onStart",
    "onUpdate",
    "onLateUpdate",
    "onFixedUpdate",
    "onPreRender",
    "onDestroy",
];

type Create = unsafe extern "C" fn(f64, u64) -> *mut u8;
type Hook = unsafe extern "C" fn(*mut u8, f64);

/// One registered class: its constructor thunk and the hooks it has (a missing hook costs nothing).
pub struct NativeClass {
    pub name: &'static str,
    pub create: Create,
    pub hooks: [Option<Hook>; 7],
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
    next_id: u32,
}

impl NativeHost {
    pub fn new(classes: Vec<NativeClass>) -> Self {
        NativeHost {
            classes,
            entries: Vec::new(),
            next_id: 1,
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
        let id = self.next_id;
        self.next_id += 1;
        // SAFETY: the thunk was generated for this class.
        let this = unsafe { (self.classes[ci].create)(id as f64, object.unwrap_or(0)) };
        let root = ts_rt_root_add(this);
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
        Ok(id)
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
        }
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
                if let Some(f) = self.call(i, HOOK_START, 0.0) {
                    faults.push(f);
                    continue;
                }
            }
            if let Some(f) = self.call(i, hook, dt) {
                faults.push(f);
            }
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
        ts_rt_root_remove(e.root);
    }
}
