use ab3de_engine::CameraData;
use ab3de_internal_shared::{
    camera_controller::{CameraController, CameraControllerInput},
    inputting,
};

pub struct AppUi;

impl AppUi {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        type_map: &mut type_map::concurrent::TypeMap,
        proxy: Box<dyn EngineViewportProxy + Send + Sync>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./copying.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("[AppUi::new] bind group layout"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("[AppUi::new] sampler"),
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[AppUi::new] pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("[AppUi::new] render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        type_map.insert::<RenderResource>(RenderResource {
            device: device.clone(),
            bind_group_layout,
            pipeline,
            sampler,
            proxy,
            size: None,
        });

        Self
    }

    pub fn show(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            MainViewport::default().ui(ui);
            ctx.request_repaint();
        });
    }
}

#[derive(Default)]
struct MainViewport;

#[derive(Clone)]
struct MainViewportData {
    camera_controller: CameraController,
    camera_data: CameraData,

    last_frame_is_shift_down: bool,
}

impl MainViewport {
    fn ui(&self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let (rect, _response) =
            ui.allocate_exact_size(rect.size(), egui::Sense::focusable_noninteractive());

        struct InputStuff {
            inputs: Vec<CameraControllerInput>,
            is_shift_down: bool,
        }

        let (dt_s, stuff) = ui.input(|i| {
            let dt_s = i.stable_dt;

            if !i.focused {
                return (dt_s, None);
            }

            let mut inputs: Vec<CameraControllerInput> = vec![];

            if i.pointer.primary_pressed() {
                inputs.push(CameraControllerInput::MouseInput {
                    button: inputting::MouseButton::Left,
                    state: inputting::ElementState::Pressed,
                });
            } else if i.pointer.primary_released() {
                inputs.push(CameraControllerInput::MouseInput {
                    button: inputting::MouseButton::Left,
                    state: inputting::ElementState::Released,
                });
            }

            {
                let drag_delta = i.pointer.delta();
                if drag_delta.x != 0.0 || drag_delta.y != 0.0 {
                    inputs.push(CameraControllerInput::MouseMotion {
                        delta: (drag_delta.x as f64, drag_delta.y as f64),
                    });
                }
            }

            if i.raw_scroll_delta.x != 0.0 || i.raw_scroll_delta.y != 0.0 {
                inputs.push(CameraControllerInput::MouseWheel {
                    delta: inputting::MouseScrollDelta::PixelDelta((
                        i.raw_scroll_delta.x as f64,
                        i.raw_scroll_delta.y as f64,
                    )),
                });
            }

            for ev in &i.events {
                if let egui::Event::Key {
                    physical_key: Some(physical_key),
                    pressed,
                    ..
                } = ev
                {
                    inputs.push(CameraControllerInput::KeyboardInput {
                        physical_key: inputting::PhysicalKey::Code(key_code_from_egui(
                            *physical_key,
                        )),
                        state: if *pressed {
                            inputting::ElementState::Pressed
                        } else {
                            inputting::ElementState::Released
                        },
                    });
                }
            }

            (
                dt_s,
                Some(InputStuff {
                    inputs,
                    is_shift_down: i.modifiers.shift,
                }),
            )
        });

        let camera_data = ui.memory_mut(|m| {
            let data = m
                .data
                .get_temp_mut_or_insert_with(ui.id(), || MainViewportData {
                    camera_controller: CameraController::default(),
                    camera_data: CameraData::default(),
                    last_frame_is_shift_down: false,
                });

            if let Some(mut stuff) = stuff {
                if data.last_frame_is_shift_down != stuff.is_shift_down {
                    let state = if stuff.is_shift_down {
                        inputting::ElementState::Pressed
                    } else {
                        inputting::ElementState::Released
                    };
                    stuff.inputs.push(CameraControllerInput::KeyboardInput {
                        physical_key: inputting::PhysicalKey::Code(inputting::KeyCode::ShiftLeft),
                        state,
                    });
                    data.last_frame_is_shift_down = stuff.is_shift_down;
                }

                for input in stuff.inputs {
                    data.camera_controller.handle_input(input);
                }
            }

            data.camera_controller
                .update_camera(&mut data.camera_data, dt_s);
            data.camera_data.clone()
        });

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            MainViewportCallback { camera_data },
        ));
    }
}

struct MainViewportCallback {
    camera_data: CameraData,
}

struct RenderResource {
    device: wgpu::Device,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,

    proxy: Box<dyn EngineViewportProxy + Send + Sync>,

    size: Option<glam::UVec2>,
}

impl egui_wgpu::CallbackTrait for MainViewportCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let res: &mut RenderResource = callback_resources.get_mut().unwrap();

        let size = screen_descriptor.size_in_pixels.into();
        let new_size = if let Some(size) = res.size {
            res.size = Some(size);
            None
        } else {
            Some(size)
        };
        res.proxy
            .request_update_and_render(new_size, self.camera_data.clone());

        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let res: &RenderResource = callback_resources.get().unwrap();

        let Some(last_view) = res.proxy.last_view() else {
            return;
        };

        let bind_group = res.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[MainViewportCallback::paint] bind group"),
            layout: &res.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&last_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&res.sampler),
                },
            ],
        });

        render_pass.set_pipeline(&res.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

pub trait EngineViewportProxy {
    fn request_update_and_render(&self, new_size: Option<glam::UVec2>, camera_data: CameraData);
    fn last_view(&self) -> Option<wgpu::TextureView>;
}

fn key_code_from_egui(key: egui::Key) -> inputting::KeyCode {
    match key {
        egui::Key::ArrowUp => inputting::KeyCode::ArrowUp,
        egui::Key::ArrowDown => inputting::KeyCode::ArrowDown,
        egui::Key::ArrowLeft => inputting::KeyCode::ArrowLeft,
        egui::Key::ArrowRight => inputting::KeyCode::ArrowRight,
        egui::Key::W => inputting::KeyCode::KeyW,
        egui::Key::S => inputting::KeyCode::KeyS,
        egui::Key::A => inputting::KeyCode::KeyA,
        egui::Key::D => inputting::KeyCode::KeyD,
        egui::Key::Space => inputting::KeyCode::Space,
        _ => inputting::KeyCode::Other,
    }
}
