use std::sync::Arc;

use cgmath::{Point3, Vector2};
use hashbrown::HashMap;
use hecs::{Bundle, Entity};
use korangar_networking::EntityData;
use ragnarok_packets::{AccountId, CharacterInformation, ClientTick, EntityId, Sex, WorldPosition};

use crate::graphics::{Buffer, ModelVertex};
use crate::loaders::{ActionLoader, AnimationLoader, AnimationState, ScriptLoader, SpriteLoader};
use crate::world::{get_entity_part_files, AnimationData, EntityType, Map, ResourceState};

struct GameState {
    world: hecs::World,
    entity_mapping: HashMap<EntityId, Entity>,
    player_entity: Option<Entity>,
}

impl GameState {
    pub fn clear(&mut self) {
        self.world.clear();
        self.entity_mapping.clear();
        self.player_entity = None;
    }

    pub fn spawn_player(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        animation_loader: &mut AnimationLoader,
        script_loader: &ScriptLoader,
        map: &Map,
        account_id: AccountId,
        character_information: CharacterInformation,
        player_position: WorldPosition,
        client_tick: ClientTick,
    ) {
        if let Some(player_entity) = self.player_entity {
            let identifier = self.world.query_one_mut::<&EntityIdentifier>(player_entity).unwrap();
            self.entity_mapping.remove(&identifier.id);
            let _ = self.world.despawn(player_entity);
        }

        let player = PlayerBundle::new(
            sprite_loader,
            action_loader,
            animation_loader,
            script_loader,
            map,
            account_id,
            character_information,
            player_position,
            client_tick,
        );

        let entity_id = player.identifier.id;

        let entity = self.world.spawn(player);
        self.entity_mapping.insert(entity_id, entity);
        self.player_entity = Some(entity);
    }

    pub fn spawn_npc(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        animation_loader: &mut AnimationLoader,
        script_loader: &ScriptLoader,
        map: &Map,
        entity_data: EntityData,
        client_tick: ClientTick,
    ) {
        let entity_id = entity_data.entity_id;

        if let Some(entity) = self.entity_mapping.get(&entity_id).copied() {
            self.entity_mapping.remove(&entity_id);
            let _ = self.world.despawn(entity);
        }

        let npc = NpcBundle::new(
            sprite_loader,
            action_loader,
            animation_loader,
            script_loader,
            map,
            &entity_data,
            client_tick,
        );

        let entity = self.world.spawn(npc);
        self.entity_mapping.insert(entity_id, entity);
    }
}

struct EntityIdentifier {
    id: EntityId,
    entity_type: EntityType,
    job_id: usize,
    sex: Sex,
}

struct Position {
    grid: Vector2<usize>,
    world: Point3<f32>,
}

struct MovementSpeed(usize);

struct MovementDestination {
    from: Vector2<usize>,
    to: Vector2<usize>,
}

struct Movement {
    steps: Vec<(Vector2<usize>, u32)>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    pathing_vertex_buffer: Option<Arc<Buffer<ModelVertex>>>,
}

struct Sprite {
    animation_data: Arc<AnimationData>,
    animation_state: AnimationState,
    head_direction: usize,
}

struct Health {
    current: usize,
    maximum: usize,
}

struct SpellPoints {
    current: usize,
    maximum: usize,
}

struct ActivityPoints {
    current: usize,
    maximum: usize,
}

struct Details {
    state: ResourceState<String>,
}

struct LoadingState {
    request_id: Option<u64>,
    sprite_loaded: bool,
    animation_loaded: bool,
}

struct CommonEntityBundle {
    identifier: EntityIdentifier,
    position: Position,
    sprite: Sprite,
    health: Health,
    movement: Option<Movement>,
    movement_destination: Option<MovementDestination>,
    movement_speed: MovementSpeed,
    details: Details,
    loading: LoadingState,
}

