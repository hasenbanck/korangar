use wgpu::{
    BindGroupLayout, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{GlobalContext, PickerTarget, Prepare};
use crate::loaders::TextureLoader;

mod entity;
mod marker;
mod tile;

const PASS_NAME: &str = "picker render pass";

pub(crate) struct PickerRenderPassContext {
    picker_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
}

impl Prepare for PickerRenderPassContext {}

impl RenderPassContext<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for PickerRenderPassContext {
    fn new(_device: &Device, _queue: &Queue, _texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self {
        let picker_texture_format = global_context.picker_buffer_texture.get_format();
        let depth_texture_format = global_context.depth_texture.get_format();

        Self {
            picker_texture_format,
            depth_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        _frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let (clear_high, clear_low) = <(u32, u32)>::from(PickerTarget::Nothing);
        let clear_color = wgpu::Color {
            r: f64::from(clear_high),
            g: f64::from(clear_low),
            b: 0.0,
            a: 0.0,
        };

        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context.picker_buffer_texture.get_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color),
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

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.picker_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_texture_format]
    }
}
