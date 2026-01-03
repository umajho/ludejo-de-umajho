struct CameraUniform {
  view_pos: vec4<f32>,
  view: mat4x4<f32>,
  view_proj: mat4x4<f32>,
  inv_proj: mat4x4<f32>,
  inv_view: mat4x4<f32>,
}