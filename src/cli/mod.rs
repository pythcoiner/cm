//! CLI argument parsing and command execution.
//!
//! This module provides the command-line interface for the cm binary,
//! including argument parsing with clap and dispatching to the appropriate
//! execution modes (run, continue, step, status, validate).

mod init;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use log::{debug, info, warn};
use thiserror::Error;

use crate::config::{ConfigError, ConfigFile};
use crate::generate::{generate_log_md, generate_roadmap_md, write_md_file, GenerateError};
use crate::manager::{Manager, ManagerConfig, ManagerError, RecoveryAction, RecoveryManager, ShutdownHandler};
use crate::state::{load_roadmap, load_state, save_state, validate_all, StateError, TaskStatus, TasksState};
use crate::tui::{self, ManagerEvent, TuiCommand};

/// Subcommands for the cm CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize .claude/commands directory with cm, feat, fix, and end commands
    Init {
        /// Overwrite existing command files
        #[arg(long)]
        force: bool,
    },
}

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

    /// An error occurred while loading configuration.
    #[error("config error: {0}")]
    ConfigError(#[from] ConfigError),

    /// Validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// No tasks to execute.
    #[error("no tasks to execute")]
    NoTasks,

    /// A background thread panicked.
    #[error("background thread panicked")]
    ThreadError,

    /// TUI error.
    #[error("tui error: {0}")]
    TuiError(String),

    /// Markdown generation error.
    #[error("generate error: {0}")]
    GenerateError(#[from] GenerateError),

    /// File log error.
    #[error("file log error: {0}")]
    FileLogError(#[from] crate::log::FileLogError),

    /// Phase log error.
    #[error("phase log error: {0}")]
    PhaseLogError(#[from] crate::log::PhaseLogError),
}

/// Claude Code Manager - Automated agent coordination for software development.
///
/// Orchestrates Claude agents to execute multi-phase development tasks
/// defined in a tasks.json file.
#[derive(Debug, Parser)]
#[command(name = "cm")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

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

    /// Dry run - print what would be done without executing.
    #[arg(long)]
    pub dry_run: bool,

    /// Regenerate markdown files from JSON state.
    #[arg(long)]
    pub regenerate: bool,

    /// Path to config file.
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to tasks.json state file.
    #[arg(long, value_name = "FILE", default_value = ".cm/tasks.json")]
    pub state: PathBuf,

    /// Claude model to use.
    #[arg(long, value_name = "MODEL")]
    pub model: Option<String>,

    /// Agent timeout in seconds.
    #[arg(long, value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Maximum cycles per task before deferring.
    #[arg(long, value_name = "N")]
    pub max_cycles: Option<u32>,

    /// Path to LOG.md file.
    #[arg(long, value_name = "FILE")]
    pub log_path: Option<PathBuf>,

    /// Working directory for build verification.
    #[arg(long, value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    /// Run in daemon mode (no TUI, stdin prompts).
    #[arg(long)]
    pub daemon: bool,

    /// Perform comprehensive sanity check on JSON files.
    #[arg(long)]
    pub sanity_check: bool,

    /// Prune cm.log entries older than 24 hours.
    #[arg(long)]
    pub prune: bool,
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

    // Register signal handlers for graceful shutdown
    let shutdown_handler = ShutdownHandler::new();
    shutdown_handler.register_signal_handlers();
    let shutdown_flag = shutdown_handler.shutdown_flag();

    info!("Claude Code Manager starting...");
    debug!("CLI arguments: {:?}", cli);

    if let Some(config) = &cli.config {
        debug!("Using config file: {:?}", config);
    }

    // Handle subcommand first (before flag dispatch)
    if let Some(ref command) = cli.command {
        return match command {
            Command::Init { force } => init::execute_init(*force),
        };
    }

    // Dispatch based on flags
    let result = if cli.prune {
        execute_prune(&cli)
    } else if cli.sanity_check {
        execute_sanity_check(&cli)
    } else if cli.regenerate {
        execute_regenerate(&cli)
    } else if cli.dry_run {
        execute_dry_run(&cli)
    } else if cli.status {
        execute_status(&cli)
    } else if cli.validate {
        execute_validate(&cli)
    } else if cli.step {
        execute_step(&cli, shutdown_flag.clone())
    } else if cli.resume {
        execute_continue(&cli, shutdown_flag.clone())
    } else {
        execute_run(&cli, shutdown_flag.clone())
    };

    match &result {
        Ok(()) => info!("Claude Code Manager finished."),
        Err(e) => warn!("Claude Code Manager finished with error: {}", e),
    }

    result
}

/// Build a ManagerConfig by merging config file and CLI arguments.
///
/// Precedence (highest to lowest):
/// 1. CLI arguments
/// 2. Config file values
/// 3. Default values
///
/// # Arguments
///
/// * `cli` - Parsed CLI arguments
///
/// # Errors
///
/// Returns an error if the config file exists but cannot be read or parsed.
fn build_manager_config(cli: &Cli) -> Result<ManagerConfig, CliError> {
    // 1. Load config file (explicit path or default)
    let config_file = if let Some(ref path) = cli.config {
        debug!("Loading config from explicit path: {:?}", path);
        Some(ConfigFile::load(path)?)
    } else {
        debug!("Trying to load default config from .cm/config.toml");
        ConfigFile::load_default()?
    };

    if let Some(ref cf) = config_file {
        debug!("Config file loaded: {:?}", cf);
    } else {
        debug!("No config file found, using defaults");
    }

    // 2. Start with defaults from ManagerConfig
    let mut config = ManagerConfig::new(cli.state.clone());

    // 3. Apply config file values (if present)
    if let Some(cf) = config_file {
        if let Some(model) = cf.model {
            config = config.model(model);
        }
        if let Some(timeout_secs) = cf.timeout_secs {
            config = config.timeout(Duration::from_secs(timeout_secs));
        }
        if let Some(max_cycles) = cf.max_cycles {
            config = config.max_cycles(max_cycles);
        }
        if let Some(log_path) = cf.log_path {
            config = config.log_path(log_path);
        }
        if let Some(working_dir) = cf.working_dir {
            config = config.working_dir(working_dir);
        }
    }

    // 4. Apply CLI overrides (highest precedence)
    if let Some(ref model) = cli.model {
        config = config.model(model.clone());
    }
    if let Some(timeout_secs) = cli.timeout {
        config = config.timeout(Duration::from_secs(timeout_secs));
    }
    if let Some(max_cycles) = cli.max_cycles {
        config = config.max_cycles(max_cycles);
    }
    if let Some(ref log_path) = cli.log_path {
        config = config.log_path(log_path.clone());
    }
    if let Some(ref working_dir) = cli.working_dir {
        config = config.working_dir(working_dir.clone());
    }

    // 5. Apply defaults for paths that weren't set
    // If log_path wasn't explicitly set, derive from state path
    if cli.log_path.is_none() {
        let default_log = cli
            .state
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("LOG.md");
        // Only set if config file didn't specify it
        if config.log_path == ManagerConfig::new(cli.state.clone()).log_path {
            config = config.log_path(default_log);
        }
    }

    // If working_dir wasn't explicitly set, use current directory
    if cli.working_dir.is_none() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Only set if config file didn't specify it
        if config.working_dir == ManagerConfig::new(cli.state.clone()).working_dir {
            config = config.working_dir(cwd);
        }
    }

    // Wire verbose flag for file logger level
    config.verbose = cli.verbose;

    debug!("Final ManagerConfig: {:?}", config);
    Ok(config)
}

/// Execute the default run mode.
///
/// Runs all tasks until completion or error.
fn execute_run(cli: &Cli, shutdown_flag: Arc<AtomicBool>) -> Result<(), CliError> {
    info!("Run mode: executing all tasks from {:?}", cli.state);

    let config = build_manager_config(cli)?;

    if cli.daemon {
        // run without TUI (daemon mode with interactive prompts)
        let mut manager = Manager::new(config, shutdown_flag)?;
        manager.run_interactive()?;
        Ok(())
    } else {
        execute_run_with_tui(config, shutdown_flag)
    }
}

/// Execute run mode with the terminal UI.
///
/// Spawns the manager in a background thread and runs the TUI on the main thread.
fn execute_run_with_tui(config: ManagerConfig, shutdown_flag: Arc<AtomicBool>) -> Result<(), CliError> {
    use std::sync::mpsc;
    use std::thread;

    info!("Starting TUI mode");

    // Load state for initial TUI display
    let initial_state = load_state(&config.state_path)?;

    // Create channels for bidirectional communication
    let (event_tx, event_rx) = mpsc::channel::<ManagerEvent>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<TuiCommand>();

    // Spawn manager in background thread
    let manager_handle = thread::spawn(move || -> Result<(), ManagerError> {
        let mut manager = Manager::new(config, shutdown_flag)?;
        manager.run_with_channels(event_tx, cmd_rx)
    });

    // Run TUI on main thread (it owns the terminal)
    tui::run_tui_with_channels(initial_state, event_rx, cmd_tx)
        .map_err(|e| CliError::TuiError(e.to_string()))?;

    // Wait for manager thread
    match manager_handle.join() {
        Ok(result) => result?,
        Err(_) => return Err(CliError::ThreadError),
    }

    Ok(())
}

/// Execute the continue mode.
///
/// Resumes from interrupted state, using recovery manager to determine
/// the appropriate action.
fn execute_continue(cli: &Cli, shutdown_flag: Arc<AtomicBool>) -> Result<(), CliError> {
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
            let config = build_manager_config(cli)?;

            let mut manager = Manager::new(config, shutdown_flag)?;
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
            let config = build_manager_config(cli)?;

            let mut manager = Manager::new(config, shutdown_flag)?;
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
            let config = build_manager_config(cli)?;

            let mut manager = Manager::new(config, shutdown_flag)?;
            manager.run()?;
        }
    }

    Ok(())
}

