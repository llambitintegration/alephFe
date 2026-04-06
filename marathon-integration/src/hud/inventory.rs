use super::{HudLayout, InventoryItem};

/// Inventory panel rendering data.
pub struct InventoryPanel {
    /// Items to render with their screen-space positions.
    pub items: Vec<InventorySlot>,
    /// Whether the panel should be visible (hidden if empty).
    pub visible: bool,
}

/// A single item slot in the inventory panel.
pub struct InventorySlot {
    /// Shape index for the item icon.
    pub icon_index: u16,
    /// Item count.
    pub count: u16,
    /// Screen-space rectangle for the slot [x, y, width, height].
    pub rect: [f32; 4],
}

impl InventoryPanel {
    /// Compute inventory panel layout.
    pub fn compute(items: &[InventoryItem], layout: &HudLayout) -> Self {
        if items.is_empty() {
            return Self {
                items: Vec::new(),
                visible: false,
            };
        }

        let slot_size = 32.0 * layout.scale;
        let padding = 4.0 * layout.scale;
        let start_x = 20.0 * layout.scale;
        let start_y = layout.screen_height as f32 - 100.0 * layout.scale;

        let slots = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let x = start_x + i as f32 * (slot_size + padding);
                InventorySlot {
                    icon_index: item.icon_index,
                    count: item.count,
                    rect: [x, start_y, slot_size, slot_size],
                }
            })
            .collect();

        Self {
            items: slots,
            visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inventory_hidden() {
        let layout = HudLayout::for_resolution(640, 480);
        let panel = InventoryPanel::compute(&[], &layout);
        assert!(!panel.visible);
        assert!(panel.items.is_empty());
    }

    #[test]
    fn items_produce_visible_slots() {
        let items = vec![
            InventoryItem { icon_index: 1, count: 3 },
            InventoryItem { icon_index: 2, count: 1 },
        ];
        let layout = HudLayout::for_resolution(640, 480);
        let panel = InventoryPanel::compute(&items, &layout);
        assert!(panel.visible);
        assert_eq!(panel.items.len(), 2);
        assert_eq!(panel.items[0].count, 3);
        assert_eq!(panel.items[1].icon_index, 2);
    }
}
