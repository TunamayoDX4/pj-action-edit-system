type StdError = Box<dyn std::error::Error>;

mod gfx;
mod renderer;

/// GUI利用機能の状態
struct GuiState {
  window: std::sync::Arc<winit::window::Window>,
  gfx: std::sync::Arc<parking_lot::Mutex<gfx::GfxState>>,
  egui: renderer::EguiRenderer,
}
impl GuiState {
  pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self, StdError> {
    let gfx = std::sync::Arc::new(parking_lot::Mutex::new(
      gfx::GfxState::new(window.clone()).await?,
    ));
    let egui = renderer::EguiRenderer::new(&gfx.lock(), &window, None, 1, true);
    Ok(Self {
      window: window.clone(),
      gfx,
      egui,
    })
  }
}

struct App {
  main: Option<GuiState>,
  script_line: String,
  error_output: Option<String>, 
  lua: mlua::Lua, 
}
impl App {
  pub fn new() -> Self {
    Self { main: None, script_line: String::new(), error_output: None, lua: mlua::Lua::new() }
  }
}
impl winit::application::ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
    let window = event_loop
      .create_window(
        winit::window::WindowAttributes::default()
          .with_active(true)
          .with_resizable(false)
          .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
      )
      .expect("Window initialize failure");
    let window = std::sync::Arc::new(window);

    // ウィンドウの中央寄せ
    if let Some(mon) = window.primary_monitor() {
      // WEB/WayLandではNoneが返るのでその際は何もしない。
      let mon_pos = mon.position();
      let mon_size = mon.size();
      let wdw_size = window.outer_size();
      window.set_outer_position(winit::dpi::PhysicalPosition::new(
        mon_pos.x + (mon_size.width / 2 - wdw_size.width / 2) as i32,
        mon_pos.y + (mon_size.height / 2 - wdw_size.height / 2) as i32,
      ));
    }

    let gui = pollster::block_on(GuiState::new(window)).expect("GUI state initialize failure");
    self.main = Some(gui);
  }

  fn window_event(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_id: winit::window::WindowId,
    event: winit::event::WindowEvent,
  ) {
    if self
      .main
      .as_ref()
      .map_or(false, |w| w.window.id() == window_id)
    {
      let mw = self.main.as_mut().unwrap();
      let _ = mw.egui.event_input(&mw.window, &event);
      match event {
        winit::event::WindowEvent::CloseRequested => event_loop.exit(),
        winit::event::WindowEvent::RedrawRequested => {
          let mut gfx = mw.gfx.lock();
          if !mw.window.is_minimized().unwrap_or(false) {
            match gfx.draw() {
              Ok(o) => o
                .rendering(&mut renderer::TestRenderer([0.1, 0.2, 0.3, 1.]), ())
                .unwrap()
                .rendering(
                  &mut mw.egui,
                  (
                    mw.window.clone(),
                    egui_wgpu::ScreenDescriptor {
                      size_in_pixels: [mw.window.inner_size().width, mw.window.inner_size().height],
                      pixels_per_point: 1.,
                    },
                    |cx| {
                      egui::Window::new(egui::RichText::new("winit window"))
                        .resizable(true)
                        .vscroll(true)
                        .default_open(false)
                        .show(cx, |ui| {
                          ui.vertical(|ui| {
                            ui.text_edit_multiline(&mut self.script_line);
                            if ui.button("Execute").clicked() {
                              match self.lua.load(self.script_line.as_str()).exec() {
                                Ok(_) => self.error_output = None, 
                                Err(e) => self.error_output = Some(format!("{e}")), 
                              }
                            }
                            if let Some(error_output) = self.error_output.as_ref() {
                              ui.label(egui::RichText::new(error_output).color(egui::Color32::from_rgb(255, 0, 0)));
                            }
                          });
                        });
                    },
                  ),
                )
                .unwrap()
                .flush_encoder()
                .finish(),
              Err(wgpu::SurfaceError::Lost) => gfx.surface_reconfig(),
              Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Outdated) => {}
              Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of memory error occured.");
                eprintln!("NOT RECOVERABLE.");
                event_loop.exit();
              }
            }
          }
          mw.window.request_redraw();
        }
        winit::event::WindowEvent::Resized(new_size) => mw.gfx.lock().surface_resize(new_size),
        _ => {}
      }
    }
  }
}

fn main() -> Result<(), StdError> {
  env_logger::Builder::from_default_env()
    .target(env_logger::Target::Stdout)
    .filter_level(if cfg!(debug_assertions) {
      log::LevelFilter::Info
    } else {
      log::LevelFilter::Info
    })
    .init();
  let event_loop = winit::event_loop::EventLoop::new()?;
  event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
  event_loop.run_app(&mut App::new())?;
  Ok(())
}
