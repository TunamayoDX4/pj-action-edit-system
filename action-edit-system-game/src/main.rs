mod app_sys;
type StdError = Box<dyn std::error::Error>;

fn main() -> Result<(), StdError> {
  // Initializing env_logger
  env_logger::Builder::from_default_env()
    .target(env_logger::Target::Stdout)
    .filter_level(if cfg!(debug_assertions) {
      log::LevelFilter::Debug
    } else {
      log::LevelFilter::Info
    })
    .init();

  // Preparing application
  log::info!("Preparing application.");
  let mut app = app_sys::AppInterface::new()?;
  let event_loop =
    winit::event_loop::EventLoopBuilder::default().build().map(|evl| {
      evl.set_control_flow(winit::event_loop::ControlFlow::Poll);
      evl
    })?;

  // Starting application
  log::info!("Run application.");
  event_loop.run_app(&mut app)?;
  log::info!("Quit the application.");

  Ok(())
}
