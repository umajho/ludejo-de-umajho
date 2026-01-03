use crate::drawing::{shaders, textures, utils::make_render_pipeline};

const CANVAS_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct CanvasSystem {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    canvas: HdrToneMappingCanvas,
}

impl CanvasSystem {
    pub fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        size: glam::UVec2,
    ) -> Self {
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.x,
            height: size.y,
            present_mode: surface_caps.present_modes[0],
            // present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        let canvas = HdrToneMappingCanvas::new(device, &config);

        Self {
            surface,
            surface_config: config,
            is_surface_configured: false,
            canvas,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(device, &self.surface_config);
        self.is_surface_configured = true;

        self.canvas.resize(device, width, height);
    }

    pub fn is_ready(&self) -> bool {
        self.is_surface_configured
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    pub fn canvas_view(&self) -> &wgpu::TextureView {
        self.canvas.view()
    }

    pub fn canvas_color_format(&self) -> wgpu::TextureFormat {
        CANVAS_COLOR_FORMAT
    }

    pub fn try_do_render_pass_and_present(
        &mut self,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        additional: impl FnOnce(&mut wgpu::CommandEncoder, wgpu::TextureView) -> (),
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_config.format.add_srgb_suffix()),
            ..Default::default()
        });
        self.canvas.do_render_pass(&mut encoder, &output_view);

        additional(&mut encoder, output_view);

        queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct HdrToneMappingCanvas {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: textures::D2CanvasHdrTexture,
    layout: wgpu::BindGroupLayout,
}

impl HdrToneMappingCanvas {
    fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Hdr::layout"),
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
            glam::UVec2::new(config.width.max(1), config.height.max(1)),
        );

        debug_assert_eq!(texture.texture().format(), CANVAS_COLOR_FORMAT);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = make_render_pipeline(
            "hdr",
            device,
            &pipeline_layout,
            config.format,
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
            device,
            "Hdr::texture",
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
            label: Some("Hdr::bind_group"),
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
