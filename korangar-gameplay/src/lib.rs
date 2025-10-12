#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]
#![cfg_attr(feature = "interface", feature(negative_impls))]

mod entity;
mod event;
mod hotkey;
mod items;
mod message;
mod types;

use std::net::SocketAddr;

use ragnarok_packets::*;

pub use self::entity::EntityData;
pub use self::event::{DisconnectReason, GameplayEvent};
pub use self::hotkey::HotkeyState;
pub use self::items::{InventoryItem, InventoryItemDetails, ItemQuantity, NoMetadata, SellItem, ShopItem};
pub use self::message::MessageColor;
pub use self::types::{
    CharacterServerLoginData, GameplayError, GameplayResult, LoginServerLoginData, NotConnectedError,
    UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

/// Buffer for gameplay events. This struct exists to reduce heap allocations
/// and is purely an optimization.
pub struct GameplayEventBuffer(pub(crate) Vec<GameplayEvent>);

impl GameplayEventBuffer {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, GameplayEvent> {
        self.0.drain(..)
    }

    pub fn push(&mut self, event: GameplayEvent) {
        self.0.push(event);
    }
}

impl IntoIterator for GameplayEventBuffer {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = GameplayEvent;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Default for GameplayEventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<GameplayEvent> for GameplayEventBuffer {
    fn from(event: GameplayEvent) -> Self {
        Self(vec![event])
    }
}

impl From<Vec<GameplayEvent>> for GameplayEventBuffer {
    fn from(events: Vec<GameplayEvent>) -> Self {
        Self(events)
    }
}

impl From<Option<GameplayEvent>> for GameplayEventBuffer {
    fn from(event: Option<GameplayEvent>) -> Self {
        match event {
            Some(event) => Self(vec![event]),
            None => Self(Vec::new()),
        }
    }
}

/// Trait that all gameplay providers must implement.
///
/// This trait defines the interface for interacting with a gameplay system,
/// whether it's a networked MMO server, a local single-player implementation,
/// or any other gameplay provider.
pub trait GameplayProvider {
    /// Poll for new gameplay events.
    /// Events are drained from the provider and added to the provided buffer.
    fn get_events(&mut self, events: &mut GameplayEventBuffer);

    /// Connect to the login server.
    fn connect_to_login_server(&mut self, packet_version: SupportedPacketVersion, address: SocketAddr, username: &str, password: &str);

    /// Connect to the character server.
    fn connect_to_character_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_data: &LoginServerLoginData,
        server: CharacterServerInformation,
    );

    /// Connect to the map server.
    fn connect_to_map_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_server_login_data: &LoginServerLoginData,
        character_server_login_data: CharacterServerLoginData,
    );

    /// Disconnect from the login server.
    fn disconnect_from_login_server(&mut self);

    /// Disconnect from the character server.
    fn disconnect_from_character_server(&mut self);

    /// Disconnect from the map server.
    fn disconnect_from_map_server(&mut self);

    /// Check if connected to the login server.
    fn is_login_server_connected(&self) -> bool;

    /// Check if connected to the character server.
    fn is_character_server_connected(&self) -> bool;

    /// Check if connected to the map server.
    fn is_map_server_connected(&self) -> bool;

    /// Request the list of characters from the character server.
    fn request_character_list(&mut self) -> Result<(), NotConnectedError>;

    /// Select a character by slot number.
    fn select_character(&mut self, character_slot: usize) -> Result<(), NotConnectedError>;

    /// Create a new character in the specified slot with the given name.
    fn create_character(&mut self, slot: usize, name: String) -> Result<(), NotConnectedError>;

    /// Delete the character with the specified ID.
    fn delete_character(&mut self, character_id: CharacterId) -> Result<(), NotConnectedError>;

    /// Switch character slots (swap two characters).
    fn switch_character_slot(&mut self, origin_slot: usize, destination_slot: usize) -> Result<(), NotConnectedError>;

    /// Notify the server that the map has finished loading.
    fn map_loaded(&mut self) -> Result<(), NotConnectedError>;

    /// Request the current client tick from the server for time
    /// synchronization.
    fn request_client_tick(&mut self) -> Result<(), NotConnectedError>;

    /// Request to respawn after death.
    fn respawn(&mut self) -> Result<(), NotConnectedError>;

    /// Log out and return to character selection.
    fn log_out(&mut self) -> Result<(), NotConnectedError>;

    /// Move the player to a specific world position.
    fn player_move(&mut self, position: WorldPosition) -> Result<(), NotConnectedError>;

    /// Warp to a specific map and tile position (GM command).
    fn warp_to_map(&mut self, map_name: String, position: TilePosition) -> Result<(), NotConnectedError>;

    /// Request details about a specific entity.
    fn entity_details(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError>;

    /// Attack a specific entity.
    fn player_attack(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError>;

    /// Send a chat message.
    fn send_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError>;

    /// Start a dialog with an NPC.
    fn start_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError>;

    /// Advance to the next dialog page.
    fn next_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError>;

    /// Close the current dialog.
    fn close_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError>;

    /// Choose a dialog option.
    fn choose_dialog_option(&mut self, npc_id: EntityId, option: i8) -> Result<(), NotConnectedError>;

    /// Request to equip an item.
    fn request_item_equip(&mut self, item_index: InventoryIndex, equip_position: EquipPosition) -> Result<(), NotConnectedError>;

    /// Request to unequip an item.
    fn request_item_unequip(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError>;

    /// Cast a skill on a specific entity.
    fn cast_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> Result<(), NotConnectedError>;

    /// Cast a ground-targeted skill.
    fn cast_ground_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        target_position: TilePosition,
    ) -> Result<(), NotConnectedError>;

    /// Start casting a channeling skill.
    fn cast_channeling_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> Result<(), NotConnectedError>;

    /// Stop casting a channeling skill.
    fn stop_channeling_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError>;

    /// Add a friend by name.
    fn add_friend(&mut self, name: String) -> Result<(), NotConnectedError>;

    /// Remove a friend.
    fn remove_friend(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError>;

    /// Reject a friend request.
    fn reject_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError>;

    /// Accept a friend request.
    fn accept_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError>;

    /// Set hotkey data for a specific tab and slot.
    fn set_hotkey_data(&mut self, tab: HotbarTab, index: HotbarSlot, hotkey_data: HotkeyData) -> Result<(), NotConnectedError>;

    /// Select whether to buy or sell from a shop.
    fn select_buy_or_sell(&mut self, shop_id: ShopId, buy_or_sell: BuyOrSellOption) -> Result<(), NotConnectedError>;

    /// Purchase items from a shop.
    fn purchase_items(&mut self, items: Vec<ShopItem<u32>>) -> Result<(), NotConnectedError>;

    /// Close the current shop.
    fn close_shop(&mut self) -> Result<(), NotConnectedError>;

    /// Sell items to a shop.
    fn sell_items(&mut self, items: Vec<SoldItemInformation>) -> Result<(), NotConnectedError>;

    /// Request to increase a stat.
    fn request_stat_up(&mut self, stat_type: StatUpType) -> Result<(), NotConnectedError>;
}

/// Packet version support definition.
#[derive(Debug, Clone, Copy)]
pub enum SupportedPacketVersion {
    _20220406,
}
