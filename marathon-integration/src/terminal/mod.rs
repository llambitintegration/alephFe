pub mod images;
mod pages;
pub mod renderer;

pub use images::{TerminalImageCache, TerminalImageData};
pub use pages::{TerminalPage, TerminalPageLayout};
pub use renderer::TerminalRenderer;

/// Terminal text group style types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalStyle {
    Logon,
    Logoff,
    Information,
    Checkpoint,
    ChapterHeader,
    Pict,
}

/// Terminal activation result.
#[derive(Debug)]
pub enum TerminalActivation {
    /// Terminal was activated, content is available.
    Activated(TerminalContent),
    /// No terminal data at this polygon.
    NoTerminal,
}

/// Content of an activated terminal.
#[derive(Debug)]
pub struct TerminalContent {
    /// Pages of terminal content after layout and conditional evaluation.
    pub pages: Vec<TerminalPage>,
    /// Target level for teleport-on-exit, if any.
    pub teleport_target: Option<usize>,
    /// Terminal index for read-status tracking.
    pub terminal_index: usize,
}

/// Tracks which terminals the player has read.
#[derive(Debug, Clone, Default)]
pub struct TerminalReadTracker {
    read_terminals: Vec<usize>,
}

impl TerminalReadTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a terminal as read.
    pub fn mark_read(&mut self, terminal_index: usize) {
        if !self.read_terminals.contains(&terminal_index) {
            self.read_terminals.push(terminal_index);
        }
    }

    /// Check if a terminal has been read.
    pub fn is_read(&self, terminal_index: usize) -> bool {
        self.read_terminals.contains(&terminal_index)
    }

    /// Get all read terminal indices (for save data).
    pub fn read_list(&self) -> &[usize] {
        &self.read_terminals
    }

    /// Restore read status from save data.
    pub fn restore(&mut self, terminals: Vec<usize>) {
        self.read_terminals = terminals;
    }
}

/// Terminal navigation state.
#[derive(Debug)]
pub struct TerminalViewState {
    /// Current page index (0-based).
    pub current_page: usize,
    /// Scroll offset within the current page (in lines).
    pub scroll_offset: usize,
    /// Total number of pages.
    pub total_pages: usize,
}

impl TerminalViewState {
    pub fn new(total_pages: usize) -> Self {
        Self {
            current_page: 0,
            scroll_offset: 0,
            total_pages,
        }
    }

    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            self.scroll_offset = 0;
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.scroll_offset = 0;
        }
    }

    pub fn is_last_page(&self) -> bool {
        self.current_page + 1 >= self.total_pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_read_tracking() {
        let mut tracker = TerminalReadTracker::new();
        assert!(!tracker.is_read(0));

        tracker.mark_read(0);
        assert!(tracker.is_read(0));
        assert!(!tracker.is_read(1));

        // Duplicate mark is idempotent
        tracker.mark_read(0);
        assert_eq!(tracker.read_list().len(), 1);
    }

    #[test]
    fn terminal_read_restore() {
        let mut tracker = TerminalReadTracker::new();
        tracker.restore(vec![1, 3, 5]);
        assert!(tracker.is_read(1));
        assert!(tracker.is_read(3));
        assert!(tracker.is_read(5));
        assert!(!tracker.is_read(2));
    }

    #[test]
    fn view_state_navigation() {
        let mut view = TerminalViewState::new(5);
        assert_eq!(view.current_page, 0);
        assert!(!view.is_last_page());

        view.next_page();
        assert_eq!(view.current_page, 1);

        view.prev_page();
        assert_eq!(view.current_page, 0);

        // Can't go before first page
        view.prev_page();
        assert_eq!(view.current_page, 0);

        // Navigate to last page
        for _ in 0..10 {
            view.next_page();
        }
        assert_eq!(view.current_page, 4);
        assert!(view.is_last_page());
    }

    #[test]
    fn view_state_scrolling() {
        let mut view = TerminalViewState::new(3);
        view.scroll_down(5);
        assert_eq!(view.scroll_offset, 1);

        view.scroll_down(5);
        view.scroll_down(5);
        assert_eq!(view.scroll_offset, 3);

        view.scroll_up();
        assert_eq!(view.scroll_offset, 2);

        // Page change resets scroll
        view.next_page();
        assert_eq!(view.scroll_offset, 0);
    }
}
