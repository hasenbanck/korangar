use std::sync::Arc;

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector2, Vector3, Vector4, Zero};
use wgpu::BlendFactor;

use super::color::Color;
#[cfg(feature = "debug")]
use super::settings::RenderOptions;
use super::vertices::ModelVertex;
use super::{Buffer, ShadowDetail, ShadowMethod, Texture, TextureSet, TileVertex, WaterVertex};
#[cfg(feature = "debug")]
use crate::MarkerIdentifier;
use crate::{CornerDiameter, ScreenClip, ScreenPosition, ScreenSize, ShadowPadding};

/// Main rendering instruction containing all data needed for a single frame
/// render.
#[derive(Default)]
pub struct RenderInstruction<'a> {
    /// Whether to render the user interface.
    pub show_interface: bool,
    /// Screen position for object picking.
    pub picker_position: ScreenPosition,
    /// Global rendering uniforms (camera, lighting, etc).
    pub uniforms: Uniforms,
    /// Ground indicator instruction.
    pub indicator: Option<IndicatorInstruction>,
    /// Interface rectangle elements.
    pub interface: &'a [InterfaceRectangleInstruction],
    /// Between 3D world and effects.
    pub bottom_layer_rectangles: &'a [RectangleInstruction],
    /// Between effects and interface.
    pub middle_layer_rectangles: &'a [RectangleInstruction],
    /// On top of everything else.
    pub top_layer_rectangles: &'a [RectangleInstruction],
    /// Main directional light.
    pub directional_light: DirectionalLightInstruction,
    /// Shadow cascade partitions for directional light.
    pub directional_light_partitions: &'a [DirectionalLightPartitionInstruction],
    /// Point lights without shadows.
    pub point_light: &'a [PointLightInstruction],
    /// Point lights with shadow casting enabled.
    pub point_light_with_shadows: &'a [PointLightWithShadowInstruction],
    /// Batched model rendering instructions.
    pub model_batches: &'a [ModelBatch],
    /// Individual model instances to render.
    pub models: &'a mut [ModelInstruction],
    /// Entity rendering instructions.
    pub entities: &'a mut [EntityInstruction],
    /// Model batches for directional shadow maps.
    pub directional_shadow_model_batches: &'a [Vec<ModelBatch>],
    /// Models for directional shadow rendering.
    pub directional_shadow_models: &'a [ModelInstruction],
    /// Entities for directional shadow rendering.
    pub directional_shadow_entities: &'a mut [Vec<EntityInstruction>],
    /// Models for point light shadow rendering.
    pub point_shadow_models: &'a [ModelInstruction],
    /// Entities for point light shadow rendering.
    pub point_shadow_entities: &'a [EntityInstruction],
    /// 2D effect instructions.
    pub effects: &'a [EffectInstruction],
    /// Water rendering instruction.
    pub water: Option<WaterInstruction<'a>>,
    /// Vertex buffer for picker.
    pub map_picker_tile_vertex_buffer: Option<&'a Buffer<TileVertex>>,
    /// Index buffer for picker.
    pub map_picker_tile_index_buffer: Option<&'a Buffer<u32>>,
    /// Font atlas texture for text rendering.
    pub font_map_texture: Option<&'a Texture>,
    /// Debug rendering toggles.
    #[cfg(feature = "debug")]
    pub render_options: RenderOptions,
    /// Debug AABB (axis-aligned bounding box) visualizations.
    #[cfg(feature = "debug")]
    pub aabb: &'a [DebugAabbInstruction],
    /// Debug circle visualizations.
    #[cfg(feature = "debug")]
    pub circles: &'a [DebugCircleInstruction],
    /// Debug rectangle visualizations.
    #[cfg(feature = "debug")]
    pub rectangles: &'a [DebugRectangleInstruction],
    /// Debug marker visualizations.
    #[cfg(feature = "debug")]
    pub marker: &'a [MarkerInstruction],
}

