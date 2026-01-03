use image::EncodableLayout;

pub trait TextureFormat {
    fn linear_rgb() -> wgpu::TextureFormat;
    fn srgb() -> wgpu::TextureFormat;
    fn bytes_per_pixel() -> usize;
    fn image_into_bytes(image: impl Into<image::DynamicImage>) -> Vec<u8>;
}

pub struct TextureFormatRgba8;
impl TextureFormat for TextureFormatRgba8 {
    fn linear_rgb() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }

    fn srgb() -> wgpu::TextureFormat {
        Self::linear_rgb().add_srgb_suffix()
    }

    fn bytes_per_pixel() -> usize {
        std::mem::size_of::<[u8; 4]>()
    }

    fn image_into_bytes(image: impl Into<image::DynamicImage>) -> Vec<u8> {
        image.into().into_rgba8().to_vec()
    }
}

pub struct TextureFormatRgba16Float;
impl TextureFormat for TextureFormatRgba16Float {
    fn linear_rgb() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba16Float
    }

    fn srgb() -> wgpu::TextureFormat {
        Self::linear_rgb().add_srgb_suffix()
    }

    fn bytes_per_pixel() -> usize {
        std::mem::size_of::<[f32; 4]>()
    }

    fn image_into_bytes(image: impl Into<image::DynamicImage>) -> Vec<u8> {
        image.into().into_rgba16().as_bytes().to_vec()
    }
}

pub struct TextureFormatRgba32Float;
impl TextureFormat for TextureFormatRgba32Float {
    fn linear_rgb() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba32Float
    }

    fn srgb() -> wgpu::TextureFormat {
        Self::linear_rgb().add_srgb_suffix()
    }

    fn bytes_per_pixel() -> usize {
        std::mem::size_of::<[f32; 4]>()
    }

    fn image_into_bytes(image: impl Into<image::DynamicImage>) -> Vec<u8> {
        image.into().into_rgba32f().as_bytes().to_vec()
    }
}
