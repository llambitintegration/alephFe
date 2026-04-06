use super::TerminalStyle;

/// A single page of terminal content.
#[derive(Debug, Clone)]
pub struct TerminalPage {
    /// Lines of styled text on this page.
    pub lines: Vec<TerminalLine>,
    /// Optional PICT resource ID for an image on this page.
    pub image_resource_id: Option<u16>,
}

/// A single line of terminal text.
#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub style: TerminalStyle,
}

/// Layout engine that splits terminal text groups into pages.
pub struct TerminalPageLayout {
    /// Maximum number of text lines per page.
    pub lines_per_page: usize,
}

impl Default for TerminalPageLayout {
    fn default() -> Self {
        Self { lines_per_page: 22 }
    }
}

impl TerminalPageLayout {
    /// Split a sequence of styled lines into pages.
    pub fn paginate(&self, lines: &[TerminalLine]) -> Vec<TerminalPage> {
        if lines.is_empty() {
            return vec![TerminalPage {
                lines: Vec::new(),
                image_resource_id: None,
            }];
        }

        lines
            .chunks(self.lines_per_page)
            .map(|chunk| TerminalPage {
                lines: chunk.to_vec(),
                image_resource_id: None,
            })
            .collect()
    }
}

/// Evaluate terminal text group conditions against game state.
///
/// `mission_success`: whether the current level's success objective has been met.
/// `groups`: list of (condition, lines) tuples where condition is:
///   - None = unconditional (always show)
///   - Some(true) = show on success
///   - Some(false) = show on failure
pub fn evaluate_conditional_groups(
    mission_success: bool,
    groups: &[(Option<bool>, Vec<TerminalLine>)],
) -> Vec<TerminalLine> {
    let mut result = Vec::new();
    for (condition, lines) in groups {
        match condition {
            None => result.extend(lines.iter().cloned()),
            Some(true) if mission_success => result.extend(lines.iter().cloned()),
            Some(false) if !mission_success => result.extend(lines.iter().cloned()),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lines(count: usize) -> Vec<TerminalLine> {
        (0..count)
            .map(|i| TerminalLine {
                text: format!("Line {i}"),
                style: TerminalStyle::Information,
            })
            .collect()
    }

    #[test]
    fn paginate_empty() {
        let layout = TerminalPageLayout::default();
        let pages = layout.paginate(&[]);
        assert_eq!(pages.len(), 1);
        assert!(pages[0].lines.is_empty());
    }

    #[test]
    fn paginate_within_single_page() {
        let layout = TerminalPageLayout::default();
        let lines = make_lines(10);
        let pages = layout.paginate(&lines);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].lines.len(), 10);
    }

    #[test]
    fn paginate_across_multiple_pages() {
        let layout = TerminalPageLayout { lines_per_page: 5 };
        let lines = make_lines(12);
        let pages = layout.paginate(&lines);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].lines.len(), 5);
        assert_eq!(pages[1].lines.len(), 5);
        assert_eq!(pages[2].lines.len(), 2);
    }

    #[test]
    fn conditional_groups_success() {
        let success_lines = vec![TerminalLine {
            text: "You won!".into(),
            style: TerminalStyle::Information,
        }];
        let failure_lines = vec![TerminalLine {
            text: "You lost!".into(),
            style: TerminalStyle::Information,
        }];
        let unconditional = vec![TerminalLine {
            text: "Welcome.".into(),
            style: TerminalStyle::Logon,
        }];

        let groups = vec![
            (None, unconditional),
            (Some(true), success_lines),
            (Some(false), failure_lines),
        ];

        let result = evaluate_conditional_groups(true, &groups);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Welcome.");
        assert_eq!(result[1].text, "You won!");
    }

    #[test]
    fn conditional_groups_failure() {
        let success_lines = vec![TerminalLine {
            text: "You won!".into(),
            style: TerminalStyle::Information,
        }];
        let failure_lines = vec![TerminalLine {
            text: "You lost!".into(),
            style: TerminalStyle::Information,
        }];

        let groups = vec![
            (Some(true), success_lines),
            (Some(false), failure_lines),
        ];

        let result = evaluate_conditional_groups(false, &groups);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "You lost!");
    }
}
