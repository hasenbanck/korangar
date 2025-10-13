//! Commonly used color functionality.

use std::ops::{Add, Mul, Sub};

use fast_srgb8::{f32_to_srgb8, srgb8_to_f32};
use korangar_graphics::{CornerDiameter, ShadowPadding};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element, StateElement};
use korangar_interface::layout::{Resolver, WindowLayout};
use mlua::{Lua, Value};
use ragnarok_formats::color::{ColorBGRA, ColorRGB};
use rust_state::Context;
use serde::{Deserialize, Serialize};

use crate::state::ClientState;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    pub red: f32,
    pub blue: f32,
    pub green: f32,
    pub alpha: f32,
}

impl Color {
    pub const BLACK: Self = Self::monochrome(0.0);
    pub const TRANSPARENT: Self = Self::rgba_u8(0, 0, 0, 0);
    pub const WHITE: Self = Self::monochrome(1.0);

    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    pub const fn rgb_u8(red: u8, green: u8, blue: u8) -> Self {
        let red = (red as f32) / 255.0;
        let green = (green as f32) / 255.0;
        let blue = (blue as f32) / 255.0;

        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub const fn rgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let red = (red as f32) / 255.0;
        let green = (green as f32) / 255.0;
        let blue = (blue as f32) / 255.0;
        let alpha = (alpha as f32) / 255.0;

        Self { red, green, blue, alpha }
    }

    pub fn rgb_hex(hex: &str) -> Self {
        assert_eq!(hex.len(), 6);

        let channel = |range| u8::from_str_radix(&hex[range], 16).unwrap();
        Color::rgb_u8(channel(0..2), channel(2..4), channel(4..6))
    }

    pub const fn monochrome(brightness: f32) -> Self {
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 1.0,
        }
    }

    pub fn monochrome_u8(brightness: u8) -> Self {
        let brightness = (brightness as f32) / 255.0;
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 1.0,
        }
    }

    pub fn red_as_u8(&self) -> u8 {
        (self.red * 255.0) as u8
    }

    pub fn green_as_u8(&self) -> u8 {
        (self.green * 255.0) as u8
    }

    pub fn blue_as_u8(&self) -> u8 {
        (self.blue * 255.0) as u8
    }

    pub fn alpha_as_u8(&self) -> u8 {
        (self.alpha * 255.0) as u8
    }

    #[cfg(feature = "debug")]
    pub const fn multiply_alpha(mut self, alpha: f32) -> Self {
        self.alpha *= alpha;
        self
    }

    pub const fn invert(&self) -> Self {
        Self::rgba(1.0 - self.red, 1.0 - self.blue, 1.0 - self.green, self.alpha)
    }

    pub fn shade(&self) -> Self {
        match (self.red_as_u8() as usize) + (self.green_as_u8() as usize) + (self.blue_as_u8() as usize) > 382 {
            true => Self::rgba_u8(
                self.red_as_u8().saturating_sub(40),
                self.green_as_u8().saturating_sub(40),
                self.blue_as_u8().saturating_sub(40),
                self.alpha_as_u8(),
            ),
            false => Self::rgba_u8(
                self.red_as_u8().saturating_add(40),
                self.green_as_u8().saturating_add(40),
                self.blue_as_u8().saturating_add(40),
                self.alpha_as_u8(),
            ),
        }
    }
}

impl From<Color> for cosmic_text::Color {
    fn from(value: Color) -> Self {
        Self::rgba(value.red_as_u8(), value.green_as_u8(), value.blue_as_u8(), value.alpha_as_u8())
    }
}

impl From<Color> for korangar_graphics::Color {
    fn from(value: Color) -> Self {
        Self {
            red: value.red,
            blue: value.blue,
            green: value.green,
            alpha: value.alpha,
        }
    }
}

impl From<korangar_graphics::Color> for Color {
    fn from(value: korangar_graphics::Color) -> Self {
        Self {
            red: value.red,
            blue: value.blue,
            green: value.green,
            alpha: value.alpha,
        }
    }
}

impl From<cosmic_text::Color> for Color {
    fn from(value: cosmic_text::Color) -> Self {
        Self::rgba_u8(value.r(), value.g(), value.b(), value.a())
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            red: self.red + rhs.red,
            blue: self.blue + rhs.blue,
            green: self.green + rhs.green,
            alpha: self.alpha + rhs.alpha,
        }
    }
}

