use wgpu::util::DeviceExt;

pub struct LightSystem {
    entry_demo: LightEntryDemo,
    bind_group_layout_demo: wgpu::BindGroupLayout,
}

impl LightSystem {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout_demo =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("[LightSystem::new] bind group layout"),
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
            });

        let entry_demo = LightEntryDemo::new(device, &bind_group_layout_demo);

        Self {
            entry_demo,
            bind_group_layout_demo,
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout_demo
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt_s: f32) {
        self.entry_demo.update(queue, dt_s);
    }

    pub fn entry_demo(&self) -> &LightEntryDemo {
        &self.entry_demo
    }
}

pub struct LightEntryDemo {
    uniform: LightUniform,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl LightEntryDemo {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let uniform = LightUniform {
            position: glam::vec3(2.0, 2.0, 2.0),
            _padding: 0,
            color: glam::vec3(1.0, 1.0, 1.0),
            _padding2: 0,
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("[LightEntryDemo::new] uniform buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[LightEntryDemo::new] bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            buffer,
            bind_group,
        }
    }

    fn update(&mut self, queue: &wgpu::Queue, dt_s: f32) {
        let old_position = self.uniform.position;
        self.uniform.position =
            (glam::Quat::from_axis_angle(glam::Vec3::Y, dt_s * std::f32::consts::TAU)
                * old_position)
                .into();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: glam::Vec3,
    _padding: u32,
    color: glam::Vec3,
    _padding2: u32,
}
