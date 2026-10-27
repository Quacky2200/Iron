// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

use platypus_core::ComponentId;

use crate::{ScriptError, ScriptRuntime, Value};

/// Drives script components through the JS-side host installed by `@platypus/engine`:
/// `__platypus_registered()`, `__platypus_create(class, id, object, name)`, `__platypus_hooks(id)`,
/// `__platypus_dispatch(id, method, ...args)`, `__platypus_run(method, ids, ...args)` (one hook
/// on many components in one call), `__platypus_destroy(id)` and, once per frame,
/// `__platypus_tick(dt)` (which advances script `Time` and returns the scaled `dt`).
///
/// The lifecycle follows Unity / s&box: `onStart` once before anything else, then
/// `onFixedUpdate(dt)` on each fixed tick ([`Self::fixed_update`]), `onUpdate(dt)` and
/// `onLateUpdate(dt)` each frame ([`Self::update_active`]), `onPreRender()` just before
/// drawing ([`Self::pre_render`]) and `onDestroy` at the end.
///
/// Rust keeps the list of components; JS only holds instances keyed by ID. Methods are
/// always called by `(component id, method name)`, so a reloaded bundle's code is
/// picked up automatically (DESIGN.md §3). Each component's hooks are looked up once,
/// so missing ones cost nothing. Components belong to GameObjects (by the script's
/// object id); those on inactive objects don't run.
#[derive(Default)]
pub struct ComponentHost {
    next_id: u32,
    components: Vec<Entry>,
    /// Whether the bundle has `__platypus_tick` (checked on the first update).
    tick_hook: Option<bool>,
    /// Whether the bundle has `__platypus_animate` (animation players).
    animate_hook: Option<bool>,
    /// Whether the bundle has `__platypus_run` (hooks batched into one call each).
    run_hook: Option<bool>,
    /// The last frame's scaled length (`frame_dt`).
    frame_dt: f64,
}

struct Entry {
    id: ComponentId,
    class: String,
    /// The GameObject it's attached to (script object id).
    object: Option<u32>,
    started: bool,
    enabled: bool,
    /// Which lifecycle methods the component has (looked up on its first run).
    hooks: Option<Hooks>,
}

/// Lifecycle methods a component defines.
#[derive(Clone, Copy, Debug, Default)]
struct Hooks {
    start: bool,
    fixed_update: bool,
    update: bool,
    late_update: bool,
    pre_render: bool,
    destroy: bool,
}

impl Hooks {
    fn has(&self, method: &str) -> bool {
        match method {
            "onStart" => self.start,
            "onFixedUpdate" => self.fixed_update,
            "onUpdate" => self.update,
            "onLateUpdate" => self.late_update,
            "onPreRender" => self.pre_render,
            "onDestroy" => self.destroy,
            _ => true,
        }
    }
}

/// Asks JS which lifecycle methods a component has (all of them if the bundle can't
/// say, so nothing is skipped).
fn hooks(rt: &mut dyn ScriptRuntime, id: ComponentId) -> Hooks {
    let Ok(Value::Array(names)) = rt.call_global("__platypus_hooks", &[id.raw().into()]) else {
        return Hooks {
            start: true,
            fixed_update: true,
            update: true,
            late_update: true,
            pre_render: true,
            destroy: true,
        };
    };
    let mut h = Hooks::default();
    for name in names.iter().filter_map(Value::as_str) {
        match name {
            "onStart" => h.start = true,
            "onFixedUpdate" => h.fixed_update = true,
            "onUpdate" => h.update = true,
            "onLateUpdate" => h.late_update = true,
            "onPreRender" => h.pre_render = true,
            "onDestroy" => h.destroy = true,
            _ => {}
        }
    }
    h
}

