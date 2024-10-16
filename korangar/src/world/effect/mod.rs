mod lookup;

use cgmath::Vector3;
use ragnarok_formats::map::EffectSource;

#[cfg(feature = "debug")]
use crate::graphics::Camera;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub trait EffectSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl EffectSourceExt for EffectSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}
