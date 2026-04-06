use super::HudLayout;

/// Weapon and ammunition display rendering data.
pub struct WeaponDisplay {
    /// Shape collection index for the weapon icon, if any.
    pub weapon_icon_index: Option<u16>,
    /// Primary trigger ammunition count, if applicable.
    pub primary_ammo: Option<u16>,
    /// Secondary trigger ammunition count, if applicable.
    pub secondary_ammo: Option<u16>,
    /// Screen-space rectangle for the weapon icon area.
    pub icon_rect: [f32; 4],
    /// Screen-space position for primary ammo text.
    pub primary_ammo_pos: [f32; 2],
    /// Screen-space position for secondary ammo text.
    pub secondary_ammo_pos: [f32; 2],
}

impl WeaponDisplay {
    /// Compute weapon display rendering data.
    pub fn compute(
        weapon_icon_index: Option<u16>,
        primary_ammo: Option<u16>,
        secondary_ammo: Option<u16>,
        layout: &HudLayout,
    ) -> Self {
        let icon_size = 64.0 * layout.scale;
        let x = layout.screen_width as f32 - icon_size - 20.0 * layout.scale;
        let y = layout.screen_height as f32 - icon_size - 20.0 * layout.scale;

        let primary_ammo_pos = [x + icon_size + 4.0 * layout.scale, y + 10.0 * layout.scale];
        let secondary_ammo_pos = [
            x + icon_size + 4.0 * layout.scale,
            y + 30.0 * layout.scale,
        ];

        Self {
            weapon_icon_index,
            primary_ammo,
            secondary_ammo,
            icon_rect: [x, y, icon_size, icon_size],
            primary_ammo_pos,
            secondary_ammo_pos,
        }
    }
}
