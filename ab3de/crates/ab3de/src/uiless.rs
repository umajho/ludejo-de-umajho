use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use ab3de_engine::Engine;
use ab3de_internal_shared::camera_controller::{CameraController, CameraControllerInput};

use crate::utils;

pub fn run(
    instance: wgpu::Instance,
    request_adapter_options: wgpu::RequestAdapterOptions<'static, 'static>,
    device_descriptor: wgpu::DeviceDescriptor<'static>,
) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;

    let mut handler = WinitWindowHandler::new(instance, request_adapter_options, device_descriptor);

    event_loop.run_app(&mut handler)?;

    Ok(())
}

pub struct WinitWindowHandler {
    instance: wgpu::Instance,
    request_adapter_options: wgpu::RequestAdapterOptions<'static, 'static>,
    device_descriptor: wgpu::DeviceDescriptor<'static>,

    state: State,

    window: Option<Arc<Window>>,
}

enum State {
    Uninitialized,
    Ready(StateReady),
}

struct StateReady {
    oev: offthread::OffthreadEngineAndViewport,
    camera_controller: CameraController,

    update_time_ms: u64,
    new_size: Option<glam::UVec2>,
    camera_data: ab3de_engine::CameraData,
}

impl WinitWindowHandler {
    pub fn new(
        instance: wgpu::Instance,
        request_adapter_options: wgpu::RequestAdapterOptions<'static, 'static>,
        device_descriptor: wgpu::DeviceDescriptor<'static>,
    ) -> Self {
        Self {
            instance,
            request_adapter_options,
            device_descriptor,
            state: State::Uninitialized,
            window: None,
        }
    }
}

