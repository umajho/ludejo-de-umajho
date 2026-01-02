use std::io::{BufReader, Cursor};

use crate::{
    drawing::{
        models::{Material, Mesh, Model, ModelVertex},
        textures,
    },
    io::fs_accessors::FsAccessor,
    model_loaders::{ModelLoader, utils::calculate_tangent_and_bitangent},
};

pub struct ObjLoader<T: FsAccessor> {
    res_loader: T,
}

impl<T: FsAccessor> ObjLoader<T> {
    pub fn new(res_loader: T) -> Self {
        Self { res_loader }
    }
}

impl<T: FsAccessor> ModelLoader for ObjLoader<T> {
    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
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
            materials.push(Material::new(
                device,
                &m.name,
                self.load_diffuse_texture(&m.diffuse_texture.unwrap(), device, queue)?,
                self.load_normal_texture(&m.normal_texture.unwrap(), device, queue)?,
                texture_bind_group_layout,
            ));
        }

        let meshes = models
            .into_iter()
            .map(|m| {
                let mut vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| ModelVertex {
                        position: [
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ]
                        .into(),
                        tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]]
                            .into(),
                        normal: if m.mesh.normals.is_empty() {
                            glam::Vec3::ZERO
                        } else {
                            [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ]
                            .into()
                        },
                        tangent: glam::Vec3::ZERO,
                        bitangent: glam::Vec3::ZERO,
                    })
                    .collect::<Vec<_>>();

                calculate_tangent_and_bitangent(&mut vertices, &m.mesh.indices);

                Mesh::new(
                    device,
                    &format!("#{}/{}", self.res_loader.name(), filename),
                    &vertices,
                    &m.mesh.indices,
                    m.mesh.material_id.unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();

        Ok(Model::new(meshes, materials))
    }
}

impl<T: FsAccessor> ObjLoader<T> {
    fn load_diffuse_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::D2DiffuseTexture> {
        let data = self.res_loader.load_binary(filename)?;
        let texture =
            textures::D2DiffuseTexture::from_image_in_memory(device, queue, &data, filename)?;
        Ok(texture)
    }
    fn load_normal_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::D2NormalTexture> {
        let data = self.res_loader.load_binary(filename)?;
        let texture =
            textures::D2NormalTexture::from_image_in_memory(device, queue, &data, filename)?;
        Ok(texture)
    }
}
