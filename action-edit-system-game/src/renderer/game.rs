use std::{collections::VecDeque, io::Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TextureHandle(u32);

struct TextureStorage {
  map: hashbrown::HashMap<String, TextureHandle>,
  size: Vec<[u32; 2]>,
  size_f: Vec<[f32; 2]>,
  bind_group: Vec<Option<GRTexture>>,
  remove_queue: VecDeque<usize>,
}
impl TextureStorage {
  pub fn load_diffuse(
    &mut self,
    gfx: &crate::gfx::GfxState,
    path: impl AsRef<std::path::Path>,
    diffuse_bindgroup_layout: &wgpu::BindGroupLayout,
  ) -> Result<Option<TextureHandle>, crate::StdError> {
    let name = match path.as_ref().file_name().map(|s| s.to_str()).flatten().map(|n| n.split('.').next()).flatten().map(|s| (s, self.map.contains_key(s))) {
      Some((fname, true)) => {
        log::warn!("file name {fname} duplicated");
        return Ok(None);
      }
      Some((fname, false)) => fname,
      None => {
        log::error!("bad file name");
        return Ok(None);
      }
    }
    .to_owned();
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

    let handle = TextureHandle(match self.remove_queue.pop_front() {
      Some(idx) => {
        self.size[idx] = [dim.0, dim.1];
        self.size_f[idx] = [dim.0 as f32, dim.1 as f32];
        self.bind_group[idx] = Some(GRTexture {
          bind_group: diffuse_bind_group,
        });
        idx
      }
      None => {
        let l = self.bind_group.len();
        self.size.push([dim.0, dim.1]);
        self.size_f.push([dim.0 as f32, dim.1 as f32]);
        self.bind_group.push(Some(GRTexture {
          bind_group: diffuse_bind_group,
        }));
        l
      }
    } as u32);
    if self.map.insert(name, handle).is_some() {
      unreachable!()
    }

    Ok(Some(handle))
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
  pub fn new(gfx: std::sync::Arc<parking_lot::Mutex<crate::gfx::GfxState>>) -> Self {
    let diffuse_bindgroup_layout = {
      let gfx = gfx.lock();
      gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("diffuse bindgroup layout"),
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
              multisampled: false,
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
          },
        ],
      })
    };
    Self {
      gfx,
      texture_storage: TextureStorage {
        map: hashbrown::HashMap::new(),
        size: Vec::new(),
        size_f: Vec::new(),
        bind_group: Vec::new(),
        remove_queue: VecDeque::new(),
      },
      diffuse_bindgroup_layout,
    }
  }
  pub fn load_diffuse(&mut self, path: impl AsRef<std::path::Path>) -> Result<Option<TextureHandle>, crate::StdError> {
    self.texture_storage.load_diffuse(&self.gfx.lock(), path, &self.diffuse_bindgroup_layout)
  }
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
