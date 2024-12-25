use wgpu::{Backends, Device, DeviceType, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

#[repr(u64)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum WgpuAdapterSelMode {
  LowPower,
  HighPower,
  Virtual,
  Software,
}
impl Default for WgpuAdapterSelMode {
  fn default() -> Self {
    Self::HighPower
  }
}
impl WgpuAdapterSelMode {
  pub fn into_priority(&self, dt: DeviceType) -> u64 {
    match self {
      WgpuAdapterSelMode::LowPower => match dt {
        DeviceType::Other => 0,
        DeviceType::IntegratedGpu => 2,
        DeviceType::DiscreteGpu => 1,
        DeviceType::VirtualGpu => 3,
        DeviceType::Cpu => 0,
      },
      WgpuAdapterSelMode::HighPower => match dt {
        DeviceType::Other => 0,
        DeviceType::IntegratedGpu => 1,
        DeviceType::DiscreteGpu => 3,
        DeviceType::VirtualGpu => 2,
        DeviceType::Cpu => 0,
      },
      WgpuAdapterSelMode::Virtual => match dt {
        DeviceType::Other => 0,
        DeviceType::IntegratedGpu => 2,
        DeviceType::DiscreteGpu => 1,
        DeviceType::VirtualGpu => 3,
        DeviceType::Cpu => 0,
      },
      WgpuAdapterSelMode::Software => match dt {
        DeviceType::Other => 0,
        DeviceType::IntegratedGpu => 2,
        DeviceType::DiscreteGpu => 1,
        DeviceType::VirtualGpu => 1,
        DeviceType::Cpu => 3,
      },
    }
  }
}

struct WgpuContextState<'a> {
  window: &'a Window,
  device: Device,
  queue: Queue,
  config: SurfaceConfiguration,
  surface: Surface<'a>,
}
impl<'a> WgpuContextState<'a> {
  pub fn re_configure(&self) {
    self.surface.configure(&self.device, &self.config);
  }
  pub fn re_size(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    if 0 < new_size.width && 0 < new_size.height {
      self.config.width = new_size.width;
      self.config.height = new_size.height;
      self.re_configure();
    }
  }
  pub fn new(window: &'a Window, adapter_select_mode: WgpuAdapterSelMode) -> Self {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::PRIMARY,
      ..Default::default()
    });
    let surface = instance
      .create_surface(window)
      .expect("Create wgpu surface failure");
    let adapter = instance
      .enumerate_adapters(Backends::all())
      .into_iter()
      .filter(|adapter| adapter.is_surface_supported(&surface))
      .fold(None, |selected, adapter| match selected {
        None => Some(adapter),
        Some(selected) => {
          if {
            let selected = selected.get_info();
            let adapter = adapter.get_info();
            adapter_select_mode.into_priority(selected.device_type)
              < adapter_select_mode.into_priority(adapter.device_type)
          } {
            Some(adapter)
          } else {
            Some(selected)
          }
        }
      })
      .expect("Adapter request failure");
    let (device, queue) = pollster::block_on(adapter.request_device(
      &wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::MULTI_DRAW_INDIRECT,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::Performance,
      },
      None,
    ))
    .expect("Create queue and device failure");
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
      .formats
      .iter()
      .find(|f| f.is_srgb())
      .copied()
      .unwrap_or(surface_caps.formats[0]);
    let config = SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface_format,
      width: window.inner_size().width,
      height: window.inner_size().height,
      present_mode: surface_caps.present_modes[0],
      desired_maximum_frame_latency: 2,
      alpha_mode: surface_caps.alpha_modes[0],
      view_formats: vec![],
    };
    let r = Self {
      window,
      device,
      queue,
      config,
      surface,
    };
    r.re_configure();
    r
  }
}

pub struct WgpuState<'a> {
  wgpu_context: WgpuContextState<'a>,
  camera: Camera2D,
  render_pipeline: wgpu::RenderPipeline,
}
impl<'a> WgpuState<'a> {
  pub fn re_size(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    self.wgpu_context.re_size(new_size);
    self.camera.aspect_reset(&self.wgpu_context);
  }
  pub fn re_configure(&mut self) {
    self.wgpu_context.re_configure();
  }
  pub fn rendering(&self) -> Result<(), wgpu::SurfaceError> {
    let output = self.wgpu_context.surface.get_current_texture()?;
    let view = output
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
      self
        .wgpu_context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("Render command encoder"),
        });
    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.draw(0..3, 0..1);
    }
    self.wgpu_context.queue.submit(Some(encoder.finish()));
    output.present();
    Ok(())
  }
  pub fn new(window: &'a Window, adapter_select_mode: WgpuAdapterSelMode) -> Self {
    // コンテキストの初期化
    let wgpu_context = WgpuContextState::new(window, adapter_select_mode);

    // レンダラの初期化
    let shader = wgpu_context
      .device
      .create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("2D Main shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("2d_main.wgsl").into()),
      });
    let shader_pipeline_layout =
      wgpu_context
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
          label: Some("2D Main shader Pipeline Layout"),
          bind_group_layouts: &[],
          push_constant_ranges: &[],
        });
    let shader_pipeline =
      wgpu_context
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("2D Main shader Pipeline"),
          layout: Some(&shader_pipeline_layout),
          vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
          },
          fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
              format: wgpu_context.config.format,
              blend: Some(wgpu::BlendState::ALPHA_BLENDING),
              write_mask: wgpu::ColorWrites::all(),
            })],
          }),
          primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
          },
          depth_stencil: None,
          multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
          },
          multiview: None,
          cache: None,
        });

    let camera = Camera2D::new(&wgpu_context, [0., 0.], [1., 1.], 0.);

    Self {
      wgpu_context,
      camera,
      render_pipeline: shader_pipeline,
    }
  }
}

pub struct Camera2D {
  pub position: [f32; 2],
  pub scale: [f32; 2],
  pub rot_angle: f32,
  aspect: f32,
}
impl Camera2D {
  pub fn new(
    graphics_state: &WgpuContextState,
    position: impl Into<[f32; 2]>,
    scale: impl Into<[f32; 2]>,
    rot_angle: impl Into<f32>,
  ) -> Self {
    let mut r = Self {
      position: position.into(),
      scale: scale.into(),
      rot_angle: rot_angle.into(),
      aspect: 0.,
    };
    r.aspect_reset(graphics_state);
    r
  }
  pub fn aspect_reset(&mut self, graphics_state: &WgpuContextState) {
    let config = &graphics_state.config;
    self.aspect = config.height as f32 / config.width as f32;
  }
}
