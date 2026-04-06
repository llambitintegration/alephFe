use crate::shell::save::{SaveManager, SaveSlotInfo};

use super::{MenuItem, MenuItemAction, MenuScreen, MenuScreenState};

/// Build the load game screen from current save slot data.
///
/// Queries the save manager for occupied slots and builds a menu
/// showing each slot with its level and difficulty info.
pub fn load_game_screen(save_manager: &SaveManager) -> MenuScreenState {
    let slots = save_manager.list_slots();
    let items = build_load_game_items(&slots);

    MenuScreenState {
        screen: MenuScreen::LoadGame,
        items,
        cursor: 0,
    }
}

/// Build menu items from save slot info.
///
/// Each slot shows either its level/difficulty info or "Empty".
/// Only occupied slots are selectable.
fn build_load_game_items(slots: &[Option<SaveSlotInfo>]) -> Vec<MenuItem> {
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| match slot {
            Some(info) => MenuItem {
                label: format!(
                    "Slot {} - Level {} ({:?})",
                    i + 1,
                    info.level_index,
                    info.difficulty,
                ),
                action: MenuItemAction::LoadSaveSlot(i),
                enabled: true,
            },
            None => MenuItem {
                label: format!("Slot {} - Empty", i + 1),
                action: MenuItemAction::LoadSaveSlot(i),
                enabled: false,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::save::MAX_SAVE_SLOTS;
    use crate::types::Difficulty;
    use crate::types::GameModeType;

    #[test]
    fn empty_slots_are_disabled() {
        let slots: Vec<Option<SaveSlotInfo>> = (0..MAX_SAVE_SLOTS).map(|_| None).collect();
        let items = build_load_game_items(&slots);

        assert_eq!(items.len(), MAX_SAVE_SLOTS);
        for item in &items {
            assert!(!item.enabled);
            assert!(item.label.contains("Empty"));
        }
    }

    #[test]
    fn occupied_slot_is_enabled() {
        let mut slots: Vec<Option<SaveSlotInfo>> = (0..MAX_SAVE_SLOTS).map(|_| None).collect();
        slots[2] = Some(SaveSlotInfo {
            slot_index: 2,
            level_index: 5,
            difficulty: Difficulty::MajorDamage,
            game_mode: GameModeType::Campaign,
        });

        let items = build_load_game_items(&slots);
        assert!(items[2].enabled);
        assert!(items[2].label.contains("Level 5"));
        assert!(items[2].label.contains("MajorDamage"));
        assert_eq!(items[2].action, MenuItemAction::LoadSaveSlot(2));
    }

    #[test]
    fn mixed_slots() {
        let mut slots: Vec<Option<SaveSlotInfo>> = (0..MAX_SAVE_SLOTS).map(|_| None).collect();
        slots[0] = Some(SaveSlotInfo {
            slot_index: 0,
            level_index: 0,
            difficulty: Difficulty::Kindergarten,
            game_mode: GameModeType::Campaign,
        });
        slots[4] = Some(SaveSlotInfo {
            slot_index: 4,
            level_index: 12,
            difficulty: Difficulty::TotalCarnage,
            game_mode: GameModeType::Campaign,
        });

        let items = build_load_game_items(&slots);
        assert!(items[0].enabled);
        assert!(!items[1].enabled);
        assert!(!items[2].enabled);
        assert!(!items[3].enabled);
        assert!(items[4].enabled);
    }
}
