use wgpu::util::DeviceExt;

use crate::{
    drawing::{models::ShapeVertex, shaders},
    textures,
};

pub struct DepthSystem {
    texture: textures::DepthTextureNonComparisonSampler,

    debug_drawer: DebugDrawer,
}

impl DepthSystem {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let texture = Self::make_texture(device, (config.width, config.height).into());

        let debug_drawer = DebugDrawer::new(device, config, &texture);

        Self {
            texture,
            debug_drawer,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.texture = Self::make_texture(device, (width, height).into());

        self.debug_drawer.update_bind_group(device, &self.texture);
    }

    pub fn view(&self) -> &wgpu::TextureView {
        self.texture.view()
    }

    #[allow(unused)]
    pub fn debug_draw(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.debug_drawer.draw(view, encoder);
    }

    fn make_texture(
        device: &wgpu::Device,
        size: glam::UVec2,
    ) -> textures::DepthTextureNonComparisonSampler {
        textures::DepthTextureNonComparisonSampler::new(device, size, "depth_texture")
    }
}

const DEPTH_VERTICES: &[ShapeVertex] = &[
    ShapeVertex {
        position: glam::vec3(0.0, 0.0, 0.0),
        tex_coords: glam::vec2(0.0, 1.0),
    },
    ShapeVertex {
        position: glam::vec3(1.0, 0.0, 0.0),
        tex_coords: glam::vec2(1.0, 1.0),
    },
    ShapeVertex {
        position: glam::vec3(1.0, 1.0, 0.0),
        tex_coords: glam::vec2(1.0, 0.0),
    },
    ShapeVertex {
        position: glam::vec3(0.0, 1.0, 0.0),
        tex_coords: glam::vec2(0.0, 0.0),
    },
];

const DEPTH_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

// https://github.com/sotrh/learn-wgpu/blob/075f2a53b5112f3275aad1746104013e7316c80b/code/beginner/tutorial8-depth/src/challenge.rs#L278
struct DebugDrawer {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_index_count: u32,
    render_pipeline: wgpu::RenderPipeline,
}

impl DebugDrawer {
    fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        texture: &textures::DepthTextureNonComparisonSampler,
    ) -> Self {
        let shader = shaders::r_depth_debug(&device);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Depth Pass Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
            ],
        });

        let bind_group = Self::make_bind_group(device, &layout, texture);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Pass VB"),
            contents: bytemuck::cast_slice(DEPTH_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Depth Pass IB"),
            contents: bytemuck::cast_slice(DEPTH_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Depth Pass Pipeline Layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Depth Pass Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: shader.vertex_state(shaders::VertexStatePartial {
                buffers: &[ShapeVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            fragment: shader.fragment_state(shaders::FragmentStatePartial {
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format.add_srgb_suffix(),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            cache: None,
        });

        Self {
            layout,
            bind_group,
            vertex_buffer,
            index_buffer,
            depth_index_count: DEPTH_INDICES.len() as u32,
            render_pipeline,
        }
    }

    fn update_bind_group(
        &mut self,
        device: &wgpu::Device,
        texture: &textures::DepthTextureNonComparisonSampler,
    ) {
        self.bind_group = Self::make_bind_group(device, &self.layout, &texture);
    }

    fn make_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &textures::DepthTextureNonComparisonSampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
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
            label: Some("depth_pass.bind_group"),
        })
    }

    #[allow(unused)]
    fn draw(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Depth Visual Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.depth_index_count, 0, 0..1);
    }
}
