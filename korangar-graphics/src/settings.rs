use std::fmt::{Display, Formatter};
#[cfg(feature = "debug")]
use std::num::NonZeroU32;

use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use serde::{Deserialize, Serialize};

use crate::ScreenSize;

/// Framerate limiting configuration for controlling maximum rendering frame
/// rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum LimitFramerate {
    /// No framerate limit applied.
    Unlimited,
    /// Limit framerate to the specified value in Hz.
    Limit(u16),
}

impl DropDownItem<LimitFramerate> for LimitFramerate {
    fn text(&self) -> &str {
        match self {
            LimitFramerate::Unlimited => "Unlimited",
            LimitFramerate::Limit(30) => "30 Hz",
            LimitFramerate::Limit(60) => "60 Hz",
            LimitFramerate::Limit(120) => "120 Hz",
            LimitFramerate::Limit(144) => "144 Hz",
            LimitFramerate::Limit(240) => "240 Hz",
            LimitFramerate::Limit(_) => unimplemented!(),
        }
    }

    fn value(&self) -> LimitFramerate {
        *self
    }
}

/// Texture sampling method for filtering and interpolation during rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum TextureSamplerType {
    /// Nearest-neighbor filtering with no interpolation.
    Nearest,
    /// Linear interpolation for smooth texture filtering.
    Linear,
    /// Anisotropic filtering with the specified level for improved quality at
    /// oblique angles.
    Anisotropic(u16),
}

impl DropDownItem<TextureSamplerType> for TextureSamplerType {
    fn text(&self) -> &str {
        match self {
            TextureSamplerType::Nearest => "Nearest",
            TextureSamplerType::Linear => "Linear",
            TextureSamplerType::Anisotropic(4) => "Anisotropic x4",
            TextureSamplerType::Anisotropic(8) => "Anisotropic x8",
            TextureSamplerType::Anisotropic(16) => "Anisotropic x16",
            TextureSamplerType::Anisotropic(_) => unimplemented!(),
        }
    }

    fn value(&self) -> TextureSamplerType {
        *self
    }
}

/// Shadow map resolution quality preset for both directional and point lights.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowResolution {
    /// Standard quality shadow resolution.
    Normal,
    /// Ultra quality shadow resolution.
    Ultra,
    /// Insane quality shadow resolution.
    Insane,
}

impl ShadowResolution {
    /// Returns the shadow map resolution in pixels for directional lights.
    pub fn directional_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 2048,
            ShadowResolution::Ultra => 3072,
            ShadowResolution::Insane => 4096,
        }
    }

    /// Returns the shadow map resolution in pixels for point lights.
    pub fn point_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 128,
            ShadowResolution::Ultra => 256,
            ShadowResolution::Insane => 512,
        }
    }
}

impl DropDownItem<ShadowResolution> for ShadowResolution {
    fn text(&self) -> &str {
        match self {
            Self::Normal => "Normal",
            Self::Ultra => "Ultra",
            Self::Insane => "Insane",
        }
    }

    fn value(&self) -> ShadowResolution {
        *self
    }
}

/// Shadow rendering detail level controlling the number of shadow samples.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowDetail {
    /// Low shadow detail with fewest samples.
    Low,
    /// Medium shadow detail with balanced quality.
    Medium,
    /// High shadow detail with more samples.
    High,
    /// Maximum shadow detail with most samples.
    Ultra,
}

impl DropDownItem<ShadowDetail> for ShadowDetail {
    fn text(&self) -> &str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    fn value(&self) -> ShadowDetail {
        *self
    }
}

impl From<ShadowDetail> for u32 {
    fn from(value: ShadowDetail) -> Self {
        match value {
            ShadowDetail::Low => 1,
            ShadowDetail::Medium => 2,
            ShadowDetail::High => 3,
            ShadowDetail::Ultra => 4,
        }
    }
}

/// Shadow rendering algorithm method for different shadow edge quality.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowMethod {
    /// Hard shadows with sharp edges and no filtering.
    Hard,
    /// Soft shadows using Percentage-Closer Filtering.
    SoftPCF,
    /// Soft shadows using Percentage-Closer Soft Shadows with variable
    /// penumbra.
    SoftPCSS,
}

impl DropDownItem<ShadowMethod> for ShadowMethod {
    fn text(&self) -> &str {
        match self {
            Self::Hard => "Hard",
            Self::SoftPCF => "Soft (PCF)",
            Self::SoftPCSS => "Soft (PCSS)",
        }
    }

    fn value(&self) -> ShadowMethod {
        *self
    }
}

