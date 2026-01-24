//! Layout definitions for the TUI.
//!
//! This module provides layout configuration for the split-view terminal interface,
//! dividing the screen into task list, stream output, and controls sections.

use ratatui::prelude::*;

/// Create the outer vertical layout for the application.
///
/// Splits the screen into:
/// - Main content area (flexible height)
/// - Controls bar (fixed 3 lines)
///
/// # Arguments
///
/// * `frame` - The frame to create the layout for
///
/// # Returns
///
/// A vector of two `Rect` areas: main content and controls bar.
pub fn create_layout(frame: &Frame) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Controls bar
        ])
        .split(frame.size())
        .to_vec()
}

/// Create the horizontal layout for the main content area.
///
/// Splits the main content into:
/// - Task list (40% width)
/// - Stream output (60% width)
///
/// # Arguments
///
/// * `area` - The area to split
///
/// # Returns
///
/// A vector of two `Rect` areas: task list and stream.
pub fn create_main_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Task list
            Constraint::Percentage(60), // Stream
        ])
        .split(area)
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_main_layout() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = create_main_layout(area);

        assert_eq!(layout.len(), 2);
        // Task list should be ~40% of width
        assert_eq!(layout[0].width, 40);
        // Stream should be ~60% of width
        assert_eq!(layout[1].width, 60);
    }
}
