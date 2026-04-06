use super::{TerminalStyle, TerminalViewState};
use super::pages::TerminalPage;

/// RGBA color as [r, g, b, a] in 0.0..1.0.
pub type Color = [f32; 4];

/// Marathon terminal color palette.
pub mod colors {
    use super::Color;

    /// Dark terminal background.
    pub const BACKGROUND: Color = [0.0, 0.02, 0.0, 0.95];
    /// Standard information text (green on dark).
    pub const INFORMATION: Color = [0.0, 0.85, 0.0, 1.0];
    /// Logon screen header text (bright green).
    pub const LOGON: Color = [0.0, 1.0, 0.0, 1.0];
    /// Logoff screen text (dimmer green).
    pub const LOGOFF: Color = [0.0, 0.6, 0.0, 1.0];
    /// Checkpoint text (yellow-green).
    pub const CHECKPOINT: Color = [0.6, 0.9, 0.0, 1.0];
    /// Chapter header text (bright white).
    pub const CHAPTER_HEADER: Color = [1.0, 1.0, 1.0, 1.0];
    /// Page indicator text.
    pub const PAGE_INDICATOR: Color = [0.0, 0.5, 0.0, 1.0];
    /// Terminal border.
    pub const BORDER: Color = [0.0, 0.4, 0.0, 1.0];
}

/// A text element for terminal rendering.
#[derive(Debug, Clone)]
pub struct TerminalText {
    /// Screen-space position [x, y].
    pub position: [f32; 2],
    /// Text content.
    pub text: String,
    /// Text color.
    pub color: Color,
    /// Font size in pixels.
    pub font_size: f32,
    /// Whether this is centered (chapter headers) or left-aligned.
    pub centered: bool,
}

/// A rectangle for terminal rendering (background, border, image placeholder).
#[derive(Debug, Clone)]
pub struct TerminalQuad {
    pub rect: [f32; 4],
    pub color: Color,
}

/// An image reference to render in the terminal.
#[derive(Debug, Clone)]
pub struct TerminalImage {
    /// Screen-space rectangle [x, y, width, height].
    pub rect: [f32; 4],
    /// PICT resource ID.
    pub resource_id: u16,
}

/// All draw commands for a terminal frame.
#[derive(Debug, Clone, Default)]
pub struct TerminalDrawList {
    pub quads: Vec<TerminalQuad>,
    pub texts: Vec<TerminalText>,
    pub images: Vec<TerminalImage>,
}

/// Layout configuration for terminal rendering.
#[derive(Debug, Clone)]
pub struct TerminalLayout {
    pub screen_width: u32,
    pub screen_height: u32,
    pub scale: f32,
    /// Terminal content area margins from screen edges.
    pub margin: f32,
    /// Line height in pixels.
    pub line_height: f32,
    /// Font size for body text.
    pub body_font_size: f32,
    /// Font size for chapter headers.
    pub header_font_size: f32,
}

impl TerminalLayout {
    pub fn for_resolution(width: u32, height: u32) -> Self {
        let scale = (width as f32 / 640.0).min(height as f32 / 480.0);
        Self {
            screen_width: width,
            screen_height: height,
            scale,
            margin: 40.0 * scale,
            line_height: 16.0 * scale,
            body_font_size: 14.0 * scale,
            header_font_size: 20.0 * scale,
        }
    }

    /// Content area width.
    pub fn content_width(&self) -> f32 {
        self.screen_width as f32 - 2.0 * self.margin
    }

    /// Number of visible lines in the content area.
    pub fn visible_lines(&self) -> usize {
        let content_height = self.screen_height as f32 - 2.0 * self.margin - 30.0 * self.scale;
        (content_height / self.line_height) as usize
    }
}

/// Terminal renderer that produces draw lists from terminal state.
pub struct TerminalRenderer {
    layout: TerminalLayout,
}

impl TerminalRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            layout: TerminalLayout::for_resolution(width, height),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.layout = TerminalLayout::for_resolution(width, height);
    }

    /// Build draw commands for the current terminal page and view state.
    pub fn build_draw_list(
        &self,
        page: &TerminalPage,
        view: &TerminalViewState,
    ) -> TerminalDrawList {
        let mut list = TerminalDrawList::default();

        // Full-screen background
        list.quads.push(TerminalQuad {
            rect: [
                0.0,
                0.0,
                self.layout.screen_width as f32,
                self.layout.screen_height as f32,
            ],
            color: colors::BACKGROUND,
        });

        // Terminal border
        let border_thickness = 2.0 * self.layout.scale;
        let margin = self.layout.margin;
        let sw = self.layout.screen_width as f32;
        let sh = self.layout.screen_height as f32;

        // Top border
        list.quads.push(TerminalQuad {
            rect: [margin, margin, sw - 2.0 * margin, border_thickness],
            color: colors::BORDER,
        });
        // Bottom border
        list.quads.push(TerminalQuad {
            rect: [margin, sh - margin, sw - 2.0 * margin, border_thickness],
            color: colors::BORDER,
        });
        // Left border
        list.quads.push(TerminalQuad {
            rect: [margin, margin, border_thickness, sh - 2.0 * margin],
            color: colors::BORDER,
        });
        // Right border
        list.quads.push(TerminalQuad {
            rect: [sw - margin, margin, border_thickness, sh - 2.0 * margin],
            color: colors::BORDER,
        });

        // Terminal image (if present)
        if let Some(resource_id) = page.image_resource_id {
            let img_height = 200.0 * self.layout.scale;
            list.images.push(TerminalImage {
                rect: [
                    margin + border_thickness + 4.0 * self.layout.scale,
                    margin + border_thickness + 4.0 * self.layout.scale,
                    self.layout.content_width() - 8.0 * self.layout.scale,
                    img_height,
                ],
                resource_id,
            });
        }

        // Text lines (with scroll offset)
        let visible_lines = self.layout.visible_lines();
        let content_x = margin + border_thickness + 8.0 * self.layout.scale;
        let content_start_y = margin + border_thickness + 8.0 * self.layout.scale;

        let start_line = view.scroll_offset;
        let end_line = (start_line + visible_lines).min(page.lines.len());

        for (display_idx, line_idx) in (start_line..end_line).enumerate() {
            let line = &page.lines[line_idx];
            let y = content_start_y + display_idx as f32 * self.layout.line_height;

            let (color, font_size, centered) = style_to_visual(&line.style, &self.layout);

            let x = if centered {
                self.layout.screen_width as f32 / 2.0
            } else {
                content_x
            };

            list.texts.push(TerminalText {
                position: [x, y],
                text: line.text.clone(),
                color,
                font_size,
                centered,
            });
        }

        // Page indicator
        let indicator_y = sh - margin + 4.0 * self.layout.scale;
        list.texts.push(TerminalText {
            position: [sw / 2.0, indicator_y],
            text: format!(
                "{}/{}",
                view.current_page + 1,
                view.total_pages
            ),
            color: colors::PAGE_INDICATOR,
            font_size: self.layout.body_font_size * 0.8,
            centered: true,
        });

        list
    }
}

