use super::d2::D2TextureRgba16Float;

pub struct D2CanvasHdrTexture(D2TextureRgba16Float);

impl D2CanvasHdrTexture {
    pub fn new(device: &wgpu::Device, label: &str, opts: NewD2CanvasHdrTextureOptions) -> Self {
        let inner = D2TextureRgba16Float::new(
            device,
            Some(label),
            super::d2::NewD2TextureOptions {
                is_color_map: true,
                size: opts.size,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                mag_filter: wgpu::FilterMode::Nearest,
            },
        );
        Self(inner)
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.0.texture()
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view()
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.0.sampler()
    }

    pub fn size(&self) -> wgpu::Extent3d {
        self.0.size()
    }
}

pub struct NewD2CanvasHdrTextureOptions {
    pub size: glam::u32::UVec2,
}
