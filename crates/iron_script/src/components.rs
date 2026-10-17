use iron_core::ComponentId;

use crate::{ScriptError, ScriptRuntime, Value};

/// Drives script components through the JS-side host installed by `@iron/engine`:
/// `__iron_registered()`, `__iron_create(class, id, object, name)`,
/// `__iron_dispatch(id, method, ...args)`, `__iron_destroy(id)` and, once per tick,
/// `__iron_tick(dt)` (which advances script `Time` and returns the scaled `dt`).
///
/// Rust keeps the list of components; JS only holds instances keyed by ID. Methods are
/// always called by `(component id, method name)`, so a reloaded bundle's code is
/// picked up automatically (DESIGN.md §3). Components belong to GameObjects (by the
/// script's object id); those on inactive objects don't update.
#[derive(Default)]
pub struct ComponentHost {
    next_id: u32,
    components: Vec<Entry>,
    /// Whether the bundle has `__iron_tick` (checked on the first update).
    tick_hook: Option<bool>,
}

struct Entry {
    id: ComponentId,
    class: String,
    /// The GameObject it's attached to (script object id).
    object: Option<u32>,
    started: bool,
    enabled: bool,
}

impl ComponentHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Component class names the loaded bundle registered.
    pub fn registered_classes(rt: &mut dyn ScriptRuntime) -> Result<Vec<String>, ScriptError> {
        match rt.call_global("__iron_registered", &[])? {
            Value::Array(items) => Ok(items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()),
            other => Err(ScriptError::new(format!(
                "__iron_registered returned {other:?}, expected an array"
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
        rt.call_global("__iron_create", &args)?;
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
            && let Err(e) = dispatch(rt, id, "onDestroy", &[])
        {
            tracing::error!(component = %id, class = %entry.class, "onDestroy: {e}");
        }
        let _ = rt.call_global("__iron_destroy", &[id.raw().into()]);
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

    /// Runs one tick for every component: `onStart()` for new components, then
    /// `onUpdate(dt)`. See [`Self::update_active`].
    pub fn update(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
    ) -> Vec<(ComponentId, ScriptError)> {
        self.update_active(rt, dt, &|_| true)
    }

    /// Runs one tick: `__iron_tick(dt)` (script time; it may scale `dt`), then for
    /// components on active objects `onStart()` if new, then `onUpdate(dt)`. A component
    /// that throws is reported and disabled; the others keep running. Returns the errors
    /// raised this tick.
    pub fn update_active(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
        active: &dyn Fn(u32) -> bool,
    ) -> Vec<(ComponentId, ScriptError)> {
        let has_tick = *self.tick_hook.get_or_insert_with(|| {
            rt.eval("typeof __iron_tick === 'function'")
                .is_ok_and(|v| v == Value::Bool(true))
        });
        let mut dt = dt;
        if has_tick {
            match rt.call_global("__iron_tick", &[dt.into()]) {
                Ok(Value::Number(scaled)) if scaled.is_finite() && scaled >= 0.0 => dt = scaled,
                Ok(_) => {}
                Err(e) => tracing::error!(target: "script", "__iron_tick: {e}"),
            }
        }
        // Components added during this loop (by `addComponent` in a script) are adopted
        // when the ops are applied, so the list doesn't change under us.
        let mut errors = Vec::new();
        for i in 0..self.components.len() {
            let entry = &mut self.components[i];
            if !entry.enabled || entry.object.is_some_and(|o| !active(o)) {
                continue;
            }
            let result = (|| {
                if !entry.started {
                    entry.started = true;
                    dispatch(rt, entry.id, "onStart", &[])?;
                }
                dispatch(rt, entry.id, "onUpdate", &[dt.into()])
            })();
            if let Err(e) = result {
                tracing::error!(component = %entry.id, class = %entry.class, "{e}; component disabled");
                entry.enabled = false;
                let _ = rt.call_global("__iron_destroy", &[entry.id.raw().into()]);
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

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

/// Hot reload, wipe and rebuild (DESIGN.md §6.1): loads `code` into the `fresh`
/// runtime and runs `bootstrap` on it (which spawns the starting components, given the
/// registered classes, into a new host); only if all of that works do `rt` and `host`
/// become the new ones (the old context is dropped). On an error (a syntax error, a
/// throwing top level or constructor) both are left as they were, so the old code
/// keeps running.
///
/// Component state isn't carried over yet: new components start fresh and run
/// `onStart` again (property serialization comes with GameObjects, TODO.md M5).
/// Returns the registered classes.
pub fn reload<R: ScriptRuntime>(
    rt: &mut R,
    mut fresh: R,
    host: &mut ComponentHost,
    name: &str,
    code: &str,
    bootstrap: impl FnOnce(&mut R, &mut ComponentHost, &[String]) -> Result<(), ScriptError>,
) -> Result<Vec<String>, ScriptError> {
    fresh.load_bundle(name, code)?;
    let classes = ComponentHost::registered_classes(&mut fresh)?;
    let mut new_host = ComponentHost::new();
    bootstrap(&mut fresh, &mut new_host, &classes)?;
    *rt = fresh;
    *host = new_host;
    Ok(classes)
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
    rt.call_global("__iron_dispatch", &all)
}
