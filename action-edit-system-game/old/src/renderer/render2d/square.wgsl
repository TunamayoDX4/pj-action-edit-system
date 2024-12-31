struct CameraUniform {
  view_proj: mat4x4<f32>, 
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec2<f32>, 
  @location(1) tex_coord: vec2<f32>, 
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>, 
  @location(0) tex_coord: vec2<f32>, 
}

struct InstanceInput {
  @location(5) pos: vec2<f32>, 
  @location(6) size: vec2<f32>, 
  @location(7) rot: vec2<f32>, 
  @location(8) uv_coord0: vec2<f32>, 
  @location(9) uv_coord1: vec2<f32>, 
}

@vertex
fn vs_main(
  model: VertexInput, 
  instance: InstanceInput, 
) -> VertexOutput {
  var out: VertexOutput;
  var pos: vec4<f32>;
  out.tex_coord = instance.uv_coord0 + model.tex_coord * instance.uv_coord1;
  out.clip_position = vec4<f32> (
    instance.size.x * model.position.x, 
    instance.size.y * model.position.y, 
    1., 
    1., 
  );
  out.clip_position = mat4x4<f32>(
    vec4<f32>(
      instance.rot.x, 
      instance.rot.y, 
      0., 0., 
    ), 
    vec4<f32>(
      -instance.rot.y, 
      instance.rot.x, 
      0., 0., 
    ), 
    vec4<f32>(
      instance.pos.x, 
      instance.pos.y, 
      1., 0., 
    ), 
    vec4<f32>(0., 0., 0., 1.), 
  ) * out.clip_position;
  out.clip_position = camera.view_proj * out.clip_position;
  return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(t_diffuse, s_diffuse, in.tex_coord);
}