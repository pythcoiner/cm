//! CLI argument parsing and command execution.
//!
//! This module provides the command-line interface for the cm binary,
//! including argument parsing with clap and dispatching to the appropriate
//! execution modes (run, continue, step, status, validate).

use std::path::PathBuf;

use clap::Parser;
use log::{debug, info, warn};
use thiserror::Error;

use crate::manager::{Manager, ManagerConfig, ManagerError, RecoveryAction, RecoveryManager};
use crate::state::{load_state, save_state, StateError, TaskStatus, TasksState};

/// Errors that can occur during CLI execution.
#[derive(Debug, Error)]
pub enum CliError {
    /// An error occurred while managing state.
    #[error("state error: {0}")]
    StateError(#[from] StateError),

    /// An error occurred during manager execution.
    #[error("manager error: {0}")]
    ManagerError(#[from] ManagerError),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// Validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// No tasks to execute.
    #[error("no tasks to execute")]
    NoTasks,
}

/// Claude Code Manager - Automated agent coordination for software development.
///
/// Orchestrates Claude agents to execute multi-phase development tasks
/// defined in a tasks.json file.
#[derive(Debug, Parser)]
#[command(name = "cm")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Resume from interrupted state.
    #[arg(long = "continue")]
    pub resume: bool,

    /// Execute one task only.
    #[arg(long)]
    pub step: bool,

    /// Show progress without executing.
    #[arg(long)]
    pub status: bool,

    /// Validate tasks.json schema.
    #[arg(long)]
    pub validate: bool,

    /// Enable verbose output.
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to config file.
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to tasks.json state file.
    #[arg(long, value_name = "FILE", default_value = ".cm/tasks.json")]
    pub state: PathBuf,
}

/// Run the CLI application.
///
/// This is the main entry point for the CLI. It parses command-line arguments
/// and dispatches to the appropriate execution mode.
///
/// # Errors
///
/// Returns an error if:
/// - The state file cannot be loaded
/// - Task execution fails
/// - Validation fails
pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();

    // Initialize logger based on verbosity
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    info!("Claude Code Manager starting...");
    debug!("CLI arguments: {:?}", cli);

    if let Some(config) = &cli.config {
        debug!("Using config file: {:?}", config);
    }

    // Dispatch based on flags
    let result = if cli.status {
        execute_status(&cli)
    } else if cli.validate {
        execute_validate(&cli)
    } else if cli.step {
        execute_step(&cli)
    } else if cli.resume {
        execute_continue(&cli)
    } else {
        execute_run(&cli)
    };

    match &result {
        Ok(()) => info!("Claude Code Manager finished."),
        Err(e) => warn!("Claude Code Manager finished with error: {}", e),
    }

    result
}

/// Execute the default run mode.
///
/// Runs all tasks until completion or error.
fn execute_run(cli: &Cli) -> Result<(), CliError> {
    info!("Run mode: executing all tasks from {:?}", cli.state);

    let config = ManagerConfig::new(cli.state.clone())
        .log_path(cli.state.parent().unwrap_or(std::path::Path::new(".")).join("LOG.md"))
        .working_dir(std::env::current_dir()?);

    let mut manager = Manager::new(config)?;
    manager.run()?;

    Ok(())
}

