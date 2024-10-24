use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use bumpalo::Bump;
use bytemuck::{Pod, Zeroable};
use hashbrown::HashMap;
use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, Device, Features,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass,
    RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureView, TextureViewDimension,
    VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext, ScreenRenderPassContext,
};
use crate::graphics::{features_supported, Buffer, GlobalContext, Prepare, RenderInstruction, Texture, EFFECT_ATTACHMENT_BLEND};
use crate::MAX_BINDING_TEXTURE_ARRAY_COUNT;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/effect.wgsl");
const DRAWER_NAME: &str = "screen effect";
const INITIAL_INSTRUCTION_SIZE: usize = 256;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct InstanceData {
    top_left: [f32; 2],
    bottom_left: [f32; 2],
    top_right: [f32; 2],
    bottom_right: [f32; 2],
    texture_top_left: [f32; 2],
    texture_bottom_left: [f32; 2],
    texture_top_right: [f32; 2],
    texture_bottom_right: [f32; 2],
    // Needs to be stored in two arrays,
    // or else we get alignment problems.
    color0: [f32; 2],
    color1: [f32; 2],
    texture_index: i32,
    padding: u32,
}

pub(crate) struct ScreenEffectDrawer {
    solid_pixel_texture: Arc<Texture>,
    instance_data_buffer: Buffer<InstanceData>,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    draw_count: usize,
    instance_data: Vec<InstanceData>,
    bump: Bump,
    lookup: HashMap<u64, i32>,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenEffectDrawer {
    type Context = ScreenRenderPassContext;
    type DrawData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let instance_data_buffer = Buffer::with_capacity(
            device,
            format!("{DRAWER_NAME} instance data"),
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (size_of::<InstanceData>() * INITIAL_INSTRUCTION_SIZE) as _,
        );

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<InstanceData>() as _),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(MAX_BINDING_TEXTURE_ARRAY_COUNT as _),
                },
            ],
        });

        let mut texture_views = vec![global_context.solid_pixel_texture.get_texture_view()];

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_views.len()) {
                texture_views.push(texture_views[0]);
            }
        }

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &instance_data_buffer, &texture_views);

        let bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[bind_group_layouts[0], bind_group_layouts[1], &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: Some(EFFECT_ATTACHMENT_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
            cache: None,
        });

        Self {
            solid_pixel_texture: global_context.solid_pixel_texture.clone(),
            instance_data_buffer,
            bind_group_layout,
            bind_group,
            pipeline,
            draw_count: 0,
            instance_data: Vec::default(),
            bump: Bump::default(),
            lookup: HashMap::default(),
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_data: Self::DrawData<'_>) {
        if self.draw_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..6, 0..self.draw_count as u32);
    }
}

impl Prepare for ScreenEffectDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        self.draw_count = instructions.effects.len();

        if self.draw_count == 0 {
            return;
        }

        self.instance_data.clear();
        self.bump.reset();
        self.lookup.clear();

        let mut texture_views = Vec::with_capacity_in(self.draw_count, &self.bump);

        for instruction in instructions.effects.iter() {
            let mut texture_index = texture_views.len() as i32;
            let id = instruction.texture.get_texture().global_id().inner();
            let potential_index = self.lookup.get(&id);

            if let Some(potential_index) = potential_index {
                texture_index = *potential_index;
            } else {
                self.lookup.insert(id, texture_index);
                texture_views.push(instruction.texture.get_texture_view());
            }

            let color = instruction.color.components_linear();
            self.instance_data.push(InstanceData {
                top_left: instruction.top_left.into(),
                bottom_left: instruction.bottom_left.into(),
                top_right: instruction.top_right.into(),
                bottom_right: instruction.bottom_right.into(),
                texture_top_left: instruction.texture_top_left.into(),
                texture_bottom_left: instruction.texture_bottom_left.into(),
                texture_top_right: instruction.texture_top_right.into(),
                texture_bottom_right: instruction.texture_bottom_right.into(),
                color0: [color[0], color[1]],
                color1: [color[2], color[3]],
                texture_index,
                padding: 0,
            });
        }

        if texture_views.is_empty() {
            texture_views.push(self.solid_pixel_texture.get_texture_view());
        }

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_views.len()) {
                texture_views.push(texture_views[0]);
            }
        }

        self.instance_data_buffer.reserve(device, self.instance_data.len());
        self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, &self.instance_data_buffer, &texture_views)
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        self.instance_data_buffer
            .write(device, staging_belt, command_encoder, &self.instance_data);
    }
}

impl ScreenEffectDrawer {
    fn create_bind_group(
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        instance_data_buffer: &Buffer<InstanceData>,
        texture_views: &[&TextureView],
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: instance_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureViewArray(texture_views),
                },
            ],
        })
    }
}
