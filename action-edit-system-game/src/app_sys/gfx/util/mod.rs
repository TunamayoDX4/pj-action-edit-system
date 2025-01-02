use std::collections::VecDeque;

use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::{BindGroup, Texture};

pub struct TextureStorage {
  table: HashMap<String, TextureID>,
  image: Vec<Option<Vec<u8>>>,
  pixel_size: Vec<usize>,
  size: Vec<[u32; 2]>,
  size_f: Vec<[f32; 2]>,
  texture: Vec<Option<Texture>>,
  bindgroup: Vec<Option<BindGroup>>,
  remove_queue: VecDeque<TextureID>,
  section: Box<TextureStorageSection>,
}

pub struct TextureStorageSection {
  table: Vec<Option<HashMap<String, TextureSectionID>>>,
  range: Vec<Option<Vec<[[u32; 2]; 2]>>>,
  range_f: Vec<Option<Vec<[[f32; 2]; 2]>>>,
  remove_queue: VecDeque<TextureSectionID>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct TextureID(u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct TextureSectionID(u32);