/// Global shader uniforms passed to all pipelines.
#[derive(Clone, Debug)]
pub struct Uniforms {
    /// Camera view transformation matrix.
    pub view_matrix: Matrix4<f32>,
    /// Perspective projection matrix.
    pub projection_matrix: Matrix4<f32>,
    /// Camera world position.
    pub camera_position: Vector4<f32>,
    /// Animation time in milliseconds for shader effects.
    pub animation_timer_ms: f32,
    /// Global ambient light color.
    pub ambient_light_color: Color,
    /// Whether enhanced lighting is enabled.
    pub enhanced_lighting: bool,
    /// Shadow rendering method (hard/soft).
    pub shadow_method: ShadowMethod,
    /// Shadow quality level.
    pub shadow_detail: ShadowDetail,
    /// Whether to use SDSM (Sample Distribution Shadow Maps).
    pub use_sdsm: bool,
    /// Whether SDSM is currently active.
    pub sdsm_enabled: bool,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            camera_position: Vector4::zero(),
            animation_timer_ms: 0.0,
            ambient_light_color: Color::default(),
            enhanced_lighting: false,
            shadow_method: ShadowMethod::Hard,
            shadow_detail: ShadowDetail::Low,
            use_sdsm: false,
            sdsm_enabled: false,
        }
    }
}

/// Water surface rendering instruction.
#[derive(Clone, Debug)]
pub struct WaterInstruction<'a> {
    /// Water surface texture.
    pub water_texture: &'a Texture,
    /// Vertex buffer for water geometry.
    pub water_vertex_buffer: &'a Buffer<WaterVertex>,
    /// Index buffer for water geometry.
    pub water_index_buffer: &'a Buffer<u32>,
    /// Texture tiling factor.
    pub texture_repeat: f32,
    /// Wave animation phase offset.
    pub waveform_phase_shift: f32,
    /// Wave height magnitude.
    pub waveform_amplitude: f32,
    /// Wave oscillation frequency.
    pub waveform_frequency: Deg<f32>,
    /// Water transparency level.
    pub water_opacity: f32,
}

/// Directional light source instruction.
#[derive(Clone, Debug)]
pub struct DirectionalLightInstruction {
    /// Combined view-projection matrix for shadow mapping.
    pub view_projection_matrix: Matrix4<f32>,
    /// Light direction vector.
    pub direction: Vector3<f32>,
    /// Light color and intensity.
    pub color: Color,
}

impl Default for DirectionalLightInstruction {
    fn default() -> Self {
        Self {
            view_projection_matrix: Matrix4::identity(),
            direction: Vector3::zero(),
            color: Color::default(),
        }
    }
}

/// Shadow cascade partition for directional light shadow mapping.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalLightPartitionInstruction {
    /// Combined view-projection matrix for this cascade.
    pub view_projection_matrix: Matrix4<f32>,
    /// Projection matrix for this cascade.
    pub projection_matrix: Matrix4<f32>,
    /// View matrix for this cascade.
    pub view_matrix: Matrix4<f32>,
    /// Far distance of this cascade's depth range.
    pub interval_end: f32,
    /// Size of a texel in world space for this cascade.
    pub world_space_texel_size: f32,
    /// Near plane distance for this cascade.
    pub near_plane: f32,
    /// Far plane distance for this cascade.
    pub far_plane: f32,
}

impl Default for DirectionalLightPartitionInstruction {
    fn default() -> Self {
        Self {
            view_projection_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            view_matrix: Matrix4::identity(),
            interval_end: 0.0,
            world_space_texel_size: 0.0,
            near_plane: 0.0,
            far_plane: 0.0,
        }
    }
}

/// Point light source without shadow casting.
#[derive(Clone, Debug)]
pub struct PointLightInstruction {
    /// World position of the light.
    pub position: Point3<f32>,
    /// Light color and intensity.
    pub color: Color,
    /// Maximum light influence distance.
    pub range: f32,
}

/// Point light source with omnidirectional shadow casting using cubemap
/// shadows. Right now point light can't cast shadows of models that are not
/// part of the map (like the debug models).
#[derive(Clone, Debug)]
pub struct PointLightWithShadowInstruction {
    /// View-projection matrices for 6 cubemap faces.
    pub view_projection_matrices: [Matrix4<f32>; 6],
    /// View matrices for 6 cubemap faces.
    pub view_matrices: [Matrix4<f32>; 6],
    /// World position of the light.
    pub position: Point3<f32>,
    /// Light color and intensity.
    pub color: Color,
    /// Maximum light influence distance.
    pub range: f32,
    /// Texture set for shadow-casting models.
    pub model_texture_set: Arc<TextureSet>,
    /// Vertex buffer for shadow-casting models.
    pub model_vertex_buffer: Arc<Buffer<ModelVertex>>,
    /// Index buffer for shadow-casting models.
    pub model_index_buffer: Arc<Buffer<u32>>,
    /// Start point inside the point_shadow_entities.
    pub entity_offset: [usize; 6],
    /// Model count inside the point_shadow_entities.
    pub entity_count: [usize; 6],
    /// Start point inside the point_shadow_models.
    pub model_offset: [usize; 6],
    /// Model count inside the point_shadow_models.
    pub model_count: [usize; 6],
}

