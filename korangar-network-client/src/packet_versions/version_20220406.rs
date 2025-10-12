use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use korangar_gameplay::{
    CharacterServerLoginData, GameplayEvent, HotkeyState, InventoryItem, InventoryItemDetails, ItemQuantity, LoginServerLoginData,
    MessageColor, NoMetadata, ShopItem, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};
use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::{NetworkEventList, NoNetworkEvents};

pub fn register_login_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, (), Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginServerLoginSuccessPacket| GameplayEvent::LoginServerConnected {
        character_servers: packet.character_server_information,
        login_data: LoginServerLoginData {
            account_id: packet.account_id,
            login_id1: packet.login_id1,
            login_id2: packet.login_id2,
            sex: packet.sex,
        },
    })?;
    packet_handler.register(|packet: LoginFailedPacket| {
        let (reason, message) = match packet.reason {
            LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed"),
            LoginFailedReason::AlreadyLoggedIn => (
                UnifiedLoginFailedReason::AlreadyLoggedIn,
                "Someone has already logged in with this id",
            ),
            LoginFailedReason::AlreadyOnline => (UnifiedLoginFailedReason::AlreadyOnline, "Already online"),
        };

        GameplayEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let (reason, message) = match packet.reason {
            LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Unregistered id"),
            LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect password"),
            LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired"),
            LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server"),
            LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team"),
            LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated"),
            LoginFailedReason2::LoginProhibitedUntil => (UnifiedLoginFailedReason::LoginProhibitedUntil, "Login prohibited until"),
            LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full"),
            LoginFailedReason2::CompanyAccountLimitReached => (
                UnifiedLoginFailedReason::CompanyAccountLimitReached,
                "Company account limit reached",
            ),
        };

        GameplayEvent::LoginServerConnectionFailed { reason, message }
    })?;

    Ok(())
}

