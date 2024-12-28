pub mod square;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D(pub [f32; 2]);
impl Vertex2D {
  const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
    0 => Float32x2
  ];
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::ATTRIBS,
    }
  }
}

struct Texture {
  texture: wgpu::Texture,
  bindgroup: wgpu::BindGroup,
}