/// Screen-space rectangle rendering instruction.
#[derive(Clone, Debug)]
pub enum RectangleInstruction {
    /// Solid color rectangle.
    Solid {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Fill color.
        color: Color,
    },
    /// Textured sprite rectangle.
    Sprite {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Tint color.
        color: Color,
        /// Texture coordinate offset.
        texture_position: Vector2<f32>,
        /// Texture coordinate size.
        texture_size: Vector2<f32>,
        /// Whether to use linear texture filtering.
        linear_filtering: bool,
        /// Source texture.
        texture: Arc<Texture>,
    },
    /// SDF (Signed Distance Field) rectangle for scalable shapes.
    Sdf {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Shape color.
        color: Color,
        /// Texture coordinate offset.
        texture_position: Vector2<f32>,
        /// Texture coordinate size.
        texture_size: Vector2<f32>,
        /// SDF texture.
        texture: Arc<Texture>,
    },
    /// Text glyph rectangle.
    Text {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Text color.
        color: Color,
        /// Font atlas coordinate offset.
        texture_position: Vector2<f32>,
        /// Font atlas coordinate size.
        texture_size: Vector2<f32>,
    },
}

/// UI rectangle rendering instruction.
#[derive(Clone, Debug)]
pub enum InterfaceRectangleInstruction {
    /// Solid color rectangle with optional shadow.
    Solid {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Clipping rectangle.
        screen_clip: ScreenClip,
        /// Fill color.
        color: Color,
        /// Corner radius for rounded corners.
        corner_diameter: CornerDiameter,
        /// Drop shadow color.
        shadow_color: Color,
        /// Shadow offset and blur.
        shadow_padding: ShadowPadding,
    },
    /// Textured sprite rectangle.
    Sprite {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Clipping rectangle.
        screen_clip: ScreenClip,
        /// Tint color.
        color: Color,
        /// Corner radius for rounded corners.
        corner_diameter: CornerDiameter,
        /// Source texture.
        texture: Arc<Texture>,
        /// Whether to use linear filtering.
        smooth: bool,
    },
    /// SDF (Signed Distance Field) rectangle for scalable shapes.
    Sdf {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Clipping rectangle.
        screen_clip: ScreenClip,
        /// Shape color.
        color: Color,
        /// Corner radius for rounded corners.
        corner_diameter: CornerDiameter,
        /// SDF texture.
        texture: Arc<Texture>,
    },
    /// Text glyph rectangle.
    Text {
        /// Rectangle position on screen.
        screen_position: ScreenPosition,
        /// Rectangle dimensions.
        screen_size: ScreenSize,
        /// Clipping rectangle.
        screen_clip: ScreenClip,
        /// Text color.
        color: Color,
        /// Font atlas coordinate offset.
        texture_position: Vector2<f32>,
        /// Font atlas coordinate size.
        texture_size: Vector2<f32>,
    },
}

/// Debug marker rendering instruction.
#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
pub struct MarkerInstruction {
    /// Marker position on screen.
    pub screen_position: ScreenPosition,
    /// Marker dimensions.
    pub screen_size: ScreenSize,
    /// Unique marker identifier.
    pub identifier: MarkerIdentifier,
}

/// Ground indicator instruction.
#[derive(Clone, Debug)]
pub struct IndicatorInstruction {
    /// Upper-left corner in world space.
    pub upper_left: Point3<f32>,
    /// Upper-right corner in world space.
    pub upper_right: Point3<f32>,
    /// Lower-left corner in world space.
    pub lower_left: Point3<f32>,
    /// Lower-right corner in world space.
    pub lower_right: Point3<f32>,
    /// Indicator tint color.
    pub color: Color,
    /// Indicator texture.
    pub texture: Arc<Texture>,
}

