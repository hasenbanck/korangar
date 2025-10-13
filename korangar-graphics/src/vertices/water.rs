use bytemuck::{Pod, Zeroable};
use cgmath::Point3;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexStepMode, vertex_attr_array};

/// Vertex representation for water surfaces.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct WaterVertex {
    /// Position of the vertex in 3D space.
    pub position: [f32; 3],
    /// Grid coordinates for water animation and tiling.
    pub grid: [i32; 2],
}

impl WaterVertex {
    /// Creates a new water vertex.
    pub fn new(position: Point3<f32>, grid_u: i32, grid_v: i32) -> Self {
        Self {
            position: position.into(),
            grid: [grid_u, grid_v],
        }
    }

    /// Returns the vertex buffer layout.
    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array!(
            0 => Float32x3,
            1 => Sint32x2,
        );

        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
