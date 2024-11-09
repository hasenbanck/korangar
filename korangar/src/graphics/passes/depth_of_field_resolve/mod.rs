mod blitter;

pub(crate) use blitter::{DepthOfFieldResolveBlitterDrawData, DepthOfFieldResolveBlitterDrawer};
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{AttachmentTexture, GlobalContext};
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "depth of field resolve render pass";

pub(crate) struct DepthOfFieldResolveRenderPassData<'a> {
    pub(crate) color_attachment_texture: &'a AttachmentTexture,
    pub(crate) depth_attachment_texture: &'a AttachmentTexture,
}

pub(crate) struct DepthOfFieldResolveRenderPassContext {
    color_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }>
    for DepthOfFieldResolveRenderPassContext
{
    type PassData<'data> = DepthOfFieldResolveRenderPassData<'data>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        Self {
            color_texture_format: global_context.forward_color_texture.get_format(),
            depth_texture_format: global_context.forward_depth_texture.get_format(),
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        _global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: pass_data.color_attachment_texture.get_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: pass_data.color_attachment_texture.get_texture_view(),
                depth_ops: None,
                stencil_ops: Some(Operations {
                    load: LoadOp::Clear(0),
                    store: StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    fn bind_group_layout(_device: &Device) -> [&'static BindGroupLayout; 0] {
        []
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.color_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_texture_format]
    }
}
