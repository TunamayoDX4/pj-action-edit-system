use ouroboros::self_referencing;
use wgpu::{Backends, Device, DeviceType, Queue, Surface, SurfaceConfiguration};
use winit::{
  application::ApplicationHandler,
  dpi::{PhysicalPosition, PhysicalSize},
  event::WindowEvent,
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  window::{Window, WindowAttributes, WindowId},
};

struct Stage {}

struct VarStorage {}

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
enum WgpuAdapterSelMode {
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

struct WgpuRenderChain<'a, 'b: 'a> {
  state: &'a WgpuState<'b>,
}

struct WgpuState<'a> {
  window: &'a Window,
  device: Device,
  queue: Queue,
  config: SurfaceConfiguration,
  surface: Surface<'a>,
}
impl<'a> WgpuState<'a> {
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
    Self {
      window,
      device,
      queue,
      config,
      surface,
    }
  }
}

#[self_referencing]
struct WindowHolder {
  window: Window,
  #[borrows(window)]
  #[not_covariant]
  wgpu_state: WgpuState<'this>,
}
impl WindowHolder {
  pub fn initialize(window: Window) -> Self {
    WindowHolderBuilder {
      window,
      wgpu_state_builder: |w| WgpuState::new(w, WgpuAdapterSelMode::HighPower),
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
      WindowEvent::RedrawRequested => {
        if let Some(main_window) = self.window_holder.as_ref() {
          if main_window.borrow_window().id() == window_id {
            main_window.borrow_window().request_redraw();
          }
        }
      }
      _ => {}
    }
  }
}

fn main() {
  let main_event_loop = EventLoop::new().expect("Main event initialize failure");
  main_event_loop.set_control_flow(ControlFlow::Poll);
  let mut app = App::new();
  match main_event_loop.run_app(&mut app) {
    Ok(_) => {}
    Err(e) => eprintln!("{e}"),
  }
}
