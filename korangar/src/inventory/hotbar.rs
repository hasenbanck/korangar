use korangar_gameplay::GameplayProvider;
use korangar_interface::element::StateElement;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData};
use rust_state::RustState;

use super::Skill;

#[derive(Default, RustState, StateElement)]
pub struct Hotbar {
    skills: [Option<Skill>; 10],
}

impl Hotbar {
    /// Set the slot without notifying the map server.
    pub fn set_slot(&mut self, slot: HotbarSlot, skill: Skill) {
        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Update the slot and notify the map server.
    pub fn update_slot(&mut self, provider: &mut dyn GameplayProvider, slot: HotbarSlot, skill: Skill) {
        let _ = provider.set_hotkey_data(HotbarTab(0), slot, HotkeyData {
            is_skill: true as u8,
            skill_id: skill.skill_id.0 as u32,
            quantity_or_skill_level: skill.skill_level,
        });

        self.skills[slot.0 as usize] = Some(skill);
    }

    /// Swap two slots in the hotbar and notify the map server.
    pub fn swap_slot(&mut self, provider: &mut dyn GameplayProvider, source_slot: HotbarSlot, destination_slot: HotbarSlot) {
        if source_slot != destination_slot {
            let first = self.skills[source_slot.0 as usize].take();
            let second = self.skills[destination_slot.0 as usize].take();

            let first_data = first
                .as_ref()
                .map(|skill| HotkeyData {
                    is_skill: true as u8,
                    skill_id: skill.skill_id.0 as u32,
                    quantity_or_skill_level: skill.skill_level,
                })
                .unwrap_or(HotkeyData::UNBOUND);

            let second_data = second
                .as_ref()
                .map(|skill| HotkeyData {
                    is_skill: true as u8,
                    skill_id: skill.skill_id.0 as u32,
                    quantity_or_skill_level: skill.skill_level,
                })
                .unwrap_or(HotkeyData::UNBOUND);

            let _ = provider.set_hotkey_data(HotbarTab(0), destination_slot, first_data);
            let _ = provider.set_hotkey_data(HotbarTab(0), source_slot, second_data);

            self.skills[source_slot.0 as usize] = second;
            self.skills[destination_slot.0 as usize] = first;
        }
    }

    /// Clear the slot without notifying the map server.
    pub fn unset_slot(&mut self, slot: HotbarSlot) {
        self.skills[slot.0 as usize] = None;
    }

    /// Clear the slot and notify the map server.
    pub fn clear_slot(&mut self, provider: &mut dyn GameplayProvider, slot: HotbarSlot) {
        let _ = provider.set_hotkey_data(HotbarTab(0), slot, HotkeyData::UNBOUND);

        self.skills[slot.0 as usize] = None;
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> &Option<Skill> {
        &self.skills[slot.0 as usize]
    }
}
