use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector2, Vector3};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer};
use korangar_util::collision::{Frustum, Sphere};
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::effect::{EffectData, Frame};
use ragnarok_formats::version::InternalVersion;
use ragnarok_packets::EntityId;

use super::error::LoadError;
use super::TextureLoader;
use crate::graphics::{Camera, Color, Texture};
use crate::loaders::GameFileLoader;
use crate::renderer::EffectRenderer;
use crate::{point_light_extent, PointLightId, PointLightManager};

fn ease_interpolate(start_value: f32, end_value: f32, time: f32, bias: f32, sub_multiplier: f32) -> f32 {
    if bias > 0.0 {
        (end_value - start_value) * time.powf(1.0 + bias / 5.0) + start_value * sub_multiplier
    } else if bias < 0.0 {
        (end_value - start_value) * (1.0 - (1.0 - time).powf(-bias / 5.0 + 1.0)) + start_value * sub_multiplier
    } else {
        (end_value - start_value) * time + start_value * sub_multiplier
    }
}

pub fn interpolate(first: &Frame, second: &Frame, frame_index: usize) -> Frame {
    let time = 1.0 / (second.frame_index as f32 - first.frame_index as f32) * (frame_index as f32 - first.frame_index as f32);
    let sub_mult = 1.0;

    // TODO: angle bias
    let angle = ease_interpolate(first.angle, second.angle, time, 0.0, sub_mult);
    let color = [
        (second.color[0] - first.color[0]) * time + first.color[0] * sub_mult,
        (second.color[1] - first.color[1]) * time + first.color[1] * sub_mult,
        (second.color[2] - first.color[2]) * time + first.color[2] * sub_mult,
        (second.color[3] - first.color[3]) * time + first.color[3] * sub_mult,
    ];

    let uv = (0..8)
        .map(|index| (second.uv[index] - first.uv[index]) * time + first.uv[index] * sub_mult)
        .next_chunk()
        .unwrap();

    // TODO: scale bias
    let xy = (0..8)
        .map(|index| ease_interpolate(first.xy[index], second.xy[index], time, 0.0, sub_mult))
        .next_chunk()
        .unwrap();

    // TODO: additional logic for animation type 2 and 3
    let texture_index = first.texture_index;

    // TODO: bezier curves
    let offset_x = (second.offset.x - first.offset.x) * time + first.offset.x * sub_mult;
    let offset_y = (second.offset.y - first.offset.y) * time + first.offset.y * sub_mult;

    Frame {
        frame_index: frame_index as i32,
        frame_type: first.frame_type,
        offset: Vector2::new(offset_x, offset_y),
        uv,
        xy,
        texture_index,
        animation_type: first.animation_type,
        delay: first.delay,
        angle,
        color,
        source_alpha: first.source_alpha,
        destination_alpha: first.destination_alpha,
        mt_present: first.mt_present,
    }
}

pub struct EffectLoader {
    game_file_loader: Arc<GameFileLoader>,
    cache: HashMap<String, Arc<Effect>>,
}

pub struct Layer {
    pub textures: Vec<Arc<Texture>>,
    pub frames: Vec<Frame>,
    pub indices: Vec<Option<usize>>,
}

impl Layer {
    fn interpolate(&self, frame_timer: &FrameTimer) -> Option<Frame> {
        if let Some(frame_index) = self.indices[frame_timer.current_frame] {
            if let Some(next_frame) = self.frames.get(frame_index + 2) {
                return Some(interpolate(&self.frames[frame_index], next_frame, frame_timer.current_frame));
            } else {
                return Some(self.frames[frame_index].clone());
            }
        }

        None
    }
}

pub struct Effect {
    frames_per_second: usize,
    max_key: usize,
    layers: Vec<Layer>,
}

pub struct FrameTimer {
    total_timer: f32,
    frames_per_second: usize,
    max_key: usize,
    current_frame: usize,
}

impl FrameTimer {
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.total_timer += delta_time;
        self.current_frame = (self.total_timer / (1.0 / self.frames_per_second as f32)) as usize;

        if self.current_frame >= self.max_key {
            // TODO: better wrapping
            self.total_timer = 0.0;
            self.current_frame = 0;
            return false;
        }

        true
    }
}

impl Effect {
    pub fn new_frame_timer(&self) -> FrameTimer {
        FrameTimer {
            total_timer: 0.0,
            frames_per_second: self.frames_per_second,
            max_key: self.max_key,
            current_frame: 0,
        }
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, frame_timer: &FrameTimer, position: Point3<f32>) {
        for layer in &self.layers {
            let Some(frame) = layer.interpolate(frame_timer) else {
                continue;
            };

            if frame.texture_index < 0.0 || frame.texture_index as usize > layer.textures.len() {
                continue;
            }

            renderer.render_effect(
                camera,
                position,
                layer.textures[frame.texture_index as usize].clone(),
                [
                    Vector2::new(frame.xy[0], frame.xy[4]),
                    Vector2::new(frame.xy[1], frame.xy[5]),
                    Vector2::new(frame.xy[3], frame.xy[7]),
                    Vector2::new(frame.xy[2], frame.xy[6]),
                ],
                [
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[3] + frame.uv[1]),
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[3] + frame.uv[1]),
                ],
                frame.offset,
                frame.angle,
                Color::rgba(
                    frame.color[0] / 255.0,
                    frame.color[1] / 255.0,
                    frame.color[2] / 255.0,
                    frame.color[3] / 255.0,
                ),
            );
        }
    }
}

