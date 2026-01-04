use wgpu::util::DeviceExt;

pub struct CameraSystem {
    entry: CameraEntry,

    bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraSystem {
    pub fn new(device: &wgpu::Device, size: glam::UVec2) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("[CameraSystem::new] bind group layout for camera"),
        });

        let entry = CameraEntry::new(device, size, &bind_group_layout);

        Self {
            entry,
            bind_group_layout,
        }
    }

    pub fn entry(&self) -> &CameraEntry {
        &self.entry
    }

    pub fn entry_mut(&mut self) -> &mut CameraEntry {
        &mut self.entry
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

// camera_bind_group: wgpu::BindGroup,

pub struct CameraEntry {
    camera: Camera,
    projection: Projection,

    uniform: CameraUniform,
    uniform_buffer: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
}

impl CameraEntry {
    fn new(
        device: &wgpu::Device,
        size: glam::UVec2,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let camera = Camera::new(CameraData {
            position: (0.0, 5.0, 10.0).into(),
            yaw_radians: -90.0_f32.to_radians(),
            pitch_radians: -20.0_f32.to_radians(),
        });
        let projection = Projection::new(size, 45.0_f32.to_radians(), 0.1, 100.0);

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera, &projection);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("[CameraSystem::new] buffer for camera uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("[CameraSystem::new] bind group for camera uniform"),
        });

        Self {
            camera,
            projection,
            uniform,
            uniform_buffer,
            bind_group,
        }
    }

    // pub fn camera(&self) -> &Camera {
    //     &self.camera
    // }

    pub fn update_camera(&mut self, queue: &wgpu::Queue, f: impl FnOnce(&mut CameraData)) {
        self.camera.update(f);
        self.update_uniform(queue);
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.projection.resize(width, height);
        self.update_uniform(queue);
    }

    // pub fn uniform_buffer(&self) -> &wgpu::Buffer {
    //     &self.uniform_buffer
    // }

    fn update_uniform(&mut self, queue: &wgpu::Queue) {
        self.uniform
            .update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(&self.uniform_buffer, 0, self.uniform.as_bytes())
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct Camera {
    data: CameraData,
    matrix: glam::Mat4,
}

impl Camera {
    fn new(data: CameraData) -> Self {
        let matrix = Self::calc_matrix(&data);

        Self { data, matrix }
    }

    pub fn position(&self) -> &glam::Vec3 {
        &self.data.position
    }

    // pub fn yaw_radians(&self) -> f32 {
    //     self.data.yaw_radians
    // }

    // pub fn pitch_radians(&self) -> f32 {
    //     self.data.pitch_radians
    // }

    fn update(&mut self, f: impl FnOnce(&mut CameraData)) {
        f(&mut self.data);
        self.matrix = Self::calc_matrix(&self.data);
    }

    fn matrix(&self) -> &glam::Mat4 {
        &self.matrix
    }

    fn calc_matrix(data: &CameraData) -> glam::Mat4 {
        let (sin_pitch, cos_pitch) = data.pitch_radians.sin_cos();
        let (sin_yaw, cos_yaw) = data.yaw_radians.sin_cos();

        glam::Mat4::look_to_rh(
            data.position,
            glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            glam::Vec3::Y,
        )
    }
}

#[derive(Debug)]
pub struct CameraData {
    pub position: glam::Vec3,
    pub yaw_radians: f32,
    pub pitch_radians: f32,
}

struct Projection {
    aspect_ratio: f32,
    fov_y_radians: f32,
    z_near: f32,
    z_far: f32,

    matrix: glam::Mat4,
}

impl Projection {
    fn new(size: glam::UVec2, fov_y_radians: f32, z_near: f32, z_far: f32) -> Self {
        let aspect_ratio = size.x as f32 / size.y as f32;

        Self {
            aspect_ratio,
            fov_y_radians,
            z_near,
            z_far,
            matrix: Self::calc_matrix(aspect_ratio, fov_y_radians, z_near, z_far),
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.aspect_ratio = width as f32 / height as f32;
        self.matrix = Self::calc_matrix(
            self.aspect_ratio,
            self.fov_y_radians,
            self.z_near,
            self.z_far,
        );
    }

    fn matrix(&self) -> &glam::Mat4 {
        &self.matrix
    }

    fn calc_matrix(aspect_ratio: f32, fov_y_radians: f32, z_near: f32, z_far: f32) -> glam::Mat4 {
        glam::Mat4::perspective_rh(fov_y_radians, aspect_ratio, z_near, z_far)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: glam::Vec4,
    view: glam::Mat4,
    view_proj: glam::Mat4,
    inv_proj: glam::Mat4,
    inv_view: glam::Mat4,
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: glam::Vec4::ZERO,
            view: glam::Mat4::IDENTITY,
            view_proj: glam::Mat4::IDENTITY,
            inv_proj: glam::Mat4::IDENTITY,
            inv_view: glam::Mat4::IDENTITY,
        }
    }

    fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position().extend(1.0);
        let proj = projection.matrix();
        let view = camera.matrix();
        let view_proj = proj * view;
        self.view = view.clone();
        self.view_proj = view_proj;
        self.inv_proj = proj.inverse();
        self.inv_view = view.transpose();
    }

    fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(std::slice::from_ref(self))
    }
}
