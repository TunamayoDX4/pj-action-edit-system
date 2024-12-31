//! Application System Implement
//! アプリケーションのシステムとの結合部分の実装

use crate::StdError;
use std::sync::Arc;
use winit::event::WindowEvent;

pub mod gfx;
pub use gfx::renderer::{RenderChainCommand, Renderer};

pub struct TestRender;
impl gfx::renderer::Renderer<()> for TestRender {
  fn request_encoder_count(&self) -> usize {
    1usize
  }

  fn rendering(
    &mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    encoder: &mut [wgpu::CommandEncoder],
    _param: (),
  ) -> Result<gfx::renderer::RenderChainCommand, StdError> {
    encoder[0].begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("Test renderer"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: surface_view,
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
    Ok(gfx::renderer::RenderChainCommand::AllowContinue)
  }
}

/// GUI Interface
pub struct AppGuiService {
  window: Arc<winit::window::Window>,
  gfx: Arc<gfx::AppGfxService>,
}
impl AppGuiService {
  pub fn new(window: winit::window::Window) -> Result<Self, StdError> {
    let window = Arc::new(window);
    let gfx =
      Arc::new(pollster::block_on(gfx::AppGfxService::new(&window))?);

    Ok(Self { window, gfx })
  }
}

pub struct AppInterface {
  gui: Option<AppGuiService>,
}
impl AppInterface {
  pub fn new() -> Result<Self, StdError> {
    Ok(Self { gui: None })
  }
}
impl winit::application::ApplicationHandler for AppInterface {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    let window_attr = winit::window::WindowAttributes::default()
      .with_active(true)
      .with_resizable(false)
      .with_enabled_buttons(winit::window::WindowButtons::CLOSE)
      .with_fullscreen(None)
      .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
    self.gui = Some({
      match event_loop.create_window(window_attr) {
        Ok(window) => {
          if let Some(mon) = window.current_monitor() {
            let mon_pos = mon.position();
            let mon_size = mon.size();
            let w_size = window.outer_size();
            let w_pos = winit::dpi::PhysicalPosition::new(
              mon_pos.x + (mon_size.width / 2 - w_size.width / 2) as i32,
              mon_pos.y + (mon_size.height / 2 - w_size.height / 2) as i32,
            );
            window.set_outer_position(w_pos);
          }
          let gui = match AppGuiService::new(window) {
            Ok(gui) => gui,
            Err(e) => {
              log::error!("Gui initialize process failure");
              log::error!("{e}");
              panic!("{e}")
            }
          };
          gui
        }
        Err(e) => {
          log::error!("Application window create failure.");
          log::error!("{e}");
          panic!("{e}")
        }
      }
    });
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    if let Some(gui) =
      self.gui.as_ref().filter(|g| g.window.id() == window_id)
    {
      match event {
        WindowEvent::CloseRequested => event_loop.exit(),
        WindowEvent::RedrawRequested => {
          match gui.gfx.rendering() {
            Ok(rc) => match rc.rendering(&mut TestRender, ()).finish() {
              Ok(_) => {}
              Err(e) => {
                log::error!("Rendering error occured.");
                log::error!("{e}");
                event_loop.exit()
              }
            },
            Err(wgpu::SurfaceError::OutOfMemory) => {
              log::error!("wgpu Out-of-Memory error occured.");
              log::error!("Cannot coverable. program will terminate");
              event_loop.exit()
            }
            Err(
              wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
            ) => {
              gui.gfx.reconfigure();
            }
            Err(wgpu::SurfaceError::Timeout) => {
              log::warn!("wgpu surface timeout!")
            }
          }
          gui.window.request_redraw()
        }
        _ => {}
      }
    }
  }
}
