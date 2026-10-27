// SPDX-License-Identifier: Apache-2.0

//! The main OS window and its DPI scale and size, for the code that doesn't need the renderer.

use std::sync::Arc;

use platypus_platform::Window;

use super::App;

impl App {
    /// The main OS window (none in a windowless run), whichever renderer draws it.
    pub(crate) fn main_window(&self) -> Option<Arc<Window>> {
        self.gfx
            .as_ref()
            .and_then(|g| g.window.clone())
            .or_else(|| self.gles.as_ref().map(|g| g.window.clone()))
    }

    /// The main window's DPI scale and client size in physical pixels, once it exists.
    pub(crate) fn main_metrics(&self) -> Option<(f64, (u32, u32))> {
        if let Some(gfx) = &self.gfx {
            return Some((gfx.dpi(), gfx.inner_size()));
        }
        let window = &self.gles.as_ref()?.window;
        let size = window.inner_size();
        Some((window.scale_factor(), (size.width, size.height)))
    }
}
