use std::{ops::Range, sync::Arc};

use wgpu::util::DeviceExt;

use crate::drawing::{
    models::{Material, Mesh, Model, ModelVertex},
    shaders,
    systems::{
        camera_system::CameraSystem, light_system::LightSystem, skybox_system::SkyboxSystem,
    },
    textures,
    utils::make_render_pipeline,
};

pub struct ModelSystem {
    entries_simple: Vec<ModelEntrySimple>,
    pipeline_simple: wgpu::RenderPipeline,

    entry_light_source_indicator: Option<ModelEntryLightSourceIndicator>,
    pipeline_light_source_indicator: wgpu::RenderPipeline,
}

impl ModelSystem {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
        skybox_sys: &SkyboxSystem,
    ) -> Self {
        let pipeline_simple = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    texture_bind_group_layout,
                    camera_sys.bind_group_layout(),
                    light_sys.bind_group_layout(),
                    skybox_sys.environment_bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });
            make_render_pipeline(
                "main",
                device,
                &layout,
                color_format,
                Some(textures::DEPTH_FORMAT),
                &[ModelVertex::desc(), SimpleInstanceData::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                &shaders::r_model_demo(device),
            )
        };

        let pipeline_light_source_indicator = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[
                    camera_sys.bind_group_layout(),
                    light_sys.bind_group_layout(),
                ],
                push_constant_ranges: &[],
            });
            make_render_pipeline(
                "light",
                device,
                &layout,
                color_format,
                Some(textures::DEPTH_FORMAT),
                &[ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                &shaders::r_model_light_source_indicator(device),
            )
        };

        Self {
            entries_simple: vec![],
            pipeline_simple,
            entry_light_source_indicator: None,
            pipeline_light_source_indicator,
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, now_ms: u64) {
        for entry in &mut self.entries_simple {
            entry.update(device, queue, now_ms);
        }
    }

    pub fn draw(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'_>,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
        skybox_sys: &SkyboxSystem,
    ) {
        for entry in &mut self.entries_simple {
            entry.draw(
                render_pass,
                &self.pipeline_simple,
                camera_sys,
                light_sys,
                skybox_sys,
            );
        }

        if let Some(entry) = &mut self.entry_light_source_indicator {
            entry.draw(
                render_pass,
                &self.pipeline_light_source_indicator,
                camera_sys,
                light_sys,
            );
        }
    }

    pub fn push_model_entry_simple(&mut self, model_entry: ModelEntrySimple) {
        self.entries_simple.push(model_entry);
    }

    pub fn set_model_entry_light_source_indicator(
        &mut self,
        model_entry: ModelEntryLightSourceIndicator,
    ) {
        self.entry_light_source_indicator = Some(model_entry);
    }
}

pub struct ModelEntrySimple {
    model: Arc<Model>,
    instances_provider: Box<dyn SimpleInstancesProvider>,
    instance_buffer: wgpu::Buffer,
}

impl ModelEntrySimple {
    pub fn new(
        device: &wgpu::Device,
        model: Arc<Model>,
        mut instances_provider: Box<dyn SimpleInstancesProvider>,
    ) -> Self {
        instances_provider.update(0);

        let instances_data_slice = instances_provider.instance_data_slice();
        let instance_data_bytes = bytemuck::cast_slice(instances_data_slice);
        let instance_buffer = Self::make_instance_buffer(device, instance_data_bytes);

        Self {
            model,
            instances_provider,
            instance_buffer,
        }
    }

    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, now_ms: u64) {
        self.instances_provider.update(now_ms);

        let instances_data_slice = self.instances_provider.instance_data_slice();
        let instance_data_bytes = bytemuck::cast_slice(instances_data_slice);

        if self.instance_buffer.size() != instance_data_bytes.len() as wgpu::BufferAddress {
            self.instance_buffer.destroy();
            self.instance_buffer = Self::make_instance_buffer(device, instance_data_bytes);
        } else {
            queue.write_buffer(&self.instance_buffer, 0, instance_data_bytes);
        }
    }

    fn draw(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
        skybox_sys: &SkyboxSystem,
    ) {
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_pipeline(pipeline);

        for mesh in self.model.meshes().iter() {
            let material = &self.model.materials()[mesh.material_index()];

            Self::draw_mesh_instanced(
                render_pass,
                mesh,
                material,
                0..self.instances_provider.instance_count() as u32,
                camera_sys,
                light_sys,
                skybox_sys,
            );
        }
    }

    fn draw_mesh_instanced(
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: &Mesh,
        material: &Material,
        instance_range: Range<u32>,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
        skybox_sys: &SkyboxSystem,
    ) {
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
        render_pass.set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, material.bind_group(), &[]);
        render_pass.set_bind_group(1, camera_sys.entry().bind_group(), &[]);
        render_pass.set_bind_group(2, light_sys.entry_demo().bind_group(), &[]);
        render_pass.set_bind_group(3, skybox_sys.environment_bind_group(), &[]);
        render_pass.draw_indexed(0..mesh.index_count(), 0, instance_range);
    }

    fn make_instance_buffer(device: &wgpu::Device, instance_data_bytes: &[u8]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: instance_data_bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }
}

