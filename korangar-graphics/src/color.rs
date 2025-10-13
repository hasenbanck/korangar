use serde::{Deserialize, Serialize};

/// Represents an sRGBA color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    /// Red color component (0.0 to 1.0).
    pub red: f32,
    /// Blue color component (0.0 to 1.0).
    pub blue: f32,
    /// Green color component (0.0 to 1.0).
    pub green: f32,
    /// Alpha (opacity) component (0.0 to 1.0).
    pub alpha: f32,
}

impl Color {
    /// Pure black color (0, 0, 0) with full opacity.
    pub const BLACK: Self = Self::monochrome(0.0);
    /// Fully transparent black color.
    pub const TRANSPARENT: Self = Self::rgba_u8(0, 0, 0, 0);
    /// Pure white color (1, 1, 1) with full opacity.
    pub const WHITE: Self = Self::monochrome(1.0);

    /// Creates a new color from floating-point RGBA components.
    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    /// Creates a new color from 8-bit RGBA components (0-255 range).
    pub const fn rgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let red = (red as f32) / 255.0;
        let green = (green as f32) / 255.0;
        let blue = (blue as f32) / 255.0;
        let alpha = (alpha as f32) / 255.0;

        Self { red, green, blue, alpha }
    }

    /// Creates a grayscale color with the given brightness value and full
    /// opacity.
    pub const fn monochrome(brightness: f32) -> Self {
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 1.0,
        }
    }

    /// Converts the sRGB color into a linear representation for the shaders.
    /// Since we use pre-multiplied alpha blending, we premultiply the alpha
    /// here too.
    pub fn components_linear(self) -> [f32; 4] {
        let srgb = [self.red, self.green, self.blue];
        let linear = srgb.map(|channel| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        [linear[0] * self.alpha, linear[1] * self.alpha, linear[2] * self.alpha, self.alpha]
    }
}

/// Converts a sRGB color into an RGBA array of floats.
impl From<Color> for [f32; 4] {
    fn from(val: Color) -> Self {
        [val.red, val.green, val.blue, val.alpha]
    }
}
