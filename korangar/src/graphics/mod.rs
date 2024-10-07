mod buffer;
mod cameras;
mod color;
mod engine;
#[cfg(feature = "debug")]
mod error;
mod features;
mod graphic_settings;
mod instruction;
mod particles;
mod passes;
#[cfg(feature = "debug")]
mod render_settings;
mod renderers;
mod sampler;
mod smoothed;
mod surface;
mod texture;
mod vertices;

use wgpu::util::StagingBelt;
use wgpu::{BufferUsages, CommandEncoder, Device, TextureFormat};

pub use self::cameras::*;
pub use self::color::*;
#[cfg(feature = "debug")]
pub use self::error::error_handler;
pub use self::features::*;
pub use self::graphic_settings::*;
pub use self::instruction::*;
pub use self::particles::*;
#[cfg(feature = "debug")]
pub use self::render_settings::*;
// TODO: NHA remove once reworked.
pub use self::renderers::*;
pub use self::smoothed::*;
pub use self::texture::*;
pub use self::vertices::*;
use crate::interface::layout::ScreenSize;
use crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS;

/// Trait to prepare all GPU data of contexts, computer and renderer.
pub(crate) trait Prepare {
    /// Prepares the GPU data and stages those inside the staging belt.
    fn prepare(
        &mut self,
        _device: &Device,
        _staging_belt: &mut StagingBelt,
        _command_encoder: &mut CommandEncoder,
        _instructions: &RenderInstruction,
    ) {
        // Fallback that does nothing. We call all prepare functions, for
        // consistency. Empty calls will get optimized by the compiler.
    }
}

/// Holds all GPU resources that are shared by multiple passes.
pub struct GlobalContext {
    pub depth_texture: Texture,
    pub picker_buffer_texture: Texture,
    pub diffuse_buffer_texture: Texture,
    pub normal_buffer_texture: Texture,
    pub water_buffer_texture: Texture,
    pub depth_buffer_texture: Texture,
    pub interface_buffer_texture: Texture,
    pub directional_shadow_map_texture: Texture,
    pub point_shadow_map_textures: [CubeTexture; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS],
    pub picker_value_buffer: Buffer<u32>,
    pub surface_texture_format: TextureFormat,
}

impl Prepare for GlobalContext {}

// TODO: NHA Create function to re-create resolution and shadow resolution
//       dependent textures.
impl GlobalContext {
    fn new(device: &Device, surface_texture_format: TextureFormat, screen_size: ScreenSize, shadow_resolution: ScreenSize) -> Self {
        let screen_factory = TextureFactory::new(device, screen_size, 1);
        let depth_texture = screen_factory.new_texture("depth", TextureFormat::Depth32Float, TextureType::Depth);
        let picker_buffer_texture = screen_factory.new_texture("picker buffer", TextureFormat::R32Uint, TextureType::ColorAttachment);

        let screen_ms_factory = TextureFactory::new(device, screen_size, 4);
        let diffuse_buffer_texture =
            screen_ms_factory.new_texture("diffuse buffer", TextureFormat::Rgba8UnormSrgb, TextureType::ColorAttachment);
        let normal_buffer_texture =
            screen_ms_factory.new_texture("normal buffer", TextureFormat::Rgba16Float, TextureType::ColorAttachment);
        let water_buffer_texture =
            screen_ms_factory.new_texture("water buffer", TextureFormat::Rgba8UnormSrgb, TextureType::ColorAttachment);
        let depth_buffer_texture = screen_ms_factory.new_texture("depth buffer", TextureFormat::Depth32Float, TextureType::DepthAttachment);
        let interface_buffer_texture =
            screen_ms_factory.new_texture("interface buffer", TextureFormat::Rgba8UnormSrgb, TextureType::ColorAttachment);

        let shadow_factory = TextureFactory::new(device, shadow_resolution, 1);
        let directional_shadow_map_texture = shadow_factory.new_texture(
            "directional shadow map",
            TextureFormat::Rgba32Float,
            TextureType::DepthAttachment,
        );
        let point_shadow_map_textures = [
            shadow_factory.new_cube_texture("point shadow map 0", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
            shadow_factory.new_cube_texture("point shadow map 1", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
            shadow_factory.new_cube_texture("point shadow map 2", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
            shadow_factory.new_cube_texture("point shadow map 3", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
            shadow_factory.new_cube_texture("point shadow map 4", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
            shadow_factory.new_cube_texture("point shadow map 5", TextureFormat::Rgba32Float, TextureType::DepthAttachment),
        ];

        let picker_value_buffer = Buffer::with_capacity(
            device,
            "picker value",
            BufferUsages::STORAGE | BufferUsages::MAP_READ,
            picker_buffer_texture.get_format().target_pixel_byte_cost().unwrap() as _,
        );

        Self {
            depth_texture,
            picker_buffer_texture,
            diffuse_buffer_texture,
            normal_buffer_texture,
            water_buffer_texture,
            depth_buffer_texture,
            interface_buffer_texture,
            directional_shadow_map_texture,
            point_shadow_map_textures,
            picker_value_buffer,
            surface_texture_format,
        }
    }
}
