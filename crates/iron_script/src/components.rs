use iron_core::ComponentId;

use crate::{ScriptError, ScriptRuntime, Value};

/// Drives script components through the JS-side host installed by `@iron/engine`:
/// `__iron_registered()`, `__iron_create(class, id)`, `__iron_dispatch(id, method, ...args)`
/// and `__iron_destroy(id)`.
///
/// Rust keeps the list of components; JS only holds instances keyed by ID. Methods are
/// always called by `(component id, method name)`, so a reloaded bundle's code is
/// picked up automatically (DESIGN.md §3).
#[derive(Default)]
pub struct ComponentHost {
    next_id: u32,
    components: Vec<Entry>,
}

struct Entry {
    id: ComponentId,
    class: String,
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

    pub fn spawn(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        class: &str,
    ) -> Result<ComponentId, ScriptError> {
        let id = ComponentId(self.next_id);
        rt.call_global("__iron_create", &[class.into(), id.raw().into()])?;
        self.next_id += 1;
        self.components.push(Entry {
            id,
            class: class.to_owned(),
            started: false,
            enabled: true,
        });
        Ok(id)
    }

    /// Runs one tick: `onStart()` for new components, then `onUpdate(dt)`.
    /// A component that throws is reported and disabled; the others keep running.
    /// Returns the errors raised this tick.
    pub fn update(
        &mut self,
        rt: &mut dyn ScriptRuntime,
        dt: f64,
    ) -> Vec<(ComponentId, ScriptError)> {
        let mut errors = Vec::new();
        for entry in self.components.iter_mut().filter(|e| e.enabled) {
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

    pub fn len(&self) -> usize {
        self.components.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
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