impl EffectLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            game_file_loader,
            cache: HashMap::new(),
        }
    }

    fn load(&mut self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load effect from {}", path.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\texture\\effect\\{path}"))
            .map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        // TODO: Add fallback
        let effect_data = EffectData::from_bytes(&mut byte_stream).map_err(LoadError::Conversion)?;

        let prefix = match path.chars().rev().position(|character| character == '\\') {
            Some(offset) => path.split_at(path.len() - offset).0,
            None => "",
        };

        let effect = Arc::new(Effect {
            frames_per_second: effect_data.frames_per_second as usize,
            max_key: effect_data.max_key as usize,
            layers: effect_data
                .layers
                .into_iter()
                .map(|layer_data| Layer {
                    textures: layer_data
                        .texture_names
                        .into_iter()
                        .map(|name| {
                            let path = format!("effect\\{}{}", prefix, name.name);
                            texture_loader.get(&path).unwrap()
                        })
                        .collect(),
                    indices: {
                        let frame_count = layer_data.frames.len();
                        let mut map = Vec::with_capacity(frame_count);
                        let mut list_index = 0;

                        if frame_count > 0 {
                            let mut previous = None;

                            for _ in 0..layer_data.frames[0].frame_index {
                                map.push(None);
                                list_index += 1;
                            }

                            for (index, frame) in layer_data.frames.iter().skip(1).enumerate() {
                                for _ in list_index..frame.frame_index as usize {
                                    map.push(previous);
                                    list_index += 1;
                                }

                                previous = Some(index);
                            }

                            // TODO: conditional
                            map.push(previous);
                            list_index += 1;
                        }

                        for _ in list_index..effect_data.max_key as usize {
                            map.push(None)
                        }

                        map
                    },
                    frames: layer_data.frames,
                })
                .collect(),
        });

        self.cache.insert(path.to_string(), effect.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(effect)
    }

    pub fn get(&mut self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        match self.cache.get(path) {
            Some(effect) => Ok(effect.clone()),
            None => self.load(path, texture_loader),
        }
    }
}

pub enum EffectCenter {
    Entity(EntityId, Point3<f32>),
    Position(Point3<f32>),
}

impl EffectCenter {
    fn to_position(&self) -> Point3<f32> {
        match self {
            EffectCenter::Entity(_, position) | EffectCenter::Position(position) => *position,
        }
    }
}

pub trait EffectBase {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool;

    fn mark_for_deletion(&mut self);

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera);

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera);
}

pub struct EffectWithLight {
    effect: Arc<Effect>,
    frame_timer: FrameTimer,
    center: EffectCenter,
    effect_offset: Vector3<f32>,
    point_light_id: PointLightId,
    light_offset: Vector3<f32>,
    light_color: Color,
    light_intensity: f32,
    repeating: bool,
    current_light_intensity: f32,
    gets_deleted: bool,
}

impl EffectWithLight {
    pub fn new(
        effect: Arc<Effect>,
        frame_timer: FrameTimer,
        center: EffectCenter,
        effect_offset: Vector3<f32>,
        point_light_id: PointLightId,
        light_offset: Vector3<f32>,
        light_color: Color,
        light_intensity: f32,
        repeating: bool,
    ) -> Self {
        Self {
            effect,
            frame_timer,
            center,
            effect_offset,
            point_light_id,
            light_offset,
            light_color,
            light_intensity,
            repeating,
            current_light_intensity: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for EffectWithLight {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        const FADE_SPEED: f32 = 5.0;

        if let EffectCenter::Entity(entity_id, position) = &mut self.center
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            let new_position = entity.get_position();
            *position = new_position;
        }

        if !self.gets_deleted && !self.frame_timer.update(delta_time) && !self.repeating {
            self.gets_deleted = true;
        }

        let (target, clamping_function): (f32, fn(f32, f32) -> f32) = match self.gets_deleted {
            true => (0.0, f32::max),
            false => (self.light_intensity, f32::min),
        };

        self.current_light_intensity += (target - self.current_light_intensity) * FADE_SPEED * delta_time;
        self.current_light_intensity = clamping_function(self.current_light_intensity, target);

        !self.gets_deleted || self.current_light_intensity > 0.1
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let frustum = Frustum::new(projection_matrix * view_matrix);

        let extent = point_light_extent(self.light_color, self.current_light_intensity);
        let light_position = self.center.to_position() + self.light_offset;

        if frustum.intersects_sphere(&Sphere::new(light_position, extent)) {
            point_light_manager.register_fading(
                self.point_light_id,
                light_position,
                self.light_color,
                self.current_light_intensity,
                self.light_intensity,
            )
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if !self.gets_deleted {
            self.effect.render(
                renderer,
                camera,
                &self.frame_timer,
                self.center.to_position() + self.effect_offset,
            );
        }
    }
}

#[derive(Default)]
pub struct EffectHolder {
    effects: Vec<(Box<dyn EffectBase + Send + Sync>, Option<EntityId>)>,
}

impl EffectHolder {
    pub fn add_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>) {
        self.effects.push((effect, None));
    }

    pub fn add_unit(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: EntityId) {
        self.effects.push((effect, Some(entity_id)));
    }

    pub fn remove_unit(&mut self, removed_entity_id: EntityId) {
        self.effects
            .iter_mut()
            .filter(|(_, entity_id)| entity_id.is_some_and(|entity_id| entity_id == removed_entity_id))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) {
        self.effects.retain_mut(|(effect, _)| effect.update(entities, delta_time));
    }

    pub fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        self.effects
            .iter()
            .for_each(|(effect, _)| effect.register_point_lights(point_light_manager, camera));
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        self.effects.iter().for_each(|(effect, _)| effect.render(renderer, camera));
    }
}
