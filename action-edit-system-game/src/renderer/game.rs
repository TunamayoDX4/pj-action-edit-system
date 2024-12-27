use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TextureHandle(u32);

struct TextureStorage {
  map: hashbrown::HashMap<String, TextureHandle>,
  size: Vec<[u32; 2]>,
  size_f: Vec<[f32; 2]>,
  bind_group: Vec<Option<GRTexture>>,
}
impl TextureStorage {
  pub fn load_diffuse(
    &mut self,
    gfx: &crate::gfx::GfxState,
    path: impl AsRef<std::path::Path>,
    diffuse_bindgroup_layout: &wgpu::BindGroupLayout,
  ) -> Result<(), crate::StdError> {
    let image = {
      let mut buffer = Vec::new();
      let mut rdr = std::io::BufReader::new(std::fs::File::open(path)?);
      rdr.read_to_end(&mut buffer)?;
      image::load_from_memory(&buffer)?
    };
    let rgba = image.to_rgba8();
    let dim = rgba.dimensions();
    let texture_size = wgpu::Extent3d {
      width: dim.0,
      height: dim.1,
      depth_or_array_layers: 1,
    };
    let diffuse_texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
      label: Some("diffuse texture"),
      size: texture_size,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8UnormSrgb,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    gfx.queue.write_texture(
      wgpu::ImageCopyTextureBase {
        texture: &diffuse_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &rgba,
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(4 * dim.0),
        rows_per_image: Some(dim.1),
      },
      texture_size,
    );
    let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let diffuse_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Nearest,
      min_filter: wgpu::FilterMode::Nearest,
      mipmap_filter: wgpu::FilterMode::Nearest,
      ..Default::default()
    });
    let diffuse_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("diffuse bind group"),
      layout: diffuse_bindgroup_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
        },
      ],
    });

    Ok(())
  }
}

struct GRTexture {
  bind_group: wgpu::BindGroup,
}

struct GameRenderer {
  gfx: std::sync::Arc<parking_lot::Mutex<crate::gfx::GfxState>>,
  texture_storage: TextureStorage,
  diffuse_bindgroup_layout: wgpu::BindGroupLayout,
}
impl GameRenderer {
  pub fn new(gfx: std::sync::Arc<parking_lot::Mutex<crate::gfx::GfxState>>) {}
}
impl crate::gfx::Renderer<()> for GameRenderer {
  fn rendering<'a: 'b, 'b>(
    &'a mut self,
    surface_texture: &wgpu::SurfaceTexture,
    surface_view: &wgpu::TextureView,
    device: &'b wgpu::Device,
    queue: &'b wgpu::Queue,
    encoder: &'b mut wgpu::CommandEncoder,
    param: (),
  ) -> Result<(), crate::StdError> {
    todo!()
  }
}