impl Sub for Color {
    type Output = Color;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            red: self.red - rhs.red,
            blue: self.blue - rhs.blue,
            green: self.green - rhs.green,
            alpha: self.alpha - rhs.alpha,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            red: self.red * rhs,
            blue: self.blue * rhs,
            green: self.green * rhs,
            alpha: self.alpha * rhs,
        }
    }
}

impl From<Color> for [f32; 3] {
    fn from(val: Color) -> Self {
        [val.red, val.green, val.blue]
    }
}

impl From<Color> for [f32; 4] {
    fn from(val: Color) -> Self {
        [val.red, val.green, val.blue, val.alpha]
    }
}

impl From<ColorRGB> for Color {
    fn from(value: ColorRGB) -> Self {
        let ColorRGB { red, blue, green } = value;
        Color {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }
}

impl From<ColorBGRA> for Color {
    fn from(value: ColorBGRA) -> Self {
        let ColorBGRA { red, blue, green, alpha } = value;
        Color::rgba_u8(red, green, blue, alpha)
    }
}

impl mlua::FromLua for Color {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        if let Value::Table(table) = value {
            // Robust color parsing in case color values are not in u8 range.
            let mut sequence = table.sequence_values::<i64>();
            let r = i64::clamp(sequence.next().unwrap_or(Ok(0))?, 0, 255) as f32;
            let g = i64::clamp(sequence.next().unwrap_or(Ok(0))?, 0, 255) as f32;
            let b = i64::clamp(sequence.next().unwrap_or(Ok(0))?, 0, 255) as f32;
            return Ok(Color::rgb(r / 255.0, g / 255.0, b / 255.0));
        }

        Err(mlua::Error::FromLuaConversionError {
            from: "Table",
            to: "Color".to_string(),
            message: Some("Could not convert color".to_string()),
        })
    }
}

impl StateElement<ClientState> for Color {
    type LayoutInfo = impl std::any::Any;
    type LayoutInfoMut = impl std::any::Any;
    type Return<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfo>
    where
        P: rust_state::Path<ClientState, Self>;
    type ReturnMut<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfoMut>
    where
        P: rust_state::Path<ClientState, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        struct Inner<P> {
            path: P,
        }

        impl<P> Element<ClientState> for Inner<P>
        where
            P: rust_state::Path<ClientState, Color>,
        {
            type LayoutInfo = BaseLayoutInfo;

            fn create_layout_info(
                &mut self,
                _: &Context<ClientState>,
                _: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, ClientState>,
            ) -> Self::LayoutInfo {
                let area = resolver.with_height(18.0);

                Self::LayoutInfo { area }
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(5.0),
                    *state.get(&self.path),
                    Color::TRANSPARENT,
                    ShadowPadding::uniform(0.0),
                );
            }
        }

        split! {
            children: (
                text! {
                    text: name,
                },
                Inner { path: self_path }
            ),
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        struct Inner<P> {
            path: P,
        }

        impl<P> Element<ClientState> for Inner<P>
        where
            P: rust_state::Path<ClientState, Color>,
        {
            type LayoutInfo = BaseLayoutInfo;

            fn create_layout_info(
                &mut self,
                _: &Context<ClientState>,
                _: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, ClientState>,
            ) -> Self::LayoutInfo {
                let area = resolver.with_height(18.0);

                Self::LayoutInfo { area }
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(5.0),
                    *state.get(&self.path),
                    Color::TRANSPARENT,
                    ShadowPadding::uniform(0.0),
                );
            }
        }

        split! {
            children: (
                text! {
                    text: name,
                },
                Inner { path: self_path }
            ),
        }
    }
}

/// Pre-multiplies the alpha of a sRGB gamma encoded pixel.
pub fn premultiply_alpha(srgba_bytes: &mut [u8]) {
    srgba_bytes.chunks_exact_mut(4).for_each(|chunk| {
        let mut red = srgb8_to_f32(chunk[0]);
        let mut green = srgb8_to_f32(chunk[1]);
        let mut blue = srgb8_to_f32(chunk[2]);
        let alpha = chunk[3] as f32 / 255.0;

        red *= alpha;
        blue *= alpha;
        green *= alpha;

        chunk[0] = f32_to_srgb8(red);
        chunk[1] = f32_to_srgb8(green);
        chunk[2] = f32_to_srgb8(blue);
    });
}

/// Returns `true` if the sRGBA gamma encoded pixel contains a pixel that has an
/// alpha value neither 0 nor 255.
pub fn contains_transparent_pixel(srgba_bytes: &[u8]) -> bool {
    srgba_bytes.chunks_exact(4).any(|bytes| bytes[3] != 0 && bytes[3] != 255)
}
