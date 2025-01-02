use bytemuck::{Pod, Zeroable};
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Instance {
  pub pos: [f32; 2],
  pub filter: [f32; 2],
  pub uv: [[f32; 2]; 2],
}
impl Instance {
  pub const VB_ATTRIB: [VertexAttribute; 3] = vertex_attr_array![
    5 => Float32x2,
    6 => Float32x4,
    7 => Float32x4,
  ];
  pub fn desc() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &Self::VB_ATTRIB,
    }
  }
}
