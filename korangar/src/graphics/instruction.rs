use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use cgmath::{Matrix4, Point3, Vector2, Vector4};
use korangar_util::collision::AABB;

use super::color::Color;
use super::vertices::ModelVertex;
use super::{Buffer, TextureGroup, TileVertex};
use crate::interface::layout::ScreenSize;
use crate::loaders::{FontLoader, TextureLoader};

pub struct GraphicsEngineDescriptor {
    pub texture_loader: TextureLoader,
    pub font_loader: Rc<RefCell<FontLoader>>,
    pub picker_value: Arc<AtomicU32>,
    #[cfg(feature = "debug")]
    pub show_wireframe: bool,
}

pub struct RenderInstruction<'a> {
    pub uniforms: &'a Uniforms<'a>,
    pub interface: &'a [InterfaceInstruction],
    pub sprite: &'a [SpriteInstruction],
    pub marker: &'a [MarkerInstruction],
    pub indicator: &'a IndicatorInstruction,

    pub directional_shadow_caster: &'a [DirectionalShadowCaster],
    pub point_shadow_caster: &'a [PointShadowCaster],

    pub models: &'a [ModelInstruction],
    pub directional_shadow_models: &'a [ModelInstruction],
    pub point_shadow_models: &'a [ModelInstruction],

    pub entity: &'a [EntityInstruction],
    pub directional_shadow_entity: &'a [ShadowEntityInstruction],
    pub point_shadow_entity: &'a [ShadowEntityInstruction],

    pub effect: &'a [EffectInstruction],
    pub water: &'a [WaterInstruction],

    pub map_tile_buffer: &'a Buffer<TileVertex>,
    pub map_vertex_buffer: &'a Buffer<ModelVertex>,
    pub map_texture_group: &'a TextureGroup,

    #[cfg(feature = "debug")]
    pub debug: &'a DebugInstruction<'a>,
}

pub struct Uniforms<'a> {
    pub projection_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,
    pub screen_size: ScreenSize,
    pub ambient_light_color: [f32; 3],
    pub time: f32,
    pub water_level: f32,
    pub directional_light: &'a DirectionalLight,
    pub point_light: &'a [PointLight],
}

pub struct DirectionalLight {
    pub projection_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,
    pub direction: Point3<f32>,
    pub color: Color,
}

pub struct PointLight {
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
}

pub struct DirectionalShadowCaster {
    pub view_projection_matrix: Matrix4<f32>,
    pub position: Point3<f32>,
}

pub struct PointShadowCaster {
    pub view_projection_matrices: [Matrix4<f32>; 6],
    pub position: Point3<f32>,
    /// Start point inside the point_shadow_models.
    pub model_offset: usize,
    /// Model count inside the point_shadow_models.
    pub model_count: usize,
    /// Start point inside the point_shadow_entity.
    pub entity_offset: usize,
    /// Model count inside the point_shadow_entity.
    pub entity_count: usize,
}

pub struct InterfaceInstruction {}

pub struct SpriteInstruction {}

pub struct MarkerInstruction {
    screen_position: Vector2<f32>,
    screen_size: Vector2<f32>,
    identifier_high: u32,
    identifier_low: u32,
}

pub struct IndicatorInstruction {
    upper_left: Vector4<f32>,
    upper_right: Vector4<f32>,
    lower_left: Vector4<f32>,
    lower_right: Vector4<f32>,
    color: Color,
}

pub struct ModelInstruction {
    pub model_matrix: Matrix4<f32>,
    pub vertex_offset: usize,
    pub vertex_count: usize,
}

pub struct EntityInstruction {
    world: Matrix4<f32>,
    texture_position: Vector2<f32>,
    texture_size: Vector2<f32>,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    identifier_high: u32,
    identifier_low: u32,
}

pub struct ShadowEntityInstruction {
    world: Matrix4<f32>,
    texture_position: Vector2<f32>,
    texture_size: Vector2<f32>,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
}

pub struct WaterInstruction {}

pub struct EffectInstruction {}

#[cfg(feature = "debug")]
pub struct DebugInstruction<'a> {
    boxes: &'a [DebugBox],
    circles: &'a [DebugCircle],
}

#[cfg(feature = "debug")]
pub struct DebugBox {
    aabb: AABB,
    color: Color,
}

#[cfg(feature = "debug")]
pub struct DebugCircle {
    position: Point3<f32>,
    radius: f32,
    color: Color,
}
