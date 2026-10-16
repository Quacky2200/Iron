use crate::WindowSurface;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no suitable GPU adapter (hardware or software) was found: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    #[error("could not create GPU device: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("could not create window surface: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("could not read back GPU buffer: {0}")]
    Readback(String),
}

/// The GPU device and queue every renderer shares.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// A GPU with no window, for tests and tools. Falls back to a software adapter
    /// (WARP / llvmpipe) when no hardware GPU is available.
    pub fn headless() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = request_adapter(&instance, None)?;
        Self::from_adapter(instance, adapter)
    }

    /// A GPU plus a surface for `target` (e.g. an `Arc<winit::window::Window>`).
    pub fn with_surface(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<(Self, WindowSurface), RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(target)?;
        let adapter = request_adapter(&instance, Some(&surface))?;
        let gpu = Self::from_adapter(instance, adapter)?;
        let surface = WindowSurface::new(&gpu, surface, width, height);
        Ok((gpu, surface))
    }

    pub fn adapter_name(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type)
    }

    fn from_adapter(instance: wgpu::Instance, adapter: wgpu::Adapter) -> Result<Self, RenderError> {
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("iron device"),
                required_limits:
                    wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                ..Default::default()
            }))?;
        tracing::info!(adapter = %adapter.get_info().name, "GPU ready");
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}

fn request_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, RenderError> {
    let request = |force_fallback_adapter| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter,
            compatible_surface: surface,
            ..Default::default()
        }))
    };
    match request(false) {
        Ok(adapter) => Ok(adapter),
        Err(e) => {
            tracing::warn!("no hardware adapter ({e}); trying software fallback");
            Ok(request(true)?)
        }
    }
}
