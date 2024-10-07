use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{GlobalContext, Prepare};
use crate::loaders::TextureLoader;

mod entity;
mod geometry;
mod indicator;

const PASS_NAME: &str = "directional shadow render pass";

pub(crate) struct DirectionalShadowRenderPassContext {
    directional_shadow_texture_format: TextureFormat,
}

impl Prepare for DirectionalShadowRenderPassContext {}

impl RenderPassContext<{ BindGroupCount::None }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }>
    for DirectionalShadowRenderPassContext
{
    fn new(_device: &Device, _queue: &Queue, _texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self {
        let directional_shadow_texture_format = global_context.directional_shadow_map_texture.get_format();

        Self {
            directional_shadow_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        _frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context.directional_shadow_map_texture.get_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.depth_texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn bind_group_layout(_device: &Device) -> [&'static BindGroupLayout; 0] {
        []
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 0] {
        []
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.directional_shadow_texture_format]
    }
}
