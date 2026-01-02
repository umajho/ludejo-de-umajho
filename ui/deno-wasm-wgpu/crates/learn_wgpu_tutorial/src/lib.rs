#![feature(cfg_select)]
#![feature(decl_macro)]

mod camera_controller;
mod drawing;
mod resources;
mod utils;

use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::event::{DeviceEvent, ElementState};
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::drawing::systems::camera_system::CameraSystem;
use crate::drawing::systems::canvas_system::CanvasSystem;
use crate::drawing::systems::depth_system::DepthSystem;
use crate::drawing::systems::light_system::LightSystem;
use crate::drawing::systems::model_system::instances_providers::demo_simple_instances_provider::DemoSimpleInstancesProvider;
use crate::drawing::systems::model_system::{
    ModelEntryLightSourceIndicator, ModelEntrySimple, ModelSystem,
};
use crate::drawing::systems::skybox_system::SkyboxSystem;
use crate::drawing::textures;
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
    device: wgpu::Device,
    queue: wgpu::Queue,

    canvas_sys: CanvasSystem,
    camera_sys: CameraSystem,
    model_sys: ModelSystem,
    depth_sys: DepthSystem,
    light_sys: LightSystem,
    skybox_sys: SkyboxSystem,

    camera_controller: camera_controller::CameraController,

    window: Arc<Window>,

    update_time_ms: u64,
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

        let canvas_sys =
            CanvasSystem::new(surface, &adapter, &device, (size.width, size.height).into());

        let texture_bind_group_layout =
            textures::make_regular_d2_texture_bind_group_layout(&device);

        let camera_sys = CameraSystem::new(&device, (size.width, size.height).into());
        let camera_controller = camera_controller::CameraController::new(4.0, 0.4);

        let light_sys = LightSystem::new(&device);

        let sky_res_loader = resources::EmbedResLoader::<resources::ResSky>::new("sky");
        let sky_bytes = sky_res_loader.load_binary("pure-sky.hdr")?;
        let cube_texture_factory = drawing::textures::CubeTextureFactory::new(&device);
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
            let obj_res_loader = resources::EmbedResLoader::<resources::ResCube>::new("cube");
            let obj_model_loader = resources::ObjLoader::new(obj_res_loader);
            obj_model_loader.load_model("cube.obj", &device, &queue, &texture_bind_group_layout)?
        };
        let cube_model = Arc::new(cube_model);

        let obj_model = if true {
            cube_model.clone()
        } else {
            let obj_res_loader = resources::EmbedResLoader::<resources::ResAoi>::new("aoi");
            let obj_model_loader = resources::PmxLoader::new(obj_res_loader);
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

            window,
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
        self.window.request_redraw();

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
            WindowEvent::MouseInput { button, state, .. } => {
                self.camera_controller.handle_mouse_input(button, state);
                true
            }
            _ => false,
        }
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
                state.camera_controller.handle_mouse(delta.0, delta.1);
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