/// Execute the continue mode.
///
/// Resumes from interrupted state, using recovery manager to determine
/// the appropriate action.
fn execute_continue(cli: &Cli) -> Result<(), CliError> {
    info!("Continue mode: resuming from {:?}", cli.state);

    let state = load_state(&cli.state)?;
    let checkpoints_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("checkpoints");
    let recovery = RecoveryManager::new(checkpoints_dir);

    let action = recovery
        .recover_from_crash(&state)
        .map_err(|e| CliError::StateError(StateError::ParseError(e.to_string())))?;

    info!("Recovery action: {}", action);

    match action {
        RecoveryAction::Continue | RecoveryAction::Retry => {
            // Resume execution - clear any interrupted state first
            let mut state = state;
            state.clear_interrupted();

            // Reset any in_progress tasks to pending for retry
            for phase in &mut state.phases {
                for task in &mut phase.tasks {
                    if task.status == TaskStatus::InProgress {
                        info!("Resetting in_progress task {} to pending", task.id);
                        task.status = TaskStatus::Pending;
                    }
                }
            }

            // Save the updated state
            save_state(&state, &cli.state)?;

            // Now run with the cleaned state
            let config = ManagerConfig::new(cli.state.clone())
                .log_path(
                    cli.state
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("LOG.md"),
                )
                .working_dir(std::env::current_dir()?);

            let mut manager = Manager::new(config)?;
            manager.run()?;
        }
        RecoveryAction::Rollback(checkpoint_id) => {
            // Restore from checkpoint and resume
            info!("Rolling back to checkpoint {}", checkpoint_id);

            let restored_state = recovery
                .restore(&checkpoint_id)
                .map_err(|e| CliError::StateError(StateError::ParseError(e.to_string())))?;

            // Save the restored state
            save_state(&restored_state, &cli.state)?;

            // Now run with the restored state
            let config = ManagerConfig::new(cli.state.clone())
                .log_path(
                    cli.state
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("LOG.md"),
                )
                .working_dir(std::env::current_dir()?);

            let mut manager = Manager::new(config)?;
            manager.run()?;
        }
        RecoveryAction::Skip => {
            // Mark interrupted task as deferred and continue
            let mut state = state;
            state.clear_interrupted();

            // Find and defer any in_progress tasks
            for phase in &mut state.phases {
                for task in &mut phase.tasks {
                    if task.status == TaskStatus::InProgress {
                        info!("Deferring interrupted task {}", task.id);
                        task.status = TaskStatus::Deferred;
                    }
                }
            }

            // Save the updated state
            save_state(&state, &cli.state)?;

            // Now run with the remaining tasks
            let config = ManagerConfig::new(cli.state.clone())
                .log_path(
                    cli.state
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("LOG.md"),
                )
                .working_dir(std::env::current_dir()?);

            let mut manager = Manager::new(config)?;
            manager.run()?;
        }
    }

    Ok(())
}

/// Execute the step mode.
///
/// Executes one task only, then pauses.
fn execute_step(cli: &Cli) -> Result<(), CliError> {
    info!("Step mode: executing one task from {:?}", cli.state);

    let config = ManagerConfig::new(cli.state.clone())
        .log_path(
            cli.state
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("LOG.md"),
        )
        .working_dir(std::env::current_dir()?);

    let mut manager = Manager::new(config)?;
    manager.step()?;

    Ok(())
}

/// Execute the status mode.
///
/// Shows progress without executing any tasks.
fn execute_status(cli: &Cli) -> Result<(), CliError> {
    info!("Status mode: showing progress from {:?}", cli.state);

    let state = load_state(&cli.state)?;
    print_status(&state);

    Ok(())
}

/// Print the current status of the project.
fn print_status(state: &TasksState) {
    // Project info
    println!("Project: {}", state.project.name);
    println!("Description: {}", state.project.description);
    println!();

    // Phase progress
    let total_phases = state.phases.len();
    let completed_phases = state
        .phases
        .iter()
        .filter(|p| p.status == crate::state::PhaseStatus::Completed)
        .count();
    println!(
        "Phase progress: {}/{} phases complete",
        completed_phases, total_phases
    );

    // Current phase and task
    if let Some(current_phase) = state.current_phase() {
        println!("Current phase: {} ({})", current_phase.name, current_phase.id);

        if let Some(current_task) = state.current_task() {
            println!("Current task: {} ({})", current_task.name, current_task.id);
        }
    }
    println!();

    // Task summary by status
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut deferred = 0;

    for phase in &state.phases {
        for task in &phase.tasks {
            match task.status {
                TaskStatus::Pending => pending += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Deferred => deferred += 1,
            }
        }
    }

    let total = pending + in_progress + completed + deferred;
    println!("Task summary:");
    println!("  Completed:   {}/{}", completed, total);
    println!("  In Progress: {}/{}", in_progress, total);
    println!("  Pending:     {}/{}", pending, total);
    println!("  Deferred:    {}/{}", deferred, total);
    println!();

    // List deferred tasks
    let deferred_tasks: Vec<_> = state
        .phases
        .iter()
        .flat_map(|p| &p.tasks)
        .filter(|t| t.status == TaskStatus::Deferred)
        .collect();

    if !deferred_tasks.is_empty() {
        println!("Deferred tasks:");
        for task in deferred_tasks {
            println!("  - {} ({})", task.name, task.id);
        }
        println!();
    }

    // Phase details
    println!("Phases:");
    for phase in &state.phases {
        let phase_tasks = phase.tasks.len();
        let phase_completed = phase
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();

        let status_icon = match phase.status {
            crate::state::PhaseStatus::Pending => "[ ]",
            crate::state::PhaseStatus::InProgress => "[~]",
            crate::state::PhaseStatus::Completed => "[x]",
        };

        println!(
            "  {} {} ({}/{} tasks)",
            status_icon, phase.name, phase_completed, phase_tasks
        );

        // List tasks in this phase
        for task in &phase.tasks {
            let task_icon = match task.status {
                TaskStatus::Pending => "    [ ]",
                TaskStatus::InProgress => "    [~]",
                TaskStatus::Completed => "    [x]",
                TaskStatus::Deferred => "    [!]",
            };
            println!("{} {}", task_icon, task.name);
        }
    }

    // Interrupted state warning
    if state.was_interrupted() {
        println!();
        println!("WARNING: Execution was interrupted. Use --continue to resume.");
        if let Some(interrupted_at) = state.interrupted_at {
            println!("Interrupted at: {}", interrupted_at);
        }
    }
}

