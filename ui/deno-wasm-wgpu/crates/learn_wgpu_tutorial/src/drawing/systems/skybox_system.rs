use crate::drawing::{
    shaders,
    systems::{camera_system::CameraSystem, canvas_system::CanvasSystem},
    textures,
    utils::make_render_pipeline,
};

pub struct SkyboxSystem {
    environment_bind_group_layout: wgpu::BindGroupLayout,
    environment_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
}

impl SkyboxSystem {
    pub fn new(
        device: &wgpu::Device,
        sky_texture: textures::CubeTexture<textures::TextureFormatRgba32Float>,
        canvas_sys: &CanvasSystem,
        camera_sys: &CameraSystem,
    ) -> Self {
        let environment_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("environment_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let environment_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment_bind_group"),
            layout: &environment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_texture.sampler()),
                },
            ],
        });

        let sky_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[
                    camera_sys.bind_group_layout(),
                    &environment_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            make_render_pipeline(
                "sky",
                &device,
                &layout,
                canvas_sys.canvas_color_format(),
                Some(textures::DEPTH_FORMAT),
                &[],
                wgpu::PrimitiveTopology::TriangleList,
                &shaders::r_sky(&device),
            )
        };

        Self {
            environment_bind_group_layout,
            environment_bind_group,
            sky_pipeline,
        }
    }

    pub fn environment_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.environment_bind_group_layout
    }

    pub fn environment_bind_group(&self) -> &wgpu::BindGroup {
        &self.environment_bind_group
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>, camera_sys: &CameraSystem) {
        render_pass.set_pipeline(&self.sky_pipeline);
        render_pass.set_bind_group(0, camera_sys.entry().bind_group(), &[]);
        render_pass.set_bind_group(1, &self.environment_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
