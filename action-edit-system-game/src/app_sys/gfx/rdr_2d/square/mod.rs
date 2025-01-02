use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use parking_lot::RwLock;
use wgpu::{
  vertex_attr_array, BindGroup, BindGroupLayout, Buffer,
  PipelineLayout, RenderPipeline, VertexAttribute,
  VertexBufferLayout,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Instance {
  pub pos: [f32; 2],
  pub size: [f32; 2],
  pub rot: [f32; 2],
  pub filter: [f32; 4],
  pub uv: [[f32; 2]; 2],
}
impl Instance {
  pub const VB_ATTRIB: [VertexAttribute; 5] = vertex_attr_array![
    5 => Float32x2,
    6 => Float32x2,
    7 => Float32x2,
    8 => Float32x4,
    9 => Float32x4,
  ];
  pub fn desc() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &Self::VB_ATTRIB,
    }
  }
}

pub struct SquareRenderer {
  camera: Arc<RwLock<super::camera::Camera2DWGPUObject>>,
  vertices: Buffer,
  indices: Buffer,
  pipeline_layout: PipelineLayout,
  pipeline: RenderPipeline,
  diffuse_bindgroup_layout: BindGroupLayout,
  diffuse_bindgroup: BindGroup,
}