/// Execute the validate mode.
///
/// Validates the tasks.json schema without executing any tasks.
fn execute_validate(cli: &Cli) -> Result<(), CliError> {
    info!("Validate mode: validating {:?}", cli.state);

    let state = load_state(&cli.state)?;

    // Validate state is well-formed
    let mut errors: Vec<String> = Vec::new();

    // Check version
    if state.version.is_empty() {
        errors.push("version is empty".to_string());
    }

    // Check project name
    if state.project.name.is_empty() {
        errors.push("project.name is empty".to_string());
    }

    // Collect all task IDs for uniqueness and dependency checks
    let mut all_task_ids: Vec<&str> = Vec::new();
    let mut task_id_set: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for phase in &state.phases {
        // Check phase ID
        if phase.id.is_empty() {
            errors.push(format!("phase has empty id (name: {})", phase.name));
        }

        for task in &phase.tasks {
            // Check task ID
            if task.id.is_empty() {
                errors.push(format!(
                    "task has empty id in phase {} (name: {})",
                    phase.id, task.name
                ));
            } else {
                // Check for duplicate task IDs
                if task_id_set.contains(task.id.as_str()) {
                    errors.push(format!("duplicate task id: {}", task.id));
                } else {
                    task_id_set.insert(&task.id);
                    all_task_ids.push(&task.id);
                }
            }
        }
    }

    // Check dependencies reference valid tasks
    for phase in &state.phases {
        for task in &phase.tasks {
            for dep_id in &task.depends_on {
                if !task_id_set.contains(dep_id.as_str()) {
                    errors.push(format!(
                        "task {} depends on non-existent task {}",
                        task.id, dep_id
                    ));
                }
            }
        }
    }

    // Check current_phase references a valid phase
    if let Some(ref current_phase) = state.current_phase {
        if !state.phases.iter().any(|p| &p.id == current_phase) {
            errors.push(format!(
                "current_phase '{}' not found in phases",
                current_phase
            ));
        }
    }

    // Check current_task references a valid task
    if let Some(ref current_task) = state.current_task {
        if !task_id_set.contains(current_task.as_str()) {
            errors.push(format!(
                "current_task '{}' not found in any phase",
                current_task
            ));
        }
    }

    if errors.is_empty() {
        println!("tasks.json is valid");
        println!("  Version: {}", state.version);
        println!("  Project: {}", state.project.name);
        println!("  Phases: {}", state.phases.len());
        println!("  Tasks: {}", all_task_ids.len());
        Ok(())
    } else {
        println!("Validation errors:");
        for error in &errors {
            println!("  - {}", error);
        }
        Err(CliError::ValidationFailed(format!(
            "{} validation error(s)",
            errors.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        GlobalContext, Phase, PhaseStatus, Project, Task, TaskContext, TaskType,
    };

    fn create_test_state() -> TasksState {
        TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "test-project".to_string(),
                description: "A test project".to_string(),
                created_at: None,
            },
            global_context: Some(GlobalContext {
                plan_summary: "Test plan".to_string(),
            }),
            phases: vec![
                Phase {
                    id: "phase-1".to_string(),
                    name: "Phase 1".to_string(),
                    status: PhaseStatus::Completed,
                    tasks: vec![Task {
                        id: "task-1".to_string(),
                        name: "Task 1".to_string(),
                        task_type: TaskType::Implement,
                        status: TaskStatus::Completed,
                        depends_on: vec![],
                        context: TaskContext {
                            files_to_read: vec![],
                            code_style_excerpt: None,
                            prior_review_issues: vec![],
                        },
                        instructions: "Do task 1".to_string(),
                        attempts: vec![],
                    }],
                },
                Phase {
                    id: "phase-2".to_string(),
                    name: "Phase 2".to_string(),
                    status: PhaseStatus::InProgress,
                    tasks: vec![
                        Task {
                            id: "task-2".to_string(),
                            name: "Task 2".to_string(),
                            task_type: TaskType::Implement,
                            status: TaskStatus::Completed,
                            depends_on: vec!["task-1".to_string()],
                            context: TaskContext {
                                files_to_read: vec![],
                                code_style_excerpt: None,
                                prior_review_issues: vec![],
                            },
                            instructions: "Do task 2".to_string(),
                            attempts: vec![],
                        },
                        Task {
                            id: "task-3".to_string(),
                            name: "Task 3".to_string(),
                            task_type: TaskType::Implement,
                            status: TaskStatus::Pending,
                            depends_on: vec!["task-2".to_string()],
                            context: TaskContext {
                                files_to_read: vec![],
                                code_style_excerpt: None,
                                prior_review_issues: vec![],
                            },
                            instructions: "Do task 3".to_string(),
                            attempts: vec![],
                        },
                        Task {
                            id: "task-4".to_string(),
                            name: "Task 4".to_string(),
                            task_type: TaskType::Implement,
                            status: TaskStatus::Deferred,
                            depends_on: vec![],
                            context: TaskContext {
                                files_to_read: vec![],
                                code_style_excerpt: None,
                                prior_review_issues: vec![],
                            },
                            instructions: "Do task 4".to_string(),
                            attempts: vec![],
                        },
                    ],
                },
            ],
            current_phase: Some("phase-2".to_string()),
            current_task: Some("task-3".to_string()),
            agent_history: vec![],
            interrupted_at: None,
        }
    }

    #[test]
    fn test_cli_parse_default() {
        let cli = Cli::parse_from(["cm"]);
        assert!(!cli.resume);
        assert!(!cli.step);
        assert!(!cli.status);
        assert!(!cli.validate);
        assert!(!cli.verbose);
        assert!(cli.config.is_none());
        assert_eq!(cli.state, PathBuf::from(".cm/tasks.json"));
    }

    #[test]
    fn test_cli_parse_continue() {
        let cli = Cli::parse_from(["cm", "--continue"]);
        assert!(cli.resume);
    }

    #[test]
    fn test_cli_parse_step() {
        let cli = Cli::parse_from(["cm", "--step"]);
        assert!(cli.step);
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(["cm", "--status"]);
        assert!(cli.status);
    }

    #[test]
    fn test_cli_parse_validate() {
        let cli = Cli::parse_from(["cm", "--validate"]);
        assert!(cli.validate);
    }

    #[test]
    fn test_cli_parse_verbose() {
        let cli = Cli::parse_from(["cm", "-v"]);
        assert!(cli.verbose);

        let cli = Cli::parse_from(["cm", "--verbose"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parse_config() {
        let cli = Cli::parse_from(["cm", "--config", "/path/to/config"]);
        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config")));
    }

    #[test]
    fn test_cli_parse_state() {
        let cli = Cli::parse_from(["cm", "--state", "/path/to/tasks.json"]);
        assert_eq!(cli.state, PathBuf::from("/path/to/tasks.json"));
    }

    #[test]
    fn test_print_status() {
        let state = create_test_state();
        // Just ensure it doesn't panic
        print_status(&state);
    }

    #[test]
    fn test_cli_error_display() {
        let err = CliError::NoTasks;
        assert!(err.to_string().contains("no tasks"));

        let err = CliError::ValidationFailed("test error".to_string());
        assert!(err.to_string().contains("validation failed"));
    }
}
