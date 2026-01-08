use std::sync::Arc;

use anyhow::Ok;
use winit::{dpi::PhysicalSize, window::Window};

pub(crate) struct Renderer {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl Renderer {
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub async fn new(window: &Arc<Window>) -> anyhow::Result<Self> {
        let window = window.clone();
        let size = window.inner_size();
        let (surface, device, queue, config) = wgpu_init(&window, size).await?;

        Ok(Renderer {
            window,
            surface,
            device,
            queue,
            config,
        })
    }
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        Ok(())
    }
}

async fn wgpu_init(
    window: &Arc<Window>,
    size: PhysicalSize<u32>,
) -> anyhow::Result<(
    wgpu::Surface<'static>,
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await?;

    let surface_caps = surface.get_capabilities(&adapter);

    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    Ok((surface, device, queue, config))
}
