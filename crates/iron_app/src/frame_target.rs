//! Where the main window's frames go: its swapchain, or (for `--bench`) an offscreen texture
//! with no OS window at all.

use iron_render::wgpu;
use iron_render::{Gpu, OffscreenTarget, WindowSurface};

use super::Gfx;

pub(crate) enum FrameTarget {
    Window(WindowSurface),
    /// No window: frames are drawn into a texture and never shown. `in_flight` is the frame
    /// before the last, which the next frame waits for (like a swapchain queueing one frame),
    /// so the GPU's time counts without the CPU running ahead unbounded.
    Offscreen {
        target: OffscreenTarget,
        in_flight: Option<wgpu::SubmissionIndex>,
    },
}

impl FrameTarget {
    pub(crate) fn offscreen(gpu: &Gpu, width: u32, height: u32) -> Self {
        FrameTarget::Offscreen {
            target: OffscreenTarget::with_format(
                gpu,
                width,
                height,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ),
            in_flight: None,
        }
    }

    /// Size in physical pixels.
    pub(crate) fn size(&self) -> (u32, u32) {
        match self {
            FrameTarget::Window(s) => s.size(),
            FrameTarget::Offscreen { target, .. } => target.size(),
        }
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        match self {
            FrameTarget::Window(s) => s.resize(gpu, width, height),
            FrameTarget::Offscreen { target, .. } => target.resize(gpu, width, height),
        }
    }

    /// Offscreen frames are never paced by a display.
    pub(crate) fn set_vsync(&mut self, gpu: &Gpu, on: bool) {
        if let FrameTarget::Window(s) = self {
            s.set_vsync(gpu, on);
        }
    }

    /// Lets `draw` render the next frame (view, format, physical size), then shows it.
    pub(crate) fn present_with(
        &mut self,
        gpu: &Gpu,
        draw: impl FnOnce(&wgpu::TextureView, wgpu::TextureFormat, (u32, u32)),
    ) {
        match self {
            FrameTarget::Window(s) => s.present_with(gpu, draw),
            FrameTarget::Offscreen { target, in_flight } => {
                let Some(view) = target.view() else {
                    return;
                };
                draw(&view, target.format(), target.size());
                let done = gpu.queue.submit(std::iter::empty());
                if let Some(previous) = in_flight.replace(done) {
                    let _ = gpu.device.poll(wgpu::PollType::Wait {
                        submission_index: Some(previous),
                        timeout: None,
                    });
                }
            }
        }
    }

    /// Waits for every frame drawn so far (the end of a measured run).
    pub(crate) fn finish(&mut self, gpu: &Gpu) {
        if let FrameTarget::Offscreen { in_flight, .. } = self
            && in_flight.take().is_some()
        {
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        }
    }
}

impl Gfx {
    /// The window's DPI scale (1 without a window).
    pub(crate) fn dpi(&self) -> f64 {
        self.window.as_ref().map_or(1.0, |w| w.scale_factor())
    }

    /// The window's client size in physical pixels (the target's without a window).
    pub(crate) fn inner_size(&self) -> (u32, u32) {
        match &self.window {
            Some(w) => {
                let s = w.inner_size();
                (s.width, s.height)
            }
            None => self.surface.size(),
        }
    }
}
