//! `createRoot(target?).render(<App/>)` for compiled games: the program hands its root element to
//! the host with `UiRoot.mount(target, element)` and the host renders it with its
//! `iron_ui_native::Runtime`. Target 0 is the screen; another number is a world panel's handle.
//!
//! Mounting again on the same target replaces the root (a hot reload does that); the host takes
//! the requests with [`take_mounts`] after the program's `init` and after anything it runs.

use std::cell::RefCell;

thread_local! {
    static MOUNTS: RefCell<Vec<(u64, u64)>> = const { RefCell::new(Vec::new()) };
}

/// `UiRoot.mount(target, element)`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_ui_mount(target: f64, element: u64) {
    MOUNTS.with(|m| m.borrow_mut().push((target as u64, element)));
}

/// `UiRoot.mount(panel, element)`: the root of a world panel, by the panel's raw handle.
#[unsafe(no_mangle)]
pub extern "C" fn ts_ui_mount_panel(panel: u64, element: u64) {
    MOUNTS.with(|m| m.borrow_mut().push((panel, element)));
}

/// The mount requests since the last call: (target, element handle for `element_from`).
pub fn take_mounts() -> Vec<(u64, u64)> {
    MOUNTS.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

/// The `UiRoot` namespace as the compiler reads it. Pass it next to the `Ui` manifest.
pub fn root_manifest_json() -> String {
    serde_json::json!({
        "namespace": "UiRoot",
        "functions": [{
            "name": "mount",
            "symbol": "ts_ui_mount",
            "params": [
                {"name": "target", "type": "f64"},
                {"name": "element", "type": "handle:UiElement"},
            ],
            "returns": "void",
        }, {
            "name": "mount",
            "symbol": "ts_ui_mount_panel",
            "params": [
                {"name": "panel", "type": "handle:WorldPanel"},
                {"name": "element", "type": "handle:UiElement"},
            ],
            "returns": "void",
        }],
    })
    .to_string()
}
