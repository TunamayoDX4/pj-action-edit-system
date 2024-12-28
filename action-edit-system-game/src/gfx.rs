pub struct GfxState {
  pub surface: wgpu::Surface<'static>,
  pub queue: wgpu::Queue,
  pub device: wgpu::Device,
  pub config: wgpu::SurfaceConfiguration,
  pub size: winit::dpi::PhysicalSize<u32>,
}
impl GfxState {
  /// グラフィクス・ステートの初期化
  pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self, crate::StdError> {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::all(),
      flags: wgpu::InstanceFlags::default(),
      dx12_shader_compiler: wgpu::Dx12Compiler::default(),
      gles_minor_version: wgpu::Gles3MinorVersion::default(),
    });
    let surface = instance.create_surface(window.clone())?;
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
      })
      .await
      .ok_or("Adapter request failure")?;
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: None,
          required_features: wgpu::Features::MULTI_DRAW_INDIRECT
            | wgpu::Features::POLYGON_MODE_LINE
            | wgpu::Features::POLYGON_MODE_LINE,
          required_limits: wgpu::Limits::default(),
          memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
      )
      .await?;
    let capabilities = surface.get_capabilities(&adapter);
    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: {
        let cap_iter = capabilities.formats.iter();
        let first = cap_iter
          .clone()
          .copied()
          .next()
          .ok_or("No capabilities: texture format")?;
        cap_iter
          .copied()
          .find(|c| *c == wgpu::TextureFormat::Bgra8UnormSrgb)
          .unwrap_or(first)
      },
      width: size.width,
      height: size.height,
      present_mode: capabilities
        .present_modes
        .iter()
        .copied()
        .next()
        .ok_or("No capabilities: present mode")?,
      desired_maximum_frame_latency: 2,
      alpha_mode: capabilities
        .alpha_modes
        .iter()
        .copied()
        .next()
        .ok_or("No capabilites: alpha mode")?,
      view_formats: vec![],
    };
    surface.configure(&device, &config);
    Ok(Self {
      surface,
      queue,
      device,
      config,
      size,
    })
  }
  pub fn surface_reconfig(&self) {
    self.surface.configure(&self.device, &self.config);
  }
  pub fn surface_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
    self.size = size;
    self.config.width = size.width;
    self.config.height = size.height;
    self.surface_reconfig();
  }

  pub fn draw(&mut self) -> Result<RenderChain, wgpu::SurfaceError> {
    let surface_texture = self.surface.get_current_texture()?;
    let surface_view = surface_texture
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    Ok(RenderChain {
      surface_texture,
      surface_view,
      encoders: Vec::from([
        self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: None,
        })
      ]),
      device: &self.device,
      queue: &self.queue,
    })
  }
}

pub trait Renderer<V> {
  fn rendering<'a: 'b, 'b>(
    &'a mut self,
    surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &'b wgpu::Device,
    queue: &'b wgpu::Queue,
    encoder: &'b mut wgpu::CommandEncoder,
    param: V,
  ) -> Result<(), crate::StdError>;
}

pub struct RenderChain<'a> {
  surface_texture: wgpu::SurfaceTexture,
  surface_view: wgpu::TextureView,
  encoders: Vec<wgpu::CommandEncoder>,
  device: &'a wgpu::Device,
  queue: &'a wgpu::Queue,
}
impl<'a> RenderChain<'a> {
  pub fn rendering<'c, V: 'c>(
    mut self,
    renderer: &mut impl Renderer<V>,
    param: V,
  ) -> Result<Self, crate::StdError> where 'a: 'c {
    renderer.rendering(
      &self.surface_texture,
      &self.surface_view,
      self.device,
      self.queue,
      self.encoders.last_mut().unwrap(),
      param,
    )?;
    Ok(self)
  }
  pub fn flush_encoder(mut self) -> Self {
    self.encoders.push(self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None }));
    Self {
      surface_texture: self.surface_texture,
      surface_view: self.surface_view,
      encoders: self.encoders,
      device: self.device,
      queue: self.queue,
    }
  }
  pub fn finish(self) {
    self
      .queue
      .submit(self.encoders.into_iter().map(|e| e.finish()));
    self.surface_texture.present();
  }
}
