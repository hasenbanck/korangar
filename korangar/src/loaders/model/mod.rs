use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};
use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::collision::AABB;
use korangar_util::math::multiply_matrix4_and_point3;
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::model::{ModelData, ModelString, NodeData};
use ragnarok_formats::version::InternalVersion;

use super::error::LoadError;
use super::{map_model_texture_to_texture_buffer, FALLBACK_MODEL_FILE};
use crate::graphics::{ModelVertex, NativeModelVertex, Texture};
use crate::loaders::{GameFileLoader, TextureLoader};
use crate::world::{Model, Node};

#[derive(new)]
pub struct ModelLoader {
    game_file_loader: Arc<GameFileLoader>,
}

impl ModelLoader {
    fn add_vertices(
        native_vertices: &mut Vec<NativeModelVertex>,
        vertex_positions: &[Point3<f32>],
        texture_coordinates: &[Vector2<f32>],
        texture_index: u16,
        reverse_vertices: bool,
        reverse_normal: bool,
    ) {
        let normal = match reverse_normal {
            true => NativeModelVertex::calculate_normal(vertex_positions[0], vertex_positions[1], vertex_positions[2]),
            false => NativeModelVertex::calculate_normal(vertex_positions[2], vertex_positions[1], vertex_positions[0]),
        };

        if reverse_vertices {
            for (vertex_position, texture_coordinates) in vertex_positions.iter().copied().zip(texture_coordinates).rev() {
                native_vertices.push(NativeModelVertex::new(
                    vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index as i32,
                    0.0, // TODO: actually add wind affinity
                ));
            }
        } else {
            for (vertex_position, texture_coordinates) in vertex_positions.iter().copied().zip(texture_coordinates) {
                native_vertices.push(NativeModelVertex::new(
                    vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index as i32,
                    0.0, // TODO: actually add wind affinity
                ));
            }
        }
    }

    fn make_vertices(node: &NodeData, main_matrix: &Matrix4<f32>, reverse_order: bool) -> Vec<NativeModelVertex> {
        let mut native_vertices = Vec::new();

        let array: [f32; 3] = node.scale.into();
        let reverse_node_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();

        if reverse_node_order {
            panic!("this can actually happen");
        }

        for face in &node.faces {
            // collect into tiny vec instead ?
            let vertex_positions: Vec<Point3<f32>> = face
                .vertex_position_indices
                .iter()
                .copied()
                .map(|index| node.vertex_positions[index as usize])
                .map(|position| multiply_matrix4_and_point3(main_matrix, position))
                .collect();

            let texture_coordinates: Vec<Vector2<f32>> = face
                .texture_coordinate_indices
                .iter()
                .copied()
                .map(|index| node.texture_coordinates[index as usize].coordinates)
                .collect();

            Self::add_vertices(
                &mut native_vertices,
                &vertex_positions,
                &texture_coordinates,
                face.texture_index,
                reverse_order,
                false,
            );

            if face.two_sided != 0 {
                Self::add_vertices(
                    &mut native_vertices,
                    &vertex_positions,
                    &texture_coordinates,
                    face.texture_index,
                    !reverse_order,
                    true,
                );
            }
        }

        native_vertices
    }

    fn calculate_matrices(node: &NodeData, parent_matrix: &Matrix4<f32>) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let main = Matrix4::from_translation(node.translation1) * Matrix4::from(node.offset_matrix);

        let scale_matrix = Matrix4::from_nonuniform_scale(node.scale.x, node.scale.y, node.scale.z);
        let rotation_matrix = Matrix4::from_axis_angle(node.rotation_axis, Rad(node.rotation_angle));
        let translation_matrix = Matrix4::from_translation(node.translation2);

        let transform = match node.rotation_keyframe_count > 0 {
            true => translation_matrix * scale_matrix,
            false => translation_matrix * rotation_matrix * scale_matrix,
        };

        let box_transform = parent_matrix * translation_matrix * rotation_matrix * scale_matrix;

        (main, transform, box_transform)
    }

    fn process_node_mesh(
        current_node: &NodeData,
        nodes: &[NodeData],
        vertex_buffer: &mut Vec<ModelVertex>,
        texture_index_mapping: &[i32],
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut AABB,
        root_node_name: &ModelString<40>,
        reverse_order: bool,
    ) -> Node {
        let node_texture_index_mapping: Vec<i32> = current_node
            .texture_indices
            .iter()
            .map(|&index| texture_index_mapping[index as usize])
            .collect();

        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);
        let vertices = NativeModelVertex::to_vertices(
            Self::make_vertices(current_node, &main_matrix, reverse_order),
            &node_texture_index_mapping,
        );

        let vertex_offset = vertex_buffer.len() as u32;
        let vertex_count = vertices.len() as u32;
        vertex_buffer.extend(vertices);

        let box_matrix = box_transform_matrix * main_matrix;
        let bounding_box = AABB::from_vertices(
            current_node
                .vertex_positions
                .iter()
                .map(|position| multiply_matrix4_and_point3(&box_matrix, *position)),
        );
        main_bounding_box.extend(&bounding_box);

        let final_matrix = match current_node.node_name == *root_node_name {
            true => {
                Matrix4::from_translation(-Vector3::new(
                    bounding_box.center().x,
                    bounding_box.max().y,
                    bounding_box.center().z,
                )) * transform_matrix
            }
            false => transform_matrix,
        };

        let child_nodes = nodes
            .iter()
            .filter(|node| node.parent_node_name == current_node.node_name)
            .filter(|node| node.parent_node_name != node.node_name)
            .map(|node| {
                Self::process_node_mesh(
                    node,
                    nodes,
                    vertex_buffer,
                    texture_index_mapping,
                    &box_transform_matrix,
                    main_bounding_box,
                    root_node_name,
                    reverse_order,
                )
            })
            .collect();

        Node::new(
            final_matrix,
            vertex_offset,
            vertex_count,
            child_nodes,
            current_node.rotation_keyframes.clone(),
        )
    }

    pub fn load(
        &mut self,
        texture_loader: &mut TextureLoader,
        texture_cache: &mut HashMap<String, i32>,
        vertex_buffer: &mut Vec<ModelVertex>,
        texture_buffer: &mut Vec<Arc<Texture>>,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Model, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}", model_file.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\model\\{model_file}"))
            .map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        let model_data = match ModelData::from_bytes(&mut byte_stream) {
            Ok(model_data) => model_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load model: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load(
                    texture_loader,
                    texture_cache,
                    vertex_buffer,
                    texture_buffer,
                    FALLBACK_MODEL_FILE,
                    reverse_order,
                );
            }
        };

        let texture_index_mapping =
            map_model_texture_to_texture_buffer(texture_loader, texture_cache, texture_buffer, &model_data.texture_names);

        let root_node_name = &model_data.root_node_name;

        let root_node = model_data
            .nodes
            .iter()
            .find(|node_data| &node_data.node_name == root_node_name)
            .expect("failed to find main node");

        let mut bounding_box = AABB::uninitialized();
        let root_node = Self::process_node_mesh(
            root_node,
            &model_data.nodes,
            vertex_buffer,
            &texture_index_mapping,
            &Matrix4::identity(),
            &mut bounding_box,
            root_node_name,
            reverse_order,
        );
        let model = Model::new(
            root_node,
            bounding_box,
            #[cfg(feature = "debug")]
            model_data,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(model)
    }
}
