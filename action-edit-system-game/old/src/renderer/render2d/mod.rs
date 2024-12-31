pub mod square;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D {
  position: [f32; 2],
  tex_coord: [f32; 2],
}
impl Vertex2D {
  const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
  ];
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &Self::ATTRIBS,
    }
  }
}

pub struct Camera2D {
  pos: nalgebra::Point2<f32>,
  size: nalgebra::Vector2<f32>,
  rot: f32,
  zoom: f32,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform([[f32; 4]; 4]);
impl CameraUniform {
  pub fn new() -> Self {
    Self([[0.; 4]; 4])
  }
  pub fn update(&mut self, camera: &Camera2D) {
    let camera_size = camera.size * camera.zoom;
    let m = nalgebra::Matrix4::<f32>::from_columns(&[
      nalgebra::Vector4::new(camera_size.x, 0., 0., 0.),
      nalgebra::Vector4::new(0., camera_size.y, 0., 0.),
      nalgebra::Vector4::new(0., 0., 1., 0.),
      nalgebra::Vector4::new(0., 0., 0., 1.),
    ]) * nalgebra::Matrix4::<f32>::from_columns(&[
      nalgebra::Vector4::new(camera.rot.cos(), camera.rot.sin(), 0., 0.),
      nalgebra::Vector4::new(-camera.rot.sin(), camera.rot.cos(), 0., 0.),
      nalgebra::Vector4::new(
        -(camera.pos.x * camera.rot.cos()
          - camera.pos.y * camera.rot.sin()),
        -(camera.pos.x * camera.rot.sin()
          + camera.pos.y * camera.rot.cos()),
        1.,
        0.,
      ),
      nalgebra::Vector4::new(0., 0., 0., 1.),
    ]);
    self.0 = m.into();
    println!("{:?}", self.0);
  }
}

struct Texture {
  texture: wgpu::Texture,
  bindgroup: wgpu::BindGroup,
}
