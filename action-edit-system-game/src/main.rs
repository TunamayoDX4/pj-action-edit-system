use mlua::prelude::*;
use ouroboros::self_referencing;
use winit::{
  application::ApplicationHandler,
  dpi::{PhysicalPosition, PhysicalSize},
  event::WindowEvent,
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  window::{Window, WindowId},
};

mod graphics;

#[self_referencing]
struct WindowHolder {
  window: Window,
  #[borrows(window)]
  #[not_covariant]
  wgpu_state: graphics::WgpuState<'this>,
}
impl WindowHolder {
  pub fn initialize(window: Window) -> Self {
    WindowHolderBuilder {
      window,
      wgpu_state_builder: |w| graphics::WgpuState::new(w, graphics::WgpuAdapterSelMode::HighPower),
    }
    .build()
  }
}

struct App {
  window_holder: Option<WindowHolder>,
}
impl App {
  pub fn new() -> Self {
    Self {
      window_holder: None,
    }
  }
}
impl ApplicationHandler for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = event_loop
      .create_window(
        Window::default_attributes()
          .with_inner_size(PhysicalSize::new(1280, 720))
          .with_active(true),
      )
      .expect("Create Main window failure");
    let wsize = window.outer_size();
    let primary_mon = event_loop
      .primary_monitor()
      .expect("Getting primary monitor handle failure");
    let pmon_size = primary_mon.size();
    let pmon_pos = primary_mon.position();
    window.set_outer_position(PhysicalPosition::new(
      pmon_pos.x + (pmon_size.width / 2 - wsize.width / 2) as i32,
      pmon_pos.y + (pmon_size.height / 2 - wsize.height / 2) as i32,
    ));
    self.window_holder = Some(WindowHolder::initialize(window))
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::CloseRequested => {
        event_loop.exit();
      }
      WindowEvent::Resized(new_size) => {
        if let Some(main_window) = self.window_holder.as_mut() {
          if main_window.borrow_window().id() == window_id {
            main_window.with_wgpu_state_mut(|ws| ws.re_size(new_size))
          }
        }
      }
      WindowEvent::RedrawRequested => {
        if let Some(main_window) = self.window_holder.as_mut() {
          if main_window.borrow_window().id() == window_id {
            main_window.borrow_window().request_redraw();
            main_window.with_wgpu_state_mut(|ws| match ws.rendering() {
              Ok(_) => {}
              Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => ws.re_configure(),
              Err(wgpu::SurfaceError::Timeout) => eprintln!("Surface timeout..."),
              Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!("Graphics Out of Memory occured!");
                event_loop.exit();
              }
            });
          }
        }
      }
      _ => {}
    }
  }
}

fn main() {
  let lua = Lua::new();
  let map_table = lua.create_table().expect("Lua table initialize failure");
  map_table.set("takashi", 32).expect("Lua table set failure.");
  map_table.set("yasushi", 14).expect("Lua table set failure.");
  lua.globals().set("map_table", map_table).expect("Lua global parameter set failure");
  lua.load("for k,v in pairs(map_table) do print(k,v) end").exec().expect("Lua executing failure");
  let main_event_loop = EventLoop::new().expect("Main event initialize failure");
  main_event_loop.set_control_flow(ControlFlow::Poll);
  let mut app = App::new();
  match main_event_loop.run_app(&mut app) {
    Ok(_) => {}
    Err(e) => eprintln!("{e}"),
  }
}
