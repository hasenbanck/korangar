//! Implements an offline experience for Korangar.

mod library;
mod map_state;
mod world_state;

use std::net::SocketAddr;
use std::time::Instant;

use korangar_gameplay::{
    CharacterServerLoginData, DisconnectReason, GameplayEvent, GameplayEventBuffer, GameplayProvider, LoginServerLoginData,
    NotConnectedError, ShopItem, SupportedPacketVersion,
};

use crate::library::Library;
use crate::world_state::WorldState;

/// An offline experience for Korangar.
pub struct OfflineSystem {
    system_start: Instant,
    event_buffer: Vec<GameplayEvent>,
    connected_to_login_server: bool,
    connected_to_character_server: bool,
    connected_to_map_server: bool,

    library: Library,
    world_state: Option<WorldState>,
    map_state: Option<WorldState>,
}

impl OfflineSystem {
    pub fn new() -> (Self, GameplayEventBuffer) {
        (
            Self {
                system_start: Instant::now(),
                event_buffer: vec![],
                connected_to_login_server: false,
                connected_to_character_server: false,
                connected_to_map_server: false,
                library: Library::new(),
                world_state: None,
                map_state: None,
            },
            GameplayEventBuffer::new(),
        )
    }
}

impl GameplayProvider for OfflineSystem {
    fn get_events(&mut self, events: &mut GameplayEventBuffer) {
        for event in self.event_buffer.drain(..) {
            events.push(event);
        }
    }

    fn connect_to_login_server(&mut self, _packet_version: SupportedPacketVersion, _address: SocketAddr, _username: &str, _password: &str) {
        self.connected_to_login_server = true;

        // TODO: We need to support creating an account with the _f / _m
        //       postfix, to select the sex.
        self.event_buffer.push(GameplayEvent::LoginServerConnected {
            character_servers: vec![ragnarok_packets::CharacterServerInformation::new(
                ragnarok_packets::ServerAddress::new([0, 0, 0, 0]),
                1234,
                "Korangar Offline".to_string(),
                0,
                0,
                0,
            )],
            login_data: LoginServerLoginData {
                account_id: ragnarok_packets::AccountId::new(0),
                login_id1: 0,
                login_id2: 0,
                sex: ragnarok_packets::Sex::Female,
            },
        });
    }

    fn connect_to_character_server(
        &mut self,
        _packet_version: SupportedPacketVersion,
        _login_data: &LoginServerLoginData,
        _server: ragnarok_packets::CharacterServerInformation,
    ) {
        self.connected_to_character_server = true;

        self.event_buffer
            .push(GameplayEvent::CharacterServerConnected { normal_slot_count: 15 });
    }

    fn connect_to_map_server(
        &mut self,
        _packet_version: SupportedPacketVersion,
        _login_server_login_data: &LoginServerLoginData,
        _character_server_login_data: CharacterServerLoginData,
    ) {
        unimplemented!()
    }

    fn disconnect_from_login_server(&mut self) {
        self.connected_to_login_server = false;

        self.event_buffer.push(GameplayEvent::AccountId {
            account_id: ragnarok_packets::AccountId(2000000),
        });

        self.event_buffer.push(GameplayEvent::LoginServerDisconnected {
            reason: DisconnectReason::ClosedByClient,
        });
    }

    fn disconnect_from_character_server(&mut self) {
        self.connected_to_character_server = false;

        self.event_buffer.push(GameplayEvent::CharacterServerDisconnected {
            reason: DisconnectReason::ClosedByClient,
        });
    }

    fn disconnect_from_map_server(&mut self) {
        self.connected_to_map_server = false;

        self.event_buffer.push(GameplayEvent::MapServerDisconnected {
            reason: DisconnectReason::ClosedByClient,
        });
    }

    fn is_login_server_connected(&self) -> bool {
        self.connected_to_login_server
    }

    fn is_character_server_connected(&self) -> bool {
        self.connected_to_character_server
    }

    fn is_map_server_connected(&self) -> bool {
        self.connected_to_map_server
    }

    fn request_character_list(&mut self) -> Result<(), NotConnectedError> {
        self.event_buffer.push(GameplayEvent::CharacterList {
            characters: vec![ragnarok_packets::CharacterInformation {
                character_id: ragnarok_packets::CharacterId(150000),
                experience: 3447,
                money: 20000,
                job_experience: 44,
                job_level: 6,
                body_state: 0,
                health_state: 0,
                effect_state: 0,
                virtue: 0,
                honor: 0,
                stat_points: 1273,
                health_points: 1060,
                maximum_health_points: 1060,
                spell_points: 216,
                maximum_spell_points: 216,
                movement_speed: 150,
                job: 0,
                head: 0,
                body: 0,
                weapon: 1,
                base_level: 99,
                sp_point: 5,
                accessory: 0,
                shield: 0,
                accessory2: 0,
                accessory3: 0,
                head_palette: 0,
                body_palette: 0,
                name: "Sasami".to_string(),
                strength: 99,
                agility: 99,
                vitality: 99,
                intelligence: 99,
                dexterity: 99,
                luck: 99,
                character_number: 0,
                hair_color: 0,
                b_is_changed_char: 1,
                map_name: "prontera.gat".to_string(),
                deletion_reverse_date: 0,
                robe_palette: 0,
                character_slot_change_count: 0,
                character_name_change_count: 0,
                sex: ragnarok_packets::Sex::Female,
            }],
        });

        Ok(())
    }

