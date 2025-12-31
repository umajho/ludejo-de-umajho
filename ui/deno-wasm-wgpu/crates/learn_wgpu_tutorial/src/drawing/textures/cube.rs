use std::marker::PhantomData;

use crate::drawing::shaders;

use super::{TextureFormat, d2::D2TextureRgba32Float};

pub struct CubeTexture<T: super::TextureFormat> {
    texture_format: PhantomData<T>,

    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

type TheTextureFormat = super::TextureFormatRgba32Float;

impl CubeTexture<TheTextureFormat> {
    fn new(device: &wgpu::Device, label: Option<&str>, opts: NewCubeTextureOptions) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: opts.size.x,
                height: opts.size.y,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TheTextureFormat::srgb(),
            usage: opts.usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture_format: PhantomData,
            texture,
            sampler,
            view,
        }
    }

    fn try_from_equirectangular_hdr_image_in_memory<
        F: FnOnce(&wgpu::TextureView, &wgpu::TextureView) -> (),
    >(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr_bytes: &[u8],
        label: &str,
        opts: CubeTextureFromHdrEquirectangularBytesOptions<F>,
    ) -> anyhow::Result<Self> {
        let hdr_decoder = image::codecs::hdr::HdrDecoder::new(std::io::Cursor::new(hdr_bytes))?;
        let meta = hdr_decoder.metadata();

        let pixels = utils::hdr_decoder_to_pixels!(hdr_decoder);

        let src = D2TextureRgba32Float::from_pixel_buffer(
            device,
            queue,
            (meta.width, meta.height),
            &bytemuck::cast_slice(&pixels),
            Some(label),
            true,
        );

        let dst = CubeTexture::new(
            device,
            Some(label),
            NewCubeTextureOptions {
                size: (opts.dst_size, opts.dst_size).into(),
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            },
        );

        let dst_view = dst.texture().create_view(&wgpu::TextureViewDescriptor {
            label: Some(label),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        (opts.compute_equirect_to_cubemap)(src.view(), &dst_view);

        Ok(dst)
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

struct NewCubeTextureOptions {
    size: glam::u32::UVec2,
    usage: wgpu::TextureUsages,
}

struct CubeTextureFromHdrEquirectangularBytesOptions<
    F: FnOnce(&wgpu::TextureView, &wgpu::TextureView) -> (),
> {
    dst_size: u32,

    compute_equirect_to_cubemap: F,
}

mod utils {
    pub macro hdr_decoder_to_pixels($hdr_decoder:ident) {
        cfg_select! {
            target_arch = "wasm32" => {{
                $hdr_decoder.read_image_native()?
                    .into_iter()
                    .map(|pix| {
                        let rgb = pix.to_hdr();
                        [rgb.0[0], rgb.0[1], rgb.0[2], 1.0f32]
                    })
                    .collect::<Vec<_>>()
              }}
              _ => {{
                let meta = $hdr_decoder.metadata();
                let mut pixels = vec![[0.0; 4]; meta.width as usize * meta.height as usize];
                $hdr_decoder.read_image_transform(
                    |pix| {
                        let rgb = pix.to_hdr();
                        [rgb.0[0], rgb.0[1], rgb.0[2], 1.0f32]
                    },
                    &mut pixels[..],
                )?;
                pixels
            }}
        }
    }
}

pub struct CubeTextureFactory {
    equirect_layout: wgpu::BindGroupLayout,
    equirect_to_cubemap: wgpu::ComputePipeline,
}

impl CubeTextureFactory {
    pub fn new(device: &wgpu::Device) -> Self {
        let equirect_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("HdrLoader::equirect_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TheTextureFormat::srgb(),
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&equirect_layout],
            push_constant_ranges: &[],
        });

        let equirect_to_cubemap = device.create_compute_pipeline(
            &shaders::c_equirectangular(device).compute_pipeline_descriptor(
                shaders::ComputePipelineDescriptorPartial {
                    label: Some("equirect_to_cube_map"),
                    layout: Some(&pipeline_layout),
                    compilation_options: Default::default(),
                    cache: None,
                },
            ),
        );

        Self {
            equirect_layout,
            equirect_to_cubemap,
        }
    }

    pub fn try_make_cube_texture_from_equirectangular_hdr_image_in_memory(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr_bytes: &[u8],
        dst_size: u32,
        label: &str,
    ) -> anyhow::Result<CubeTexture<TheTextureFormat>> {
        CubeTexture::try_from_equirectangular_hdr_image_in_memory(
            device,
            queue,
            hdr_bytes,
            label,
            CubeTextureFromHdrEquirectangularBytesOptions {
                dst_size,
                compute_equirect_to_cubemap: |src_view, dst_view| {
                    self.compute_equirect_to_cubemap(
                        device, queue, src_view, dst_view, dst_size, label,
                    )
                },
            },
        )
    }

    fn compute_equirect_to_cubemap(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
        dst_size: u32,
        label: &str,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.equirect_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(dst_view),
                },
            ],
        });

        let mut encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });

            let num_workgroups = (dst_size + 15) / 16;
            pass.set_pipeline(&self.equirect_to_cubemap);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);
        }

        queue.submit(Some(encoder.finish()));
    }
}