pub fn register_character_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, (), Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            LoginFailedReason::ServerClosed => "Server closed",
            LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
            LoginFailedReason::AlreadyOnline => "Already online",
        };

        GameplayEvent::CharacterServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(
        |packet: CharacterServerLoginSuccessPacket| GameplayEvent::CharacterServerConnected {
            normal_slot_count: packet.normal_slot_count as usize,
        },
    )?;
    packet_handler.register(|packet: RequestCharacterListSuccessPacket| GameplayEvent::CharacterList {
        characters: packet.character_information,
    })?;
    packet_handler.register_noop::<CharacterListPacket>()?;
    packet_handler.register_noop::<CharacterSlotPagePacket>()?;
    packet_handler.register_noop::<CharacterBanListPacket>()?;
    packet_handler.register_noop::<LoginPincodePacket>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register(|packet: CharacterSelectionSuccessPacket| {
        let login_data = CharacterServerLoginData {
            server_ip: IpAddr::V4(packet.map_server_ip.into()),
            server_port: packet.map_server_port,
            character_id: packet.character_id,
        };

        GameplayEvent::CharacterSelected { login_data }
    })?;
    packet_handler.register(|packet: CharacterSelectionFailedPacket| {
        let (reason, message) = match packet.reason {
            CharacterSelectionFailedReason::RejectedFromServer => (
                UnifiedCharacterSelectionFailedReason::RejectedFromServer,
                "Rejected from server",
            ),
        };

        GameplayEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|_: MapServerUnavailablePacket| {
        let reason = UnifiedCharacterSelectionFailedReason::MapServerUnavailable;
        let message = "Map server currently unavailable";

        GameplayEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: CreateCharacterSuccessPacket| GameplayEvent::CharacterCreated {
        character_information: packet.character_information,
    })?;
    packet_handler.register(|packet: CharacterCreationFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterCreationFailedReason::CharacterNameAlreadyUsed => "Character name is already used",
            CharacterCreationFailedReason::NotOldEnough => "You are not old enough to create a character",
            CharacterCreationFailedReason::NotAllowedToUseSlot => "You are not allowed to use this character slot",
            CharacterCreationFailedReason::CharacterCerationFailed => "Character creation failed",
        };

        GameplayEvent::CharacterCreationFailed { reason, message }
    })?;
    packet_handler.register(|_: CharacterDeletionSuccessPacket| GameplayEvent::CharacterDeleted)?;
    packet_handler.register(|packet: CharacterDeletionFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterDeletionFailedReason::NotAllowed => "You are not allowed to delete this character",
            CharacterDeletionFailedReason::CharacterNotFound => "Character was not found",
            CharacterDeletionFailedReason::NotEligible => "Character is not eligible for deletion",
        };
        GameplayEvent::CharacterDeletionFailed { reason, message }
    })?;
    packet_handler.register(|packet: SwitchCharacterSlotResponsePacket| match packet.status {
        SwitchCharacterSlotResponseStatus::Success => GameplayEvent::CharacterSlotSwitched,
        SwitchCharacterSlotResponseStatus::Error => GameplayEvent::CharacterSlotSwitchFailed,
    })?;

    Ok(())
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, (), Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    // This is a bit of a workaround for the way that the inventory is
    // sent. There is a single packet to start the inventory list,
    // followed by an arbitary number of item packets, and in the
    // end a sinle packet to mark the list as complete.
    //
    // This variable provides some transient storage shared by all the inventory
    // handlers.
    let inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));

    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: BroadcastMessagePacket| GameplayEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Broadcast,
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        GameplayEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: OverheadMessagePacket| {
        // FIX: This should be a different event.
        GameplayEvent::ChatMessage {
            text: packet.message,
            color: MessageColor::Broadcast,
        }
    })?;
    packet_handler.register(|packet: ServerMessagePacket| GameplayEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    packet_handler.register_noop::<MessageTablePacket>()?;
    packet_handler.register(|packet: EntityMessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        };
        GameplayEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register_noop::<DisplayEmotionPacket>()?;
    packet_handler.register(|packet: EntityMovePacket| {
        let EntityMovePacket {
            entity_id,
            from_to,
            starting_timestamp,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        GameplayEvent::EntityMove {
            entity_id,
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register_noop::<EntityStopMovePacket>()?;
    packet_handler.register(|packet: PlayerMovePacket| {
        let PlayerMovePacket {
            starting_timestamp,
            from_to,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        GameplayEvent::PlayerMove {
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register(|packet: ChangeMapPacket| {
        let ChangeMapPacket { map_name, position } = packet;

        let map_name = map_name.replace(".gat", "");

        GameplayEvent::ChangeMap { map_name, position }
    })?;
    packet_handler.register(|packet: ResurrectionPacket| GameplayEvent::ResurrectPlayer {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntityAppearedPacket| GameplayEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityAppeared2Packet| GameplayEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: MovingEntityAppearedPacket| GameplayEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityDisappearedPacket| GameplayEvent::RemoveEntity {
        entity_id: packet.entity_id,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: UpdateStatPacket| {
        let UpdateStatPacket { stat_type } = packet;
        GameplayEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket1| {
        let UpdateStatPacket1 { stat_type } = packet;
        GameplayEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket2| {
        let UpdateStatPacket2 { stat_type } = packet;
        GameplayEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket3| {
        let UpdateStatPacket3 { stat_type } = packet;
        GameplayEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register_noop::<UpdateAttackRangePacket>()?;
    packet_handler.register_noop::<NewMailStatusPacket>()?;
    packet_handler.register_noop::<AchievementUpdatePacket>()?;
    packet_handler.register_noop::<AchievementListPacket>()?;
    packet_handler.register_noop::<CriticalWeightUpdatePacket>()?;
    packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
        SpriteChangeType::Base => Some(GameplayEvent::ChangeJob {
            account_id: packet.account_id,
            job_id: packet.value,
        }),
        SpriteChangeType::Hair => Some(GameplayEvent::ChangeHair {
            account_id: packet.account_id,
            hair_id: packet.value,
        }),
        _ => None,
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyStartPacket| {
            *inventory_items.borrow_mut() = Some(Vec::new());
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: RegularItemListPacket| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item_information| {
                    let RegularItemInformation {
                        index,
                        item_id,
                        item_type,
                        amount,
                        equipped_position,
                        slot,
                        hire_expiration_date,
                        flags,
                    } = item_information;

                    InventoryItem {
                        index,
                        metadata: NoMetadata,
                        item_id,
                        item_type,
                        slot,
                        hire_expiration_date,
                        details: InventoryItemDetails::Regular {
                            amount,
                            equipped_position,
                            flags,
                        },
                    }
                }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: EquippableItemListPacket| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item| {
                    let EquippableItemInformation {
                        index,
                        item_id,
                        item_type,
                        equip_position,
                        equipped_position,
                        slot,
                        hire_expiration_date,
                        bind_on_equip_type,
                        w_item_sprite_number,
                        option_count,
                        option_data,
                        refinement_level,
                        enchantment_level,
                        flags,
                    } = item;

                    InventoryItem {
                        index,
                        metadata: NoMetadata,
                        item_id,
                        item_type,
                        slot,
                        hire_expiration_date,
                        details: InventoryItemDetails::Equippable {
                            equip_position,
                            equipped_position,
                            bind_on_equip_type,
                            w_item_sprite_number,
                            option_count,
                            option_data,
                            refinement_level,
                            enchantment_level,
                            flags,
                        },
                    }
                }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyEndPacket| {
            let items = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");
            GameplayEvent::SetInventory { items }
        }
    })?;
    packet_handler.register_noop::<EquippableSwitchItemListPacket>()?;
    packet_handler.register_noop::<MapTypePacket>()?;
    packet_handler.register(|packet: UpdateSkillTreePacket| {
        let UpdateSkillTreePacket { skill_information } = packet;
        GameplayEvent::SkillTree { skill_information }
    })?;
    packet_handler.register(|packet: UpdateHotkeysPacket| GameplayEvent::SetHotkeyData {
        tab: packet.tab,
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;
    packet_handler.register(|packet: InitialStatsPacket| {
        let InitialStatsPacket {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            ..
        } = packet;

        GameplayEvent::InitialStats {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
        }
    })?;
    packet_handler.register_noop::<UpdatePartyInvitationStatePacket>()?;
    packet_handler.register_noop::<UpdateShowEquipPacket>()?;
    packet_handler.register_noop::<UpdateConfigurationPacket>()?;
    packet_handler.register_noop::<NavigateToMonsterPacket>()?;
    packet_handler.register_noop::<MarkMinimapPositionPacket>()?;
    packet_handler.register(|packet: NextButtonPacket| {
        let NextButtonPacket { npc_id } = packet;

        GameplayEvent::AddNextButton { npc_id }
    })?;
    packet_handler.register(|packet: CloseButtonPacket| {
        let CloseButtonPacket { npc_id } = packet;

        GameplayEvent::AddCloseButton { npc_id }
    })?;
    packet_handler.register(|packet: DialogMenuPacket| {
        let DialogMenuPacket { npc_id, message } = packet;

        let choices = message.split(':').map(String::from).filter(|text| !text.is_empty()).collect();

        GameplayEvent::AddChoiceButtons { choices, npc_id }
    })?;
    packet_handler.register_noop::<DisplaySpecialEffectPacket>()?;
    packet_handler.register_noop::<DisplaySkillCooldownPacket>()?;
    packet_handler.register_noop::<DisplaySkillEffectAndDamagePacket>()?;
    packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket| GameplayEvent::HealEffect {
        entity_id: packet.destination_entity_id,
        heal_amount: packet.heal_amount as usize,
    })?;
    packet_handler.register_noop::<DisplayPlayerHealEffect>()?;
    packet_handler.register_noop::<StatusChangePacket>()?;
    packet_handler.register_noop::<QuestNotificationPacket1>()?;
    packet_handler.register_noop::<HuntingQuestNotificationPacket>()?;
    packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket>()?;
    packet_handler.register_noop::<QuestRemovedPacket>()?;
    packet_handler.register_noop::<QuestListPacket>()?;
    packet_handler.register(|packet: VisualEffectPacket| {
        let VisualEffectPacket { entity_id, effect } = packet;

        let effect_path = match effect {
            VisualEffect::BaseLevelUp => "angel.str",
            VisualEffect::JobLevelUp => "joblvup.str",
            VisualEffect::RefineFailure => "bs_refinefailed.str",
            VisualEffect::RefineSuccess => "bs_refinesuccess.str",
            VisualEffect::GameOver => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::PharmacySuccess => "p_success.str",
            VisualEffect::PharmacyFailure => "p_failed.str",
            VisualEffect::BaseLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::JobLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::BaseLevelUpTaekwon => "help_angel\\help_angel\\help_angel.str",
        };

        GameplayEvent::VisualEffect { effect_path, entity_id }
    })?;
    packet_handler.register_noop::<DisplayGainedExperiencePacket>()?;
    packet_handler.register_noop::<DisplayImagePacket>()?;
    packet_handler.register_noop::<StateChangePacket>()?;

    packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
        QuestEffect::None => GameplayEvent::RemoveQuestEffect {
            entity_id: packet.entity_id,
        },
        _ => GameplayEvent::AddQuestEffect { quest_effect: packet },
    })?;
    packet_handler.register(|packet: ItemPickupPacket| {
        let ItemPickupPacket {
            index,
            count,
            item_id,
            is_identified,
            is_broken,
            cards,
            equip_position,
            item_type,
            result,
            hire_expiration_date,
            bind_on_equip_type,
            option_data,
            favorite,
            look,
            refinement_level,
            enchantment_level,
        } = packet;

        if result != ItemPickupResult::Success {
            todo!();
        }

        // TODO: Not sure where to store these, since the *InventoryItem packets are not
        // sending these either. We will certainly use them at some point though.
        let _ = (favorite, look);

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount: count,
                equipped_position: equip_position,
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, is_identified != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Equippable {
                equip_position,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type,
                w_item_sprite_number: 0,
                option_count: option_data.len() as u8,
                option_data,
                refinement_level,
                enchantment_level,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, is_identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, is_broken != 0);
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index,
            item_id,
            item_type,
            slot: cards,
            hire_expiration_date,
            details,
        };

        GameplayEvent::IventoryItemAdded { item }
    })?;
    packet_handler.register(|packet: RemoveItemFromInventoryPacket| GameplayEvent::InventoryItemRemoved {
        reason: packet.remove_reason,
        index: packet.index,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: ServerTickPacket| GameplayEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RequestPlayerDetailsSuccessPacket| GameplayEvent::UpdateEntityDetails {
        entity_id: EntityId(packet.character_id.0),
        name: packet.name,
    })?;
    packet_handler.register(|packet: RequestEntityDetailsSuccessPacket| GameplayEvent::UpdateEntityDetails {
        entity_id: packet.entity_id,
        name: packet.name,
    })?;
    packet_handler.register(|packet: UpdateEntityHealthPointsPacket| {
        let UpdateEntityHealthPointsPacket {
            entity_id,
            health_points,
            maximum_health_points,
        } = packet;

        GameplayEvent::UpdateEntityHealth {
            entity_id,
            health_points: health_points as usize,
            maximum_health_points: maximum_health_points as usize,
        }
    })?;
    packet_handler.register(|packet: RequestPlayerAttackFailedPacket| {
        let RequestPlayerAttackFailedPacket {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        } = packet;

        GameplayEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        }
    })?;
    packet_handler.register(|packet: DamagePacket1| match packet.damage_type {
        DamageType::Damage => Some(GameplayEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(GameplayEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
        }),
        DamageType::StandUp => Some(GameplayEvent::PlayerStandUp {
            entity_id: packet.destination_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: DamagePacket3| match packet.damage_type {
        DamageType::Damage => Some(GameplayEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(GameplayEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
        }),
        DamageType::StandUp => Some(GameplayEvent::PlayerStandUp {
            entity_id: packet.destination_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        GameplayEvent::OpenDialog { text, npc_id }
    })?;
    packet_handler.register(|packet: RequestEquipItemStatusPacket| match packet.result {
        RequestEquipItemStatus::Success => Some(GameplayEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: packet.equipped_position,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket| match packet.result {
        RequestUnequipItemStatus::Success => Some(GameplayEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: EquipPosition::NONE,
        }),
        _ => None,
    })?;
    packet_handler.register_noop::<Packet8302>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register(|packet: MapServerLoginSuccessPacket| GameplayEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RestartResponsePacket| match packet.result {
        RestartResponseStatus::Ok => GameplayEvent::LoggedOut,
        RestartResponseStatus::Nothing => GameplayEvent::ChatMessage {
            text: "Failed to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: DisconnectResponsePacket| match packet.result {
        DisconnectResponseStatus::Ok => GameplayEvent::LoggedOut,
        DisconnectResponseStatus::Wait10Seconds => GameplayEvent::ChatMessage {
            text: "Please wait 10 seconds before trying to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register_noop::<UseSkillSuccessPacket>()?;
    packet_handler.register_noop::<ToUseSkillSuccessPacket>()?;
    packet_handler.register(|packet: NotifySkillUnitPacket| {
        let NotifySkillUnitPacket {
            entity_id,
            position,
            unit_id,
            ..
        } = packet;

        GameplayEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
        }
    })?;
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        GameplayEvent::RemoveSkillUnit { entity_id }
    })?;
    packet_handler.register_noop::<NotifyGroundSkillPacket>()?;
    packet_handler.register(|packet: FriendListPacket| GameplayEvent::SetFriendList {
        friend_list: packet.friend_list,
    })?;
    packet_handler.register_noop::<FriendOnlineStatusPacket>()?;
    packet_handler.register(|packet: FriendRequestPacket| GameplayEvent::FriendRequest {
        requestee: packet.requestee,
    })?;
    packet_handler.register(|packet: FriendRequestResultPacket| {
        let text = match packet.result {
            FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
            FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
            FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
            FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
        };

        let mut events = vec![GameplayEvent::ChatMessage {
            text,
            color: MessageColor::Information,
        }];

        if matches!(packet.result, FriendRequestResult::Accepted) {
            events.push(GameplayEvent::FriendAdded { friend: packet.friend });
        }

        events
    })?;
    packet_handler.register(|packet: NotifyFriendRemovedPacket| GameplayEvent::FriendRemoved {
        account_id: packet.account_id,
        character_id: packet.character_id,
    })?;
    packet_handler.register_noop::<PartyInvitePacket>()?;
    packet_handler.register_noop::<StatusChangeSequencePacket>()?;
    packet_handler.register_noop::<ReputationPacket>()?;
    packet_handler.register_noop::<ClanInfoPacket>()?;
    packet_handler.register_noop::<ClanOnlineCountPacket>()?;
    packet_handler.register_noop::<ChangeMapCellPacket>()?;
    packet_handler.register_noop::<OpenMarketPacket>()?;
    packet_handler.register(|packet: BuyOrSellPacket| GameplayEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
    packet_handler.register(|packet: ShopItemListPacket| {
        let items = packet
            .items
            .into_iter()
            .map(|item| ShopItem {
                metadata: NoMetadata,
                item_id: item.item_id,
                item_type: item.item_type,
                price: item.price,
                quantity: ItemQuantity::Infinite,
                weight: 0,
                location: item.location,
            })
            .collect();

        GameplayEvent::OpenShop { items }
    })?;
    packet_handler.register(|packet: BuyShopItemsResultPacket| GameplayEvent::BuyingCompleted { result: packet.result })?;
    packet_handler.register_noop::<ParameterChangePacket>()?;
    packet_handler.register(|packet: SellListPacket| GameplayEvent::SellItemList { items: packet.items })?;
    packet_handler.register(|packet: SellItemsResultPacket| GameplayEvent::SellingCompleted { result: packet.result })?;
    packet_handler.register_noop::<RequestStatUpResponsePacket>()?;
    packet_handler.register_noop::<EquipAmmunitionPacket>()?;
    packet_handler.register_noop::<AmmunitionActionPacket>()?;

    Ok(())
}
