use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, StencilState, TextureSampleType, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DirectionalShadowRenderPassContext, Drawer, RenderPassContext,
};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/filter.wgsl");
const DRAWER_NAME: &str = "filter";

pub(crate) struct DirectionalShadowFilterDrawData<'a> {
    pub(crate) source_texture: &'a AttachmentTexture,
    pub(crate) is_horizontal: bool,
}

pub(crate) struct DirectionalShadowFilterDrawer {
    horizontal_pipeline: RenderPipeline,
    vertical_pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for DirectionalShadowFilterDrawer {
    type Context = DirectionalShadowRenderPassContext;
    type DrawData<'data> = DirectionalShadowFilterDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Self::Context::bind_group_layout(device)[0],
                Self::Context::bind_group_layout(device)[1],
                AttachmentTexture::bind_group_layout(device, TextureSampleType::Float { filterable: true }, false),
            ],
            push_constant_ranges: &[],
        });

        let horizontal_pipeline = Self::create_pipeline(device, &render_pass_context, &shader_module, &pipeline_layout, true);
        let vertical_pipeline = Self::create_pipeline(device, &render_pass_context, &shader_module, &pipeline_layout, false);

        Self {
            vertical_pipeline,
            horizontal_pipeline,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        match draw_data.is_horizontal {
            true => pass.set_pipeline(&self.horizontal_pipeline),
            false => pass.set_pipeline(&self.vertical_pipeline),
        }
        pass.set_bind_group(2, draw_data.source_texture.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}

impl DirectionalShadowFilterDrawer {
    fn create_pipeline(
        device: &Device,
        render_pass_context: &&DirectionalShadowRenderPassContext,
        shader_module: &ShaderModule,
        pipeline_layout: &PipelineLayout,
        is_horizontal: bool,
    ) -> RenderPipeline {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: match is_horizontal {
                    true => Some("fs_horizontal"),
                    false => Some("fs_vertical"),
                },
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: None,
                    write_mask: ColorWrites::RED,
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        })
    }
}
