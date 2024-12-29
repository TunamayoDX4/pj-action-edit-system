use std::io::Read;

use image::GenericImageView;
use wgpu::util::DeviceExt;

use super::Vertex2D;

pub const INDEX: [u16; 6] = [0, 1, 3, 0, 3, 2];
pub const VERTEX: [Vertex2D; 4] = [
  Vertex2D {
    position: [-1., -1.],
    tex_coord: [0., 1.],
  },
  Vertex2D {
    position: [1., -1.],
    tex_coord: [1., 1.],
  },
  Vertex2D {
    position: [-1., 1.],
    tex_coord: [0., 0.],
  },
  Vertex2D {
    position: [1., 1.],
    tex_coord: [1., 0.],
  },
];

#[derive(Debug, Clone)]
pub struct Instance {
  pub position: nalgebra::Point2<f32>,
  pub size: nalgebra::Vector2<f32>,
  pub rot: f32,
  pub uv: [[f32; 2]; 2],
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
  /// インスタンスの格納ベクトルと更新フラグ
  instances: (Vec<RawInstance>, bool),
  camera: super::Camera2D,
  camera_uniform: super::CameraUniform,
  camera_buffer: wgpu::Buffer,
  camera_bindgroup_layout: wgpu::BindGroupLayout,
  camera_bindgroup: wgpu::BindGroup,
  render_pipeline_layout: wgpu::PipelineLayout,
  render_pipeline: wgpu::RenderPipeline,
}
impl SquareRender {
  pub fn new(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    config: &wgpu::SurfaceConfiguration,
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
    let vertex_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex buffer"),
        contents: bytemuck::cast_slice(&VERTEX),
        usage: wgpu::BufferUsages::VERTEX,
      });
    let index_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index buffer"),
        contents: bytemuck::cast_slice(&INDEX),
        usage: wgpu::BufferUsages::INDEX,
      });
    let instances = (Vec::new(), false);
    let instance_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(&instances.0),
        usage: wgpu::BufferUsages::VERTEX,
      });
    let camera = super::Camera2D {
      pos: [0., 180.].into(),
      size: [1. / config.width as f32, 1. / config.height as f32].into(),
      rot: 0. * std::f32::consts::PI / 180.,
      zoom: 1.0,
    };
    let mut camera_uniform = super::CameraUniform::new();
    camera_uniform.update(&camera);
    let camera_buffer =
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      });
    let camera_bindgroup_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Camera bindgroup layout"),
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
    let camera_bindgroup =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera bindgroup"),
        layout: &camera_bindgroup_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: camera_buffer.as_entire_binding(),
        }],
      });
    let render_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render pipeline layout"),
        bind_group_layouts: &[
          &diffuse_bindgroup_layout,
          &camera_bindgroup_layout,
        ],
        push_constant_ranges: &[],
      });
    let shader =
      device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader"),
        source: wgpu::ShaderSource::Wgsl(
          include_str!("square.wgsl").into(),
        ),
      });
    let render_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: Some("vs_main"),
          compilation_options: wgpu::PipelineCompilationOptions::default(),
          buffers: &[super::Vertex2D::desc(), RawInstance::desc()],
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader,
          entry_point: Some("fs_main"),
          compilation_options: wgpu::PipelineCompilationOptions::default(),
          targets: &[Some(wgpu::ColorTargetState {
            format: config.format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
          })],
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          strip_index_format: None,
          front_face: wgpu::FrontFace::Ccw,
          cull_mode: Some(wgpu::Face::Back),
          polygon_mode: wgpu::PolygonMode::Fill,
          unclipped_depth: false,
          conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
          count: 1,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
      });
    Ok(Self {
      texture,
      diffuse_bindgroup,
      diffuse_bindgroup_layout,
      texture_size: nalgebra::Vector2::new(
        dimensions.0 as f32,
        dimensions.1 as f32,
      ),
      vertex_buffer,
      index_buffer,
      instance_buffer,
      instances,
      camera,
      camera_uniform,
      camera_buffer,
      camera_bindgroup_layout,
      camera_bindgroup,
      render_pipeline_layout,
      render_pipeline,
    })
  }

  pub fn instance_register<'a>(
    &mut self,
    instances: impl Iterator<Item = &'a Instance>,
  ) {
    instances.map(|i| i.to_raw()).for_each(|i| self.instances.0.push(i));
    self.instances.1 = true;
  }

  pub fn camera_update(
    &mut self,
    camera: impl FnOnce(&mut super::Camera2D),
  ) {
    camera(&mut self.camera)
  }
}
impl crate::gfx::Renderer<()> for SquareRender {
  fn rendering<'a: 'b, 'b>(
    &'a mut self,
    _surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &'b wgpu::Device,
    queue: &'b wgpu::Queue,
    encoder: &'b mut wgpu::CommandEncoder,
    _param: (),
  ) -> Result<(), crate::StdError> {
    if self.instances.1 {
      self.instance_buffer =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Instance buffer"),
          contents: bytemuck::cast_slice(&self.instances.0),
          usage: wgpu::BufferUsages::VERTEX,
        });
      //self.instances.0.clear();
    }
    self.camera_uniform.update(&self.camera);
    queue.write_buffer(
      &self.camera_buffer,
      0,
      bytemuck::cast_slice(&[self.camera_uniform]),
    );
    let mut rpass =
      encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Square renderer"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: surface_view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      });

    rpass.set_pipeline(&self.render_pipeline);
    rpass.set_bind_group(0, &self.diffuse_bindgroup, &[]);
    rpass.set_bind_group(1, &self.camera_bindgroup, &[]);
    rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
    rpass.set_index_buffer(
      self.index_buffer.slice(..),
      wgpu::IndexFormat::Uint16,
    );
    rpass.draw_indexed(
      0..INDEX.len() as _,
      0,
      0..self.instances.0.len() as _,
    );
    Ok(())
  }
}
