use std::sync::Arc;

use crate::{
    camera_controller::CameraController,
    drawing::{
        systems::{
            camera_system::CameraSystem,
            canvas_system::CanvasSystem,
            depth_system::DepthSystem,
            light_system::LightSystem,
            model_system::{
                ModelEntryLightSourceIndicator, ModelEntrySimple, ModelSystem,
                instances_providers::demo_simple_instances_provider::DemoSimpleInstancesProvider,
            },
            skybox_system::SkyboxSystem,
        },
        textures,
    },
    embedded_demo_resources,
    io::{
        fs_accessors::{FsAccessor, embed_fs_accessor::EmbedFsAccessor},
        window_handling::{Input, PhysicalKey, SimpleApplicationEventHandler},
    },
    model_loaders::{ModelLoader, obj_loader::ObjLoader, pmx_loader::PmxLoader},
    utils,
};

pub struct App {
    request_redraw: Box<dyn Fn() + 'static>,

    device: wgpu::Device,
    queue: wgpu::Queue,

    canvas_sys: CanvasSystem,
    camera_sys: CameraSystem,
    model_sys: ModelSystem,
    depth_sys: DepthSystem,
    light_sys: LightSystem,
    skybox_sys: SkyboxSystem,

    camera_controller: CameraController,

    update_time_ms: u64,
}

impl App {
    pub async fn try_new_as_boxed_handler(
        surface_target: wgpu::SurfaceTarget<'static>,
        request_redraw: Box<dyn Fn() + 'static>,
        size: glam::UVec2,
    ) -> anyhow::Result<Box<dyn SimpleApplicationEventHandler>> {
        Ok(Box::new(
            Self::try_new(surface_target, request_redraw, size).await?,
        ))
    }

    pub async fn try_new(
        surface_target: wgpu::SurfaceTarget<'static>,
        request_redraw: Box<dyn Fn() + 'static>,
        size: glam::UVec2,
    ) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(surface_target).unwrap();

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

        let canvas_sys = CanvasSystem::new(surface, &adapter, &device, size);

        let texture_bind_group_layout =
            textures::make_regular_d2_texture_bind_group_layout(&device);

        let camera_sys = CameraSystem::new(&device, size);
        let camera_controller = CameraController::new(4.0, 0.4);

        let light_sys = LightSystem::new(&device);

        let sky_res_loader = EmbedFsAccessor::<embedded_demo_resources::ResSky>::new("sky");
        let sky_bytes = sky_res_loader.load_binary("pure-sky.hdr")?;
        let cube_texture_factory = textures::CubeTextureFactory::new(&device);
        let sky_texture = cube_texture_factory
            .try_make_cube_texture_from_equirectangular_hdr_image_in_memory(
                &device,
                &queue,
                &sky_bytes,
                1080,
                "Sky Texture",
            )?;

        let skybox_sys = SkyboxSystem::new(&device, sky_texture, &canvas_sys, &camera_sys);

        let depth_sys = DepthSystem::new(&device, canvas_sys.surface_config());

        let mut model_sys = ModelSystem::new(
            &device,
            canvas_sys.canvas_color_format(),
            &texture_bind_group_layout,
            &camera_sys,
            &light_sys,
            &skybox_sys,
        );

        let cube_model = {
            let obj_res_loader = EmbedFsAccessor::<embedded_demo_resources::ResCube>::new("cube");
            let obj_model_loader = ObjLoader::new(obj_res_loader);
            obj_model_loader.load_model("cube.obj", &device, &queue, &texture_bind_group_layout)?
        };
        let cube_model = Arc::new(cube_model);

        let obj_model = if true {
            cube_model.clone()
        } else {
            let obj_res_loader = EmbedFsAccessor::<embedded_demo_resources::ResAoi>::new("aoi");
            let obj_model_loader = PmxLoader::new(obj_res_loader);
            let obj_model = obj_model_loader.load_model(
                "A.I.VOICE_琴葉葵_ver1.02.pmx",
                &device,
                &queue,
                &texture_bind_group_layout,
            )?;
            Arc::new(obj_model)
        };

        const NUM_INSTANCES_PER_ROW: usize = 10;
        let instance_provider = DemoSimpleInstancesProvider::new(NUM_INSTANCES_PER_ROW);
        model_sys.push_model_entry_simple(ModelEntrySimple::new(
            &device,
            obj_model,
            Box::new(instance_provider),
        ));

        model_sys.set_model_entry_light_source_indicator(ModelEntryLightSourceIndicator::new(
            cube_model,
        ));

        Ok(Self {
            request_redraw,

            device,
            queue,

            canvas_sys,
            camera_sys,
            model_sys,
            depth_sys,
            light_sys,
            skybox_sys,

            camera_controller,

            update_time_ms: utils::now_ms(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas_sys.resize(&self.device, width, height);
            self.camera_sys
                .entry_mut()
                .resize(&self.queue, width, height);

            self.depth_sys.resize(&self.device, width, height);
        }
    }

    pub fn update(&mut self) {
        let last_update_time_ms = core::mem::replace(&mut self.update_time_ms, utils::now_ms());
        let time_delta_ms = self.update_time_ms - last_update_time_ms;
        let time_delta_s = time_delta_ms as f32 / 1000.0;

        self.camera_sys
            .entry_mut()
            .update_camera(&self.queue, |camera_data| {
                self.camera_controller
                    .update_camera(camera_data, time_delta_s);
            });

        self.light_sys.update(&self.queue, time_delta_s);

        self.model_sys
            .update(&self.device, &self.queue, self.update_time_ms);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        (self.request_redraw)();

        if !self.canvas_sys.is_ready() {
            return Ok(());
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.canvas_sys.canvas_view(),
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
                    view: &self.depth_sys.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.model_sys.draw(
                &mut render_pass,
                &self.camera_sys,
                &self.light_sys,
                &self.skybox_sys,
            );

            self.skybox_sys.draw(&mut render_pass, &self.camera_sys);
        }

        self.canvas_sys.try_do_render_pass_and_present(
            &self.queue,
            encoder,
            #[allow(unused)]
            |encoder, surface_view| {
                // self.depth_sys.debug_draw(&surface_view, encoder);
            },
        )?;

        Ok(())
    }
}

impl SimpleApplicationEventHandler for App {
    fn handle_input(&mut self, input: Input) -> bool {
        match input {
            Input::MouseMotion { delta } => {
                self.camera_controller.handle_mouse(delta.0, delta.1);
                true
            }
            Input::KeyboardInput {
                physical_key: PhysicalKey::Code(key),
                state,
            } => self.camera_controller.process_keyboard(key, state),
            Input::MouseWheel { delta } => {
                self.camera_controller.handle_mouse_scroll(&delta);
                true
            }
            Input::MouseInput { button, state } => {
                self.camera_controller.handle_mouse_input(button, state);
                true
            }
            _ => false,
        }
    }

    fn handle_resized(&mut self, (width, height): (u32, u32)) {
        self.resize(width, height);
    }

    fn handle_redraw_requested(
        &mut self,
        get_window_size: Option<Box<dyn FnOnce() -> (u32, u32)>>,
    ) {
        self.update();

        match self.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let Some(get_window_size) = get_window_size else {
                    todo!();
                };

                let size = (get_window_size)();
                self.resize(size.0, size.1);
            }
            Err(e) => {
                log::error!("Unable to render {}", e)
            }
        }
    }
}
