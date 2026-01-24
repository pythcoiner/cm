//! Widget rendering for the TUI.
//!
//! This module provides rendering functions for the task list, agent stream,
//! and controls bar widgets.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

use crate::state::{Phase, TaskStatus};

use super::StreamLine;

/// Render the task list widget.
///
/// Displays all phases with their nested tasks, using status icons
/// and highlighting the current task.
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `area` - The area to render in
/// * `phases` - The phases to display
/// * `current_task_id` - The ID of the currently executing task, if any
pub fn render_task_list(
    frame: &mut Frame,
    area: Rect,
    phases: &[Phase],
    current_task_id: Option<&str>,
) {
    let mut items: Vec<ListItem> = Vec::new();

    for phase in phases {
        // Add phase header
        let phase_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        items.push(ListItem::new(Line::from(Span::styled(
            format!("{} {}", phase_status_icon(&phase.status), &phase.name),
            phase_style,
        ))));

        // Add tasks
        for task in &phase.tasks {
            let is_current = current_task_id == Some(task.id.as_str());
            let icon = task_status_icon(&task.status);

            let task_style = if is_current {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match task.status {
                    TaskStatus::Completed => Style::default().fg(Color::Green),
                    TaskStatus::InProgress => Style::default().fg(Color::Yellow),
                    TaskStatus::Pending => Style::default().fg(Color::Gray),
                    TaskStatus::Deferred => Style::default().fg(Color::Red),
                }
            };

            let prefix = if is_current { " > " } else { "   " };
            items.push(ListItem::new(Line::from(Span::styled(
                format!("{}{} {}", prefix, icon, &task.name),
                task_style,
            ))));
        }

        // Add empty line between phases
        items.push(ListItem::new(""));
    }

    let list = List::new(items).block(
        Block::default()
            .title(" Tasks ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    );

    frame.render_widget(list, area);
}

/// Render the agent stream widget.
///
/// Displays a scrollable log of prompts, responses, errors, and build results.
/// Lines are color-coded by type.
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `area` - The area to render in
/// * `lines` - The stream lines to display
/// * `scroll_offset` - Current scroll offset from top
pub fn render_stream(frame: &mut Frame, area: Rect, lines: &[StreamLine], scroll_offset: u16) {
    let inner_area = Block::default()
        .title(" Agent Stream ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .inner(area);

    // Calculate visible lines based on area height
    let visible_height = inner_area.height as usize;
    let total_lines = lines.len();

    // Calculate which lines to display
    let start = scroll_offset as usize;
    let end = (start + visible_height).min(total_lines);

    let visible_lines: Vec<Line> = lines
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|line| match line {
            StreamLine::Prompt(text) => Line::from(Span::styled(
                format!("> {}", text),
                Style::default().fg(Color::Blue),
            )),
            StreamLine::Response(text) => {
                Line::from(Span::styled(text.as_str(), Style::default().fg(Color::Green)))
            }
            StreamLine::Error(text) => Line::from(Span::styled(
                format!("[ERROR] {}", text),
                Style::default().fg(Color::Red),
            )),
            StreamLine::BuildResult(text) => {
                Line::from(Span::styled(text.as_str(), Style::default().fg(Color::Magenta)))
            }
        })
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(
            Block::default()
                .title(" Agent Stream ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Render scrollbar if there are more lines than visible
    if total_lines > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(scroll_offset as usize)
            .viewport_content_length(visible_height);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Render the controls bar widget.
///
/// Displays keyboard shortcuts and current status (paused/running).
///
/// # Arguments
///
/// * `frame` - The frame to render to
/// * `area` - The area to render in
/// * `paused` - Whether execution is currently paused
pub fn render_controls(frame: &mut Frame, area: Rect, paused: bool) {
    let status = if paused {
        Span::styled(" PAUSED ", Style::default().bg(Color::Yellow).fg(Color::Black))
    } else {
        Span::styled(" RUNNING ", Style::default().bg(Color::Green).fg(Color::Black))
    };

    let controls = Line::from(vec![
        status,
        Span::raw("  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw(" Pause  "),
        Span::styled("[i]", Style::default().fg(Color::Cyan)),
        Span::raw(" Interrupt  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit  "),
        Span::styled("[Up/Down]", Style::default().fg(Color::Cyan)),
        Span::raw(" Scroll"),
    ]);

    let paragraph = Paragraph::new(controls)
        .block(
            Block::default()
                .title(" Controls ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Get the status icon for a task.
fn task_status_icon(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Completed => "\u{2713}",  // ✓
        TaskStatus::InProgress => "\u{25CF}", // ● (filled circle as spinner alternative)
        TaskStatus::Pending => "\u{25CB}",    // ○
        TaskStatus::Deferred => "\u{2298}",   // ⊘
    }
}

/// Get the status icon for a phase.
fn phase_status_icon(status: &crate::state::PhaseStatus) -> &'static str {
    match status {
        crate::state::PhaseStatus::Completed => "\u{2713}",  // ✓
        crate::state::PhaseStatus::InProgress => "\u{25B6}", // ▶
        crate::state::PhaseStatus::Pending => "\u{25CB}",    // ○
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PhaseStatus;

    #[test]
    fn test_task_status_icon() {
        assert_eq!(task_status_icon(&TaskStatus::Completed), "\u{2713}");
        assert_eq!(task_status_icon(&TaskStatus::InProgress), "\u{25CF}");
        assert_eq!(task_status_icon(&TaskStatus::Pending), "\u{25CB}");
        assert_eq!(task_status_icon(&TaskStatus::Deferred), "\u{2298}");
    }

    #[test]
    fn test_phase_status_icon() {
        assert_eq!(phase_status_icon(&PhaseStatus::Completed), "\u{2713}");
        assert_eq!(phase_status_icon(&PhaseStatus::InProgress), "\u{25B6}");
        assert_eq!(phase_status_icon(&PhaseStatus::Pending), "\u{25CB}");
    }
}
