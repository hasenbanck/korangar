use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use rust_state::RustState;
use wgpu::{Adapter, Device, PresentMode, SurfaceConfiguration, SurfaceTexture, TextureFormat};

use crate::ScreenSize;

/// Information about which presentation modes are supported by the surface.
#[derive(RustState, Debug, Clone, Copy)]
pub struct PresentModeInfo {
    /// Vsync On (Fast)
    pub supports_mailbox: bool,
    /// Vsync Off
    pub supports_immediate: bool,
}

impl PresentModeInfo {
    /// Creates present mode information by querying the adapter and surface
    /// capabilities.
    pub fn from_adapter(adapter: &Adapter, surface: &wgpu::Surface) -> PresentModeInfo {
        let mut present_mode_info = PresentModeInfo {
            supports_immediate: false,
            supports_mailbox: false,
        };

        surface
            .get_capabilities(adapter)
            .present_modes
            .iter()
            .for_each(|present_mode| match present_mode {
                PresentMode::Mailbox => present_mode_info.supports_mailbox = true,
                PresentMode::Immediate => present_mode_info.supports_immediate = true,
                _ => {}
            });

        present_mode_info
    }
}

/// Manages the rendering surface, including configuration and presentation
/// modes.
pub(crate) struct Surface {
    device: Device,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    present_mode_info: PresentModeInfo,
    invalid: bool,
}

impl Surface {
    /// Creates a new surface with the specified configuration and presentation
    /// settings.
    pub fn new(
        adapter: &Adapter,
        device: Device,
        surface: wgpu::Surface<'static>,
        window_width: u32,
        window_height: u32,
        triple_buffering: bool,
        vsync: bool,
    ) -> Self {
        let window_width = window_width.max(1);
        let window_height = window_height.max(1);

        let mut config = surface.get_default_config(adapter, window_width, window_height).unwrap();

        let surfaces_formats: Vec<TextureFormat> = surface.get_capabilities(adapter).formats;

        #[cfg(feature = "debug")]
        {
            print_debug!("Supported surface formats:");
            for format in &surfaces_formats {
                print_debug!("{:?}", format);
            }
        }

        let present_mode_info = PresentModeInfo::from_adapter(adapter, &surface);

        config.format = surfaces_formats.first().copied().expect("not surface formats found");
        config.desired_maximum_frame_latency = match triple_buffering {
            true => 2,
            false => 1,
        };
        config.present_mode = match vsync {
            false if present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        {
            print_debug!("Surface present mode is {:?}", config.present_mode.magenta());
            print_debug!("Surface format is {:?}", config.format);
        }

        surface.configure(&device, &config);

        Self {
            device,
            surface,
            config,
            present_mode_info,
            invalid: false,
        }
    }

    /// Acquires the next frame's surface texture for rendering.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn acquire(&mut self) -> SurfaceTexture {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            // On timeout, we will just try again.
            Err(wgpu::SurfaceError::Timeout) => self.surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                wgpu::SurfaceError::Lost
                | wgpu::SurfaceError::Other
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try.
                | wgpu::SurfaceError::OutOfMemory
                // If the surface is outdated, or was lost, reconfigure it.
                | wgpu::SurfaceError::Outdated
            ) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        };

        if frame.suboptimal {
            self.invalid = true;
        }

        frame
    }

    /// Marks the surface as invalid, requiring reconfiguration.
    pub fn invalidate(&mut self) {
        self.invalid = true;
    }

    /// Returns whether the surface is invalid and needs reconfiguration.
    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    /// Reconfigures the surface with the current settings and clears the
    /// invalid flag.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn reconfigure(&mut self) {
        self.invalid = false;
        self.surface.configure(&self.device, &self.config);
    }

    /// Enables or disables vsync and marks the surface for reconfiguration.
    pub fn set_vsync(&mut self, enabled: bool) {
        self.config.present_mode = match enabled {
            false if self.present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if self.present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        print_debug!("set surface present mode to {:?}", self.config.present_mode.magenta());

        self.invalidate();
    }

    /// Enables or disables triple buffering by adjusting frame latency.
    pub fn set_triple_buffering(&mut self, enabled: bool) {
        self.config.desired_maximum_frame_latency = match enabled {
            true => 2,
            false => 1,
        };
    }

    /// Updates the window size and marks the surface for reconfiguration.
    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.config.width = window_size.width as u32;
        self.config.height = window_size.height as u32;
        self.invalidate();
    }

    /// Returns information about supported presentation modes.
    pub fn present_mode_info(&self) -> PresentModeInfo {
        self.present_mode_info
    }

    /// Returns the texture format used by the surface.
    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    /// Returns the window size as a 2D vector in pixels.
    pub fn window_size(&self) -> Vector2<usize> {
        Vector2 {
            x: self.config.width as usize,
            y: self.config.height as usize,
        }
    }

    /// Returns the window size as a ScreenSize.
    pub fn window_screen_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.config.width as f32,
            height: self.config.height as f32,
        }
    }
}
