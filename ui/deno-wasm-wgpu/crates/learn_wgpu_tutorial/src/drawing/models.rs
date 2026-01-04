use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::drawing::textures;

pub struct Model {
    meshes: Arc<Vec<Mesh>>,
    materials: Vec<Material>,
}

impl Model {
    pub fn new(meshes: Vec<Mesh>, materials: Vec<Material>) -> Self {
        Self {
            meshes: Arc::new(meshes),
            materials,
        }
    }

    pub fn meshes(&self) -> Arc<Vec<Mesh>> {
        self.meshes.clone()
    }

    pub fn materials(&self) -> &Vec<Material> {
        &self.materials
    }
}

pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    material_index: usize,
}

impl Mesh {
    pub fn new(
        name: &str,
        device: &wgpu::Device,
        vertices: &[ModelVertex],
        indices: &[u32],
        material_index: usize,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("[Mesh::new] vertex buffer for {}", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("[Mesh::new] index buffer for {}", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            material_index,
        }
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn material_index(&self) -> usize {
        self.material_index
    }
}

pub struct Material {
    bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: textures::D2DiffuseTexture,
        normal_texture: textures::D2NormalTexture,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(diffuse_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(diffuse_texture.sampler()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(normal_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(normal_texture.sampler()),
                },
            ],
            label: Some(&format!("[Material::new] bind group for {}", name)),
        });

        Self { bind_group }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: glam::Vec3,
    pub tex_coords: glam::Vec2,
    pub normal: glam::Vec3,

    pub tangent: glam::Vec3,
    pub bitangent: glam::Vec3,
}

impl ModelVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3, 4 => Float32x3];

    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: glam::Vec3,
    pub tex_coords: glam::Vec2,
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