/// Batched model rendering instruction for efficient instanced rendering.
pub struct ModelBatch {
    /// Starting index in the model instruction array.
    pub offset: usize,
    /// Number of model instances in this batch.
    pub count: usize,
    /// Shared texture set for all models in batch.
    pub texture_set: Arc<TextureSet>,
    /// Shared vertex buffer for all models in batch.
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
    /// Shared index buffer for all models in batch.
    pub index_buffer: Arc<Buffer<u32>>,
}

/// Individual 3D model instance rendering instruction.
#[derive(Clone, Debug)]
pub struct ModelInstruction {
    /// Model-to-world transformation matrix.
    pub model_matrix: Matrix4<f32>,
    /// Starting index in the index buffer.
    pub index_offset: u32,
    /// Number of indices to render.
    pub index_count: u32,
    /// Base vertex offset for indexed rendering.
    pub base_vertex: i32,
    /// Index into the texture set.
    pub texture_index: i32,
    /// Distance from camera for sorting.
    pub distance: f32,
    /// Whether model uses transparency.
    pub transparent: bool,
}

/// Billboard sprite entity rendering instruction.
#[derive(Clone, Debug)]
pub struct EntityInstruction {
    /// World transformation matrix.
    pub world: Matrix4<f32>,
    /// Additional transform for sprite frame part.
    pub frame_part_transform: Matrix4<f32>,
    /// Texture coordinate offset.
    pub texture_position: Vector2<f32>,
    /// Texture coordinate size.
    pub texture_size: Vector2<f32>,
    /// Size of the sprite frame in pixels.
    pub frame_size: Vector2<f32>,
    /// Additional depth offset for layering.
    pub extra_depth_offset: f32,
    /// Base depth offset from ground.
    pub depth_offset: f32,
    /// Sprite curvature factor.
    pub curvature: f32,
    /// Sprite tint color.
    pub color: Color,
    /// Whether to mirror the sprite horizontally.
    pub mirror: bool,
    /// Entity identifier.
    pub entity_id: u32,
    /// Whether entity should be pickable with mouse.
    pub add_to_picker: bool,
    /// Sprite texture.
    pub texture: Arc<Texture>,
    /// Distance from camera for sorting.
    pub distance: f32,
}

/// 2D effect rendering instruction with custom blending.
#[derive(Clone, Debug)]
pub struct EffectInstruction {
    /// Top-left corner screen position.
    pub top_left: ScreenPosition,
    /// Bottom-left corner screen position.
    pub bottom_left: ScreenPosition,
    /// Top-right corner screen position.
    pub top_right: ScreenPosition,
    /// Bottom-right corner screen position.
    pub bottom_right: ScreenPosition,
    /// Top-left texture coordinate.
    pub texture_top_left: Vector2<f32>,
    /// Bottom-left texture coordinate.
    pub texture_bottom_left: Vector2<f32>,
    /// Top-right texture coordinate.
    pub texture_top_right: Vector2<f32>,
    /// Bottom-right texture coordinate.
    pub texture_bottom_right: Vector2<f32>,
    /// Effect tint color.
    pub color: Color,
    /// Blend factor for source color.
    pub source_blend_factor: BlendFactor,
    /// Blend factor for destination color.
    pub destination_blend_factor: BlendFactor,
    /// Effect texture.
    pub texture: Arc<Texture>,
}

/// Debug AABB (axis-aligned bounding box) wireframe rendering instruction.
#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugAabbInstruction {
    /// World transformation matrix for the bounding box.
    pub world: Matrix4<f32>,
    /// Wireframe color.
    pub color: Color,
}

/// Debug circle rendering instruction.
#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugCircleInstruction {
    /// World position of the circle center.
    pub position: Point3<f32>,
    /// Circle color.
    pub color: Color,
    /// Screen position of the circle.
    pub screen_position: ScreenPosition,
    /// Screen dimensions of the circle.
    pub screen_size: ScreenSize,
}

/// Debug rectangle wireframe rendering instruction.
#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugRectangleInstruction {
    /// World transformation matrix for the rectangle.
    pub world: Matrix4<f32>,
    /// Wireframe color.
    pub color: Color,
}
