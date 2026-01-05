use std::sync::Arc;

use crate::{
    drawing::{
        systems::{
            camera_system::{CameraData, CameraEntry, CameraSystem},
            canvas_system::{CANVAS_COLOR_FORMAT, CanvasEntry},
            depth_system::DepthEntry,
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
    io::fs_accessors::{FsAccessor, embed_fs_accessor::EmbedFsAccessor},
    model_loaders::{
        ModelLoader, obj_loader::ObjLoader, pmx_loader::PmxLoader, virtual_loader::VirtualLoader,
    },
};

pub struct Engine {
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera_sys: CameraSystem,
    model_sys: ModelSystem,
    light_sys: LightSystem,
    skybox_sys: SkyboxSystem,
}

impl Engine {
    pub fn try_new(device: wgpu::Device, queue: wgpu::Queue) -> anyhow::Result<Self> {
        let texture_bind_group_layout = textures::make_regular_d2_texture_bind_group_layout(
            "[Engine::try_new] texture bind group layout",
            &device,
        );

        let camera_sys = CameraSystem::new(&device);

        let light_sys = LightSystem::new(&device);

        let cube_texture_factory = textures::CubeTextureFactory::new(&device);

        let sky_texture = {
            const FILE_NAME: &str = "pure-sky.hdr";

            let sky_res_loader = EmbedFsAccessor::<embedded_demo_resources::ResSky>::new("sky");
            let sky_bytes = sky_res_loader.load_binary(FILE_NAME)?;

            cube_texture_factory.try_make_cube_texture_from_equirectangular_hdr_image_in_memory(
                FILE_NAME, &device, &queue, &sky_bytes, 1080,
            )?
        };

        let skybox_sys = SkyboxSystem::new(&device, sky_texture, &camera_sys);

        let mut model_sys = ModelSystem::new(
            &device,
            CANVAS_COLOR_FORMAT,
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

        let simple_cube_mesh_for_light_source_indicator =
            VirtualLoader::make_cube_mesh_with_minimal_effort_for_light_source_indicators(
                &device, 1.0,
            );

        model_sys.set_model_entry_light_source_indicator(ModelEntryLightSourceIndicator::new(
            Arc::new(simple_cube_mesh_for_light_source_indicator),
        ));

        Ok(Self {
            device,
            queue,

            camera_sys,
            model_sys,
            light_sys,
            skybox_sys,
        })
    }

    pub fn make_viewport(
        &self,
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        size: glam::UVec2,
    ) -> Viewport {
        Viewport::new(surface, adapter, &self.device, size, &self.camera_sys)
    }

    pub fn update(&mut self, now_ms: u64, dt_s: f32) {
        self.light_sys.update(&self.queue, dt_s);
        self.model_sys.update(&self.device, &self.queue, now_ms);
    }

    pub fn render(&mut self, viewport: &Viewport) -> Result<(), wgpu::SurfaceError> {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("[Engine::render] render encoder"),
            });

        viewport.render(&self.queue, encoder, |render_pass, camera_entry| {
            self.model_sys
                .draw(render_pass, camera_entry, &self.light_sys, &self.skybox_sys);

            self.skybox_sys.draw(render_pass, camera_entry);
        })?;

        Ok(())
    }
}

pub struct Viewport {
    canvas_entry: CanvasEntry,
    depth_entry: DepthEntry,
    camera_entry: CameraEntry,
}

impl Viewport {
    fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        size: glam::UVec2,
        camera_sys: &CameraSystem,
    ) -> Self {
        let canvas_entry = CanvasEntry::new(surface, &adapter, &device, size);
        let depth_entry = DepthEntry::new(&device, canvas_entry.surface_config());
        let camera_entry = camera_sys.make_entry(device, size);

        Self {
            canvas_entry,
            depth_entry,
            camera_entry,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas_entry.resize(device, width, height);
            self.camera_entry.resize(queue, width, height);
            self.depth_entry.resize(device, width, height);
        }
    }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, f: impl FnOnce(&mut CameraData)) {
        self.camera_entry.update_camera(queue, f);
    }

    fn render(
        &self,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        draw_fn: impl FnOnce(&mut wgpu::RenderPass, &CameraEntry) -> (),
    ) -> Result<(), wgpu::SurfaceError> {
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("[Viewport::render] render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.canvas_entry.canvas_view(),
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
                    view: &self.depth_entry.view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            draw_fn(&mut render_pass, &self.camera_entry);
        }

        self.canvas_entry.try_do_render_pass_and_present(
            queue,
            encoder,
            #[allow(unused)]
            |encoder, surface_view| {
                // self.depth_sys.debug_draw(&surface_view, encoder);
            },
        )?;

        Ok(())
    }
}