impl ComponentHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Component class names the loaded bundle registered.
    pub fn registered_classes(rt: &mut dyn ScriptRuntime) -> Result<Vec<String>, ScriptError> {
        match rt.call_global("__platypus_registered", &[])? {
            Value::Array(items) => Ok(items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()),
            other => Err(ScriptError::new(format!(
                "__platypus_registered returned {other:?}, expected an array"
            ))),
        }
    }

    /// Creates a component of `class` with no GameObject of its own in the engine.
    pub fn spawn(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        class: &str,
    ) -> Result<ComponentId, ScriptError> {
        self.spawn_on(rt, class, None, class)
    }

    /// Creates a component of `class` attached to GameObject `object` (script id), which
    /// scripts see named `name`.
    pub fn spawn_on(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        class: &str,
        object: Option<u32>,
        name: &str,
    ) -> Result<ComponentId, ScriptError> {
        let id = ComponentId(self.next_id);
        let mut args = vec![class.into(), id.raw().into()];
        if let Some(object) = object {
            args.extend([object.into(), name.into()]);
        }
        rt.call_global("__platypus_create", &args)?;
        self.next_id += 1;
        self.adopt(id, class, object);
        Ok(id)
    }

    /// Takes over a component scripts created themselves (`addComponent`): the engine
    /// drives it from the next tick.
    pub fn adopt(&mut self, id: ComponentId, class: &str, object: Option<u32>) {
        self.components.push(Entry {
            id,
            class: class.to_owned(),
            object,
            started: false,
            enabled: true,
            hooks: None,
        });
    }

    /// Removes a component: `onDestroy()` (if it started), then the script instance.
    pub fn remove(&mut self, rt: &mut dyn ScriptRuntime, id: ComponentId) {
        let Some(i) = self.components.iter().position(|e| e.id == id) else {
            return;
        };
        let entry = self.components.remove(i);
        if entry.started
            && entry.enabled
            && entry.hooks.is_none_or(|h| h.destroy)
            && let Err(e) = dispatch(rt, id, "onDestroy", &[])
        {
            tracing::error!(component = %id, class = %entry.class, "onDestroy: {e}");
        }
        let _ = rt.call_global("__platypus_destroy", &[id.raw().into()]);
    }

    /// Removes every component on these GameObjects (destroyed with their objects).
    pub fn remove_on_objects(&mut self, rt: &mut dyn ScriptRuntime, objects: &[u32]) {
        let ids: Vec<ComponentId> = self
            .components
            .iter()
            .filter(|e| e.object.is_some_and(|o| objects.contains(&o)))
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.remove(rt, id);
        }
    }

    /// Runs one frame for every component. See [`Self::update_active`].
    pub fn update(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
    ) -> Vec<(ComponentId, ScriptError)> {
        self.update_active(rt, dt, &|_| true)
    }

    /// Runs one frame: `__platypus_tick(dt)` (script time; it may scale `dt`), then, for
    /// components on active objects, `onStart()` if new and `onUpdate(dt)`, then
    /// `onLateUpdate(dt)` for all of them. A component that throws is reported and
    /// disabled; the others keep running. Returns the errors raised.
    pub fn update_active(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        let has_tick = *self.tick_hook.get_or_insert_with(|| {
            rt.eval("typeof __platypus_tick === 'function'")
                .is_ok_and(|v| v == Value::Bool(true))
        });
        let mut dt = dt;
        if has_tick {
            match rt.call_global("__platypus_tick", &[dt.into()]) {
                Ok(Value::Number(scaled)) if scaled.is_finite() && scaled >= 0.0 => dt = scaled,
                Ok(_) => {}
                Err(e) => tracing::error!(target: "script", "__platypus_tick: {e}"),
            }
        }
        self.frame_dt = dt;
        let mut errors = self.run(rt, "onUpdate", &[dt.into()], active);
        // Animation poses objects after updates set its parameters, before late updates
        // (cameras following animated things) read them.
        let has_animate = *self.animate_hook.get_or_insert_with(|| {
            rt.eval("typeof __platypus_animate === 'function'")
                .is_ok_and(|v| v == Value::Bool(true))
        });
        if has_animate && let Err(e) = rt.call_global("__platypus_animate", &[dt.into()]) {
            tracing::error!(target: "script", "__platypus_animate: {e}");
        }
        errors.extend(self.run(rt, "onLateUpdate", &[dt.into()], active));
        errors
    }

    /// The last frame's length as scripts saw it (scaled by `Time.timeScale`).
    pub fn frame_dt(&self) -> f64 {
        self.frame_dt
    }

    /// Runs one fixed tick: `onFixedUpdate(dt)` (after `onStart` for new components).
    pub fn fixed_update(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        self.run(rt, "onFixedUpdate", &[dt.into()], active)
    }

    /// Runs `onPreRender()`, just before the frame is drawn.
    pub fn pre_render(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        self.run(rt, "onPreRender", &[], active)
    }

    /// Calls `method` on every enabled component on an active object, starting new
    /// ones first. Components added while this runs (`addComponent` in a script) are
    /// adopted when the ops are applied, so the list doesn't change under us.
    ///
    /// With `__platypus_run`, one call runs the method on a whole run of components (a call
    /// into JS per component cost more than many hooks do); a new component's `onStart`
    /// ends the run, so everything still happens in the same order.
    fn run(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        method: &str,
        args: &[Value],
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        let batched = *self.run_hook.get_or_insert_with(|| {
            rt.eval("typeof __platypus_run === 'function'")
                .is_ok_and(|v| v == Value::Bool(true))
        });
        if !batched {
            return self.run_each(rt, method, args, active);
        }
        let mut errors = Vec::new();
        let mut batch: Vec<usize> = Vec::new();
        for i in 0..self.components.len() {
            let entry = &mut self.components[i];
            if !entry.enabled || entry.object.is_some_and(|o| !active(o)) {
                continue;
            }
            let h = *entry.hooks.get_or_insert_with(|| hooks(rt, entry.id));
            if !self.components[i].started {
                self.run_batch(rt, method, args, &mut batch, &mut errors);
                self.components[i].started = true;
                if h.start
                    && let Err(e) = dispatch(rt, self.components[i].id, "onStart", &[])
                {
                    self.fail(rt, i, e, &mut errors);
                    continue;
                }
            }
            if h.has(method) {
                batch.push(i);
            }
        }
        self.run_batch(rt, method, args, &mut batch, &mut errors);
        errors
    }

    /// Calls `method` on the components at these indices in one `__platypus_run`; one that
    /// throws is disabled and the rest after it still run.
    fn run_batch(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        method: &str,
        args: &[Value],
        batch: &mut Vec<usize>,
        errors: &mut Vec<(ComponentId, ScriptError)>,
    ) {
        let mut from = 0;
        while from < batch.len() {
            let ids = batch[from..]
                .iter()
                .map(|&i| self.components[i].id.raw().into())
                .collect();
            let mut all = Vec::with_capacity(args.len() + 2);
            all.push(method.into());
            all.push(Value::Array(ids));
            all.extend_from_slice(args);
            let Err(e) = rt.call_global("__platypus_run", &all) else {
                break;
            };
            let at = match rt.eval("globalThis.__platypus_run_at") {
                Ok(Value::Number(n)) if n >= 0.0 && (from + n as usize) < batch.len() => n as usize,
                // Not a component's throw (the call itself failed): blame the first.
                _ => 0,
            };
            self.fail(rt, batch[from + at], e, errors);
            from += at + 1;
        }
        batch.clear();
    }

    /// Reports a component's error and disables it.
    fn fail(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        i: usize,
        e: ScriptError,
        errors: &mut Vec<(ComponentId, ScriptError)>,
    ) {
        let entry = &mut self.components[i];
        tracing::error!(component = %entry.id, class = %entry.class, "{e}; component disabled");
        entry.enabled = false;
        let _ = rt.call_global("__platypus_destroy", &[entry.id.raw().into()]);
        errors.push((entry.id, e));
    }

    /// [`Self::run`] one component at a time, for bundles without `__platypus_run`.
    fn run_each(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        method: &str,
        args: &[Value],
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        let mut errors = Vec::new();
        for i in 0..self.components.len() {
            let entry = &mut self.components[i];
            if !entry.enabled || entry.object.is_some_and(|o| !active(o)) {
                continue;
            }
            let h = *entry.hooks.get_or_insert_with(|| hooks(rt, entry.id));
            let result = (|| {
                if !entry.started {
                    entry.started = true;
                    if h.start {
                        dispatch(rt, entry.id, "onStart", &[])?;
                    }
                }
                if h.has(method) {
                    dispatch(rt, entry.id, method, args)?;
                }
                Ok::<_, ScriptError>(())
            })();
            if let Err(e) = result {
                tracing::error!(component = %entry.id, class = %entry.class, "{e}; component disabled");
                entry.enabled = false;
                let _ = rt.call_global("__platypus_destroy", &[entry.id.raw().into()]);
                errors.push((entry.id, e));
            }
        }
        errors
    }

    pub fn is_enabled(&self, id: ComponentId) -> bool {
        self.components.iter().any(|e| e.id == id && e.enabled)
    }

    pub fn contains(&self, id: ComponentId) -> bool {
        self.components.iter().any(|e| e.id == id)
    }

    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Forgets which lifecycle methods each component has, so they're asked again (after
    /// a reload in place swapped their classes).
    pub fn refresh_hooks(&mut self) {
        for entry in &mut self.components {
            entry.hooks = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

/// Hot reload, wipe and rebuild (DESIGN.md §6.1): `load` loads the new code into the
/// `fresh` runtime (one bundle, or a dev build's vendor and game bundles), then
/// `bootstrap` runs on it (it spawns the starting components, given the registered
/// classes, into a new host, or restores saved state); only if all of that works do
/// `rt` and `host` become the new ones (the old context is dropped). On an error (a
/// syntax error, a throwing top level or constructor) both are left as they were, so
/// the old code keeps running. Returns the registered classes.
pub fn reload<R: ScriptRuntime>(
    rt: &mut R,
    mut fresh: R,
    host: &mut ComponentHost,
    load: impl FnOnce(&mut R) -> Result<(), ScriptError>,
    bootstrap: impl FnOnce(&mut R, &mut ComponentHost, &[String]) -> Result<(), ScriptError>,
) -> Result<Vec<String>, ScriptError> {
    load(&mut fresh)?;
    let classes = ComponentHost::registered_classes(&mut fresh)?;
    let mut new_host = ComponentHost::new();
    bootstrap(&mut fresh, &mut new_host, &classes)?;
    *rt = fresh;
    *host = new_host;
    Ok(classes)
}

/// Runs a dev build's game code (CommonJS, its imports from the vendor bundle) inside
/// `__platypus_hot_begin` / `__platypus_hot_end`, which undo its last top level and move live
/// components onto the new classes.
pub fn load_game<R: ScriptRuntime>(rt: &mut R, name: &str, code: &str) -> Result<(), ScriptError> {
    let has = |rt: &mut R, f: &str| {
        rt.eval(&format!("typeof {f} === 'function'"))
            .is_ok_and(|v| v == Value::Bool(true))
    };
    if has(rt, "__platypus_hot_begin") {
        rt.call_global("__platypus_hot_begin", &[])?;
    }
    // The wrapper shares the code's first line, so error line numbers stay right.
    let wrapped = format!(
        "(function (require, module, exports) {{ {code}\n}})(globalThis.__platypus_require, {{ exports: {{}} }}, {{}});"
    );
    let loaded = rt.load_bundle(name, &wrapped);
    if has(rt, "__platypus_hot_end") {
        rt.call_global("__platypus_hot_end", &[])?;
    }
    loaded
}

/// The usual [`reload`] bootstrap without a scene: one component per class.
pub fn spawn_each(
    rt: &mut dyn ScriptRuntime,
    host: &mut ComponentHost,
    classes: &[String],
) -> Result<(), ScriptError> {
    for class in classes {
        host.spawn(rt, class)?;
    }
    Ok(())
}

fn dispatch(
    rt: &mut dyn ScriptRuntime,
    id: ComponentId,
    method: &str,
    args: &[Value],
) -> Result<Value, ScriptError> {
    let mut all = Vec::with_capacity(args.len() + 2);
    all.push(id.raw().into());
    all.push(method.into());
    all.extend_from_slice(args);
    rt.call_global("__platypus_dispatch", &all)
}
