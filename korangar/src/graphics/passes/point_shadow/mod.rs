mod entity;
mod geometry;
mod indicator;

use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};

use bytemuck::{Pod, Zeroable};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferUsages, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp, TextureFormat, TextureSampleType, TextureView,
    TextureViewDimension,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{Buffer, GlobalContext, Prepare, RenderInstruction, Texture};
use crate::loaders::TextureLoader;
use crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS;

const PASS_NAME: &str = "point shadow render pass";

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct PointShadowUniforms {
    view_projection_matrices: [[[f32; 4]; 4]; 6],
    position: [f32; 4],
}

#[derive(Copy, Clone)]
pub(crate) struct PointShadowRenderPassData {
    pub(crate) shadow_caster_index: usize,
    pub(crate) face_index: usize,
}

pub(crate) struct PointShadowRenderPassContext {
    walk_indicator: Arc<Texture>,
    point_shadow_texture_format: TextureFormat,
    buffer: Buffer<PointShadowUniforms>,
    bind_group: BindGroup,
    uniforms_buffer: Vec<PointShadowUniforms>,
}

impl Prepare for PointShadowRenderPassContext {
    fn prepare(
        &mut self,
        device: &Device,
        staging_belt: &mut StagingBelt,
        command_encoder: &mut CommandEncoder,
        instructions: &RenderInstruction,
    ) {
        self.uniforms_buffer.clear();
        instructions.uniforms.point_shadow_caster.iter().for_each(|caster| {
            let uniform = PointShadowUniforms {
                view_projection_matrices: [
                    caster.view_projection_matrices[0].into(),
                    caster.view_projection_matrices[1].into(),
                    caster.view_projection_matrices[2].into(),
                    caster.view_projection_matrices[3].into(),
                    caster.view_projection_matrices[4].into(),
                    caster.view_projection_matrices[5].into(),
                ],
                position: caster.position.to_homogeneous().into(),
            };
            self.uniforms_buffer.push(uniform);
        });
        self.buffer.write(device, staging_belt, command_encoder, &self.uniforms_buffer);

        // TODO: NHA Do we need to re-create this?
        self.bind_group = Self::create_bind_group(device, &self.buffer);
    }
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::None }, { DepthAttachmentCount::One }, PointShadowRenderPassData>
    for PointShadowRenderPassContext
{
    fn new(device: &Device, _queue: &Queue, texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self {
        let walk_indicator = texture_loader.get("grid.tga").unwrap();
        let point_shadow_texture_format = global_context.point_shadow_map_textures[0].get_texture_format();

        let buffer = Buffer::with_capacity(
            device,
            PASS_NAME,
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            (size_of::<PointShadowUniforms>() * NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS) as _,
        );

        let bind_group = Self::create_bind_group(device, &buffer);
        let uniforms_buffer = Vec::with_capacity(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS);

        Self {
            walk_indicator,
            point_shadow_texture_format,
            buffer,
            bind_group,
            uniforms_buffer,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        _frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: PointShadowRenderPassData,
    ) -> RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context.point_shadow_map_textures[pass_data.shadow_caster_index].get_texture_face_view(pass_data.face_index),
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
                    visibility: ShaderStages::VERTEX,
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

    fn color_attachment_formats(&self) -> [TextureFormat; 0] {
        []
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.point_shadow_texture_format]
    }
}

impl PointShadowRenderPassContext {
    fn create_bind_group(device: &Device, buffer: &Buffer<PointShadowUniforms>) -> BindGroup {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[0],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        bind_group
    }
}
