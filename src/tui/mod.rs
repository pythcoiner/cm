//! Terminal UI for cm using ratatui.
//!
//! This module provides a real-time terminal user interface for monitoring
//! and controlling the cm orchestration process. It displays task progress,
//! agent output streams, and provides keyboard controls for pausing and
//! interrupting execution.

mod layout;
mod widgets;

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::Terminal;

use crate::state::TasksState;

pub use layout::{create_layout, create_main_layout};
pub use widgets::{render_controls, render_stream, render_task_list};

/// Main application state for the TUI.
pub struct App {
    /// The current tasks state from tasks.json.
    pub state: TasksState,
    /// Lines of output from the agent stream.
    pub stream_lines: Vec<StreamLine>,
    /// Current scroll offset for the stream view.
    pub scroll_offset: u16,
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Whether execution is paused.
    pub paused: bool,
    /// The ID of the currently executing task.
    pub current_task_id: Option<String>,
}

impl App {
    /// Create a new App instance with the given tasks state.
    pub fn new(state: TasksState) -> Self {
        let current_task_id = state.current_task.clone();
        Self {
            state,
            stream_lines: Vec::new(),
            scroll_offset: 0,
            should_quit: false,
            paused: false,
            current_task_id,
        }
    }

    /// Scroll up in the stream view.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down in the stream view.
    pub fn scroll_down(&mut self) {
        let max_scroll = self.stream_lines.len().saturating_sub(1) as u16;
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Add a line to the stream output.
    pub fn add_stream_line(&mut self, line: StreamLine) {
        self.stream_lines.push(line);
        // Auto-scroll to bottom when new content is added
        let max_scroll = self.stream_lines.len().saturating_sub(1) as u16;
        self.scroll_offset = max_scroll;
    }

    /// Update the tasks state.
    pub fn update_state(&mut self, state: TasksState) {
        self.current_task_id = state.current_task.clone();
        self.state = state;
    }
}

/// A line of output in the agent stream.
#[derive(Debug, Clone)]
pub enum StreamLine {
    /// A prompt sent to the agent.
    Prompt(String),
    /// A response from the agent.
    Response(String),
    /// An error message.
    Error(String),
    /// A build result message.
    BuildResult(String),
}

/// Commands that can be sent from the TUI to the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    /// Pause or resume execution.
    Pause,
    /// Interrupt the current task.
    Interrupt,
    /// Quit the application.
    Quit,
}

/// Events sent from the manager to the TUI.
#[derive(Debug, Clone)]
pub enum ManagerEvent {
    /// A task has started.
    TaskStarted {
        /// The ID of the task.
        task_id: String,
    },
    /// Output from the agent.
    AgentOutput(String),
    /// Result of a build operation.
    BuildResult {
        /// Whether the build succeeded.
        success: bool,
        /// Message describing the result.
        message: String,
    },
    /// A task has completed.
    TaskCompleted {
        /// The ID of the task.
        task_id: String,
    },
    /// An error occurred.
    Error(String),
    /// The tasks state has been updated.
    StateUpdated(TasksState),
}

/// Create channels for communication between the TUI and manager.
///
/// Returns a tuple of:
/// - `Sender<ManagerEvent>`: Send events from manager to TUI
/// - `Receiver<ManagerEvent>`: Receive events in TUI
/// - `Sender<TuiCommand>`: Send commands from TUI to manager
/// - `Receiver<TuiCommand>`: Receive commands in manager
pub fn create_channels() -> (
    Sender<ManagerEvent>,
    Receiver<ManagerEvent>,
    Sender<TuiCommand>,
    Receiver<TuiCommand>,
) {
    let (event_tx, event_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    (event_tx, event_rx, cmd_tx, cmd_rx)
}

/// Run the TUI application.
///
/// This function sets up the terminal, runs the main event loop,
/// and restores the terminal on exit.
///
/// # Arguments
///
/// * `state` - Initial tasks state to display
///
/// # Errors
///
/// Returns an error if terminal setup or restoration fails.
pub fn run_tui(state: TasksState) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new(state);

    let result = run_event_loop(&mut terminal, &mut app);

    // Always restore terminal, even if there was an error
    restore_terminal()?;

    result
}

