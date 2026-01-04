mod cube;
mod d2;
mod d2_canvas_hdr;
mod d2_diffuse;
mod d2_normal;
mod depth;
mod formats;

#[allow(unused)]
pub use cube::CubeTexture;
pub use cube::CubeTextureFactory;
pub use d2_canvas_hdr::{D2CanvasHdrTexture, NewD2CanvasHdrTextureOptions};
pub use d2_diffuse::D2DiffuseTexture;
pub use d2_normal::D2NormalTexture;
#[allow(unused)]
pub use depth::DepthTexture;
pub use depth::{DEPTH_FORMAT, DepthTextureNonComparisonSampler};
pub use formats::*;

pub fn make_regular_d2_texture_bind_group_layout(
    label: &str,
    device: &wgpu::Device,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[
            // diffuse texture
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
            // diffuse texture sampler
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // normal texture
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // normal texture sampler
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}
