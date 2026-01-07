use ab3de_engine::Engine;
use snafu::ResultExt;

pub fn run() -> Result<(), RunError> {
    let native_options = eframe::NativeOptions::default();

    match eframe::run_native(
        "ab3de",
        native_options,
        Box::new(|cc| Ok(Box::new(App::try_new(cc).context(NewAppSnafu)?))),
    ) {
        Ok(()) => Ok(()),
        Err(e) => Err(RunError::EframeRunNativeError { source: e }),
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum RunError {
    NewAppError { source: NewAppError },
    EframeRunNativeError { source: eframe::Error },
}

struct App {
    app_ui: ab3de_ui::AppUi,
}

impl App {
    fn try_new(cc: &eframe::CreationContext<'_>) -> Result<App, NewAppError> {
        let Some(wgpu_render_state) = &cc.wgpu_render_state else {
            return Err(NewAppError::WgpuNotAvailable);
        };

        let oev = offthread::OffthreadEngineAndViewport::new(
            wgpu_render_state.device.clone(),
            wgpu_render_state.queue.clone(),
            wgpu_render_state.target_format,
        );

        let type_map = &mut wgpu_render_state.renderer.write().callback_resources;

        Ok(Self {
            app_ui: ab3de_ui::AppUi::new(
                &wgpu_render_state.device,
                wgpu_render_state.target_format,
                type_map,
                Box::new(oev),
            ),
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.app_ui.show(ctx);
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum NewAppError {
    WgpuNotAvailable,
}

mod offthread {
    use std::sync::{Arc, Mutex};

    use crate::utils;

    use super::*;

    use ab3de_engine::{CameraData, Viewport, ViewportConfiguration};
    use ab3de_ui::EngineViewportProxy;

    pub struct OffthreadEngineAndViewport {
        target_format: wgpu::TextureFormat,
        command_tx: std::sync::mpsc::SyncSender<Command>,
        textures: Arc<Mutex<Textures>>,
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
            target_format: wgpu::TextureFormat,
        ) -> Self {
            let (command_tx, command_rx) = std::sync::mpsc::sync_channel::<Command>(0);

            let textures = Arc::new(Mutex::new(Textures::new(target_format)));

            std::thread::spawn({
                let textures = textures.clone();
                move || {
                    let mut engine = Engine::try_new(device.clone(), queue.clone()).unwrap();
                    let mut viewport: Option<Viewport> = None;

                    let mut latest_size: Option<glam::UVec2> = None;
                    let mut update_time_ms = utils::now_ms();

                    while let Ok(cmd) = command_rx.recv() {
                        match cmd {
                            Command::UpdateAndRender {
                                new_size,
                                camera_data,
                            } => {
                                if let Some(new_size) = new_size {
                                    latest_size = Some(new_size);
                                }

                                let viewport = match &mut viewport {
                                    Some(viewport) => {
                                        if let Some(new_size) = new_size {
                                            viewport
                                                .resize(&device, &queue, new_size.x, new_size.y);
                                        }
                                        viewport
                                    }
                                    None => {
                                        let new_size = match new_size {
                                            Some(size) => size,
                                            None => continue,
                                        };
                                        viewport =
                                            Some(engine.make_viewport(ViewportConfiguration {
                                                size: new_size,
                                                color_format: target_format,
                                            }));
                                        viewport.as_mut().unwrap()
                                    }
                                };

                                let last_ms =
                                    core::mem::replace(&mut update_time_ms, utils::now_ms());
                                let dt_ms = update_time_ms - last_ms;
                                let dt_s = dt_ms as f32 / 1000.0;

                                viewport.update_camera(&queue, |camera_data_ref| {
                                    *camera_data_ref = camera_data;
                                });

                                engine.update(update_time_ms, dt_s);

                                let current_view = {
                                    let mut textures = textures.lock().unwrap();
                                    textures.next_view(&device, latest_size.unwrap())
                                };

                                engine.render(&viewport, &current_view);

                                {
                                    let mut textures = textures.lock().unwrap();
                                    textures.advance_index();
                                }
                            }
                        }
                    }
                }
            });

            Self {
                target_format,
                command_tx,
                textures,
            }
        }
    }

    impl EngineViewportProxy for OffthreadEngineAndViewport {
        fn request_update_and_render(
            &self,
            new_size: Option<glam::UVec2>,
            camera_data: CameraData,
        ) {
            match self.command_tx.try_send(Command::UpdateAndRender {
                new_size,
                camera_data,
            }) {
                Ok(_) | Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                Err(e) => {
                    panic!("{:?}", e)
                }
            }
        }

        fn last_view(&self) -> Option<wgpu::TextureView> {
            let textures = self.textures.lock().unwrap();
            textures.textures[textures.last_texture_index]
                .as_ref()
                .map(|texture| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        label: Some("[OffthreadEngineAndViewport::last_view] texture view"),
                        format: Some(self.target_format),
                        ..Default::default()
                    })
                })
        }
    }

    struct Textures {
        format: wgpu::TextureFormat,

        textures: Vec<Option<wgpu::Texture>>,
        last_texture_index: usize,
    }

    impl Textures {
        fn new(format: wgpu::TextureFormat) -> Self {
            Self {
                format,
                textures: vec![None, None],
                last_texture_index: 0,
            }
        }

        fn next_index(&self) -> usize {
            (self.last_texture_index + 1) % self.textures.len()
        }

        fn next_view(
            &mut self,
            device: &wgpu::Device,
            latest_size: glam::UVec2,
        ) -> wgpu::TextureView {
            let next_index = self.next_index();

            if self.textures[next_index]
                .as_ref()
                .is_none_or(|t| t.width() != latest_size.x || t.height() != latest_size.y)
            {
                if let Some(t) = &self.textures[next_index] {
                    t.destroy();
                }

                self.textures[next_index] = Some(device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("[Textures::resize] texture"),
                    size: wgpu::Extent3d {
                        width: latest_size.x,
                        height: latest_size.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.format.add_srgb_suffix(),
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[self.format, self.format.add_srgb_suffix()],
                }));
            }

            self.textures[next_index]
                .as_ref()
                .unwrap()
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("[Textures::must_next_view] texture view"),
                    ..Default::default()
                })
        }

        fn advance_index(&mut self) {
            self.last_texture_index = self.next_index();
        }
    }
}
