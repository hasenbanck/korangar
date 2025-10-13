#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MarkerIdentifier {
    Object(u32),
    LightSource(u32),
    SoundSource(u32),
    EffectSource(u32),
    Particle(u16, u16),
    Entity(u32),
    Shadow(u32),
}

#[cfg(feature = "debug")]
impl MarkerIdentifier {
    pub const SIZE: f32 = 1.5;
}