/// Execute the step mode.
///
/// Executes one task only, then pauses.
fn execute_step(cli: &Cli, shutdown_flag: Arc<AtomicBool>) -> Result<(), CliError> {
    info!("Step mode: executing one task from {:?}", cli.state);

    let config = build_manager_config(cli)?;

    let mut manager = Manager::new(config, shutdown_flag)?;
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

/// Get a status icon for a task status.
fn status_icon(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "[ ]",
        TaskStatus::InProgress => "[~]",
        TaskStatus::Completed => "[x]",
        TaskStatus::Deferred => "[!]",
    }
}

/// Execute the regenerate mode.
///
/// Regenerates markdown files (LOG.md, ROADMAP.md) from JSON state files.
fn execute_regenerate(cli: &Cli) -> Result<(), CliError> {
    info!("Regenerate mode: regenerating markdown files from JSON");

    let mut files_regenerated = 0;

    // Regenerate LOG.md from tasks.json log_records
    let state = load_state(&cli.state)?;
    let log_path = cli
        .log_path
        .clone()
        .unwrap_or_else(|| {
            cli.state
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("LOG.md")
        });

    let log_content = generate_log_md(&state.log_records);
    write_md_file(&log_content, &log_path)?;
    println!("Regenerated: {:?} ({} records)", log_path, state.log_records.len());
    files_regenerated += 1;

    // Regenerate ROADMAP.md from roadmap.json if it exists
    let roadmap_json_path = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("roadmap.json");

    if roadmap_json_path.exists() {
        let roadmap = load_roadmap(&roadmap_json_path)?;
        let roadmap_md_path = cli
            .state
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("ROADMAP.md");

        let roadmap_content = generate_roadmap_md(&roadmap);
        write_md_file(&roadmap_content, &roadmap_md_path)?;
        println!(
            "Regenerated: {:?} ({} phases, {}/{} items)",
            roadmap_md_path,
            roadmap.phases.len(),
            roadmap.completed_items(),
            roadmap.total_items()
        );
        files_regenerated += 1;
    } else {
        println!("Skipped ROADMAP.md: {:?} not found", roadmap_json_path);
    }

    println!("\n{} file(s) regenerated.", files_regenerated);
    Ok(())
}