/// Run the TUI with event channels for integration with the manager.
///
/// This allows the TUI to receive events from and send commands to
/// the manager running in another thread.
///
/// # Arguments
///
/// * `state` - Initial tasks state to display
/// * `event_rx` - Receiver for manager events
/// * `cmd_tx` - Sender for TUI commands
///
/// # Errors
///
/// Returns an error if terminal setup or restoration fails.
pub fn run_tui_with_channels(
    state: TasksState,
    event_rx: Receiver<ManagerEvent>,
    cmd_tx: Sender<TuiCommand>,
) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new(state);

    let result = run_event_loop_with_channels(&mut terminal, &mut app, event_rx, cmd_tx);

    restore_terminal()?;

    result
}

/// Set up the terminal for TUI mode.
fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to normal mode.
fn restore_terminal() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Handle a key event and return an optional command.
fn handle_key_event(app: &mut App, key: KeyEvent) -> Option<TuiCommand> {
    match key.code {
        KeyCode::Char('p') => {
            app.paused = !app.paused;
            Some(TuiCommand::Pause)
        }
        KeyCode::Char('i') => Some(TuiCommand::Interrupt),
        KeyCode::Char('q') => {
            app.should_quit = true;
            Some(TuiCommand::Quit)
        }
        KeyCode::Up => {
            app.scroll_up();
            None
        }
        KeyCode::Down => {
            app.scroll_down();
            None
        }
        KeyCode::PageUp => {
            // Scroll up by 10 lines
            for _ in 0..10 {
                app.scroll_up();
            }
            None
        }
        KeyCode::PageDown => {
            // Scroll down by 10 lines
            for _ in 0..10 {
                app.scroll_down();
            }
            None
        }
        _ => None,
    }
}

/// Draw the UI to the terminal.
fn draw_ui(frame: &mut Frame, app: &App) {
    let outer_layout = create_layout(frame);
    let main_layout = create_main_layout(outer_layout[0]);

    render_task_list(
        frame,
        main_layout[0],
        &app.state.phases,
        app.current_task_id.as_deref(),
    );
    render_stream(frame, main_layout[1], &app.stream_lines, app.scroll_offset);
    render_controls(frame, outer_layout[1], app.paused);
}

/// Run the main event loop (standalone mode).
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw_ui(frame, app))?;

        // Poll for events with a timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key_event(app, key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

