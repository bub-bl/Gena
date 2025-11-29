use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;

// const QUAD_VERTICES: &[Vertex] = &[
//     Vertex {
//         position: [-0.5, -0.5],
//         uv: [0.0, 0.0],
//     }, // bas-gauche
//     Vertex {
//         position: [0.5, -0.5],
//         uv: [1.0, 0.0],
//     }, // bas-droite
//     Vertex {
//         position: [0.5, 0.5],
//         uv: [1.0, 1.0],
//     }, // haut-droite
//     Vertex {
//         position: [-0.5, 0.5],
//         uv: [0.0, 1.0],
//     }, // haut-gauche
// ];

const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl Vertex {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }

    // pub fn quad_vertices() -> &'static [Vertex] {
    //     QUAD_VERTICES
    // }

    pub fn quad_vertices() -> [Vertex; 4] {
        let size = 100.0; // Taille en pixels
        [
            Vertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
            }, // haut-gauche
            Vertex {
                position: [size, 0.0],
                uv: [1.0, 0.0],
            }, // haut-droite
            Vertex {
                position: [size, size],
                uv: [1.0, 1.0],
            }, // bas-droite
            Vertex {
                position: [0.0, size],
                uv: [0.0, 1.0],
            }, // bas-gauche
        ]
    }

    pub fn quad_indices() -> &'static [u16] {
        QUAD_INDICES
    }
}
