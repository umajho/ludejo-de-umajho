use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Shape {
    // vertices: &'static [Vertex],
    indices: &'static [u16],

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl Shape {
    pub fn new(
        device: &wgpu::Device,
        vertices: &'static [Vertex],
        indices: &'static [u16],
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            // vertices,
            indices,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn num_indices(&self) -> u32 {
        self.indices.len() as u32
    }
}

pub struct Shapes {
    normal: Shape,
}

impl Shapes {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            normal: Shape::new(device, Self::normal().0, Self::normal().1),
        }
    }

    const fn normal() -> (&'static [Vertex], &'static [u16]) {
        let vertices = &[
            // 0: A
            Vertex {
                position: [-0.0868241, 0.49240386, 0.0],
                tex_coords: [0.4131759, 1.0 - 0.99240386],
            },
            // 1: B
            Vertex {
                position: [-0.49513406, 0.06958647, 0.0],
                tex_coords: [0.0048659444, 1.0 - 0.56958647],
            },
            // 2: C
            Vertex {
                position: [-0.21918549, -0.44939706, 0.0],
                tex_coords: [0.28081453, 1.0 - 0.05060294],
            },
            // 3: D
            Vertex {
                position: [0.35966998, -0.3473291, 0.0],
                tex_coords: [0.85967, 1.0 - 0.1526709],
            },
            // 4: E
            Vertex {
                position: [0.44147372, 0.2347359, 0.0],
                tex_coords: [0.9414737, 1.0 - 0.7347359],
            }, // E
        ];

        let indices = &[/**/ 0, 1, 4, /**/ 1, 2, 4, /**/ 2, 3, 4];

        (vertices, indices)
    }

    pub fn get(&self, _is_challenge: bool) -> &Shape {
        &self.normal
    }
}
