mod deferred;
mod directional_shadow;
mod interface;
mod picker;
mod point_shadow;
mod screen;
mod selector;

use std::marker::ConstParamTy;

pub(crate) use deferred::DeferredRenderPassContext;
pub(crate) use directional_shadow::DirectionalShadowRenderPassContext;
pub(crate) use interface::InterfaceRenderPassContext;
pub(crate) use picker::PickerRenderPassContext;
pub(crate) use point_shadow::{PointShadowRenderPassContext, PointShadowRenderPassData};
pub(crate) use screen::ScreenRenderPassContext;
pub(crate) use selector::SelectorComputePassContext;
use wgpu::{BindGroupLayout, CommandEncoder, ComputePass, Device, Queue, RenderPass, TextureFormat, TextureView};

use crate::graphics::{GlobalContext, Prepare};
use crate::loaders::TextureLoader;

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum BindGroupCount {
    None = 0,
    One = 1,
}

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum ColorAttachmentCount {
    None = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum DepthAttachmentCount {
    None = 0,
    One = 1,
}

/// Gives render passes the context they need to execute. They are the owner of
/// that resources that are pass specific and shared by multiple renderer. They
/// may hold all attachments that they need to render to, but they can also
/// accept global attachments, that are part of the global context.
pub trait RenderPassContext<
    const BIND: BindGroupCount,
    const COLOR: ColorAttachmentCount,
    const DEPTH: DepthAttachmentCount,
    PassData = Option<()>,
>: Prepare
{
    /// Creates a new render pass context.
    fn new(device: &Device, queue: &Queue, texture_loader: &mut TextureLoader, global_context: &GlobalContext) -> Self;

    /// Crates a render new pass.
    fn create_pass<'encoder>(
        &mut self,
        frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: PassData,
    ) -> RenderPass<'encoder>;

    /// The bind group layout of the render pass.
    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; BIND as usize];

    /// The formats of all color attachments that this pass sets.
    fn color_attachment_formats(&self) -> [TextureFormat; COLOR as usize];

    /// The formats of all depth attachments that this pass sets.
    fn depth_attachment_output_format(&self) -> [TextureFormat; DEPTH as usize];
}

/// Gives compute passes the context they need to execute. They are the owner of
/// that resources that are pass specific and shared by multiple renderer.
pub trait ComputePassContext<const BIND: BindGroupCount, PassData = Option<()>>: Prepare {
    /// Creates a new compute pass context.
    fn new(device: &Device, queue: &Queue, global_context: &GlobalContext) -> Self;

    /// Crates a compute new pass.
    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: PassData,
    ) -> ComputePass<'encoder>;

    /// The bind group layout of the compute pass.
    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; BIND as usize];
}

pub trait Render<const BIND: BindGroupCount, const COLOR: ColorAttachmentCount, const DEPTH: DepthAttachmentCount, DrawData = Option<()>>:
    Prepare
{
    type Context: RenderPassContext<BIND, COLOR, DEPTH>;

    fn new(device: &Device, queue: &Queue, global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self;

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_dara: DrawData);
}

pub trait Compute<const BIND: BindGroupCount, DispatchData = Option<()>>: Prepare {
    type Context: ComputePassContext<BIND>;

    fn new(device: &Device, queue: &Queue, global_context: &GlobalContext, compute_pass_context: &Self::Context) -> Self;

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, draw_dara: DispatchData);
}
