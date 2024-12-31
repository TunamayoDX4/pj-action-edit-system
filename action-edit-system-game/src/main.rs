use std::sync::Arc;

type StdError = Box<dyn std::error::Error>;

pub struct AppInterface {
  window: Option<Arc<winit::window::Window>>, 
}
impl AppInterface {
  pub fn new() -> Result<Self, StdError> {
    Ok(Self { window: None })
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
    self.window = Some({
      match event_loop.create_window(window_attr) {
        Ok(window) => std::sync::Arc::new(window), 
        Err(e) => {
          log::error!("Application window create failure.");
          log::error!("{e}")
          panic!()
        }
      }
    });
    todo!()
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
  ) {
    todo!()
  }
}

fn main() -> Result<(), StdError> {
  Ok(())
}
