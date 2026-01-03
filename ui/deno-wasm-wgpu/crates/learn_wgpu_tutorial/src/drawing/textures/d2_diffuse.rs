use super::d2::D2TextureRgba8;

pub struct D2DiffuseTexture(D2TextureRgba8);

impl D2DiffuseTexture {
    pub fn from_image_in_memory(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> anyhow::Result<Self> {
        let inner = D2TextureRgba8::from_image_in_memory(device, queue, bytes, label, true)?;
        Ok(Self(inner))
    }

    // pub fn from_image(
    //     device: &wgpu::Device,
    //     queue: &wgpu::Queue,
    //     img: &image::DynamicImage,
    //     label: Option<&str>,
    // ) -> Self {
    //     let inner = D2TextureRgba8::from_image(device, queue, img, label, true);
    //     Self(inner)
    // }

    // pub fn texture(&self) -> &wgpu::Texture {
    //     &self.0.texture()
    // }

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
