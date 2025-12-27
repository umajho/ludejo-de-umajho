use std::ops::Range;

use wgpu::util::DeviceExt;

use crate::textures;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl ShapeVertex {
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
        vertices: &'static [ShapeVertex],
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

    const fn normal() -> (&'static [ShapeVertex], &'static [u16]) {
        let vertices = &[
            // 0: A
            ShapeVertex {
                position: [-0.0868241, 0.49240386, 0.0],
                tex_coords: [0.4131759, 1.0 - 0.99240386],
            },
            // 1: B
            ShapeVertex {
                position: [-0.49513406, 0.06958647, 0.0],
                tex_coords: [0.0048659444, 1.0 - 0.56958647],
            },
            // 2: C
            ShapeVertex {
                position: [-0.21918549, -0.44939706, 0.0],
                tex_coords: [0.28081453, 1.0 - 0.05060294],
            },
            // 3: D
            ShapeVertex {
                position: [0.35966998, -0.3473291, 0.0],
                tex_coords: [0.85967, 1.0 - 0.1526709],
            },
            // 4: E
            ShapeVertex {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl ModelVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: textures::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group);
    }
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group);
        }
    }
}

impl<'a, 'b> DrawModel<'a> for wgpu::RenderPass<'b>
where
    'b: 'a,
{
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