impl From<ShadowMethod> for u32 {
    fn from(value: ShadowMethod) -> Self {
        match value {
            ShadowMethod::Hard => 0,
            ShadowMethod::SoftPCF => 1,
            ShadowMethod::SoftPCSS => 2,
        }
    }
}

/// Multisample Anti-Aliasing (MSAA) setting for edge smoothing during
/// rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Msaa {
    /// MSAA disabled.
    Off,
    /// 2x MSAA with 2 samples per pixel.
    X2,
    /// 4x MSAA with 4 samples per pixel.
    X4,
    /// 8x MSAA with 8 samples per pixel.
    X8,
    /// 16x MSAA with 16 samples per pixel.
    X16,
}

impl DropDownItem<Msaa> for Msaa {
    fn text(&self) -> &str {
        match self {
            Msaa::Off => "Off",
            Msaa::X2 => "x2",
            Msaa::X4 => "x4",
            Msaa::X8 => "x8",
            Msaa::X16 => "x16",
        }
    }

    fn value(&self) -> Msaa {
        *self
    }
}

impl Display for Msaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Msaa::Off => "Off".fmt(f),
            Msaa::X2 => "x2".fmt(f),
            Msaa::X4 => "x4".fmt(f),
            Msaa::X8 => "x8".fmt(f),
            Msaa::X16 => "x16".fmt(f),
        }
    }
}

impl From<u32> for Msaa {
    fn from(value: u32) -> Self {
        match value {
            1 => Msaa::Off,
            2 => Msaa::X2,
            4 => Msaa::X4,
            8 => Msaa::X8,
            16 => Msaa::X16,
            _ => panic!("Unknown sample count"),
        }
    }
}

impl Msaa {
    /// Returns the number of samples per pixel for this MSAA setting.
    pub fn sample_count(self) -> u32 {
        match self {
            Msaa::Off => 1,
            Msaa::X2 => 2,
            Msaa::X4 => 4,
            Msaa::X8 => 8,
            Msaa::X16 => 16,
        }
    }

    /// Returns true if multisampling is enabled.
    pub fn multisampling_activated(self) -> bool {
        self != Msaa::Off
    }
}

/// Supersample Anti-Aliasing (SSAA) setting for rendering at higher resolution
/// then downsampling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Ssaa {
    /// SSAA disabled.
    Off,
    /// 2x SSAA rendering at square root of 2 times the resolution.
    X2,
    /// 3x SSAA rendering at square root of 3 times the resolution.
    X3,
    /// 4x SSAA rendering at double the resolution.
    X4,
}

impl DropDownItem<Ssaa> for Ssaa {
    fn text(&self) -> &str {
        match self {
            Ssaa::Off => "Off",
            Ssaa::X2 => "x2",
            Ssaa::X3 => "x3",
            Ssaa::X4 => "x4",
        }
    }

    fn value(&self) -> Ssaa {
        *self
    }
}

impl Display for Ssaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ssaa::Off => "Off".fmt(f),
            Ssaa::X2 => "x2".fmt(f),
            Ssaa::X3 => "x3".fmt(f),
            Ssaa::X4 => "x4".fmt(f),
        }
    }
}

impl Ssaa {
    /// Calculates the rendering resolution based on the base screen size and
    /// SSAA multiplier.
    pub fn calculate_size(self, base_size: ScreenSize) -> ScreenSize {
        match self {
            Ssaa::Off => base_size,
            Ssaa::X2 => base_size * f32::sqrt(2.0),
            Ssaa::X3 => base_size * f32::sqrt(3.0),
            Ssaa::X4 => base_size * 2.0,
        }
    }

    /// Returns true if supersampling is enabled.
    pub fn supersampling_activated(self) -> bool {
        self != Ssaa::Off
    }
}

/// Screen-space anti-aliasing method applied as a post-processing effect.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScreenSpaceAntiAliasing {
    /// No screen-space anti-aliasing.
    Off,
    /// Fast Approximate Anti-Aliasing for efficient edge smoothing.
    Fxaa,
}

impl DropDownItem<ScreenSpaceAntiAliasing> for ScreenSpaceAntiAliasing {
    fn text(&self) -> &str {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off",
            ScreenSpaceAntiAliasing::Fxaa => "FXAA",
        }
    }

    fn value(&self) -> ScreenSpaceAntiAliasing {
        *self
    }
}

impl Display for ScreenSpaceAntiAliasing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off".fmt(f),
            ScreenSpaceAntiAliasing::Fxaa => "FXAA".fmt(f),
        }
    }
}

