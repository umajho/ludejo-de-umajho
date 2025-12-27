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

pub struct ResLoader<T: rust_embed::RustEmbed> {
    name: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T: rust_embed::RustEmbed> ResLoader<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: std::marker::PhantomData,
        }
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

    pub fn load_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::Texture> {
        let data = self.load_binary(filename)?;
        let texture = textures::Texture::from_bytes(device, queue, &data, filename)?;
        Ok(texture)
    }

    pub fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Model> {
        let obj_text = self.load_binary(filename)?;
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
                let mat_text = self.load_string(p.to_str().unwrap()).unwrap();
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
                        tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
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
                    label: Some(&format!("#{}/{} Vertex Buffer", self.name, filename)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("#{}/{} Index Buffer", self.name, filename)),
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
