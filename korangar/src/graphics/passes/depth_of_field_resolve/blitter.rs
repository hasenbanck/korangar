use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, StencilState, TextureSampleType, VertexState,
};

use crate::graphics::passes::depth_of_field_resolve::DepthOfFieldResolveRenderPassContext;
use crate::graphics::passes::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/blitter.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("shader/blitter_msaa.wgsl");
const DRAWER_NAME: &str = "depth of field blitter";

pub(crate) struct DepthOfFieldResolveBlitterDrawData<'a> {
    pub(crate) color_attachment_texture: &'a AttachmentTexture,
    pub(crate) depth_attachment_texture: &'a AttachmentTexture,
}

pub(crate) struct DepthOfFieldResolveBlitterDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for DepthOfFieldResolveBlitterDrawer {
    type Context = DepthOfFieldResolveRenderPassContext;
    type DrawData<'data> = DepthOfFieldResolveBlitterDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let color_texture_format = render_pass_context.color_attachment_formats()[0];
        let depth_texture_format = render_pass_context.depth_attachment_output_format()[0];

        let shader_module = match global_context.msaa.multisampling_activated() {
            true => device.create_shader_module(SHADER_MSAA),
            false => device.create_shader_module(SHADER),
        };

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(device, TextureSampleType::Float { filterable: true }, false);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut constants = std::collections::HashMap::new();
        constants.insert("SAMPLE_COUNT".to_string(), f64::from(global_context.msaa.sample_count()));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &[Some(ColorTargetState {
                    format: color_texture_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: depth_texture_format,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Always,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: if global_context.msaa.multisampling_activated() {
                MultisampleState {
                    count: global_context.msaa.sample_count(),
                    mask: 0,
                    alpha_to_coverage_enabled: false,
                }
            } else {
                MultisampleState::default()
            },
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, draw_data.color_attachment_texture.get_bind_group(), &[]);
        pass.set_bind_group(1, draw_data.depth_attachment_texture.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}
