use crate::drawing::models::Model;

pub mod obj_loader;
pub mod pmx_loader;
pub mod virtual_loader;

/// ## TODO
///
/// - Decouple from [`crate::drawing`]. The returned [`Model`] should be
///   general, should be (de)serializable, which means it should not depend on
///   creating things with [`wgpu::Device`].
pub trait ModelLoader {
    fn load_model(
        &self,
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Model>;
}

pub(self) mod utils {
    use crate::drawing::{models::ModelVertex, textures};

    pub fn new_flat_normal_texture(
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

    pub fn calculate_tangent_and_bitangent(vertices: &mut [ModelVertex], indices: &[u32]) {
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
}
