use std::sync::Arc;

use egui::{
  epaint::text::FontInsert, Context, FontDefinitions, ViewportId,
};
use egui_wgpu::ScreenDescriptor;
use egui_winit::{EventResponse, State};
use wgpu::TextureFormat;
use winit::{event::WindowEvent, window::Window};

pub struct EguiRenderer {
  state: State,
  renderer: egui_wgpu::Renderer,
  pixels_per_point: f32,
}
impl EguiRenderer {
  pub fn new(
    gfx: &Arc<super::AppGfxService>,
    window: &Arc<Window>,
    output_depth_format: Option<TextureFormat>,
    msaa_samples: u32,
    dithering: bool,
    pixels_per_point: f32,
  ) -> Self {
    let egui_ctx = Context::default();
    let state = State::new(
      egui_ctx,
      ViewportId::ROOT,
      window,
      Some(window.scale_factor() as f32),
      window.theme(),
      Some(1024 * 2),
    );
    let gfx_config = gfx.config.read();
    let renderer = egui_wgpu::Renderer::new(
      &gfx.device,
      gfx_config.config.format,
      output_depth_format,
      msaa_samples,
      dithering,
    );
    Self {
      state,
      renderer,
      pixels_per_point,
    }
  }
  pub fn set_pixels_per_point(&mut self, v: f32) {
    self.pixels_per_point = v;
  }
  pub fn event_input(
    &mut self,
    window: &Window,
    event: &WindowEvent,
  ) -> EventResponse {
    self.state.on_window_event(window, event)
  }
  pub fn replace_font(&self, font_name: impl ToString, font_bin: Vec<u8>) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
      font_name.to_string(),
      Arc::new(egui::FontData::from_owned(font_bin)),
    );
    fonts
      .families
      .entry(egui::FontFamily::Proportional)
      .or_default()
      .insert(0, font_name.to_string());
    fonts
      .families
      .entry(egui::FontFamily::Monospace)
      .or_default()
      .push(font_name.to_string());
    self.state.egui_ctx().set_fonts(fonts);
  }
  pub fn add_font(&self, font_insert: FontInsert) {
    self.state.egui_ctx().add_font(font_insert)
  }
}

impl<'c, 'd, F> super::render_chain::Renderer<(&'d Arc<Window>, F)>
  for EguiRenderer
where
  F: 'c + FnOnce(&Context),
{
  fn request_encoder_count(&self) -> usize {
    1
  }

  fn rendering(
    &mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut [wgpu::CommandEncoder],
    (window, f): (&'d Arc<Window>, F),
  ) -> Result<crate::app_sys::RenderChainCommand, crate::StdError> {
    let raw_input = self.state.take_egui_input(window);
    self.state.egui_ctx().begin_pass(raw_input);
    f(self.state.egui_ctx());
    self.state.egui_ctx().set_pixels_per_point(self.pixels_per_point);
    let wsize = window.inner_size();
    let screen_descriptor = ScreenDescriptor {
      size_in_pixels: [wsize.width, wsize.height],
      pixels_per_point: self.pixels_per_point,
    };
    let full_output = self.state.egui_ctx().end_pass();
    self.state.handle_platform_output(window, full_output.platform_output);
    let tris = self.state.egui_ctx().tessellate(
      full_output.shapes,
      self.state.egui_ctx().pixels_per_point(),
    );
    for (id, image_delta) in &full_output.textures_delta.set {
      self.renderer.update_texture(device, queue, *id, image_delta);
    }
    self.renderer.update_buffers(
      device,
      queue,
      &mut encoder[0],
      &tris,
      &screen_descriptor,
    );
    let render_pass =
      encoder[0].begin_render_pass(&wgpu::RenderPassDescriptor {
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
    self.renderer.render(
      &mut render_pass.forget_lifetime(),
      &tris,
      &screen_descriptor,
    );
    for id in full_output.textures_delta.free.iter() {
      self.renderer.free_texture(id);
    }
    Ok(super::render_chain::RenderChainCommand::AllowContinue)
  }
}
