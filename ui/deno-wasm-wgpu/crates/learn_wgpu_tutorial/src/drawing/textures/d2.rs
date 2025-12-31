use std::marker::PhantomData;

use image::GenericImageView;

pub type D2TextureRgba8 = D2Texture<super::TextureFormatRgba8>;
pub type D2TextureRgba16Float = D2Texture<super::TextureFormatRgba16Float>;
pub type D2TextureRgba32Float = D2Texture<super::TextureFormatRgba32Float>;

pub(super) struct D2Texture<T: super::TextureFormat> {
    texture_format: PhantomData<T>,

    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl<T: super::TextureFormat> D2Texture<T> {
    pub(super) fn from_pixel_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dimensions: (u32, u32),
        pixel_buffer: &[u8],
        label: Option<&str>,
        is_color_map: bool,
    ) -> Self {
        let texture = Self::new(
            device,
            label,
            NewD2TextureOptions {
                is_color_map,
                size: dimensions.into(),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                mag_filter: wgpu::FilterMode::Linear,
            },
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixel_buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(T::bytes_per_pixel() as u32 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture.size(),
        );

        texture
    }

    pub(super) fn new(
        device: &wgpu::Device,
        label: Option<&str>,
        opts: NewD2TextureOptions,
    ) -> Self {
        let format = if opts.is_color_map {
            T::srgb()
        } else {
            T::linear_rgb()
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: opts.size.x,
                height: opts.size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: opts.usage,
            view_formats: &[format.add_srgb_suffix()],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: opts.mag_filter,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture_format: PhantomData,
            texture,
            view,
            sampler,
        }
    }

    pub(super) fn from_image_in_memory(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        is_color_map: bool,
    ) -> anyhow::Result<Self> {
        let img = image::load_from_memory(bytes)?;
        let texture = Self::from_image(device, queue, &img, Some(label), is_color_map);
        Ok(texture)
    }

    pub(super) fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        is_color_map: bool,
    ) -> Self {
        let rgba = T::image_into_bytes(img.clone());
        let dimensions = img.dimensions();

        Self::from_pixel_buffer(device, queue, dimensions, &rgba, label, is_color_map)
    }

    pub(super) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub(super) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub(super) fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub(super) fn size(&self) -> wgpu::Extent3d {
        self.texture.size()
    }
}

pub(super) struct NewD2TextureOptions {
    pub is_color_map: bool,

    pub size: glam::u32::UVec2,
    pub usage: wgpu::TextureUsages,

    pub mag_filter: wgpu::FilterMode,
}