impl ApplicationHandler for WinitWindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let State::Uninitialized = &mut self.state else {
            return;
        };

        let window_attributes = winit::window::WindowAttributes::default();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        let surface = self.instance.create_surface(window.clone()).unwrap();

        let adapter = {
            let adapter_future = self.instance.request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..self.request_adapter_options
            });
            pollster::block_on(adapter_future).unwrap()
        };

        self.device_descriptor.label = Some("[ApplicationHandler::resumed]");
        let (device, queue) = {
            let device_future = adapter.request_device(&self.device_descriptor);
            pollster::block_on(device_future).unwrap()
        };

        let oev =
            offthread::OffthreadEngineAndViewport::new(device, queue, surface, adapter, window);

        self.state = State::Ready(StateReady {
            oev,
            camera_controller: CameraController::default(),
            update_time_ms: utils::now_ms(),
            new_size: None,
            camera_data: ab3de_engine::CameraData::default(),
        });
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let s = match &mut self.state {
            State::Ready(state) => state,
            _ => return,
        };

        let _has_consumed = match event {
            DeviceEvent::MouseMotion { delta } => s
                .camera_controller
                .handle_input(CameraControllerInput::MouseMotion { delta }),
            _ => false,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let s = match &mut self.state {
            State::Ready(state) => state,
            _ => return,
        };

        let Some(window) = &self.window else {
            return;
        };

        if window_id != window.id() {
            return;
        }

        let has_consumed = match &event {
            WindowEvent::KeyboardInput { event, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::KeyboardInput {
                        physical_key: conv::physical_key_from_winit(event.physical_key),
                        state: conv::element_state_from_winit(event.state),
                    })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::MouseWheel {
                        delta: conv::mouse_scroll_delta_from_winit(*delta),
                    })
            }
            WindowEvent::MouseInput { button, state, .. } => {
                s.camera_controller
                    .handle_input(CameraControllerInput::MouseInput {
                        button: conv::mouse_button_from_winit(*button),
                        state: conv::element_state_from_winit(*state),
                    })
            }
            _ => false,
        };
        if has_consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key:
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                s.new_size = Some((size.width, size.height).into());
            }
            WindowEvent::RedrawRequested => {
                let last_ms = core::mem::replace(&mut s.update_time_ms, utils::now_ms());
                let dt_ms = s.update_time_ms - last_ms;
                let dt_s = dt_ms as f32 / 1000.0;

                s.camera_controller.update_camera(&mut s.camera_data, dt_s);

                match s
                    .oev
                    .try_update_and_render(s.new_size, s.camera_data.clone())
                {
                    Some(_) => {
                        s.new_size = None;
                    }
                    None => {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

mod conv {
    use ab3de_internal_shared::inputting::{
        ElementState, KeyCode, MouseButton, MouseScrollDelta, PhysicalKey,
    };

    pub fn physical_key_from_winit(value: winit::keyboard::PhysicalKey) -> PhysicalKey {
        match value {
            winit::keyboard::PhysicalKey::Code(code) => {
                PhysicalKey::Code(key_code_from_winit(code))
            }
            _ => PhysicalKey::Other,
        }
    }

    fn key_code_from_winit(value: winit::keyboard::KeyCode) -> KeyCode {
        match value {
            winit::keyboard::KeyCode::ArrowUp => KeyCode::ArrowUp,
            winit::keyboard::KeyCode::ArrowDown => KeyCode::ArrowDown,
            winit::keyboard::KeyCode::ArrowLeft => KeyCode::ArrowLeft,
            winit::keyboard::KeyCode::ArrowRight => KeyCode::ArrowRight,
            winit::keyboard::KeyCode::KeyW => KeyCode::KeyW,
            winit::keyboard::KeyCode::KeyS => KeyCode::KeyS,
            winit::keyboard::KeyCode::KeyA => KeyCode::KeyA,
            winit::keyboard::KeyCode::KeyD => KeyCode::KeyD,
            winit::keyboard::KeyCode::Space => KeyCode::Space,
            winit::keyboard::KeyCode::ShiftLeft => KeyCode::ShiftLeft,
            _ => KeyCode::Other,
        }
    }

    pub fn mouse_scroll_delta_from_winit(
        value: winit::event::MouseScrollDelta,
    ) -> MouseScrollDelta {
        match value {
            winit::event::MouseScrollDelta::LineDelta(x, y) => MouseScrollDelta::LineDelta(x, y),
            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                MouseScrollDelta::PixelDelta((pos.x, pos.y))
            }
        }
    }

    pub fn mouse_button_from_winit(value: winit::event::MouseButton) -> MouseButton {
        match value {
            winit::event::MouseButton::Left => MouseButton::Left,
            _ => MouseButton::Other,
        }
    }

    pub fn element_state_from_winit(value: winit::event::ElementState) -> ElementState {
        match value {
            winit::event::ElementState::Pressed => ElementState::Pressed,
            winit::event::ElementState::Released => ElementState::Released,
        }
    }
}

mod offthread {
    use super::*;

    use ab3de_engine::{CameraData, ViewportConfiguration};

    pub struct OffthreadEngineAndViewport {
        command_tx: std::sync::mpsc::SyncSender<Command>,
    }

    #[derive(Debug)]
    enum Command {
        UpdateAndRender {
            new_size: Option<glam::UVec2>,
            camera_data: CameraData,
        },
    }

    impl OffthreadEngineAndViewport {
        pub fn new(
            device: wgpu::Device,
            queue: wgpu::Queue,
            surface: wgpu::Surface<'static>,
            adapter: wgpu::Adapter,
            window: Arc<Window>,
        ) -> Self {
            let size = window.inner_size();

            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0]);
            let mut surface_config = wgpu::SurfaceConfiguration {
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

            surface.configure(&device, &surface_config);

            let (command_tx, command_rx) = std::sync::mpsc::sync_channel::<Command>(0);

            std::thread::spawn(move || {
                let mut engine = Engine::try_new(device.clone(), queue.clone()).unwrap();
                let mut viewport = engine.make_viewport(ViewportConfiguration {
                    size: (size.width, size.height).into(),
                    color_format: surface_format,
                });

                let mut update_time_ms = utils::now_ms();

                while let Ok(cmd) = command_rx.recv() {
                    match cmd {
                        Command::UpdateAndRender {
                            new_size,
                            camera_data,
                        } => {
                            if let Some(size) = new_size {
                                surface_config.width = size.x;
                                surface_config.height = size.y;
                                surface.configure(&device, &surface_config);

                                viewport.resize(&device, &queue, size.x, size.y);
                            }

                            let last_ms = core::mem::replace(&mut update_time_ms, utils::now_ms());
                            let dt_ms = update_time_ms - last_ms;
                            let dt_s = dt_ms as f32 / 1000.0;

                            viewport.update_camera(&queue, |camera_data_ref| {
                                *camera_data_ref = camera_data;
                            });

                            engine.update(update_time_ms, dt_s);

                            match surface.get_current_texture() {
                                Ok(output_texture) => {
                                    let output_view = output_texture.texture.create_view(
                                        &wgpu::TextureViewDescriptor {
                                            format: Some(surface_format.add_srgb_suffix()),
                                            ..Default::default()
                                        },
                                    );

                                    engine.render(&viewport, &output_view);
                                    output_texture.present();
                                }
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    let size = window.inner_size();
                                    viewport.resize(&device, &queue, size.width, size.height);
                                }
                                Err(e) => {
                                    log::error!("Unable to render {}", e)
                                }
                            };

                            window.request_redraw();
                        }
                    }
                }
            });

            Self { command_tx }
        }

        pub fn try_update_and_render(
            &self,
            new_size: Option<glam::UVec2>,
            camera_data: CameraData,
        ) -> Option<()> {
            match self.command_tx.try_send(Command::UpdateAndRender {
                new_size,
                camera_data,
            }) {
                Ok(_) => Some(()),
                Err(std::sync::mpsc::TrySendError::Full(_)) => None,
                Err(e) => {
                    panic!("{:?}", e)
                }
            }
        }
    }
}
