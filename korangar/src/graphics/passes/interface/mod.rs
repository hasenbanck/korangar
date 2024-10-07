use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp, TextureFormat, TextureSampleType, TextureView,
    TextureViewDimension,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{GlobalContext, Prepare, Texture};
use crate::loaders::TextureLoader;

mod rectangle;
mod sprite;
mod text;

const PASS_NAME: &str = "interface render pass";

pub(crate) struct InterfaceRenderPassContext {
    checked_box_texture: Arc<Texture>,
    unchecked_box_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    interface_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
    bind_group: BindGroup,
}

impl Prepare for InterfaceRenderPassContext {}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }>
    for InterfaceRenderPassContext
{
    fn new(device: &Device, _queue: &Queue, texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self {
        let checked_box_texture = texture_loader.get("checked_box.png").unwrap();
        let unchecked_box_texture = texture_loader.get("unchecked_box.png").unwrap();
        let expanded_arrow_texture = texture_loader.get("expanded_arrow.png").unwrap();
        let collapsed_arrow_texture = texture_loader.get("collapsed_arrow.png").unwrap();

        let interface_texture_format = global_context.interface_buffer_texture.get_format();
        let depth_texture_format = global_context.depth_texture.get_format();

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[0],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureViewArray(&[
                    checked_box_texture.get_texture_view(),
                    unchecked_box_texture.get_texture_view(),
                    expanded_arrow_texture.get_texture_view(),
                    collapsed_arrow_texture.get_texture_view(),
                ]),
            }],
        });

        Self {
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            interface_texture_format,
            depth_texture_format,
            bind_group,
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
                view: global_context.interface_buffer_texture.get_texture_view(),
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

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout = LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(4),
                }],
            })
        });

        [layout]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.interface_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_texture_format]
    }
}
