use std::io::{BufReader, Cursor};

use crate::{
    drawing::{
        models::{Material, Mesh, Model, ModelVertex},
        textures,
    },
    io::fs_accessors::FsAccessor,
    model_loaders::{
        ModelLoader,
        utils::{calculate_tangent_and_bitangent, new_flat_normal_texture},
    },
};

pub struct PmxLoader<T: FsAccessor> {
    res_loader: T,
}

impl<T: FsAccessor> PmxLoader<T> {
    pub fn new(res_loader: T) -> Self {
        Self { res_loader }
    }
}

impl<T: FsAccessor> ModelLoader for PmxLoader<T> {
    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
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
                self.load_diffuse_texture(&texture_list[m.texture_index as usize], device, queue)?;
            let size = diffuse_texture.size();

            let normal_texture = new_flat_normal_texture(device, queue, size.width, size.height);

            materials.push(Material::new(
                device,
                &m.local_name,
                diffuse_texture,
                normal_texture,
                texture_bind_group_layout,
            ));

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
                            position: v.position.into(),
                            tex_coords: [v.uv[0], v.uv[1]].into(),
                            normal: v.normal.into(),
                            tangent: glam::Vec3::ZERO,
                            bitangent: glam::Vec3::ZERO,
                        };
                        let local_index = vertices.len() as u32;
                        vertices.push(vertex);
                        global_to_local_vertex_index_map.insert(global_vertex_index, local_index);
                        local_index
                    };
                    indices.push(local_vertex_index);
                }
            }

            calculate_tangent_and_bitangent(&mut vertices, &indices);

            meshes.push(Mesh::new(
                device,
                &format!("#{}/{}", self.res_loader.name(), filename),
                &vertices,
                &indices,
                m_i,
            ));

            triangle_index_offset += triangle_count;
        }

        Ok(Model::new(meshes, materials))
    }
}

impl<T: FsAccessor> PmxLoader<T> {
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
    #[allow(unused)]
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
