@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
  var out: VertexOutput;

  out.uv = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
  out.clip_position = vec4f(out.uv * 2.0 - 1.0, 0.0, 1.0);
  out.uv.y = 1.0 - out.uv.y;

  return out;
}

struct VertexOutput {
  @location(0) uv: vec2<f32>,
  @builtin(position) clip_position: vec4<f32>,
}

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

@fragment
fn fs_main(vs: VertexOutput) -> @location(0) vec4f {
  return textureSample(src_texture, src_sampler, vs.uv);
}