/// Map terminal style to visual properties (color, font size, centered).
fn style_to_visual(style: &TerminalStyle, layout: &TerminalLayout) -> (Color, f32, bool) {
    match style {
        TerminalStyle::Information => (colors::INFORMATION, layout.body_font_size, false),
        TerminalStyle::Logon => (colors::LOGON, layout.body_font_size, false),
        TerminalStyle::Logoff => (colors::LOGOFF, layout.body_font_size, false),
        TerminalStyle::Checkpoint => (colors::CHECKPOINT, layout.body_font_size, false),
        TerminalStyle::ChapterHeader => (colors::CHAPTER_HEADER, layout.header_font_size, true),
        TerminalStyle::Pict => (colors::INFORMATION, layout.body_font_size, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::pages::TerminalLine;

    fn make_page(line_count: usize) -> TerminalPage {
        TerminalPage {
            lines: (0..line_count)
                .map(|i| TerminalLine {
                    text: format!("Line {i}"),
                    style: TerminalStyle::Information,
                })
                .collect(),
            image_resource_id: None,
        }
    }

    #[test]
    fn basic_draw_list() {
        let renderer = TerminalRenderer::new(640, 480);
        let page = make_page(5);
        let view = TerminalViewState::new(1);

        let list = renderer.build_draw_list(&page, &view);

        // Background + 4 border quads = 5 quads
        assert_eq!(list.quads.len(), 5);
        // 5 text lines + page indicator = 6 texts
        assert_eq!(list.texts.len(), 6);
        assert!(list.images.is_empty());
    }

    #[test]
    fn page_with_image() {
        let renderer = TerminalRenderer::new(640, 480);
        let page = TerminalPage {
            lines: vec![TerminalLine {
                text: "Some text".into(),
                style: TerminalStyle::Information,
            }],
            image_resource_id: Some(1200),
        };
        let view = TerminalViewState::new(1);

        let list = renderer.build_draw_list(&page, &view);
        assert_eq!(list.images.len(), 1);
        assert_eq!(list.images[0].resource_id, 1200);
    }

    #[test]
    fn scroll_offset_skips_lines() {
        let renderer = TerminalRenderer::new(640, 480);
        let page = make_page(30);
        let mut view = TerminalViewState::new(1);
        view.scroll_offset = 5;

        let list = renderer.build_draw_list(&page, &view);
        // First visible text should be "Line 5" (after the visible lines + page indicator)
        // Texts: visible_lines worth of content lines + page indicator
        let first_content_text = &list.texts[0];
        assert_eq!(first_content_text.text, "Line 5");
    }

    #[test]
    fn page_indicator_shows_position() {
        let renderer = TerminalRenderer::new(640, 480);
        let page = make_page(3);
        let mut view = TerminalViewState::new(5);
        view.current_page = 2;

        let list = renderer.build_draw_list(&page, &view);
        let indicator = list.texts.last().unwrap();
        assert_eq!(indicator.text, "3/5");
    }

    #[test]
    fn chapter_header_is_centered() {
        let renderer = TerminalRenderer::new(640, 480);
        let page = TerminalPage {
            lines: vec![TerminalLine {
                text: "Chapter One".into(),
                style: TerminalStyle::ChapterHeader,
            }],
            image_resource_id: None,
        };
        let view = TerminalViewState::new(1);

        let list = renderer.build_draw_list(&page, &view);
        // First content text should be centered
        assert!(list.texts[0].centered);
    }

    #[test]
    fn style_colors_are_distinct() {
        let layout = TerminalLayout::for_resolution(640, 480);
        let (info_color, _, _) = style_to_visual(&TerminalStyle::Information, &layout);
        let (logon_color, _, _) = style_to_visual(&TerminalStyle::Logon, &layout);
        let (header_color, _, _) = style_to_visual(&TerminalStyle::ChapterHeader, &layout);

        assert_ne!(info_color, logon_color);
        assert_ne!(info_color, header_color);
    }
}