    fn select_character(&mut self, _character_slot: usize) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn create_character(&mut self, _slot: usize, _name: String) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn delete_character(&mut self, _character_id: ragnarok_packets::CharacterId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn switch_character_slot(&mut self, _origin_slot: usize, _destination_slot: usize) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn map_loaded(&mut self) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn request_client_tick(&mut self) -> Result<(), NotConnectedError> {
        let now = Instant::now();
        let client_tick = now.duration_since(self.system_start).as_millis();

        self.event_buffer.push(GameplayEvent::UpdateClientTick {
            client_tick: ragnarok_packets::ClientTick(client_tick as u32),
            received_at: now,
        });

        Ok(())
    }

    fn respawn(&mut self) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn log_out(&mut self) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn player_move(&mut self, _position: ragnarok_packets::WorldPosition) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn warp_to_map(&mut self, _map_name: String, _position: ragnarok_packets::TilePosition) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn entity_details(&mut self, _entity_id: ragnarok_packets::EntityId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn player_attack(&mut self, _entity_id: ragnarok_packets::EntityId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn send_chat_message(&mut self, _player_name: &str, _text: &str) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn start_dialog(&mut self, _npc_id: ragnarok_packets::EntityId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn next_dialog(&mut self, _npc_id: ragnarok_packets::EntityId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn close_dialog(&mut self, _npc_id: ragnarok_packets::EntityId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn choose_dialog_option(&mut self, _npc_id: ragnarok_packets::EntityId, _option: i8) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn request_item_equip(
        &mut self,
        _item_index: ragnarok_packets::InventoryIndex,
        _equip_position: ragnarok_packets::EquipPosition,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn request_item_unequip(&mut self, _item_index: ragnarok_packets::InventoryIndex) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn cast_skill(
        &mut self,
        _skill_id: ragnarok_packets::SkillId,
        _skill_level: ragnarok_packets::SkillLevel,
        _entity_id: ragnarok_packets::EntityId,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn cast_ground_skill(
        &mut self,
        _skill_id: ragnarok_packets::SkillId,
        _skill_level: ragnarok_packets::SkillLevel,
        _target_position: ragnarok_packets::TilePosition,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn cast_channeling_skill(
        &mut self,
        _skill_id: ragnarok_packets::SkillId,
        _skill_level: ragnarok_packets::SkillLevel,
        _entity_id: ragnarok_packets::EntityId,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn stop_channeling_skill(&mut self, _skill_id: ragnarok_packets::SkillId) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn add_friend(&mut self, _name: String) -> Result<(), NotConnectedError> {
        // Currently not supported (What is a "friend" in an offline game?).
        Ok(())
    }

    fn remove_friend(
        &mut self,
        _account_id: ragnarok_packets::AccountId,
        _character_id: ragnarok_packets::CharacterId,
    ) -> Result<(), NotConnectedError> {
        // Currently not supported (What is a "friend" in an offline game?).
        Ok(())
    }

    fn reject_friend_request(
        &mut self,
        _account_id: ragnarok_packets::AccountId,
        _character_id: ragnarok_packets::CharacterId,
    ) -> Result<(), NotConnectedError> {
        // Currently not supported (What is a "friend" in an offline game?).
        Ok(())
    }

    fn accept_friend_request(
        &mut self,
        _account_id: ragnarok_packets::AccountId,
        _character_id: ragnarok_packets::CharacterId,
    ) -> Result<(), NotConnectedError> {
        // Currently not supported (What is a "friend" in an offline game?).
        Ok(())
    }

    fn set_hotkey_data(
        &mut self,
        _tab: ragnarok_packets::HotbarTab,
        _index: ragnarok_packets::HotbarSlot,
        _hotkey_data: ragnarok_packets::HotkeyData,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn select_buy_or_sell(
        &mut self,
        _shop_id: ragnarok_packets::ShopId,
        _buy_or_sell: ragnarok_packets::BuyOrSellOption,
    ) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn purchase_items(&mut self, _items: Vec<ShopItem<u32>>) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn close_shop(&mut self) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn sell_items(&mut self, _items: Vec<ragnarok_packets::SoldItemInformation>) -> Result<(), NotConnectedError> {
        unimplemented!()
    }

    fn request_stat_up(&mut self, _stat_type: ragnarok_packets::StatUpType) -> Result<(), NotConnectedError> {
        unimplemented!()
    }
}
