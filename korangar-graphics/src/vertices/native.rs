use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use smallvec::{SmallVec, smallvec_inline};

use crate::{Color, ModelVertex};

/// Native format model vertex used during loading before conversion to GPU
/// format.
#[derive(Clone)]
pub struct NativeModelVertex {
    /// Position of the vertex in 3D space.
    pub position: Point3<f32>,
    /// Normal vector for lighting calculations.
    pub normal: Vector3<f32>,
    /// UV coordinates for texture mapping.
    pub texture_coordinates: Vector2<f32>,
    /// Index of the texture into the texture array.
    pub texture_index: i32,
    /// Vertex color.
    pub color: Color,
    /// How much this vertex is affected by wind animation.
    pub wind_affinity: f32,
    /// Smoothing groups for normal calculation and interpolation.
    pub smoothing_groups: SmallVec<[i32; 3]>,
}

impl NativeModelVertex {
    /// Creates a new native model vertex with all properties.
    pub fn new(
        position: Point3<f32>,
        normal: Vector3<f32>,
        texture_coordinates: Vector2<f32>,
        texture_index: i32,
        color: Color,
        wind_affinity: f32,
        smoothing_groups: SmallVec<[i32; 3]>,
    ) -> Self {
        Self {
            position,
            normal,
            texture_coordinates,
            texture_index,
            color,
            wind_affinity,
            smoothing_groups,
        }
    }

    /// Creates a zeroed native model vertex with default values.
    pub const fn zeroed() -> NativeModelVertex {
        NativeModelVertex {
            position: Point3::new(0.0, 0.0, 0.0),
            normal: Vector3::new(0.0, 0.0, 0.0),
            texture_coordinates: Vector2::new(0.0, 0.0),
            texture_index: 0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.0),
            wind_affinity: 0.0,
            smoothing_groups: smallvec_inline![0; 3],
        }
    }

    /// Converts this native vertex into a GPU-compatible model vertex.
    fn into_model_vertex(self) -> ModelVertex {
        ModelVertex::new(
            self.position,
            self.normal,
            self.texture_coordinates,
            self.color,
            self.texture_index,
            self.wind_affinity,
        )
    }

    /// Converts native vertices to GPU model vertices, optionally applying
    /// texture index remapping.
    pub fn convert_to_model_vertices(mut native_vertices: Vec<NativeModelVertex>, texture_mapping: Option<&[i32]>) -> Vec<ModelVertex> {
        match texture_mapping {
            None => native_vertices.drain(..).map(|vertex| vertex.into_model_vertex()).collect(),
            Some(texture_mapping) => native_vertices
                .drain(..)
                .map(|mut vertex| {
                    vertex.texture_index = texture_mapping[vertex.texture_index as usize];
                    vertex.into_model_vertex()
                })
                .collect(),
        }
    }

    /// Calculates the face normal for a triangle from three vertex positions.
    /// Returns `None` if the triangle is degenerated (if the triangle lost one
    /// or two dimension).
    pub fn calculate_normal(
        first_position: Point3<f32>,
        second_position: Point3<f32>,
        third_position: Point3<f32>,
    ) -> Option<Vector3<f32>> {
        const DEGENERATE_EPSILON: f32 = 1e-5;

        let delta_position_1 = second_position - first_position;
        let delta_position_2 = third_position - first_position;
        let normal = delta_position_1.cross(delta_position_2);

        match normal.magnitude() > DEGENERATE_EPSILON {
            true => Some(normal.normalize()),
            false => None,
        }
    }
}
