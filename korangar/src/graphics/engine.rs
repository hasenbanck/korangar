use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use korangar_debug::profile_block;
use wgpu::util::StagingBelt;
use wgpu::{
    Adapter, BindGroupLayout, CommandBuffer, CommandEncoder, CommandEncoderDescriptor, Device, Features, Instance, InstanceFlags, Limits,
    Maintain, MemoryHints, Queue, SurfaceTexture, TextureFormat, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::{error_handler, features_supported, set_supported_features, GlobalContext, Prepare, ShadowDetail, Surface};
use crate::graphics::instruction::{GraphicsEngineDescriptor, RenderInstruction};
use crate::graphics::passes::{
    ComputePassContext, DeferredRenderPassContext, DirectionalShadowRenderPassContext, InterfaceRenderPassContext, PickerRenderPassContext,
    PointShadowRenderPassContext, PointShadowRenderPassData, RenderPassContext, ScreenRenderPassContext, SelectorComputePassContext,
};
use crate::interface::layout::ScreenSize;
use crate::loaders::{FontLoader, TextureLoader};
use crate::{MAX_BINDING_TEXTURE_ARRAY_COUNT, NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS};

// TODO: NHA Check if we can remove the "Pipeline Bindings" stage, since that
//       could be solved with push constants.
/// Binding lifetimes:
///
/// The save default limit for bound bind-groups is 4.
///
/// Set 0: Global Bindings
///  0 => Uniform Buffer
///  1 => Nearest Sampler
///  2 => Linear Sampler
///  3 => Anisotropic Sampler
///  4 => Texture Binding Array of cached sprites
///
/// Set 1: Map Bindings
///  0 => Texture Binding Array of map textures
///
/// Set 2: Pass Bindings (For example point shadow data)
///
/// Set 3: Pipeline Bindings (For example indirection buffers)
///
/// Push Constants: Draw Data
struct GraphicsEngine {
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
    staging_belt: StagingBelt,
    surface: Option<Surface>,
    previous_surface_texture_format: Option<TextureFormat>,

    engine_context: Option<EngineContext>,

    texture_loader: TextureLoader,
    font_loader: Rc<RefCell<FontLoader>>,

    max_frame_count: usize,
    // TODO: NHA This doesn't seem to work, can we remove it?
    #[cfg(feature = "debug")]
    show_wireframe: bool,
}

struct EngineContext {
    global_context: GlobalContext,

    selector_compute_pass_context: SelectorComputePassContext,
    interface_render_pass_context: InterfaceRenderPassContext,
    picker_render_pass_context: PickerRenderPassContext,
    directional_shadow_pass_context: DirectionalShadowRenderPassContext,
    point_shadow_pass_context: PointShadowRenderPassContext,
    deferred_pass_context: DeferredRenderPassContext,
    screen_pass_context: ScreenRenderPassContext,

    copy_computer: (),
    interface_renderer: (),
    picker_renderer: (),
    directional_shadow_renderer: (),
    point_shadow_renderer: (),
    deferred_geometry_renderer: (),
    deferred_entity_renderer: (),
    ambient_light_renderer: (),
    directional_light_renderer: (),
    point_light_renderer: (),
    overlay_renderer: (),
}

impl GraphicsEngine {
    pub fn initialize(descriptor: GraphicsEngineDescriptor) -> GraphicsEngine {
        time_phase!("create adapter", {
            let trace_dir = std::env::var("WGPU_TRACE");
            let backends = wgpu::util::backend_bits_from_env().unwrap_or_default();
            let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
            let gles_minor_version = wgpu::util::gles_minor_version_from_env().unwrap_or_default();
            let flags = InstanceFlags::from_build_config().with_env();

            let instance = Instance::new(wgpu::InstanceDescriptor {
                backends,
                flags,
                dx12_shader_compiler,
                gles_minor_version,
            });

            let adapter = pollster::block_on(async { wgpu::util::initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let required_features = Features::PUSH_CONSTANTS | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
            #[cfg(feature = "debug")]
            let required_features = required_features | Features::POLYGON_MODE_LINE;

            let adapter_features = adapter.features();
            assert!(
                adapter_features.contains(required_features),
                "Adapter does not support required features: {:?}",
                required_features - adapter_features
            );
            set_supported_features(adapter_features);

            #[cfg(feature = "debug")]
            {
                let supported = match features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
                    true => "supported".green(),
                    false => "unsupported".yellow(),
                };
                print_debug!("PARTIALLY_BOUND_BINDING_ARRAY: {}", supported);
            }

            let required_limits = Limits {
                max_push_constant_size: 128,
                max_sampled_textures_per_shader_stage: u32::try_from(MAX_BINDING_TEXTURE_ARRAY_COUNT).unwrap(),
                ..Default::default()
            }
            .using_resolution(adapter.limits());

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            label: None,
                            required_features: adapter_features | required_features,
                            required_limits,
                            memory_hints: MemoryHints::Performance,
                        },
                        trace_dir.ok().as_ref().map(std::path::Path::new),
                    )
                    .await
                    .unwrap()
            });
            let device = Arc::new(device);
            let queue = Arc::new(queue);

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Box::new(error_handler));

            let staging_belt = StagingBelt::new(1024 * 1024);

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
        });

        Self {
            instance,
            adapter,
            device,
            queue,
            staging_belt,
            surface: None,
            previous_surface_texture_format: None,
            engine_context: None,
            texture_loader: descriptor.texture_loader,
            font_loader: descriptor.font_loader,
            max_frame_count: 2,
            #[cfg(feature = "debug")]
            show_wireframe: descriptor.show_wireframe,
        }
    }

    pub fn on_resume(&mut self, window: Arc<Window>, shadow_detail: &ShadowDetail) {
        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if self.surface.is_none() {
            time_phase!("create surface", {
                let inner_size: ScreenSize = window.inner_size().max(PhysicalSize::new(1, 1)).into();
                let raw_surface = self.instance.create_surface(window).unwrap();
                let surface = Surface::new(
                    &self.adapter,
                    self.device.clone(),
                    raw_surface,
                    inner_size.width as u32,
                    inner_size.height as u32,
                );
                let surface_texture_format = surface.format();

                let shadow_resolution = shadow_detail.directional_shadow_resolution();
                let shadow_resolution = ScreenSize::uniform(shadow_resolution as f32);

                if self.previous_surface_texture_format != Some(surface_texture_format) {
                    self.previous_surface_texture_format = Some(surface_texture_format);
                    self.engine_context = None;

                    time_phase!("create contexts", {
                        let global_context = GlobalContext::new(&self.device, surface_texture_format, inner_size, shadow_resolution);

                        let selector_compute_pass_context = SelectorComputePassContext::new(&self.device, &self.queue, &global_context);

                        let interface_render_pass_context =
                            InterfaceRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                        let picker_render_pass_context =
                            PickerRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                        let directional_shadow_pass_context =
                            DirectionalShadowRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                        let point_shadow_pass_context =
                            PointShadowRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                        let deferred_pass_context =
                            DeferredRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                        let screen_pass_context =
                            ScreenRenderPassContext::new(&self.device, &self.queue, &mut self.texture_loader, &global_context);
                    });

                    time_phase!("create computer and renderer", {
                        let copy_computer = TestComputer::new(&self.device, &self.queue, &global_context, &selector_compute_pass_context);

                        let interface_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &interface_render_pass_context);
                        let picker_renderer = TestRenderer::new(&self.device, &self.queue, &global_context, &picker_render_pass_context);
                        let directional_shadow_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &directional_shadow_pass_context);
                        let point_shadow_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &point_shadow_pass_context);
                        let deferred_geometry_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &deferred_pass_context);
                        let deferred_entity_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &deferred_pass_context);
                        let ambient_light_renderer = TestRenderer::new(&self.device, &self.queue, &global_context, &screen_pass_context);
                        let directional_light_renderer =
                            TestRenderer::new(&self.device, &self.queue, &global_context, &screen_pass_context);
                        let point_light_renderer = TestRenderer::new(&self.device, &self.queue, &global_context, &screen_pass_context);
                        let overlay_renderer = TestRenderer::new(&self.device, &self.queue, &global_context, &screen_pass_context);
                    });

                    self.engine_context = Some(EngineContext {
                        global_context,
                        interface_render_pass_context,
                        picker_render_pass_context,
                        directional_shadow_pass_context,
                        point_shadow_pass_context,
                        deferred_pass_context,
                        screen_pass_context,
                        interface_renderer,
                        picker_renderer,
                        directional_shadow_renderer,
                        point_shadow_renderer,
                        deferred_geometry_renderer,
                        deferred_entity_renderer,
                        ambient_light_renderer,
                        directional_light_renderer,
                        point_light_renderer,
                        overlay_renderer,
                        selector_compute_pass_context,
                        copy_computer,
                    })
                }

                self.surface = Some(surface);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "surface".magenta());
            });
        }
    }

    pub fn on_suspended(&mut self) {
        // Android devices are expected to drop their surface view.
        if cfg!(target_os = "android") {
            self.surface = None;
        }
    }

    pub fn on_resize(&mut self, screen_size: ScreenSize) {
        if let Some(surface) = self.surface.as_mut() {
            surface.update_window_size(screen_size);
        }
    }

    // TODO: NHA Expose all other settings.
    pub fn set_framerate_limit(&mut self, enabled: bool) {
        if let Some(surface) = self.surface.as_mut() {
            surface.set_frame_limit(enabled)
        }
    }

    pub fn set_shadow_detail(&mut self, shadow_detail: ShadowDetail) {
        if let Some(engine_context) = self.engine_context.as_mut() {
            let shadow_resolution = shadow_detail.directional_shadow_resolution();
            let shadow_resolution = ScreenSize::uniform(shadow_resolution as f32);
            // TODO: NHA Update all buffers that depend ton shadow_detail.
        }
    }

    #[cfg(feature = "debug")]
    pub fn set_show_wireframe(&mut self, enabled: bool) {
        self.show_wireframe = enabled;
    }

    pub fn wait_for_next_frame(&mut self) -> SurfaceTexture {
        // Before we wait for the next frame, we verify that the surface is still valid.
        if let Some(surface) = self.surface.as_mut()
            && surface.is_invalid()
        {
            #[cfg(feature = "debug")]
            profile_block!("re-create buffers");

            surface.reconfigure();
            let screen_size = surface.window_screen_size();

            if let Some(context) = self.engine_context.as_mut() {
                // TODO: NHA We need to re-create all screen-size dependent
                //       buffer.
            }
        }

        let (_, swap_chain) = self.surface.as_mut().expect("surface not set").acquire();
        swap_chain
    }

    pub fn render_next_frame(&mut self, frame: SurfaceTexture, instruction: RenderInstruction) {
        let shadow_caster_count = instruction.point_shadow_caster.len();
        assert!(shadow_caster_count < NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS);

        // Reclaim all staging buffers that the GPU has finished reading from.
        self.staging_belt.recall();

        // Calculate and stage the uploading of GPU data that is needed for the frame.
        let prepare_command_buffer = self.prepare_frame_data(instruction);

        // Record all draw commands.
        let draw_command_buffer = self.draw_frame(&frame, shadow_caster_count);

        // Queue all staging belt writes.
        self.staging_belt.finish();

        // We need to wait for the last submission to finish to be able to resolve all
        // outstanding mapping callback.
        self.device.poll(Maintain::Wait);
        self.queue.submit([prepare_command_buffer, draw_command_buffer]);

        // Schedule the presentation of the frame.
        frame.present();
    }

    fn prepare_frame_data(&mut self, instruction: RenderInstruction) -> CommandBuffer {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let context = self.engine_context.as_mut().unwrap();

        let mut visitor = PrepareVisitor {
            device: &self.device,
            staging_belt: &mut self.staging_belt,
            encoder: &mut encoder,
            instruction: &instruction,
        };

        visitor.prepare(&mut context.global_context);

        visitor.prepare(&mut context.selector_compute_pass_context);
        visitor.prepare(&mut context.interface_render_pass_context);
        visitor.prepare(&mut context.picker_render_pass_context);
        visitor.prepare(&mut context.directional_shadow_pass_context);
        visitor.prepare(&mut context.point_shadow_pass_context);
        visitor.prepare(&mut context.deferred_pass_context);
        visitor.prepare(&mut context.screen_pass_context);

        visitor.prepare(&mut context.copy_computer);
        visitor.prepare(&mut context.interface_renderer);
        visitor.prepare(&mut context.picker_renderer);
        visitor.prepare(&mut context.directional_shadow_renderer);
        visitor.prepare(&mut context.point_shadow_renderer);
        visitor.prepare(&mut context.deferred_geometry_renderer);
        visitor.prepare(&mut context.deferred_entity_renderer);
        visitor.prepare(&mut context.ambient_light_renderer);
        visitor.prepare(&mut context.directional_light_renderer);
        visitor.prepare(&mut context.point_light_renderer);
        visitor.prepare(&mut context.overlay_renderer);

        encoder.finish()
    }

    fn draw_frame(&mut self, frame: &SurfaceTexture, shadow_caster_count: usize) -> CommandBuffer {
        let frame_view = &frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let context = self.engine_context.as_mut().unwrap();

        {
            let mut render_pass =
                context
                    .interface_render_pass_context
                    .create_pass(frame_view, &mut encoder, &context.global_context, None);
            context.interface_renderer.draw(&mut render_pass, None);
        }

        {
            let mut render_pass = context
                .picker_render_pass_context
                .create_pass(frame_view, &mut encoder, &context.global_context, None);
            context.picker_renderer.draw(&mut render_pass, None);
        }

        {
            let mut compute_pass = context
                .selector_compute_pass_context
                .create_pass(&mut encoder, &context.global_context, None);
            context.copy_computer.dispatch(&mut compute_pass, None);
        }

        {
            let mut render_pass =
                context
                    .directional_shadow_pass_context
                    .create_pass(frame_view, &mut encoder, &context.global_context, None);
            context.directional_shadow_renderer.draw(&mut render_pass, None);
        }

        {
            (0..shadow_caster_count).for_each(|shadow_caster_index| {
                (0..6).for_each(|face_index| {
                    let mut render_pass = context.point_shadow_pass_context.create_pass(
                        frame_view,
                        &mut encoder,
                        &context.global_context,
                        PointShadowRenderPassData {
                            shadow_caster_index,
                            face_index,
                        },
                    );

                    // TODO: NHA Set the face index as draw data for the point_shadow_renderer.
                    context.point_shadow_renderer.draw(&mut render_pass, None);
                });
            });
        }

        {
            let mut render_pass = context
                .deferred_pass_context
                .create_pass(frame_view, &mut encoder, &context.global_context, None);
            context.deferred_geometry_renderer.draw(&mut render_pass, None);
            context.deferred_entity_renderer.draw(&mut render_pass, None);
        }

        {
            let mut render_pass = context
                .screen_pass_context
                .create_pass(frame_view, &mut encoder, &context.global_context, None);
            context.ambient_light_renderer.draw(&mut render_pass, None);
            context.directional_light_renderer.draw(&mut render_pass, None);
            context.point_light_renderer.draw(&mut render_pass, None);
            context.overlay_renderer.draw(&mut render_pass, None);
        }

        encoder.finish()
    }

    fn global_bind_group_layout() -> &'static BindGroupLayout {
        todo!()
    }

    fn map_bind_group_layout() -> &'static BindGroupLayout {
        todo!()
    }
}

struct PrepareVisitor<'a> {
    device: &'a Device,
    staging_belt: &'a mut StagingBelt,
    encoder: &'a mut CommandEncoder,
    instruction: &'a RenderInstruction<'a>,
}

impl<'a> PrepareVisitor<'a> {
    fn prepare(&mut self, context: &mut impl Prepare) {
        context.prepare(self.device, self.staging_belt, self.encoder, self.instruction);
    }
}
