use super::MenuScreenState;

/// RGBA color as [r, g, b, a] in 0.0..1.0.
pub type Color = [f32; 4];

/// Colors for menu rendering, matching Marathon's visual style.
pub mod colors {
    use super::Color;

    pub const BACKGROUND: Color = [0.0, 0.0, 0.0, 0.95];
    pub const TITLE_TEXT: Color = [0.0, 0.8, 0.0, 1.0];
    pub const ITEM_TEXT: Color = [0.7, 0.7, 0.7, 1.0];
    pub const ITEM_HIGHLIGHTED: Color = [0.0, 1.0, 0.0, 1.0];
    pub const ITEM_DISABLED: Color = [0.3, 0.3, 0.3, 0.6];
    pub const CURSOR: Color = [0.0, 1.0, 0.0, 0.8];
}

/// A text element to render on the menu screen.
#[derive(Debug, Clone)]
pub struct MenuText {
    /// Screen-space position [x, y].
    pub position: [f32; 2],
    /// Text content.
    pub text: String,
    /// Text color.
    pub color: Color,
    /// Font size in pixels.
    pub font_size: f32,
    /// Horizontal alignment.
    pub alignment: TextAlignment,
}

/// Text horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
}

/// A rectangular element (background, cursor highlight).
#[derive(Debug, Clone)]
pub struct MenuQuad {
    /// Screen-space rectangle [x, y, width, height].
    pub rect: [f32; 4],
    /// Fill color.
    pub color: Color,
}

/// All draw commands for a menu frame.
#[derive(Debug, Clone, Default)]
pub struct MenuDrawList {
    pub quads: Vec<MenuQuad>,
    pub texts: Vec<MenuText>,
}

/// Configuration for menu layout.
#[derive(Debug, Clone)]
pub struct MenuLayout {
    pub screen_width: u32,
    pub screen_height: u32,
    pub scale: f32,
}

impl MenuLayout {
    pub fn for_resolution(width: u32, height: u32) -> Self {
        let scale = (width as f32 / 640.0).min(height as f32 / 480.0);
        Self {
            screen_width: width,
            screen_height: height,
            scale,
        }
    }
}

/// Menu renderer that builds draw lists from menu state.
pub struct MenuRenderer {
    layout: MenuLayout,
}

impl MenuRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layout: MenuLayout::for_resolution(width, height),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.layout = MenuLayout::for_resolution(width, height);
    }

    /// Build draw commands for the current menu screen.
    pub fn build_draw_list(&self, screen: &MenuScreenState) -> MenuDrawList {
        let mut list = MenuDrawList::default();

        // Full-screen background
        list.quads.push(MenuQuad {
            rect: [0.0, 0.0, self.layout.screen_width as f32, self.layout.screen_height as f32],
            color: colors::BACKGROUND,
        });

        let title_font_size = 24.0 * self.layout.scale;
        let item_font_size = 16.0 * self.layout.scale;
        let center_x = self.layout.screen_width as f32 / 2.0;
        let item_spacing = 30.0 * self.layout.scale;
        let start_y = self.layout.screen_height as f32 * 0.3;

        // Screen title
        let title = match screen.screen {
            super::MenuScreen::MainMenu => "MARATHON",
            super::MenuScreen::NewGame => "SELECT DIFFICULTY",
            super::MenuScreen::LoadGame => "LOAD GAME",
            super::MenuScreen::Preferences => "PREFERENCES",
            super::MenuScreen::PauseMenu => "PAUSED",
        };

        list.texts.push(MenuText {
            position: [center_x, start_y - 40.0 * self.layout.scale],
            text: title.to_string(),
            color: colors::TITLE_TEXT,
            font_size: title_font_size,
            alignment: TextAlignment::Center,
        });

        // Menu items
        let item_width = 300.0 * self.layout.scale;
        let item_height = 24.0 * self.layout.scale;

        for (i, item) in screen.items.iter().enumerate() {
            let y = start_y + i as f32 * item_spacing;
            let is_selected = i == screen.cursor;

            // Cursor highlight
            if is_selected {
                list.quads.push(MenuQuad {
                    rect: [
                        center_x - item_width / 2.0,
                        y - item_height / 4.0,
                        item_width,
                        item_height,
                    ],
                    color: colors::CURSOR,
                });
            }

            let color = if !item.enabled {
                colors::ITEM_DISABLED
            } else if is_selected {
                colors::ITEM_HIGHLIGHTED
            } else {
                colors::ITEM_TEXT
            };

            list.texts.push(MenuText {
                position: [center_x, y],
                text: item.label.clone(),
                color,
                font_size: item_font_size,
                alignment: TextAlignment::Center,
            });
        }

        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::MenuStack;

    #[test]
    fn main_menu_draw_list() {
        let renderer = MenuRenderer::new(640, 480);
        let stack = MenuStack::new();
        let list = renderer.build_draw_list(stack.current());

        // Background quad + cursor highlight quad = 2 quads
        assert_eq!(list.quads.len(), 2);
        // Title + 4 menu items = 5 texts
        assert_eq!(list.texts.len(), 5);
        // Title should be "MARATHON"
        assert_eq!(list.texts[0].text, "MARATHON");
    }

    #[test]
    fn new_game_screen_draw_list() {
        let renderer = MenuRenderer::new(640, 480);
        let screen = MenuStack::new_game_screen();
        let list = renderer.build_draw_list(&screen);

        // Background + cursor highlight = 2 quads
        assert_eq!(list.quads.len(), 2);
        // Title + 5 difficulty items = 6 texts
        assert_eq!(list.texts.len(), 6);
        assert_eq!(list.texts[0].text, "SELECT DIFFICULTY");
    }

    #[test]
    fn cursor_at_position_highlights_correct_item() {
        let renderer = MenuRenderer::new(640, 480);
        let mut stack = MenuStack::new();
        stack.cursor_down(); // cursor now at index 1

        let list = renderer.build_draw_list(stack.current());
        // The highlighted item should be at index 1 (Load Game)
        // texts[0] = title, texts[1] = New Game, texts[2] = Load Game (highlighted)
        assert_eq!(list.texts[2].color, colors::ITEM_HIGHLIGHTED);
    }

    #[test]
    fn resolution_scaling() {
        let renderer_low = MenuRenderer::new(640, 480);
        let renderer_high = MenuRenderer::new(1920, 1080);
        let stack = MenuStack::new();

        let list_low = renderer_low.build_draw_list(stack.current());
        let list_high = renderer_high.build_draw_list(stack.current());

        // Title font should be larger at higher resolution
        assert!(list_high.texts[0].font_size > list_low.texts[0].font_size);
    }
}
