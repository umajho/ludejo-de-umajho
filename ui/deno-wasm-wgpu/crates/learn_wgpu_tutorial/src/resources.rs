use std::io::{BufReader, Cursor};

use glam::Vec3;
use rust_embed::Embed;

use crate::drawing::{
    models::{Material, Mesh, Model, ModelVertex},
    textures,
};

#[derive(Embed)]
#[folder = "res/cube"]
pub struct ResCube;

#[derive(Embed)]
#[folder = "res/aoi"]
pub struct ResAoi;

#[derive(Embed)]
#[folder = "res/sky"]
pub struct ResSky;

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

/// TODO: normalize all dakuten & handakuten. (Otherwise RustEmbed won't find
/// files in some cases on the web build.)
fn normalize_filename(filename: &str) -> String {
    filename.replace("\u{30d4}", "\u{30d2}\u{309a}") // ???
}

impl<T: rust_embed::RustEmbed> ResLoader for EmbedResLoader<T> {
    fn name(&self) -> &str {
        self.name
    }

    fn load_binary(&self, filename: &str) -> anyhow::Result<Vec<u8>> {
        let file = T::get(&normalize_filename(filename))
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        Ok(file.data.into_owned())
    }

    fn load_string(&self, filename: &str) -> anyhow::Result<String> {
        let file = T::get(&normalize_filename(filename))
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        let s = std::str::from_utf8(&file.data)?;
        Ok(s.to_string())
    }
}

pub trait ModelLoader {
    fn load_diffuse_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::D2DiffuseTexture>;
    fn load_normal_texture(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<textures::D2NormalTexture>;
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
                            Vec3::ZERO
                        } else {
                            [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ]
                            .into()
                        },
                        tangent: Vec3::ZERO,
                        bitangent: Vec3::ZERO,
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

pub struct PmxLoader<T: ResLoader> {
    res_loader: T,
}

impl<T: ResLoader> PmxLoader<T> {
    pub fn new(res_loader: T) -> Self {
        Self { res_loader }
    }
}

impl<T: ResLoader> ModelLoader for PmxLoader<T> {
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

fn new_flat_normal_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
) -> textures::D2NormalTexture {
    let normal_data = vec![128u8, 128u8, 255u8].repeat((width * height) as usize);
    let image = image::RgbImage::from_vec(width, height, normal_data).unwrap();
    let image = image::DynamicImage::ImageRgb8(image);

    textures::D2NormalTexture::from_image(device, queue, &image, None)
}

fn calculate_tangent_and_bitangent(vertices: &mut [ModelVertex], indices: &[u32]) {
    let mut triangles_included = vec![0u32; vertices.len()];

    for c in indices.chunks(3) {
        let (c0, c1, c2) = (c[0] as usize, c[1] as usize, c[2] as usize);

        let v0 = &vertices[c0];
        let v1 = &vertices[c1];
        let v2 = &vertices[c2];

        let pos0: glam::Vec3 = v0.position.into();
        let pos1: glam::Vec3 = v1.position.into();
        let pos2: glam::Vec3 = v2.position.into();

        let uv0: glam::Vec2 = v0.tex_coords.into();
        let uv1: glam::Vec2 = v1.tex_coords.into();
        let uv2: glam::Vec2 = v2.tex_coords.into();

        let delta_pos1 = pos1 - pos0;
        let delta_pos2 = pos2 - pos0;

        let delta_uv1 = uv1 - uv0;
        let delta_uv2 = uv2 - uv0;

        let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
        let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
        let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

        vertices[c0].tangent = (tangent + glam::Vec3::from(vertices[c0].tangent)).into();
        vertices[c0].bitangent = (bitangent + glam::Vec3::from(vertices[c0].bitangent)).into();
        vertices[c1].tangent = (tangent + glam::Vec3::from(vertices[c1].tangent)).into();
        vertices[c1].bitangent = (bitangent + glam::Vec3::from(vertices[c1].bitangent)).into();
        vertices[c2].tangent = (tangent + glam::Vec3::from(vertices[c2].tangent)).into();
        vertices[c2].bitangent = (bitangent + glam::Vec3::from(vertices[c2].bitangent)).into();

        triangles_included[c0] += 1;
        triangles_included[c1] += 1;
        triangles_included[c2] += 1;
    }

    for (i, n) in triangles_included.into_iter().enumerate() {
        let denom = 1.0 / (n as f32);
        let v = &mut vertices[i];
        v.tangent = (glam::Vec3::from(v.tangent) * denom).into();
        v.bitangent = (glam::Vec3::from(v.bitangent) * denom).into();
    }
}
