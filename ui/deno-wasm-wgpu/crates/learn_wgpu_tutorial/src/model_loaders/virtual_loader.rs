use crate::drawing::models::{Mesh, ModelVertex};

pub struct VirtualLoader;

impl VirtualLoader {
    /// Author: GitHub Copilot.
    pub fn make_cube_mesh_with_minimal_effort_for_light_source_indicators(
        device: &wgpu::Device,
        size: f32,
    ) -> Mesh {
        let half = size / 2.0;

        // 8 corner positions
        let positions = [
            glam::Vec3::new(-half, half, half),   // 0: a (top north-west)
            glam::Vec3::new(-half, half, -half),  // 1: b (top south-west)
            glam::Vec3::new(half, half, -half),   // 2: c (top south-east)
            glam::Vec3::new(half, half, half),    // 3: d (top north-east)
            glam::Vec3::new(-half, -half, half),  // 4: e (bottom north-west)
            glam::Vec3::new(-half, -half, -half), // 5: f (bottom south-west)
            glam::Vec3::new(half, -half, -half),  // 6: g (bottom south-east)
            glam::Vec3::new(half, -half, half),   // 7: h (bottom north-east)
        ];

        let vertices: Vec<ModelVertex> = positions
            .iter()
            .map(|&pos| {
                ModelVertex {
                    position: pos,
                    // not used for light source indicators.
                    tex_coords: (0.0, 0.0).into(),
                    // not used for light source indicators.
                    normal: glam::Vec3::ZERO,
                    // not used for light source indicators.
                    tangent: glam::Vec3::ZERO,
                    // not used for light source indicators.
                    bitangent: glam::Vec3::ZERO,
                }
            })
            .collect();

        let indices = vec![
            0, 1, 2, 0, 2, 3, // Top face (y = half)
            5, 4, 7, 5, 7, 6, // Bottom face (y = -half)
            1, 0, 4, 1, 4, 5, // West face (x = -half)
            3, 2, 6, 3, 6, 7, // East face (x = half)
            2, 1, 5, 2, 5, 6, // South face (z = -half)
            0, 3, 7, 0, 7, 4, // North face (z = half)
        ];

        Mesh::new(
            device,
            "VirtualLoader::make_cube_mesh_with_minimal_effort_for_light_source_indicators",
            &vertices,
            &indices,
            /* not used for light source indicators. */ 0,
        )
    }
}
