use std::sync::OnceLock;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    ShaderStages, StoreOp, TextureFormat, TextureSampleType, TextureView, TextureViewDimension,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{GlobalContext, Prepare};
use crate::loaders::TextureLoader;

mod aabb;
mod ambient;
mod ambient_light;
#[cfg(feature = "debug")]
mod debug;
mod directional_light;
mod effect;
mod overlay;
mod sphere;
mod sprite;
mod water_light;

const PASS_NAME: &str = "screen render pass";

pub(crate) struct ScreenRenderPassContext {
    screen_texture_format: TextureFormat,
    bind_group: BindGroup,
}

impl Prepare for ScreenRenderPassContext {}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenRenderPassContext {
    fn new(device: &Device, _queue: &Queue, _texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self {
        let screen_texture_format = global_context.surface_texture_format;

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[0],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(global_context.diffuse_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(global_context.normal_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(global_context.water_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(global_context.depth_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(global_context.directional_shadow_map_texture.get_texture_view()),
                },
            ],
        });

        Self {
            screen_texture_format,
            bind_group,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        _global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("deferred render screen"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout = LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        });

        [layout]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.screen_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