/// Execute the sanity-check mode.
///
/// Performs comprehensive validation of JSON files (tasks.json and roadmap.json).
/// Checks JSON syntax, schema compliance, and cross-references.
fn execute_sanity_check(cli: &Cli) -> Result<(), CliError> {
    info!("Sanity check mode: validating .cm directory");

    // Determine the .cm directory from the state path
    let cm_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new(".cm"));

    let result = validate_all(cm_dir);

    // Print errors
    if !result.errors.is_empty() {
        println!("Errors ({}):", result.errors.len());
        for error in &result.errors {
            println!("  - {}", error);
        }
        println!();
    }

    // Print warnings
    if !result.warnings.is_empty() {
        println!("Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("  ! {}", warning);
        }
        println!();
    }

    // Summary
    if result.is_valid() {
        println!("Sanity check passed - all JSON files are valid");
        Ok(())
    } else {
        println!("Sanity check failed - {} error(s) found", result.errors.len());
        Err(CliError::ValidationFailed(format!(
            "sanity check failed with {} error(s)",
            result.errors.len()
        )))
    }
}

/// Execute the prune mode.
///
/// Prunes cm.log and all phase log entries older than 24 hours.
fn execute_prune(cli: &Cli) -> Result<(), CliError> {
    let cm_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."));

    let file_log_path = cm_dir.join("cm.log");
    let logs_dir = cm_dir.join("logs");

    let mut total_removed = 0;
    let mut total_kept = 0;

    // Prune main cm.log
    if file_log_path.exists() {
        info!("Pruning main log file: {:?}", file_log_path);
        let logger = crate::log::FileLogger::new(file_log_path.clone())?;
        let stats = logger.prune()?;
        println!(
            "Pruned cm.log: {} entries removed, {} entries kept",
            stats.removed_count, stats.kept_count
        );
        total_removed += stats.removed_count;
        total_kept += stats.kept_count;
    } else {
        println!("No main log file found at {:?}", file_log_path);
    }

    // Prune all phase logs
    if logs_dir.exists() {
        info!("Pruning phase logs in: {:?}", logs_dir);
        let stats = crate::log::PhaseLogger::prune_all(&logs_dir)?;
        println!(
            "Pruned phase logs: {} entries removed, {} entries kept",
            stats.removed_count, stats.kept_count
        );
        total_removed += stats.removed_count;
        total_kept += stats.kept_count;
    } else {
        println!("No phase logs directory found at {:?}", logs_dir);
    }

    // Print total summary
    println!(
        "\nTotal: {} entries removed, {} entries kept",
        total_removed, total_kept
    );

    Ok(())
}