impl Drop for ModelEntrySimple {
    fn drop(&mut self) {
        self.instance_buffer.destroy();
    }
}

pub struct ModelEntryLightSourceIndicator {
    /// ## TODO
    ///
    /// instead of using [`Mesh`], use a specialized lightweight mesh type that
    /// only carries positions and indices.
    mesh: Arc<Mesh>,
}

impl ModelEntryLightSourceIndicator {
    pub fn new(mesh: Arc<Mesh>) -> Self {
        Self { mesh }
    }

    fn draw(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
    ) {
        render_pass.set_pipeline(pipeline);

        Self::draw_light_mesh(render_pass, &self.mesh, camera_sys, light_sys);
    }

    fn draw_light_mesh(
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: &Mesh,
        camera_sys: &CameraSystem,
        light_sys: &LightSystem,
    ) {
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer().slice(..));
        render_pass.set_index_buffer(mesh.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, camera_sys.entry().bind_group(), &[]);
        render_pass.set_bind_group(1, light_sys.entry_demo().bind_group(), &[]);
        render_pass.draw_indexed(0..mesh.index_count(), 0, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimpleInstanceData {
    model: glam::Mat4,
    normal: glam::Mat3,
    scale: glam::Vec3,
}

impl SimpleInstanceData {
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
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub trait SimpleInstancesProvider {
    fn update(&mut self, now_ms: u64);
    fn instance_data_slice(&self) -> &[SimpleInstanceData];
    fn instance_count(&self) -> usize;
}

pub mod instances_providers {
    pub mod demo_simple_instances_provider {
        use std::{ops::Range, vec};

        use crate::drawing::systems::model_system::{SimpleInstanceData, SimpleInstancesProvider};

        struct Instance {
            position: glam::Vec3,
            rotation: glam::Quat,
            scale: glam::Vec3,
        }

        impl From<&Instance> for super::super::SimpleInstanceData {
            fn from(value: &Instance) -> Self {
                Self {
                    model: glam::Mat4::from_translation(value.position)
                        * glam::Mat4::from_quat(value.rotation.normalize()),
                    normal: glam::Mat3::from_quat(value.rotation.normalize()),
                    scale: value.scale,
                }
            }
        }

        pub struct DemoSimpleInstancesProvider {
            instances: Vec<Instance>,
            instance_data_vec: Vec<super::super::SimpleInstanceData>,

            instances_per_row: usize,
            instance_displacement: glam::Vec3,
        }

        impl DemoSimpleInstancesProvider {
            pub fn new(instances_per_row: usize) -> Self {
                let scale = glam::vec3(1.0, 1.0, 1.0);

                let instance_displacement = glam::vec3(
                    instances_per_row as f32 * 0.5,
                    0.0,
                    instances_per_row as f32 * 0.5,
                );

                let instances = (0..instances_per_row)
                    .flat_map(|_z| {
                        (0..instances_per_row).map(move |_x| Instance {
                            position: glam::Vec3::ZERO,
                            rotation: glam::Quat::from_rotation_z(0.0),
                            scale,
                        })
                    })
                    .collect::<Vec<_>>();

                let mut v = Self {
                    instances,
                    instance_data_vec: vec![], // will be filled in update.
                    instances_per_row,
                    instance_displacement,
                };

                v.update(0);
                v
            }

            fn instances_to_data_vec(instances: &[Instance]) -> Vec<SimpleInstanceData> {
                instances.iter().map(SimpleInstanceData::from).collect()
            }
        }

        impl SimpleInstancesProvider for DemoSimpleInstancesProvider {
            fn update(&mut self, now_ms: u64) {
                const TRANSLATE_Y_AMPLITUDE: f32 = 0.5;
                const SCALE_AMPLITUDE_RANGE: Range<f32> = 0.6..1.2;

                const GLOBAL_SCALE_CONTROL: f32 = 0.8;
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

                        let position = glam::vec3(
                            x,
                            TRANSLATE_Y_AMPLITUDE
                                * (final_progress_1 * std::f32::consts::TAU).sin(),
                            z,
                        ) - self.instance_displacement;
                        let rotation =
                            glam::Quat::from_rotation_y(final_progress_2 * std::f32::consts::TAU)
                                * glam::Quat::from_rotation_x(
                                    final_progress_3 * std::f32::consts::TAU,
                                );

                        let scale = SCALE_AMPLITUDE_RANGE.start
                            + (SCALE_AMPLITUDE_RANGE.end - SCALE_AMPLITUDE_RANGE.start)
                                * (final_progress_4 * std::f32::consts::TAU).sin().abs();

                        instance.position = position;
                        instance.rotation = rotation;
                        instance.scale = glam::vec3(scale, scale, scale) * GLOBAL_SCALE_CONTROL;
                    }
                }

                self.instance_data_vec = Self::instances_to_data_vec(&self.instances);
            }

            fn instance_data_slice(&self) -> &[SimpleInstanceData] {
                &self.instance_data_vec
            }

            fn instance_count(&self) -> usize {
                self.instances.len()
            }
        }
    }
}
