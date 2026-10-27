// SPDX-License-Identifier: Apache-2.0

//! A GLES 3 context through EGL: ANGLE on Windows (`libEGL.dll`, which draws through Direct3D
//! 11), the system's EGL (Mesa) elsewhere. The library is loaded here, not linked, so a program
//! that never asks for it needs no EGL, and ANGLE can come from a folder of the caller's choice.

use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;

use khronos_egl as egl;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};

type Instance = egl::DynamicInstance<egl::EGL1_4>;

/// `EGL_OPENGL_ES3_BIT` (EGL_KHR_create_context; not in EGL 1.4's constants).
const OPENGL_ES3_BIT: egl::Int = 0x0040;

/// A current GLES 3 context and its surface.
pub struct EglContext {
    egl: Arc<Instance>,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
}

impl EglContext {
    /// A context drawing into a small off-screen surface, for rendering to framebuffers
    /// (tests, screenshots). `angle_dir`: the folder with ANGLE's `libEGL.dll` and
    /// `libGLESv2.dll`; without one the usual library search applies (next to the program).
    pub fn headless(angle_dir: Option<&Path>) -> Result<Self, String> {
        let egl = load(angle_dir)?;
        let display = default_display(&egl)?;
        let config = choose_config(&egl, display, egl::PBUFFER_BIT)?;
        let context = create_context(&egl, display, config)?;
        let surface = egl
            .create_pbuffer_surface(display, config, &[egl::WIDTH, 1, egl::HEIGHT, 1, egl::NONE])
            .map_err(|e| format!("EGL create_pbuffer_surface: {e}"))?;
        Self::finish(egl, display, context, surface)
    }

    /// A context presenting to `window` (Win32 and Xlib; any other windowing system is an
    /// error, so the caller can fall back to another renderer).
    pub fn window<W>(angle_dir: Option<&Path>, window: &W) -> Result<Self, String>
    where
        W: HasWindowHandle + HasDisplayHandle,
    {
        let win = window.window_handle().map_err(|e| e.to_string())?.as_raw();
        let disp = window.display_handle().map_err(|e| e.to_string())?.as_raw();
        let (native_display, native_window) = match (disp, win) {
            #[cfg(windows)]
            (_, RawWindowHandle::Win32(h)) => (egl::DEFAULT_DISPLAY, h.hwnd.get() as *mut c_void),
            (RawDisplayHandle::Xlib(d), RawWindowHandle::Xlib(h)) => (
                d.display.map_or(egl::DEFAULT_DISPLAY, |p| p.as_ptr()),
                h.window as *mut c_void,
            ),
            _ => return Err("EGL: this windowing system isn't supported".into()),
        };
        let egl = load(angle_dir)?;
        let display = unsafe { egl.get_display(native_display) }.ok_or("EGL: no display")?;
        let config = choose_config(&egl, display, egl::WINDOW_BIT)?;
        let context = create_context(&egl, display, config)?;
        // A plain surface: ANGLE's sRGB one would encode on write, but reading it back
        // returned the unencoded bytes, so the blit encodes itself (`Target::srgb: false`).
        let surface = unsafe { egl.create_window_surface(display, config, native_window, None) }
            .map_err(|e| format!("EGL create_window_surface: {e}"))?;
        Self::finish(egl, display, context, surface)
    }

    fn finish(
        egl: Instance,
        display: egl::Display,
        context: egl::Context,
        surface: egl::Surface,
    ) -> Result<Self, String> {
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|e| format!("EGL make_current: {e}"))?;
        Ok(Self {
            egl: Arc::new(egl),
            display,
            context,
            surface,
        })
    }

    /// Shows the frame.
    pub fn swap_buffers(&self) -> Result<(), String> {
        self.egl
            .swap_buffers(self.display, self.surface)
            .map_err(|e| format!("EGL swap_buffers: {e}"))
    }

    /// Waits for the display's refresh in `swap_buffers` (or not).
    pub fn set_vsync(&self, on: bool) {
        let _ = self.egl.swap_interval(self.display, i32::from(on));
    }

    /// The surface's size in pixels (it follows the window).
    pub fn size(&self) -> (u32, u32) {
        let q = |a| {
            self.egl
                .query_surface(self.display, self.surface, a)
                .unwrap_or(0)
                .max(0) as u32
        };
        (q(egl::WIDTH), q(egl::HEIGHT))
    }

    /// The GL entry points of the current context.
    pub fn gl(&self) -> glow::Context {
        let egl = self.egl.clone();
        unsafe {
            glow::Context::from_loader_function(move |name| {
                egl.get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const c_void)
            })
        }
    }
}

impl Drop for EglContext {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.terminate(self.display);
    }
}

fn default_display(egl: &Instance) -> Result<egl::Display, String> {
    let display =
        unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }.ok_or("EGL: no default display")?;
    Ok(display)
}

fn choose_config(
    egl: &Instance,
    display: egl::Display,
    surface_type: egl::Int,
) -> Result<egl::Config, String> {
    egl.initialize(display)
        .map_err(|e| format!("EGL initialize: {e}"))?;
    egl.bind_api(egl::OPENGL_ES_API)
        .map_err(|e| format!("EGL bind_api: {e}"))?;
    egl.choose_first_config(
        display,
        &[
            egl::SURFACE_TYPE,
            surface_type,
            egl::RENDERABLE_TYPE,
            OPENGL_ES3_BIT,
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::ALPHA_SIZE,
            8,
            egl::NONE,
        ],
    )
    .map_err(|e| format!("EGL choose_config: {e}"))?
    .ok_or_else(|| "EGL: no GLES 3 config".to_string())
}

fn create_context(
    egl: &Instance,
    display: egl::Display,
    config: egl::Config,
) -> Result<egl::Context, String> {
    egl.create_context(
        display,
        config,
        None,
        &[egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE],
    )
    .map_err(|e| format!("EGL create_context: {e}"))
}

fn load(angle_dir: Option<&Path>) -> Result<Instance, String> {
    #[cfg(windows)]
    {
        if let Some(dir) = angle_dir {
            // libEGL.dll finds libGLESv2.dll through the DLL search path.
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe { windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW(wide.as_ptr()) };
        }
        unsafe { Instance::load_required_from_filename("libEGL.dll") }
            .map_err(|e| format!("EGL: can't load libEGL.dll (ANGLE): {e}"))
    }
    #[cfg(not(windows))]
    {
        let _ = angle_dir;
        unsafe { Instance::load_required() }.map_err(|e| format!("EGL: can't load libEGL: {e}"))
    }
}
