use crate::{Color, Gpu, encode_clear};

/// A window's swapchain. Tolerates 0×0 sizes (minimized windows) by skipping frames.
pub struct WindowSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    configured: bool,
}

impl WindowSurface {
    pub(crate) fn new(gpu: &Gpu, surface: wgpu::Surface<'static>, width: u32, height: u32) -> Self {
        let caps = surface.get_capabilities(&gpu.adapter);
        // Prefer an sRGB format so colours written in linear space display correctly.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        let mut s = Self {
            surface,
            config,
            configured: false,
        };
        s.resize(gpu, width, height);
        s
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        if width == 0 || height == 0 {
            self.configured = false;
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&gpu.device, &self.config);
        self.configured = true;
    }

    /// Clears the window to `color` and presents it. Skips the frame when the window
    /// is minimized, occluded or the surface needs reconfiguring.
    pub fn present_clear(&mut self, gpu: &Gpu, color: Color) {
        if !self.configured {
            return;
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) => f,
            wgpu::CurrentSurfaceTexture::Suboptimal(f) => {
                // Draw this frame, reconfigure before the next one.
                self.configured = false;
                f
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&gpu.device, &self.config);
                return;
            }
            other => {
                tracing::debug!(?other, "skipping frame");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encode_clear(&mut encoder, &view, color);
        gpu.queue.submit([encoder.finish()]);
        gpu.queue.present(frame);
        if !self.configured {
            self.surface.configure(&gpu.device, &self.config);
            self.configured = true;
        }
    }
}
