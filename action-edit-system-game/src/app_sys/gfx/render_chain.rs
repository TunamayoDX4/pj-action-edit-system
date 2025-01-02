use crate::StdError;
use parking_lot::{
  lock_api::{MappedMutexGuard, MutexGuard},
  Mutex,
};
use wgpu::{CommandEncoder, Device, Queue};

/// Trait for renderer that use WGPU
pub trait Renderer<V> {
  fn request_encoder_count(&self) -> usize;
  fn rendering(
    &mut self,
    surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &Device,
    queue: &Queue,
    encoder: &mut [CommandEncoder],
    param: V,
  ) -> Result<RenderChainCommand, StdError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderChainCommand {
  AllowContinue,
  Submit,
}

pub struct RenderChainBase {
  active: Mutex<Vec<CommandEncoder>>,
  finished: Mutex<Vec<CommandEncoder>>,
}
impl RenderChainBase {
  pub(super) fn new() -> Self {
    Self {
      active: Mutex::new(Vec::new()),
      finished: Mutex::new(Vec::new()),
    }
  }

  #[inline]
  fn submit(&self) {
    let mut finished = self.finished.lock();
    self.active.lock().drain(..).for_each(|ce| finished.push(ce));
  }

  #[inline]
  fn prepare<'a: 'b, 'b, V>(
    &'a self,
    device: &wgpu::Device,
    renderer: &impl Renderer<V>,
  ) -> MappedMutexGuard<'b, parking_lot::RawMutex, [CommandEncoder]> {
    let c = renderer.request_encoder_count();
    let mut act_lock: MutexGuard<'b, _, _> = self.active.lock();
    if c < act_lock.len() {
      MutexGuard::map(act_lock, |ce: &mut Vec<CommandEncoder>| {
        &mut ce[..c]
      })
    } else {
      let mut finished = self.finished.lock();
      act_lock.drain(..).for_each(|ce| finished.push(ce));
      for _ in 0..c {
        act_lock.push(device.create_command_encoder(
          &wgpu::CommandEncoderDescriptor {
            label: Some("gfx_implement Auto-Generate CommandEncoder"),
          },
        ));
      }
      MutexGuard::map(act_lock, |ce: &mut Vec<CommandEncoder>| {
        ce.as_mut_slice()
      })
    }
  }
}

pub struct RenderChain<'gfx> {
  texture: wgpu::SurfaceTexture,
  view: wgpu::TextureView,
  device: &'gfx Device,
  queue: &'gfx Queue,
  base: &'gfx RenderChainBase,
  error: Result<(), StdError>,
}
impl<'gfx> RenderChain<'gfx> {
  pub(super) fn new(
    context: &'gfx super::AppGfxService,
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
  ) -> Self {
    Self {
      texture,
      view,
      device: &context.device,
      queue: &context.queue,
      base: &context.chain_base,
      error: Ok(()),
    }
  }

  pub fn rendering<'c, V: 'c>(
    self,
    renderer: &mut impl Renderer<V>,
    param: V,
  ) -> Self {
    match self.error {
      Ok(_) => {
        let mut command_encoderes =
          self.base.prepare(&self.device, renderer);
        match renderer.rendering(
          &self.texture,
          &self.view,
          &self.device,
          &self.queue,
          &mut command_encoderes,
          param,
        ) {
          Ok(o) => {
            match o {
              RenderChainCommand::AllowContinue => {}
              RenderChainCommand::Submit => self.base.submit(),
            }
            Self {
              texture: self.texture,
              view: self.view,
              device: self.device,
              queue: self.queue,
              base: self.base,
              error: Ok(()),
            }
          }
          Err(e) => {
            log::error!("Error occured in rendering process");
            Self {
              texture: self.texture,
              view: self.view,
              device: self.device,
              queue: self.queue,
              base: self.base,
              error: Err(e),
            }
          }
        }
      }
      Err(e) => {
        log::error!("Error detected. skip to rendering");
        Self {
          texture: self.texture,
          view: self.view,
          device: self.device,
          queue: self.queue,
          base: self.base,
          error: Err(e),
        }
      }
    }
  }

  pub fn finish(self) -> Result<(), StdError> {
    self.base.submit();
    self
      .queue
      .submit(self.base.finished.lock().drain(..).map(|e| e.finish()));
    self.texture.present();
    self.error
  }
}
