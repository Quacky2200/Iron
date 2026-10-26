//! The main OS window and its DPI scale and size, for the code that doesn't need the renderer.

use std::sync::Arc;

use iron_platform::Window;

use super::App;

impl App {
    /// The main OS window (none in a windowless run).
    pub(crate) fn main_window(&self) -> Option<Arc<Window>> {
        self.gfx.as_ref().and_then(|g| g.window.clone())
    }

    /// The main window's DPI scale and client size in physical pixels, once it exists.
    pub(crate) fn main_metrics(&self) -> Option<(f64, (u32, u32))> {
        let gfx = self.gfx.as_ref()?;
        Some((gfx.dpi(), gfx.inner_size()))
    }
}
