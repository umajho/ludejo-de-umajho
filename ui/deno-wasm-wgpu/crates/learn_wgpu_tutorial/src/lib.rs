#![feature(cfg_select)]
#![feature(decl_macro)]

mod camera;
mod copied;
mod hdr_tonemapping;
mod models;
mod resources;
mod textures;
mod utils;

use std::ops::Range;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use cgmath::prelude::*;

use wgpu::util::DeviceExt;
use winit::event::{DeviceEvent, ElementState, MouseButton};
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::models::{DrawModel, Model, ModelVertex, Vertex};
use crate::resources::{ModelLoader, ResLoader};

pub fn run() -> anyhow::Result<()> {
    cfg_select! {
        target_arch = "wasm32" => {
            console_log::init_with_level(log::Level::Debug).unwrap_throw();
        }
        _ => {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = cfg_select! {
        target_arch = "wasm32" => { App::new(&event_loop) }
        _ => { App::new() }
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipelines: RenderPipelines,
    hdr: hdr_tonemapping::HdrPipeline,
    environment_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    obj_model: Model,
    camera: camera::Camera,
    projection: camera::Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_controller: camera::CameraController,
    camera_bind_group: wgpu::BindGroup,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    depth_pass: copied::DepthPass,
    instances: Instances,

    mouse_pressed: bool,

    window: Arc<Window>,

    update_time_ms: u64,

    is_challenge: bool,
}

impl State {
    pub async fn try_new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::defaults(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            // present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let hdr = hdr_tonemapping::HdrPipeline::new(&device, &config);

        let sky_res_loader = resources::EmbedResLoader::<resources::ResSky>::new("sky");
        let sky_bytes = sky_res_loader.load_binary("pure-sky.hdr")?;
        let hdr_loader = resources::HdrLoader::new(&device);
        let sky_texture = hdr_loader.from_equirectangular_bytes(
            &device,
            &queue,
            &sky_bytes,
            1080,
            Some("Sky Texture"),
        )?;

        let environment_layout =
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
            layout: &environment_layout,
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
                bind_group_layouts: &[&camera_bind_group_layout, &environment_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::include_wgsl!("sky.wgsl");
            new_render_pipeline(
                "sky",
                &device,
                &layout,
                hdr.format(),
                Some(textures::Texture::DEPTH_FORMAT),
                &[],
                wgpu::PrimitiveTopology::TriangleList,
                &device.create_shader_module(shader),
            )
        };

        let render_pipelines = RenderPipelines::new(
            &device,
            &hdr,
            &texture_bind_group_layout,
            &camera_bind_group_layout,
            &light_bind_group_layout,
            &environment_layout,
        );

        let depth_pass = copied::DepthPass::new(
            &device,
            &config,
            &device.create_shader_module(wgpu::include_wgsl!("depth.wgsl")),
        );

        let obj_res_loader = resources::EmbedResLoader::<resources::ResCube>::new("cube");
        let obj_model_loader = resources::ObjLoader::new(obj_res_loader);
        let obj_model =
            obj_model_loader.load_model("cube.obj", &device, &queue, &texture_bind_group_layout)?;

        // let obj_res_loader = resources::EmbedResLoader::<resources::ResAoi>::new("aoi");
        // let obj_model_loader = resources::PmxLoader::new(obj_res_loader);
        // let obj_model = obj_model_loader.load_model(
        //     "A.I.VOICE_琴葉葵_ver1.02.pmx",
        //     &device,
        //     &queue,
        //     &texture_bind_group_layout,
        // )?;

        const NUM_INSTANCES_PER_ROW: u32 = 10;
        let instances = Instances::new(&device, NUM_INSTANCES_PER_ROW as usize);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipelines,
            hdr,
            environment_bind_group,
            sky_pipeline,
            obj_model,
            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            light_uniform,
            light_buffer,
            light_bind_group,
            depth_pass,
            instances,

            mouse_pressed: false,

            update_time_ms: utils::now_ms(),

            window,

            is_challenge: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            self.projection.resize(width, height);

            self.depth_pass.resize(&self.device, &self.config);

            self.hdr.resize(&self.device, width, height);
        }
    }

    pub fn update(&mut self) {
        let last_update_time_ms = core::mem::replace(&mut self.update_time_ms, utils::now_ms());
        let time_delta_ms = self.update_time_ms - last_update_time_ms;

        self.camera_controller
            .update_camera(&mut self.camera, time_delta_ms as f32 / 1000.0);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        {
            let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
            self.light_uniform.position = (cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_y(),
                cgmath::Rad(time_delta_ms as f32 / 1000.0 * std::f32::consts::TAU),
            ) * old_position)
                .into();
            self.queue.write_buffer(
                &self.light_buffer,
                0,
                bytemuck::cast_slice(&[self.light_uniform]),
            );
        }

        self.instances.update(&self.device, self.update_time_ms);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.hdr.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_pass.texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_vertex_buffer(1, self.instances.instance_buffer.slice(..));

            render_pass.set_pipeline(self.render_pipelines.light());
            render_pass.draw_light_model(
                &self.obj_model,
                &self.camera_bind_group,
                &self.light_bind_group,
            );

            render_pass.set_pipeline(self.render_pipelines.main(self.is_challenge));
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
                &self.light_bind_group,
                &self.environment_bind_group,
            );

            render_pass.set_pipeline(&self.sky_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.environment_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.hdr.process(&mut encoder, &view);

        // self.depth_pass.render(&view, &mut encoder);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.handle_mouse_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }
}

pub struct RenderPipelines {
    main_regular: wgpu::RenderPipeline,

    light: wgpu::RenderPipeline,
}

impl RenderPipelines {
    pub fn new(
        device: &wgpu::Device,
        hdr: &hdr_tonemapping::HdrPipeline,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        environment_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let main_regular_pipeline = {
            let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    texture_bind_group_layout,
                    camera_bind_group_layout,
                    light_bind_group_layout,
                    environment_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            new_render_pipeline(
                "main",
                device,
                &layout,
                hdr.format(),
                Some(textures::Texture::DEPTH_FORMAT),
                &[ModelVertex::desc(), InstanceRaw::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                &shader,
            )
        };

        let light_pipeline = {
            let shader = device.create_shader_module(wgpu::include_wgsl!("light.wgsl"));
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[camera_bind_group_layout, light_bind_group_layout],
                push_constant_ranges: &[],
            });
            new_render_pipeline(
                "light",
                device,
                &layout,
                hdr.format(),
                Some(textures::Texture::DEPTH_FORMAT),
                &[ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                &shader,
            )
        };

        Self {
            main_regular: main_regular_pipeline,
            light: light_pipeline,
        }
    }

    pub fn main(&self, _is_challenge: bool) -> &wgpu::RenderPipeline {
        &self.main_regular
    }

    pub fn light(&self) -> &wgpu::RenderPipeline {
        &self.light
    }
}

pub fn new_render_pipeline(
    name: &str,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    topology: wgpu::PrimitiveTopology,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(format!("Render Pipeline: {}", name).as_str()),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format.add_srgb_suffix(),
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2, // Corresponds to bilinear filtering
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    inv_view: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view: cgmath::Matrix4::identity().into(),
            view_proj: cgmath::Matrix4::identity().into(),
            inv_proj: cgmath::Matrix4::identity().into(),
            inv_view: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        let proj = projection.calc_matrix();
        let view = camera.calc_matrix();
        let view_proj = proj * view;
        self.view = view.into();
        self.view_proj = view_proj.into();
        self.inv_proj = proj.invert().unwrap().into();
        self.inv_view = view.transpose().into();
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    scale: cgmath::Vector3<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
    scale: [f32; 3],
}

impl From<&Instance> for InstanceRaw {
    fn from(value: &Instance) -> Self {
        Self {
            model: (cgmath::Matrix4::from_translation(value.position)
                * cgmath::Matrix4::from(value.rotation))
            .into(),
            normal: cgmath::Matrix3::from(value.rotation).into(),
            scale: value.scale.into(),
        }
    }
}

impl InstanceRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
        5 => Float32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x3,
        10 => Float32x3,
        11 => Float32x3,
        12 => Float32x3,
    ];

    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

struct Instances {
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    instances_per_row: usize,
    instance_displacement: cgmath::Vector3<f32>,
}

impl Instances {
    const GLOBAL_SCALE_CONTROL: f32 = 0.8;

    fn new(device: &wgpu::Device, instances_per_row: usize) -> Self {
        let scale = cgmath::Vector3::new(1.0, 1.0, 1.0);

        let instance_displacement = cgmath::Vector3::new(
            instances_per_row as f32 * 0.5,
            0.0,
            instances_per_row as f32 * 0.5,
        );

        let instances = (0..instances_per_row)
            .flat_map(|_z| {
                (0..instances_per_row).map(move |_x| Instance {
                    position: cgmath::Vector3::zero(),
                    rotation: cgmath::Quaternion::from_axis_angle(
                        cgmath::Vector3::unit_z(),
                        cgmath::Deg(0.0),
                    ),
                    scale,
                })
            })
            .collect::<Vec<_>>();

        let instance_buffer = Self::new_buffer(device, &instances);

        let mut v = Self {
            instances,
            instance_buffer,
            instances_per_row,
            instance_displacement,
        };

        v.update(device, 0);
        v
    }

    fn len(&self) -> usize {
        self.instances.len()
    }

    const TRANSLATE_Y_AMPLITUDE: f32 = 0.5;
    const SCALE_AMPLITUDE_RANGE: Range<f32> = 0.6..1.2;

    fn update(&mut self, device: &wgpu::Device, now_ms: u64) {
        const SPACE_BETWEEN: f32 = 3.0;

        let progress_1 = (now_ms % 2000) as f32 / 2000.0;
        let progress_2 = (now_ms % 3000) as f32 / 3000.0;
        let progress_3 = (now_ms % 5000) as f32 / 5000.0;
        let progress_4 = (now_ms % 7000) as f32 / 7000.0;
        // let progress = 0.0;

        for i in 0..self.instances_per_row {
            for j in 0..self.instances_per_row {
                let x = SPACE_BETWEEN * (i as f32 - self.instances_per_row as f32 / 2.0);
                let z = SPACE_BETWEEN * (j as f32 - self.instances_per_row as f32 / 2.0);

                let local_progress = (i as f32 * self.instances_per_row as f32 + j as f32)
                    / (self.instances_per_row * self.instances_per_row) as f32;
                let final_progress_1 = progress_1 + local_progress;
                let final_progress_2 = progress_2 + local_progress;
                let final_progress_3 = progress_3 + local_progress;
                let final_progress_4 = progress_4 + local_progress;

                let instance = &mut self.instances[i * self.instances_per_row + j];

                let position = cgmath::Vector3 {
                    x,
                    y: Self::TRANSLATE_Y_AMPLITUDE
                        * (final_progress_1 * std::f32::consts::TAU).sin(),
                    z,
                } - self.instance_displacement;
                let rotation = cgmath::Quaternion::from_axis_angle(
                    cgmath::Vector3::unit_y(),
                    cgmath::Rad(final_progress_2 * std::f32::consts::TAU),
                ) * cgmath::Quaternion::from_axis_angle(
                    cgmath::Vector3::unit_x(),
                    cgmath::Rad(final_progress_3 * std::f32::consts::TAU),
                );

                let scale = Self::SCALE_AMPLITUDE_RANGE.start
                    + (Self::SCALE_AMPLITUDE_RANGE.end - Self::SCALE_AMPLITUDE_RANGE.start)
                        * (final_progress_4 * std::f32::consts::TAU).sin().abs();

                instance.position = position;
                instance.rotation = rotation;
                instance.scale =
                    cgmath::Vector3::new(scale, scale, scale) * Self::GLOBAL_SCALE_CONTROL;
            }
        }

        self.instance_buffer = Self::new_buffer(device, &self.instances);
    }

    fn new_buffer(device: &wgpu::Device, instances: &[Instance]) -> wgpu::Buffer {
        let instance_data = instances.iter().map(InstanceRaw::from).collect::<Vec<_>>();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    #[cfg(target_arch = "wasm32")]
    pub fn new(event_loop: &EventLoop<State>) -> Self {
        Self {
            proxy: Some(event_loop.create_proxy()),
            state: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = winit::window::WindowAttributes::default();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        cfg_select! {
          target_arch = "wasm32" => {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::try_new(window)
                                    .await
                                    .expect("Unabled to create canvas!!!")
                            )
                            .is_ok()
                    )
                })
            }
          }
          _ => {
            self.state = Some(pollster::block_on(State::try_new(window)).unwrap());
          }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        match event {
            DeviceEvent::MouseMotion { delta } => {
                if state.mouse_pressed {
                    state.camera_controller.handle_mouse(delta.0, delta.1);
                }
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        if window_id != state.window.id() {
            return;
        }

        if state.input(&event) {
            return;
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();

                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e)
                    }
                }
            }
            _ => {}
        }
    }
}

// lib.rs
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    _padding: u32,
    color: [f32; 3],
    _padding2: u32,
}
