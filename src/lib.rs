//! Claude Code Manager (cm) - Automated agent coordination for software development.
//!
//! This library provides the core functionality for orchestrating Claude agents
//! to execute multi-phase development tasks.

// Module declarations - placeholder modules until implementation
// These will be expanded into separate files/directories as implementation progresses

/// Producer socket for the am fleet monitor.
pub mod am;

/// Main orchestration loop, state machine, and crash recovery.
pub mod manager;

/// tasks.json handling, Task/Phase structs, and JSON validation.
pub mod state;

/// Agent spawning, prompt building, and response parsing.
pub mod agent;

/// Build verification: cargo build/clippy/test and git operations.
pub mod build;

/// Per-phase detailed logging.
pub mod log;

/// Deterministic markdown generation from JSON state.
pub mod generate;

/// CLI argument parsing with clap.
pub mod cli;

/// Configuration file support.
pub mod config;

/// Embedded command files for cm init.
pub mod command;

/// Post-run log gathering for the review agent.
pub mod review;
