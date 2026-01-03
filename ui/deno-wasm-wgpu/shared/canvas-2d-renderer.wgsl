//! author: ChatGPT

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let uv = vec2<f32>(
    f32((i << 1u) & 2u),
    f32(i & 2u)
  );
  return vec4f(uv * 2.0 - 1.0, 0.0, 1.0);
}

@group(0) @binding(0) var mySampler: sampler;
@group(0) @binding(1) var myTexture: texture_2d<f32>;

@fragment
fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let dims = vec2f(textureDimensions(myTexture));
  let uv = pos.xy / dims;
  return textureSample(myTexture, mySampler, uv);
}