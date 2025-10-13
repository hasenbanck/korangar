use cgmath::Vector3;
#[cfg(feature = "debug")]
use korangar_graphics::MarkerIdentifier;
use ragnarok_formats::map::SoundSource;

#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::Camera;

pub trait SoundSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl SoundSourceExt for SoundSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}
