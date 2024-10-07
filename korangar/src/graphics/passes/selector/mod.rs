mod copy;

use std::sync::OnceLock;

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, CommandEncoder, ComputePass, ComputePassDescriptor, Device, Queue, ShaderStages,
};

use super::{BindGroupCount, ComputePassContext};
use crate::graphics::{GlobalContext, Prepare};

const PASS_NAME: &str = "selector compute pass";

pub(crate) struct SelectorComputePassContext {
    bind_group: BindGroup,
}

impl Prepare for SelectorComputePassContext {}

impl ComputePassContext<{ BindGroupCount::One }> for SelectorComputePassContext {
    fn new(device: &Device, _queue: &Queue, global_context: &GlobalContext) -> Self {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::bind_group_layout(device)[0],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: global_context.picker_value_buffer.as_entire_binding(),
            }],
        });

        Self { bind_group }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        _global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> ComputePass<'encoder> {
        encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some(PASS_NAME),
            timestamp_writes: None,
        })
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout = LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
        });

        [layout]
    }
}
