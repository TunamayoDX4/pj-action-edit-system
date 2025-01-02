use crate::StdError;
use parking_lot::RwLock;
use std::sync::Arc;
use wgpu::{
  Device, Instance, Queue, RequestAdapterOptionsBase, Surface,
  SurfaceConfiguration, SurfaceError,
};
use winit::{dpi::PhysicalSize, window::Window};
pub mod rdr_2d;
pub mod rdr_egui;
pub mod render_chain;
pub mod util;

/// WGPU config wrapper
pub struct AppGfxConfig {
  pub config: SurfaceConfiguration,
  pub wsize: PhysicalSize<u32>,
}
impl AppGfxConfig {
  pub fn resize(&mut self, wsize: PhysicalSize<u32>) {
    self.config.width = wsize.width;
    self.config.height = wsize.height;
    self.wsize = wsize;
  }
  pub fn configure(&self, device: &Device, surface: &Surface) {
    surface.configure(device, &self.config);
  }
}

/// WGPU wrapper interface
pub struct AppGfxService {
  surface: Surface<'static>,
  device: Device,
  queue: Queue,
  config: RwLock<AppGfxConfig>,
  chain_base: render_chain::RenderChainBase,
}
impl AppGfxService {
  pub async fn new(window: &Arc<Window>) -> Result<Self, StdError> {
    let instance = Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::all(),
      ..Default::default()
    });
    let surface = instance.create_surface(window.clone())?;
    let adapter = match instance
      .request_adapter(&RequestAdapterOptionsBase {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
      })
      .await
    {
      Some(adapter) => Ok::<_, StdError>(adapter),
      None => {
        log::error!("Adapter request failure.");
        Err("Available adapter is not exist".into())
      }
    }?;
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: Some("Main adapter device"),
          required_features: wgpu::Features::MULTI_DRAW_INDIRECT_COUNT
            | wgpu::Features::MULTI_DRAW_INDIRECT,
          required_limits: wgpu::Limits::default(),
          memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
      )
      .await?;
    let capabilities = surface.get_capabilities(&adapter);
    let wsize = window.inner_size();
    let config = SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: capabilities
        .formats
        .iter()
        .copied()
        .find(|c| *c == wgpu::TextureFormat::Bgra8UnormSrgb)
        .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb),
      width: wsize.width,
      height: wsize.height,
      present_mode: wgpu::PresentMode::Immediate,
      desired_maximum_frame_latency: 2,
      alpha_mode: capabilities
        .alpha_modes
        .iter()
        .copied()
        .next()
        .unwrap_or(wgpu::CompositeAlphaMode::default()),
      view_formats: vec![],
    };
    let config = RwLock::new(AppGfxConfig { config, wsize });
    config.read().configure(&device, &surface);
    Ok(Self {
      surface,
      device,
      queue,
      config,
      chain_base: render_chain::RenderChainBase::new(),
    })
  }

  pub fn reconfigure(&self) {
    self.config.read().configure(&self.device, &self.surface);
  }

  pub fn resize(&self, wsize: PhysicalSize<u32>) {
    if wsize.width != 0 && wsize.height != 0 {
      let mut config = self.config.write();
      config.resize(wsize);
      config.configure(&self.device, &self.surface);
    }
  }

  pub fn rendering(
    &self,
  ) -> Result<render_chain::RenderChain, SurfaceError> {
    let texture = self.surface.get_current_texture()?;
    let view =
      texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(render_chain::RenderChain::new(self, texture, view))
  }
}