/// Debug rendering options for controlling visibility and behavior of various
/// rendering features.
#[cfg(feature = "debug")]
#[derive(Copy, Clone, Default, rust_state::RustState, StateElement)]
pub struct RenderOptions {
    /// Display frames per second counter.
    pub show_frames_per_second: bool,
    /// Enable frustum culling to skip rendering off-screen objects.
    pub frustum_culling: bool,
    /// Display bounding boxes around objects.
    pub show_bounding_boxes: bool,
    /// Render the map geometry.
    pub show_map: bool,
    /// Render map objects.
    pub show_objects: bool,
    /// Render entities.
    pub show_entities: bool,
    /// Render entities with paper-style visualization.
    pub show_entities_paper: bool,
    /// Render entities with debug visualization.
    pub show_entities_debug: bool,
    /// Render water surfaces.
    pub show_water: bool,
    /// Display interaction indicators.
    pub show_indicators: bool,
    /// Enable ambient lighting.
    pub enable_ambient_lighting: bool,
    /// Enable directional lighting.
    pub enable_directional_lighting: bool,
    /// Enable point light sources.
    pub enable_point_lights: bool,
    /// Enable lighting for particle effects.
    pub enable_particle_lighting: bool,
    /// Use debug camera instead of normal camera.
    pub use_debug_camera: bool,
    /// Render geometry in wireframe mode.
    pub show_wireframe: bool,
    /// Display markers for map objects.
    pub show_object_markers: bool,
    /// Display markers for light sources.
    pub show_light_markers: bool,
    /// Display markers for sound emitters.
    pub show_sound_markers: bool,
    /// Display markers for effects.
    pub show_effect_markers: bool,
    /// Display markers for particles.
    pub show_particle_markers: bool,
    /// Display markers for entities.
    pub show_entity_markers: bool,
    /// Display markers for shadow casters.
    pub show_shadow_markers: bool,
    /// Display map tile grid.
    pub show_map_tiles: bool,
    /// Display pathfinding data.
    pub show_pathing: bool,
    /// Display the picker buffer used for object selection.
    pub show_picker_buffer: bool,
    /// Display specified directional shadow map cascade.
    pub show_directional_shadow_map: Option<NonZeroU32>,
    /// Display specified point light shadow map.
    pub show_point_shadow_map: Option<NonZeroU32>,
    /// Display light culling count buffer.
    pub show_light_culling_count_buffer: bool,
    /// Display the font atlas texture.
    pub show_font_map: bool,
    /// Display Sample Distribution Shadow Map partitions.
    pub show_sdsm_partitions: bool,
    /// Display debug info for rectangle rendering instructions.
    pub show_rectangle_instructions: bool,
    /// Display debug info for glyph rendering instructions.
    pub show_glyph_instructions: bool,
    /// Display debug info for sprite rendering instructions.
    pub show_sprite_instructions: bool,
    /// Display debug info for signed distance field rendering instructions.
    pub show_sdf_instructions: bool,
}

#[cfg(feature = "debug")]
impl RenderOptions {
    /// Creates a new RenderOptions with default debug settings.
    pub fn new() -> Self {
        Self {
            show_frames_per_second: false,
            frustum_culling: true,
            show_bounding_boxes: false,
            show_map: true,
            show_objects: true,
            show_entities: true,
            show_entities_paper: false,
            show_entities_debug: false,
            show_water: true,
            show_indicators: true,
            enable_ambient_lighting: true,
            enable_directional_lighting: true,
            enable_point_lights: true,
            enable_particle_lighting: true,
            use_debug_camera: false,
            show_wireframe: false,
            show_object_markers: false,
            show_light_markers: false,
            show_sound_markers: false,
            show_effect_markers: false,
            show_particle_markers: false,
            show_entity_markers: false,
            show_shadow_markers: false,
            show_map_tiles: false,
            show_pathing: false,
            show_picker_buffer: false,
            show_directional_shadow_map: None,
            show_point_shadow_map: None,
            show_light_culling_count_buffer: false,
            show_sdsm_partitions: false,
            show_font_map: false,
            show_rectangle_instructions: false,
            show_glyph_instructions: false,
            show_sprite_instructions: false,
            show_sdf_instructions: false,
        }
    }
}

#[cfg(feature = "debug")]
impl RenderOptions {
    /// Returns true if any debug buffer visualization is enabled.
    pub fn show_buffers(&self) -> bool {
        self.show_picker_buffer
            || self.show_directional_shadow_map.is_some()
            || self.show_point_shadow_map.is_some()
            || self.show_light_culling_count_buffer
            || self.show_sdsm_partitions
            || self.show_font_map
    }
}