impl CommonEntityBundle {
    fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        animation_loader: &mut AnimationLoader,
        script_loader: &ScriptLoader,
        map: &Map,
        entity_data: &EntityData,
        client_tick: ClientTick,
    ) -> Self {
        let id = entity_data.entity_id;
        let job_id = entity_data.job as usize;
        let grid_position = entity_data.position;
        let grid_position = Vector2::new(grid_position.x, grid_position.y);
        let get_world_position = map.get_world_position(grid_position);
        let head_direction = entity_data.head_direction;

        let movement_speed = entity_data.movement_speed as usize;
        let health_points = entity_data.health_points as usize;
        let maximum_health_points = entity_data.maximum_health_points as usize;
        let sex = entity_data.sex;

        let entity_type = match job_id {
            45 => EntityType::Warp,
            111 => EntityType::Hidden, // TODO: check that this is correct
            // 111 | 139 => None,
            0..=44 | 4000..=5999 => EntityType::Player,
            46..=999 | 10000..=19999 => EntityType::Npc,
            1000..=3999 | 20000..=29999 => EntityType::Monster,
            _ => EntityType::Npc,
        };

        let entity_part_files = get_entity_part_files(script_loader, entity_type, job_id, sex);
        let animation_data = animation_loader
            .get(sprite_loader, action_loader, entity_type, &entity_part_files)
            .unwrap();
        let details_state = ResourceState::Unavailable;
        let animation_state = AnimationState::new(client_tick);

        // TODO Write a system that resolves the destination!
        // common.move_from_to(map, position_from, position_to,
        // client_tick);

        let movement = None;
        let movement_destination = match entity_data.destination {
            None => None,
            Some(destination) => {
                let from = Vector2::new(entity_data.position.x, entity_data.position.y);
                let to = Vector2::new(destination.x, destination.y);
                Some(MovementDestination { from, to })
            }
        };
        let movement_speed = MovementSpeed(movement_speed);

        Self {
            identifier: EntityIdentifier {
                id,
                entity_type,
                job_id,
                sex,
            },
            position: Position {
                grid: grid_position,
                world: get_world_position,
            },
            sprite: Sprite {
                animation_data,
                animation_state,
                head_direction,
            },
            health: Health {
                current: health_points,
                maximum: maximum_health_points,
            },
            details: Details { state: details_state },
            movement,
            movement_destination,
            movement_speed,
            loading: LoadingState {
                request_id: None,
                sprite_loaded: false,
                animation_loaded: false,
            },
        }
    }
}

#[derive(Bundle)]
struct PlayerBundle {
    identifier: EntityIdentifier,
    position: Position,
    sprite: Sprite,
    health: Health,
    movement: Option<Movement>,
    movement_destination: Option<MovementDestination>,
    movement_speed: MovementSpeed,
    details: Details,
    loading: LoadingState,
    activity_points: ActivityPoints,
    spell_points: SpellPoints,
}

impl PlayerBundle {
    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        animation_loader: &mut AnimationLoader,
        script_loader: &ScriptLoader,
        map: &Map,
        account_id: AccountId,
        character_information: CharacterInformation,
        player_position: WorldPosition,
        client_tick: ClientTick,
    ) -> Self {
        let activity_points = ActivityPoints { current: 0, maximum: 0 };

        let spell_points = SpellPoints {
            current: character_information.spell_points as usize,
            maximum: character_information.maximum_spell_points as usize,
        };

        let entity_data = EntityData::from_character(account_id, character_information, player_position);

        let CommonEntityBundle {
            identifier,
            position,
            sprite,
            health,
            movement,
            movement_destination,
            movement_speed,
            details,
            loading,
        } = CommonEntityBundle::new(
            sprite_loader,
            action_loader,
            animation_loader,
            script_loader,
            map,
            &entity_data,
            client_tick,
        );

        Self {
            identifier,
            position,
            sprite,
            health,
            movement,
            movement_destination,
            movement_speed,
            details,
            loading,
            activity_points,
            spell_points,
        }
    }
}

#[derive(Bundle)]
struct NpcBundle {
    identifier: EntityIdentifier,
    position: Position,
    sprite: Sprite,
    health: Health,
    movement: Option<Movement>,
    movement_destination: Option<MovementDestination>,
    movement_speed: MovementSpeed,
    details: Details,
    loading: LoadingState,
}

impl NpcBundle {
    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        animation_loader: &mut AnimationLoader,
        script_loader: &ScriptLoader,
        map: &Map,
        entity_data: &EntityData,
        client_tick: ClientTick,
    ) -> Self {
        let CommonEntityBundle {
            identifier,
            position,
            sprite,
            health,
            movement,
            movement_destination,
            movement_speed,
            details,
            loading,
        } = CommonEntityBundle::new(
            sprite_loader,
            action_loader,
            animation_loader,
            script_loader,
            map,
            entity_data,
            client_tick,
        );

        Self {
            identifier,
            position,
            sprite,
            health,
            movement,
            movement_destination,
            movement_speed,
            details,
            loading,
        }
    }
}
