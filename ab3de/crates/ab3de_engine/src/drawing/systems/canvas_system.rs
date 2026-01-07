use crate::drawing::{shaders, textures, utils::make_render_pipeline};

pub const CANVAS_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct CanvasEntry {
    config: CanvasEntryConfiguration,
    canvas: HdrToneMappingCanvas,
}

pub struct CanvasEntryConfiguration {
    pub size: glam::UVec2,
    pub color_format: wgpu::TextureFormat,
}

impl CanvasEntry {
    pub fn new(device: &wgpu::Device, config: CanvasEntryConfiguration) -> Self {
        let canvas = HdrToneMappingCanvas::new(device, &config);

        Self { config, canvas }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.canvas.resize(device, width, height);
    }

    pub fn config(&self) -> &CanvasEntryConfiguration {
        &self.config
    }

    pub fn canvas_view(&self) -> &wgpu::TextureView {
        self.canvas.view()
    }

    pub fn try_do_render_pass_and_present(
        &self,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        additional: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView) -> (),
    ) {
        self.canvas.do_render_pass(&mut encoder, &output_view);

        additional(&mut encoder, output_view);

        queue.submit(std::iter::once(encoder.finish()));
    }
}

struct HdrToneMappingCanvas {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: textures::D2CanvasHdrTexture,
    layout: wgpu::BindGroupLayout,
}

impl HdrToneMappingCanvas {
    fn new(device: &wgpu::Device, config: &CanvasEntryConfiguration) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("[HdrToneMappingCanvas::new] bind group layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let (texture, bind_group) = Self::make_texture_and_bind_group(
            device,
            &layout,
            glam::UVec2::new(config.size.x.max(1), config.size.y.max(1)),
        );

        debug_assert_eq!(texture.texture().format(), CANVAS_COLOR_FORMAT);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[HdrToneMappingCanvas::new] render pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = make_render_pipeline(
            "[HdrToneMappingCanvas::new] render pipeline",
            device,
            &pipeline_layout,
            config.color_format,
            None,
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            &shaders::r_hdr_tonemapping(device),
        );

        Self {
            pipeline,
            bind_group,
            texture,
            layout,
        }
    }

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        (self.texture, self.bind_group) = Self::make_texture_and_bind_group(
            device,
            &self.layout,
            glam::UVec2::new(width, height),
        );
    }

    fn make_texture_and_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: glam::UVec2,
    ) -> (textures::D2CanvasHdrTexture, wgpu::BindGroup) {
        let texture = textures::D2CanvasHdrTexture::new(
            "memory:hdr-tone-mapping-canvas",
            device,
            textures::NewD2CanvasHdrTextureOptions { size },
        );
        let bind_group = Self::make_bind_group(device, layout, &texture);
        (texture, bind_group)
    }

    fn make_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &textures::D2CanvasHdrTexture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[HdrToneMappingCanvas::make_bind_group] bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texture.sampler()),
                },
            ],
        })
    }

    fn view(&self) -> &wgpu::TextureView {
        self.texture.view()
    }

    fn do_render_pass(&self, encoder: &mut wgpu::CommandEncoder, output: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Hdr::process"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
