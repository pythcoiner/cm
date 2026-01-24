//! Claude Code Manager (cm) - Automated agent coordination for software development.
//!
//! This library provides the core functionality for orchestrating Claude agents
//! to execute multi-phase development tasks.

// Module declarations - placeholder modules until implementation
// These will be expanded into separate files/directories as implementation progresses

/// Main orchestration loop, state machine, and crash recovery.
pub mod manager {
    // TODO: Implement orchestration loop
    // TODO: Implement state machine
    // TODO: Implement crash recovery
}

/// tasks.json handling, Task/Phase structs, and JSON validation.
pub mod state {
    // TODO: Implement tasks.json loading/saving
    // TODO: Implement Task, Phase, TaskStatus structs
    // TODO: Implement JSON schema validation
}

/// Agent spawning, prompt building, and response parsing.
pub mod agent {
    // TODO: Implement agent spawning with `claude -p`
    // TODO: Implement prompt building with context isolation
    // TODO: Implement JSON response parsing
}

/// Build verification: cargo build/clippy/test and git operations.
pub mod build {
    // TODO: Implement cargo build/clippy/test verification
    // TODO: Implement git commit/status operations
}

/// LOG.md management for audit trail.
pub mod log {
    // TODO: Implement LOG.md append-only management
}

/// Terminal UI with ratatui for real-time progress display.
pub mod tui {
    // TODO: Implement TUI app with terminal setup
    // TODO: Implement split view layout
    // TODO: Implement TaskList, Stream, Controls widgets
}

/// CLI argument parsing with clap.
pub mod cli {
    // TODO: Implement CLI argument parsing
}
