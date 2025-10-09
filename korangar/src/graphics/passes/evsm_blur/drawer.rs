use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, TextureFormat, TextureSampleType, TextureViewDimension,
    VertexState,
};

use super::EvsmBlurRenderPassContext;
use crate::graphics::passes::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const DRAWER_NAME: &str = "EVSM blur";

#[derive(Copy, Clone)]
pub(crate) enum EvsmBlurDirection {
    Horizontal,
    Vertical,
}

pub(crate) struct EvsmBlurDrawData<'a> {
    pub(crate) direction: EvsmBlurDirection,
    pub(crate) input_bind_group: &'a wgpu::BindGroup,
}

pub(crate) struct EvsmBlurDrawer {
    horizontal_pipeline: RenderPipeline,
    vertical_pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for EvsmBlurDrawer {
    type Context = EvsmBlurRenderPassContext;
    type DrawData<'data> = EvsmBlurDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        _render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("evsm_blur", "blur");

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let input_texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureViewDimension::D2,
            TextureSampleType::Float { filterable: true },
            false,
        );

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], &input_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create horizontal pipeline with IS_HORIZONTAL = true
        let horizontal_constants = &[
            // IS_HORIZONTAL
            ("0", 1.0),
        ];

        let horizontal_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{DRAWER_NAME} horizontal")),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: horizontal_constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: horizontal_constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create vertical pipeline with IS_HORIZONTAL = false
        let vertical_constants = &[
            // IS_HORIZONTAL
            ("0", 0.0),
        ];

        let vertical_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&format!("{DRAWER_NAME} vertical")),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: vertical_constants,
                    zero_initialize_workgroup_memory: false,
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: vertical_constants,
                    zero_initialize_workgroup_memory: false,
                },
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            horizontal_pipeline,
            vertical_pipeline,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        // Select pipeline based on direction
        let pipeline = match draw_data.direction {
            EvsmBlurDirection::Horizontal => &self.horizontal_pipeline,
            EvsmBlurDirection::Vertical => &self.vertical_pipeline,
        };

        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, draw_data.input_bind_group, &[]);
        // Draw 3 vertices (fullscreen triangle)
        pass.draw(0..3, 0..1);
    }
}
