use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Vector4};
use wgpu::{
  util::DeviceExt, vertex_attr_array, BindGroup, BindGroupLayout, Buffer,
  BufferAsyncError, BufferDescriptor, BufferUsages, Device, MapMode,
  VertexAttribute, VertexBufferLayout,
};

use crate::StdError;
pub mod square;
pub mod tile;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
  pub pos: [f32; 2],
  pub uv: [f32; 2],
}
impl Vertex {
  pub const VB_ATTRIB: [VertexAttribute; 2] = vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
  ];
  pub fn desc() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::VB_ATTRIB,
    }
  }
}

pub struct Camera2D {
  pub pos: nalgebra::Point2<f32>,
  pub size: nalgebra::Vector2<f32>,
  pub rot: f32,
  pub zoom: f32,
}

pub struct Camera2DWGPUObject {
  uniform: Camera2DUniform,
  mapped: Vec<Arc<Buffer>>,
  recv:
    crossbeam::channel::Receiver<Result<Arc<Buffer>, BufferAsyncError>>,
  send: crossbeam::channel::Sender<Result<Arc<Buffer>, BufferAsyncError>>,
  buffer: Buffer,
  bindgroup_layout: BindGroupLayout,
  bindgroup: BindGroup,
}
impl Camera2DWGPUObject {
  pub fn new(device: &Device) -> Self {
    let (send, recv) = crossbeam::channel::unbounded();
    let buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("2D Camera buffer"),
        contents: bytemuck::cast_slice(&Camera2DUniform::new().0),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
      });
    let bindgroup_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("2D Camera bindgroup Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }],
      });
    let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("2D Camera bindgroup"),
      layout: &bindgroup_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: buffer.as_entire_binding(),
      }],
    });
    Self {
      uniform: Camera2DUniform::new(),
      mapped: Vec::new(),
      recv,
      send,
      bindgroup_layout,
      bindgroup,
      buffer,
    }
  }
  fn update(
    &mut self,
    device: &Device,
    camera: &Camera2D,
  ) -> Result<Arc<Buffer>, StdError> {
    self.uniform.update(camera);
    let mapped = if 1 < self.mapped.len() {
      self.mapped.swap_remove(0)
    } else {
      self.recv.iter().fold(Ok::<_, StdError>(()), |e, b| {
        e?;
        self.mapped.push(b?);
        Ok(())
      })?;
      if 1 < self.mapped.len() {
        self.mapped.swap_remove(0)
      } else {
        Arc::new(device.create_buffer(&BufferDescriptor {
          label: Some("(internal) Camera2DWGPUObject Staging buffer"),
          size: std::mem::size_of_val(&self.uniform.0) as _,
          usage: BufferUsages::COPY_SRC | BufferUsages::MAP_WRITE,
          mapped_at_creation: true,
        }))
      }
    };
    mapped
      .slice(..)
      .get_mapped_range_mut()
      .copy_from_slice(bytemuck::cast_slice(&self.uniform.0));
    Ok(mapped)
  }
}
impl<'c> super::render_chain::Renderer<&'c Camera2D>
  for Camera2DWGPUObject
{
  fn request_encoder_count(&self) -> usize {
    1
  }

  fn rendering(
    &mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    _surface_view: &wgpu::TextureView,
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    encoder: &mut [wgpu::CommandEncoder],
    camera: &'c Camera2D,
  ) -> Result<crate::app_sys::RenderChainCommand, crate::StdError> {
    let staging = self.update(device, camera)?;
    encoder[0].copy_buffer_to_buffer(
      &staging,
      0,
      &self.buffer,
      0,
      std::mem::size_of_val(&self.uniform.0) as _,
    );
    let send = self.send.clone();
    staging.clone().slice(..).map_async(MapMode::Write, move |e| {
      send.send(e.map(|_| staging)).unwrap()
    });
    Ok(crate::app_sys::RenderChainCommand::AllowContinue)
  }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Camera2DUniform([[f32; 4]; 4]);
impl Camera2DUniform {
  pub fn new() -> Self {
    Self([[0.; 4]; 4])
  }
  pub fn update(&mut self, camera: &Camera2D) {
    let camera_size = camera.size * camera.zoom;
    let m = Matrix4::<f32>::from_columns(&[
      Vector4::new(camera_size.x, 0., 0., 0.),
      Vector4::new(0., camera_size.y, 0., 0.),
      Vector4::new(0., 0., 1., 0.),
      Vector4::new(0., 0., 0., 1.),
    ]) * Matrix4::<f32>::from_columns(&[
      Vector4::new(camera.rot.cos(), camera.rot.sin(), 0., 0.),
      Vector4::new(-camera.rot.sin(), camera.rot.cos(), 0., 0.),
      Vector4::new(
        -(camera.pos.x * camera.rot.cos()
          - camera.pos.y * camera.rot.sin()),
        -(camera.pos.x * camera.rot.sin()
          + camera.pos.y * camera.rot.cos()),
        1.,
        0.,
      ),
      Vector4::new(0., 0., 0., 1.),
    ]);
    self.0 = m.into();
  }
}

pub const VERTICES: &'static [Vertex] = &[
  Vertex {
    pos: [-1., -1.],
    uv: [0., 1.],
  },
  Vertex {
    pos: [1., -1.],
    uv: [1., 1.],
  },
  Vertex {
    pos: [-1., 1.],
    uv: [0., 0.],
  },
  Vertex {
    pos: [1., 1.],
    uv: [1., 0.],
  },
];

pub const INDICES: &'static [u16] = &[0, 1, 3, 0, 3, 2];
