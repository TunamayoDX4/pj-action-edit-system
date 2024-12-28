pub mod game;

pub struct TestRenderer(pub [f32; 4]);
impl crate::gfx::Renderer<()> for TestRenderer {
  fn rendering<'a: 'b, 'b>(
    &'a mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    _device: &'b wgpu::Device,
    _queue: &'b wgpu::Queue,
    encoder: &'b mut wgpu::CommandEncoder,
    _param: (),
  ) -> Result<(), crate::StdError> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Test renderer"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: surface_view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color {
            r: self.0[0] as f64,
            g: self.0[1] as f64,
            b: self.0[2] as f64,
            a: self.0[3] as f64,
          }),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });
    Ok(())
  }
}

pub struct EguiRenderer {
  state: egui_winit::State,
  renderer: egui_wgpu::Renderer,
}
impl EguiRenderer {
  pub fn new(
    gfx: &crate::gfx::GfxState,
    window: &winit::window::Window,
    depth_buffer_format: Option<wgpu::TextureFormat>,
    msaa_samples: u32,
    dithering: bool,
  ) -> Self {
    let egui_ctx = egui::Context::default();
    let state = egui_winit::State::new(
      egui_ctx,
      egui::viewport::ViewportId::ROOT,
      window,
      Some(window.scale_factor() as f32),
      window.theme(),
      Some(1024 * 2),
    );
    let renderer = egui_wgpu::Renderer::new(
      &gfx.device,
      gfx.config.format,
      depth_buffer_format,
      msaa_samples,
      dithering,
    );
    Self { state, renderer }
  }
  pub fn set_pixel_per_point(&mut self, v: f32) {
    self.state.egui_ctx().set_pixels_per_point(v);
  }
  pub fn event_input(
    &mut self,
    window: &winit::window::Window,
    event: &winit::event::WindowEvent,
  ) -> egui_winit::EventResponse {
    self.state.on_window_event(window, event)
  }
}
impl<'c, F>
  crate::gfx::Renderer<(
    std::sync::Arc<winit::window::Window>,
    egui_wgpu::ScreenDescriptor,
    F,
  )> for EguiRenderer
where
  F: 'c + FnOnce(&egui::Context),
{
  fn rendering<'a: 'b, 'b>(
    &'a mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &'b wgpu::Device,
    queue: &'b wgpu::Queue,
    encoder: &'b mut wgpu::CommandEncoder,
    (window, screen_descriptor, f): (
      std::sync::Arc<winit::window::Window>,
      egui_wgpu::ScreenDescriptor,
      F,
    ),
  ) -> Result<(), crate::StdError> {
    let raw_input = self.state.take_egui_input(&window);
    self.state.egui_ctx().begin_pass(raw_input);
    f(self.state.egui_ctx());
    self.set_pixel_per_point(screen_descriptor.pixels_per_point);
    let full_output = self.state.egui_ctx().end_pass();
    self
      .state
      .handle_platform_output(&window, full_output.platform_output);
    let tris = self
      .state
      .egui_ctx()
      .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
    for (id, image_delta) in &full_output.textures_delta.set {
      self
        .renderer
        .update_texture(device, queue, *id, image_delta);
    }
    self
      .renderer
      .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
    let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("egui render pass"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: surface_view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
    });
    self
      .renderer
      .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
    for id in &full_output.textures_delta.free {
      self.renderer.free_texture(id);
    }

    Ok(())
  }
}