/// Run the event loop with channels for manager integration.
fn run_event_loop_with_channels(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_rx: Receiver<ManagerEvent>,
    cmd_tx: Sender<TuiCommand>,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw_ui(frame, app))?;

        // Check for manager events (non-blocking)
        while let Ok(event) = event_rx.try_recv() {
            match event {
                ManagerEvent::TaskStarted { task_id } => {
                    app.current_task_id = Some(task_id.clone());
                    app.add_stream_line(StreamLine::Prompt(format!("Starting task: {}", task_id)));
                }
                ManagerEvent::AgentOutput(output) => {
                    app.add_stream_line(StreamLine::Response(output));
                }
                ManagerEvent::BuildResult { success, message } => {
                    let line = if success {
                        StreamLine::BuildResult(format!("[OK] {}", message))
                    } else {
                        StreamLine::Error(format!("[FAIL] {}", message))
                    };
                    app.add_stream_line(line);
                }
                ManagerEvent::TaskCompleted { task_id } => {
                    app.add_stream_line(StreamLine::Response(format!(
                        "Task completed: {}",
                        task_id
                    )));
                }
                ManagerEvent::Error(err) => {
                    app.add_stream_line(StreamLine::Error(err));
                }
                ManagerEvent::StateUpdated(state) => {
                    app.update_state(state);
                }
            }
        }

        // Poll for keyboard events with a short timeout
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(cmd) = handle_key_event(app, key) {
                    // Send command to manager (ignore send errors if manager disconnected)
                    let _ = cmd_tx.send(cmd);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Phase, PhaseStatus, Project, Task, TaskContext, TaskStatus, TaskType};

    fn create_test_state() -> TasksState {
        TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "test".to_string(),
                description: "test project".to_string(),
                created_at: None,
            },
            global_context: None,
            phases: vec![Phase {
                id: "phase-1".to_string(),
                name: "Phase 1".to_string(),
                status: PhaseStatus::InProgress,
                tasks: vec![Task {
                    id: "task-1".to_string(),
                    name: "Task 1".to_string(),
                    task_type: TaskType::Implement,
                    status: TaskStatus::Pending,
                    depends_on: vec![],
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Do task 1".to_string(),
                    attempts: vec![],
                }],
            }],
            current_phase: Some("phase-1".to_string()),
            current_task: Some("task-1".to_string()),
            agent_history: vec![],
            interrupted_at: None,
        }
    }

    #[test]
    fn test_app_new() {
        let state = create_test_state();
        let app = App::new(state);

        assert!(!app.should_quit);
        assert!(!app.paused);
        assert_eq!(app.scroll_offset, 0);
        assert!(app.stream_lines.is_empty());
        assert_eq!(app.current_task_id, Some("task-1".to_string()));
    }

    #[test]
    fn test_app_scroll() {
        let state = create_test_state();
        let mut app = App::new(state);

        // Add some stream lines
        for i in 0..10 {
            app.add_stream_line(StreamLine::Response(format!("Line {}", i)));
        }

        // After adding, scroll should be at bottom
        assert_eq!(app.scroll_offset, 9);

        // Scroll up
        app.scroll_up();
        assert_eq!(app.scroll_offset, 8);

        // Scroll down
        app.scroll_down();
        assert_eq!(app.scroll_offset, 9);

        // Can't scroll past end
        app.scroll_down();
        assert_eq!(app.scroll_offset, 9);
    }

    #[test]
    fn test_handle_key_event_quit() {
        let state = create_test_state();
        let mut app = App::new(state);

        let key = KeyEvent::from(KeyCode::Char('q'));
        let cmd = handle_key_event(&mut app, key);

        assert!(app.should_quit);
        assert_eq!(cmd, Some(TuiCommand::Quit));
    }

    #[test]
    fn test_handle_key_event_pause() {
        let state = create_test_state();
        let mut app = App::new(state);

        assert!(!app.paused);

        let key = KeyEvent::from(KeyCode::Char('p'));
        let cmd = handle_key_event(&mut app, key);

        assert!(app.paused);
        assert_eq!(cmd, Some(TuiCommand::Pause));

        // Toggle back
        let cmd = handle_key_event(&mut app, key);
        assert!(!app.paused);
        assert_eq!(cmd, Some(TuiCommand::Pause));
    }

    #[test]
    fn test_handle_key_event_interrupt() {
        let state = create_test_state();
        let mut app = App::new(state);

        let key = KeyEvent::from(KeyCode::Char('i'));
        let cmd = handle_key_event(&mut app, key);

        assert_eq!(cmd, Some(TuiCommand::Interrupt));
    }

    #[test]
    fn test_create_channels() {
        let (event_tx, event_rx, cmd_tx, cmd_rx) = create_channels();

        // Test event channel
        event_tx
            .send(ManagerEvent::AgentOutput("test".to_string()))
            .unwrap();
        let event = event_rx.recv().unwrap();
        assert!(matches!(event, ManagerEvent::AgentOutput(_)));

        // Test command channel
        cmd_tx.send(TuiCommand::Pause).unwrap();
        let cmd = cmd_rx.recv().unwrap();
        assert_eq!(cmd, TuiCommand::Pause);
    }
}
