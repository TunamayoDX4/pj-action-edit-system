use std::io::Read;

use image::GenericImageView;

use super::Vertex2D;

pub const INDEX: [u16; 6] = [0, 1, 3, 0, 3, 2];
pub const VERTEX: [Vertex2D; 4] = [
  Vertex2D([-1., -1.]),
  Vertex2D([1., -1.]),
  Vertex2D([-1., 1.]),
  Vertex2D([1., 1.]),
];

#[derive(Debug, Clone)]
pub struct Instance {
  position: nalgebra::Point2<f32>,
  size: nalgebra::Vector2<f32>,
  rot: f32,
  uv: [[f32; 2]; 2],
}
impl Instance {
  pub fn to_raw(&self) -> RawInstance {
    RawInstance {
      position: [self.position.x, self.position.y],
      size: [self.size.x, self.size.y],
      rot: [self.rot.cos(), self.rot.sin()],
      uv: self.uv,
    }
  }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawInstance {
  position: [f32; 2],
  size: [f32; 2],
  rot: [f32; 2],
  uv: [[f32; 2]; 2],
}
impl RawInstance {
  pub const ATTRIB: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    5 => Float32x2,
    6 => Float32x2,
    7 => Float32x2,
    8 => Float32x2,
    9 => Float32x2,
  ];
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as _,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &Self::ATTRIB,
    }
  }
}

pub struct SquareRender {
  texture: wgpu::Texture,
  diffuse_bindgroup: wgpu::BindGroup,
  diffuse_bindgroup_layout: wgpu::BindGroupLayout,
  texture_size: nalgebra::Vector2<f32>,
  vertex_buffer: wgpu::Buffer,
  index_buffer: wgpu::Buffer,
  instance_buffer: wgpu::Buffer,
  instances: Vec<Instance>,
}
impl SquareRender {
  pub fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: impl AsRef<std::path::Path>,
  ) -> Result<Self, crate::StdError> {
    let image = {
      let mut buffer = Vec::new();
      let mut rdr = std::io::BufReader::new(std::fs::File::open(path)?);
      rdr.read_to_end(&mut buffer)?;
      image::load_from_memory(&buffer)?
    };
    let rgba = image.to_rgba8();
    let dimensions = image.dimensions();
    let texture_size = wgpu::Extent3d {
      width: dimensions.0,
      height: dimensions.1,
      depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("Diffuse texture"),
      size: texture_size,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &rgba,
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(4 * dimensions.0),
        rows_per_image: Some(dimensions.1),
      },
      texture_size,
    );
    let texture_view =
      texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      ..Default::default()
    });
    let diffuse_bindgroup_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Diffuse texture bindgroup layout"),
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              multisampled: false,
              view_dimension: wgpu::TextureViewDimension::D2,
              sample_type: wgpu::TextureSampleType::Float {
                filterable: true,
              },
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(
              wgpu::SamplerBindingType::Filtering,
            ),
            count: None,
          },
        ],
      });
    let diffuse_bindgroup =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Diffuse bindgroup"),
        layout: &diffuse_bindgroup_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&texture_view),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
          },
        ],
      });

    todo!()
  }
}
