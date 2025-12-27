use std::io::{BufReader, Cursor};

use rust_embed::Embed;

use wgpu::util::DeviceExt;

use crate::{
    models::{Material, Mesh, Model, ModelVertex},
    textures,
};

#[derive(Embed)]
#[folder = "res/cube"]
pub struct ResCube;

#[derive(Embed)]
#[folder = "res/aoi"]
pub struct ResAoi;

pub trait ResLoader {
    fn name(&self) -> &str;
    fn load_binary(&self, filename: &str) -> anyhow::Result<Vec<u8>>;
    fn load_string(&self, filename: &str) -> anyhow::Result<String>;
}

pub struct EmbedResLoader<T: rust_embed::RustEmbed> {
    name: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T: rust_embed::RustEmbed> EmbedResLoader<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: rust_embed::RustEmbed> ResLoader for EmbedResLoader<T> {
    fn name(&self) -> &str {
        self.name
    }

    fn load_binary(&self, filename: &str) -> anyhow::Result<Vec<u8>> {
        let file = T::get(filename)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        Ok(file.data.into_owned())
    }

    fn load_string(&self, filename: &str) -> anyhow::Result<String> {
        let file = T::get(filename)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        let s = std::str::from_utf8(&file.data)?;
        Ok(s.to_string())
    }
}

pub trait ModelLoader {
    fn load_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::Texture>;
    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Model>;
}

pub struct ObjLoader<T: ResLoader> {
    res_loader: T,
}

impl<T: ResLoader> ObjLoader<T> {
    pub fn new(res_loader: T) -> Self {
        Self { res_loader }
    }
}

impl<T: ResLoader> ModelLoader for ObjLoader<T> {
    fn load_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::Texture> {
        let data = self.res_loader.load_binary(filename)?;
        let texture = textures::Texture::from_bytes(device, queue, &data, filename)?;
        Ok(texture)
    }

    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Model> {
        let obj_text = self.res_loader.load_binary(filename)?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            move |p| {
                let mat_text = self.res_loader.load_string(p.to_str().unwrap()).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let diffuse_texture = self.load_texture(&m.diffuse_texture.unwrap(), device, queue)?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: m.name,
                diffuse_texture,
                bind_group,
            });
        }

        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| ModelVertex {
                        position: [
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ],
                        tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                        normal: if m.mesh.normals.is_empty() {
                            [0.0, 0.0, 0.0]
                        } else {
                            [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ]
                        },
                    })
                    .collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "#{}/{} Vertex Buffer",
                        self.res_loader.name(),
                        filename
                    )),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "#{}/{} Index Buffer",
                        self.res_loader.name(),
                        filename
                    )),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: filename.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Model { meshes, materials })
    }
}

pub struct PmxLoader<T: ResLoader> {
    res_loader: T,
}

impl<T: ResLoader> PmxLoader<T> {
    pub fn new(res_loader: T) -> Self {
        Self { res_loader }
    }
}

impl<T: ResLoader> ModelLoader for PmxLoader<T> {
    fn load_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::Texture> {
        let data = self.res_loader.load_binary(filename)?;
        let texture = textures::Texture::from_bytes(device, queue, &data, filename)?;
        Ok(texture)
    }

    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Model> {
        use mmd::pmx::reader::*;

        let pmx_data = self.res_loader.load_binary(filename)?;
        let pmx_cursor = Cursor::new(pmx_data);
        let pmx_reader = BufReader::new(pmx_cursor);

        let pmx_header = mmd::HeaderReader::new(pmx_reader)?;

        let mut pmx_vertices_r = VertexReader::<_>::new(pmx_header)?;
        let pmx_vertices = pmx_vertices_r
            .iter::<mmd::DefaultConfig>()
            .collect::<mmd::Result<Vec<_>>>()?;

        let mut pmx_surfaces_r = SurfaceReader::<_>::new(pmx_vertices_r)?;
        let pmx_surfaces = pmx_surfaces_r
            .iter::<mmd::DefaultConfig>()
            .collect::<mmd::Result<Vec<_>>>()?;

        let mut pmx_textures_r = TextureReader::new(pmx_surfaces_r)?;
        let texture_list = pmx_textures_r.iter().collect::<mmd::Result<Vec<_>>>()?;

        let mut pmx_materials_r = MaterialReader::<_>::new(pmx_textures_r)?;
        let pmx_materials = pmx_materials_r
            .iter::<mmd::DefaultConfig>()
            .collect::<mmd::Result<Vec<_>>>()?;

        let mut pmx_bones_r = BoneReader::<_>::new(pmx_materials_r)?;
        let _pmx_bones = pmx_bones_r
            .iter::<mmd::DefaultConfig>()
            .collect::<mmd::Result<Vec<_>>>()?;

        let mut pmx_morphs_r = MorphReader::<_>::new(pmx_bones_r)?;
        let _pmx_morphs = pmx_morphs_r
            .iter::<mmd::DefaultConfig>()
            .collect::<mmd::Result<Vec<_>>>()?;

        let mut materials = Vec::new();
        let mut meshes = Vec::new();
        let mut triangle_index_offset = 0;
        for (m_i, m) in pmx_materials.iter().enumerate() {
            let diffuse_texture =
                self.load_texture(&texture_list[m.texture_index as usize], device, queue)?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: m.local_name.clone(),
                diffuse_texture,
                bind_group,
            });

            let mut global_to_local_vertex_index_map = std::collections::HashMap::new();
            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            // `surface` here means vertex, not triangle. what?
            let triangle_count = (m.surface_count / 3) as usize;

            for i in 0..triangle_count {
                let surface = &pmx_surfaces[triangle_index_offset + i];
                for &global_vertex_index in &surface[..] {
                    let local_vertex_index = if let Some(&local_index) =
                        global_to_local_vertex_index_map.get(&global_vertex_index)
                    {
                        local_index
                    } else {
                        let v = &pmx_vertices[global_vertex_index as usize];
                        let vertex = ModelVertex {
                            position: [v.position[0], v.position[1], v.position[2]],
                            tex_coords: [v.uv[0], 1.0 - v.uv[1]],
                            normal: [v.normal[0], v.normal[1], v.normal[2]],
                        };
                        let local_index = vertices.len() as u32;
                        vertices.push(vertex);
                        global_to_local_vertex_index_map.insert(global_vertex_index, local_index);
                        local_index
                    };
                    indices.push(local_vertex_index);
                }
            }

            meshes.push(Mesh {
                name: filename.to_string(),
                vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "#{}/{} Vertex Buffer",
                        self.res_loader.name(),
                        filename
                    )),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!(
                        "#{}/{} Index Buffer",
                        self.res_loader.name(),
                        filename
                    )),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                num_elements: indices.len() as u32,
                material: m_i,
            });

            triangle_index_offset += triangle_count;
        }

        Ok(Model { meshes, materials })
    }
}
