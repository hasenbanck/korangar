use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use cgmath::{Matrix4, Point3};
use korangar_util::collision::AABB;

use super::color::Color;
use super::vertices::ModelVertex;
use super::{Buffer, TextureGroup};
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
    pub geometry: &'a GeometryInstruction<'a>,
    pub water: &'a [WaterInstruction],
    pub entity: &'a [EntityInstruction],
    pub effect: &'a [EffectInstruction],
    pub rectangle: &'a [RectangleInstruction],
    pub sprite: &'a [SpriteInstruction],
    pub indicator: &'a [IndicatorInstruction],
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
    pub point_shadow_caster: &'a [&'a PointShadowCaster],
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

pub struct PointShadowCaster {
    pub view_projection_matrices: [Matrix4<f32>; 6],
    pub position: Point3<f32>,
}

pub struct InterfaceInstruction {}

pub struct GeometryInstruction<'a> {
    pub vertex_buffer: &'a Buffer<ModelVertex>,
    pub texture_group: &'a TextureGroup,
    pub models: &'a [ModelInstruction],
}

pub struct ModelInstruction {
    pub model_matrix: Matrix4<f32>,
    pub vertex_offset: usize,
}

pub struct WaterInstruction {}

pub struct EntityInstruction {}

pub struct EffectInstruction {}

pub struct RectangleInstruction {}

pub struct SpriteInstruction {}

pub struct IndicatorInstruction {}

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
