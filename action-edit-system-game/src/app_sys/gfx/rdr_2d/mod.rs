use bytemuck::{Pod, Zeroable};
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout};
pub mod camera;
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