/// Execute the dry-run mode.
///
/// Prints what would be done without executing any tasks.
fn execute_dry_run(cli: &Cli) -> Result<(), CliError> {
    let state = load_state(&cli.state)?;

    println!("Dry run mode - no changes will be made\n");
    println!("Project: {}", state.project.name);
    println!();

    // Print what tasks would run
    for phase in &state.phases {
        println!("Phase: {} ({:?})", phase.name, phase.status);
        for task in &phase.tasks {
            let runnable = if state.is_task_blocked(&task.id) {
                "blocked"
            } else if task.status == TaskStatus::Completed {
                "done"
            } else if task.status == TaskStatus::Deferred {
                "deferred"
            } else {
                "would run"
            };
            println!(
                "  {} {} - {} ({})",
                status_icon(&task.status),
                task.id,
                task.name,
                runnable
            );
        }
    }

    Ok(())
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
                        roadmap_item_id: None,
                        implem_completed_at: None,
                        baseline_commit: None,
                        review_cycles_completed: 0,
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
                            roadmap_item_id: None,
                            implem_completed_at: None,
                            baseline_commit: None,
                            review_cycles_completed: 0,
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
                            roadmap_item_id: None,
                            implem_completed_at: None,
                            baseline_commit: None,
                            review_cycles_completed: 0,
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
                            roadmap_item_id: None,
                            implem_completed_at: None,
                            baseline_commit: None,
                            review_cycles_completed: 0,
                        },
                    ],
                },
            ],
            current_phase: Some("phase-2".to_string()),
            current_task: Some("task-3".to_string()),
            agent_history: vec![],
            log_records: vec![],
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
        assert!(!cli.daemon);
        assert!(cli.config.is_none());
        assert_eq!(cli.state, PathBuf::from(".cm/tasks.json"));
    }

    #[test]
    fn test_cli_parse_daemon() {
        let cli = Cli::parse_from(["cm", "--daemon"]);
        assert!(cli.daemon);
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
    fn test_cli_parse_sanity_check() {
        let cli = Cli::parse_from(["cm", "--sanity-check"]);
        assert!(cli.sanity_check);
    }

    #[test]
    fn test_cli_parse_prune() {
        let cli = Cli::parse_from(["cm", "--prune"]);
        assert!(cli.prune);
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

    #[test]
    fn test_cli_parse_model() {
        let cli = Cli::parse_from(["cm", "--model", "claude-opus-4-5-20251101"]);
        assert_eq!(cli.model, Some("claude-opus-4-5-20251101".to_string()));
    }

    #[test]
    fn test_cli_parse_timeout() {
        let cli = Cli::parse_from(["cm", "--timeout", "600"]);
        assert_eq!(cli.timeout, Some(600));
    }

    #[test]
    fn test_cli_parse_max_cycles() {
        let cli = Cli::parse_from(["cm", "--max-cycles", "10"]);
        assert_eq!(cli.max_cycles, Some(10));
    }

    #[test]
    fn test_cli_parse_log_path() {
        let cli = Cli::parse_from(["cm", "--log-path", "/tmp/LOG.md"]);
        assert_eq!(cli.log_path, Some(PathBuf::from("/tmp/LOG.md")));
    }

    #[test]
    fn test_cli_parse_working_dir() {
        let cli = Cli::parse_from(["cm", "--working-dir", "/home/user/project"]);
        assert_eq!(cli.working_dir, Some(PathBuf::from("/home/user/project")));
    }

    #[test]
    fn test_cli_parse_all_options() {
        let cli = Cli::parse_from([
            "cm",
            "--config", "/path/to/config.toml",
            "--state", "/path/to/tasks.json",
            "--model", "claude-opus-4-5-20251101",
            "--timeout", "600",
            "--max-cycles", "10",
            "--log-path", "/tmp/LOG.md",
            "--working-dir", "/home/user/project",
            "--verbose",
        ]);

        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
        assert_eq!(cli.state, PathBuf::from("/path/to/tasks.json"));
        assert_eq!(cli.model, Some("claude-opus-4-5-20251101".to_string()));
        assert_eq!(cli.timeout, Some(600));
        assert_eq!(cli.max_cycles, Some(10));
        assert_eq!(cli.log_path, Some(PathBuf::from("/tmp/LOG.md")));
        assert_eq!(cli.working_dir, Some(PathBuf::from("/home/user/project")));
        assert!(cli.verbose);
    }

    #[test]
    fn test_build_manager_config_defaults() {
        let cli = Cli::parse_from(["cm", "--state", "/tmp/tasks.json"]);
        let config = build_manager_config(&cli).unwrap();

        assert_eq!(config.state_path, PathBuf::from("/tmp/tasks.json"));
        assert_eq!(config.model, "claude-sonnet-4-5-20250929");
        assert_eq!(config.timeout, Duration::from_secs(300));
        assert_eq!(config.max_cycles, 5);
    }

    #[test]
    fn test_build_manager_config_cli_overrides() {
        let cli = Cli::parse_from([
            "cm",
            "--state", "/tmp/tasks.json",
            "--model", "claude-opus-4-5-20251101",
            "--timeout", "600",
            "--max-cycles", "10",
            "--log-path", "/custom/LOG.md",
            "--working-dir", "/custom/dir",
        ]);
        let config = build_manager_config(&cli).unwrap();

        assert_eq!(config.model, "claude-opus-4-5-20251101");
        assert_eq!(config.timeout, Duration::from_secs(600));
        assert_eq!(config.max_cycles, 10);
        assert_eq!(config.log_path, PathBuf::from("/custom/LOG.md"));
        assert_eq!(config.working_dir, PathBuf::from("/custom/dir"));
    }

    #[test]
    fn test_cli_config_error_display() {
        let err = CliError::ConfigError(ConfigError::NotFound(PathBuf::from("/nonexistent")));
        assert!(err.to_string().contains("config error"));
    }
}
