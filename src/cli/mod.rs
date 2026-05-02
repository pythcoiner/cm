//! CLI argument parsing and command execution.
//!
//! This module provides the command-line interface for the cm binary,
//! including argument parsing with clap and dispatching to the appropriate
//! execution modes (run, continue, step, status, validate).

mod init;
mod token;
mod update_pricing;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, info, warn};
use thiserror::Error;

use crate::config::{ConfigError, ConfigFile};
use crate::generate::{generate_roadmap_md, generate_tasks_md, write_md_file, GenerateError};
use crate::log::{init_log_file, TeeWriter};
use crate::manager::{Manager, ManagerConfig, ManagerError, RecoveryAction, RecoveryManager, ShutdownHandler};
use crate::state::{load_roadmap, load_state, save_roadmap, save_state, validate_all, PhaseStatus, StateError, TaskStatus, TasksState};

/// Subcommands for the cm CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install Claude Code wizards and agent templates into the current repo
    Init {
        /// Overwrite existing command files
        #[arg(long)]
        force: bool,
    },
    /// Show Claude Code token usage and estimated API cost
    Token {
        /// Aggregate across all projects instead of just the current one
        #[arg(long)]
        global: bool,

        /// Daily breakdown by date
        #[arg(long, group = "bucket")]
        daily: bool,

        /// Weekly breakdown by ISO week
        #[arg(long, group = "bucket")]
        weekly: bool,

        /// Monthly breakdown
        #[arg(long, group = "bucket")]
        monthly: bool,

        /// Per-project total spend, sorted from most to least costly
        #[arg(long, conflicts_with = "bucket")]
        breakdown: bool,
    },
    /// Refresh `[pricing.*]` in `.cm/config.toml` from LiteLLM's public dataset
    UpdatePricing {
        /// Print what would change without modifying the file
        #[arg(long)]
        dry_run: bool,
    },
}

/// Model choice for Claude agent spawning.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ModelChoice {
    /// Claude Sonnet 4.5 (alias for latest sonnet model)
    Sonnet,
    /// Claude Opus 4.5 (alias for latest opus model)
    Opus,
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


    /// Markdown generation error.
    #[error("generate error: {0}")]
    GenerateError(#[from] GenerateError),

    /// Phase log error.
    #[error("phase log error: {0}")]
    PhaseLogError(#[from] crate::log::PhaseLogError),

    /// Token subcommand error.
    #[error("token error: {0}")]
    TokenError(#[from] token::TokenError),

    /// Update-pricing subcommand error.
    #[error("update-pricing error: {0}")]
    UpdatePricingError(#[from] update_pricing::UpdatePricingError),
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

    /// Claude model to use (sonnet or opus).
    #[arg(long, value_enum)]
    pub model: Option<ModelChoice>,

    /// Maximum cycles per task before deferring.
    #[arg(long, value_name = "N")]
    pub max_cycles: Option<u32>,

    /// Working directory for build verification.
    #[arg(long, value_name = "DIR")]
    pub working_dir: Option<PathBuf>,


    /// Perform comprehensive sanity check on JSON files.
    #[arg(long)]
    pub sanity_check: bool,

    /// Prune phase log entries older than 24 hours.
    #[arg(long)]
    pub prune: bool,

    /// Reset a phase to pending status and clear execution state.
    #[arg(long, value_name = "PHASE_ID")]
    pub reset: Option<String>,
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

    // Initialize log file early (before any logging)
    // Determine .cm directory from state path
    let cm_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new(".cm"));
    if let Err(e) = init_log_file(cm_dir) {
        eprintln!("Warning: failed to initialize log file: {e}");
    }

    // Initialize logger with TeeWriter to mirror output to cm.log
    let filter = if cli.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter))
        .target(env_logger::Target::Pipe(Box::new(TeeWriter::stderr())))
        .init();

    // Register signal handlers for graceful shutdown
    let shutdown_handler = ShutdownHandler::new();
    shutdown_handler.register_signal_handlers();
    let shutdown_flag = shutdown_handler.shutdown_flag();

    info!("Claude Code Manager starting...");
    debug!("CLI arguments: {cli:?}");

    if let Some(config) = &cli.config {
        debug!("Using config file: {config:?}");
    }

    // Handle subcommand first (before flag dispatch)
    if let Some(ref command) = cli.command {
        return match command {
            Command::Init { force } => init::execute_init(*force),
            Command::Token {
                global,
                daily,
                weekly,
                monthly,
                breakdown,
            } => {
                let bucket = match (daily, weekly, monthly) {
                    (true, _, _) => Some(token::BucketMode::Daily),
                    (_, true, _) => Some(token::BucketMode::Weekly),
                    (_, _, true) => Some(token::BucketMode::Monthly),
                    _ => None,
                };
                token::execute_token(token::TokenOpts {
                    global: *global,
                    bucket,
                    breakdown: *breakdown,
                })
                .map_err(CliError::from)
            }
            Command::UpdatePricing { dry_run } => update_pricing::execute_update_pricing(
                update_pricing::UpdatePricingOpts { dry_run: *dry_run },
            )
            .map_err(CliError::from),
        };
    }

    // Dispatch based on flags
    let result = if cli.reset.is_some() {
        execute_reset(&cli)
    } else if cli.prune {
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
        Err(e) => warn!("Claude Code Manager finished with error: {e}"),
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
        debug!("Loading config from explicit path: {path:?}");
        Some(ConfigFile::load(path)?)
    } else {
        debug!("Trying to load default config from .cm/config.toml");
        ConfigFile::load_default()?
    };

    if let Some(ref cf) = config_file {
        debug!("Config file loaded: {cf:?}");
    } else {
        debug!("No config file found, using defaults");
    }

    // 2. Start with defaults from ManagerConfig
    let mut config = ManagerConfig::new(cli.state.clone());

    // 3. Apply config file values (if present)
    if let Some(cf) = config_file {
        if let Some(model) = cf.model {
            // Validate model value from config file
            if model != "sonnet" && model != "opus" {
                return Err(CliError::ValidationFailed(format!(
                    "invalid model '{model}' in config file, must be 'sonnet' or 'opus'"
                )));
            }
            config = config.model(model);
        }
        if let Some(max_cycles) = cf.max_cycles {
            config = config.max_cycles(max_cycles);
        }
        if let Some(working_dir) = cf.working_dir {
            config = config.working_dir(working_dir);
        }
        if let Some(build_commands) = cf.build_commands {
            config = config.build_commands(build_commands);
        }
    }

    // 4. Apply CLI overrides (highest precedence)
    if let Some(model_choice) = cli.model {
        let model_str = match model_choice {
            ModelChoice::Sonnet => "sonnet",
            ModelChoice::Opus => "opus",
        };
        config = config.model(model_str.to_string());
    }
    if let Some(max_cycles) = cli.max_cycles {
        config = config.max_cycles(max_cycles);
    }
    if let Some(ref working_dir) = cli.working_dir {
        config = config.working_dir(working_dir.clone());
    }

    // 5. Apply defaults for paths that weren't set
    // If working_dir wasn't explicitly set, use current directory
    if cli.working_dir.is_none() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        // Only set if config file didn't specify it
        if config.working_dir == ManagerConfig::new(cli.state.clone()).working_dir {
            config = config.working_dir(cwd);
        }
    }

    debug!("Final ManagerConfig: {config:?}");
    Ok(config)
}

/// Execute the default run mode.
///
/// Runs all tasks until completion or error.
fn execute_run(cli: &Cli, shutdown_flag: Arc<AtomicBool>) -> Result<(), CliError> {
    info!("Run mode: executing all tasks from {:?}", cli.state);
    let config = build_manager_config(cli)?;
    let mut manager = Manager::new(config, shutdown_flag)?;
    let run_result = manager.run_interactive();
    let has_issues = manager.run_post_run_review();
    if let Err(e) = run_result {
        eprintln!("Run error: {e}");
        std::process::exit(if has_issues { 1 } else { 2 });
    }
    if has_issues {
        std::process::exit(1);
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

    info!("Recovery action: {action}");

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
            let run_result = manager.run();
            let has_issues = manager.run_post_run_review();
            if let Err(e) = run_result {
                eprintln!("Continue error: {e}");
                std::process::exit(if has_issues { 1 } else { 2 });
            }
            if has_issues {
                std::process::exit(1);
            }
        }
        RecoveryAction::Rollback(checkpoint_id) => {
            // Restore from checkpoint and resume
            info!("Rolling back to checkpoint {checkpoint_id}");

            let restored_state = recovery
                .restore(&checkpoint_id)
                .map_err(|e| CliError::StateError(StateError::ParseError(e.to_string())))?;

            // Save the restored state
            save_state(&restored_state, &cli.state)?;

            // Now run with the restored state
            let config = build_manager_config(cli)?;

            let mut manager = Manager::new(config, shutdown_flag)?;
            let run_result = manager.run();
            let has_issues = manager.run_post_run_review();
            if let Err(e) = run_result {
                eprintln!("Continue error: {e}");
                std::process::exit(if has_issues { 1 } else { 2 });
            }
            if has_issues {
                std::process::exit(1);
            }
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
            let run_result = manager.run();
            let has_issues = manager.run_post_run_review();
            if let Err(e) = run_result {
                eprintln!("Continue error: {e}");
                std::process::exit(if has_issues { 1 } else { 2 });
            }
            if has_issues {
                std::process::exit(1);
            }
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
    let step_result = manager.step();
    let has_issues = manager.run_post_run_review();
    if let Err(e) = step_result {
        eprintln!("Step error: {e}");
        std::process::exit(if has_issues { 1 } else { 2 });
    }
    if has_issues {
        std::process::exit(1);
    }
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
        "Phase progress: {completed_phases}/{total_phases} phases complete"
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
    println!("  Completed:   {completed}/{total}");
    println!("  In Progress: {in_progress}/{total}");
    println!("  Pending:     {pending}/{total}");
    println!("  Deferred:    {deferred}/{total}");
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
            crate::state::PhaseStatus::Deferred => "[!]",
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
            println!("Interrupted at: {interrupted_at}");
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
                "current_phase '{current_phase}' not found in phases"
            ));
        }
    }

    // Check current_task references a valid task
    if let Some(ref current_task) = state.current_task {
        if !task_id_set.contains(current_task.as_str()) {
            errors.push(format!(
                "current_task '{current_task}' not found in any phase"
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
            println!("  - {error}");
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
/// Regenerates markdown files (ROADMAP.md) from JSON state files.
fn execute_regenerate(cli: &Cli) -> Result<(), CliError> {
    info!("Regenerate mode: regenerating markdown files from JSON");

    let mut files_regenerated = 0;

    // Load state for TASKS.md generation
    let state = load_state(&cli.state)?;

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
        println!("Skipped ROADMAP.md: {roadmap_json_path:?} not found");
    }

    // Regenerate TASKS.md from tasks.json
    let tasks_md_path = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("TASKS.md");

    let tasks_content = generate_tasks_md(&state);
    write_md_file(&tasks_content, &tasks_md_path)?;
    let completed_tasks: usize = state.phases.iter().map(|p| p.tasks.iter().filter(|t| t.status == crate::state::TaskStatus::Completed).count()).sum();
    let total_tasks: usize = state.phases.iter().map(|p| p.tasks.len()).sum();
    println!(
        "Regenerated: {:?} ({} phases, {}/{} tasks)",
        tasks_md_path,
        state.phases.len(),
        completed_tasks,
        total_tasks
    );
    files_regenerated += 1;

    println!("\n{files_regenerated} file(s) regenerated.");
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
            println!("  - {error}");
        }
        println!();
    }

    // Print warnings
    if !result.warnings.is_empty() {
        println!("Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("  ! {warning}");
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
/// Prunes all phase log entries older than 24 hours.
fn execute_prune(cli: &Cli) -> Result<(), CliError> {
    let cm_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."));

    let logs_dir = cm_dir.join("logs");

    // Prune all phase logs
    if logs_dir.exists() {
        info!("Pruning phase logs in: {logs_dir:?}");
        let stats = crate::log::PhaseLogger::prune_all(&logs_dir)?;
        println!(
            "Pruned phase logs: {} entries removed, {} entries kept",
            stats.removed_count, stats.kept_count
        );
    } else {
        println!("No phase logs directory found at {logs_dir:?}");
    }

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

/// Execute the reset mode.
///
/// Resets a phase to pending status and clears all execution state.
/// Also resets linked roadmap items and removes the phase log file.
fn execute_reset(cli: &Cli) -> Result<(), CliError> {
    info!("Reset mode: resetting phase {:?}", cli.reset);

    let phase_id = cli.reset.as_ref().unwrap();
    let mut state = load_state(&cli.state)?;

    // Find and reset the phase
    let phase = state.get_phase_mut(phase_id)?;

    // Collect roadmap item IDs linked to tasks in this phase (before resetting)
    let roadmap_item_ids: Vec<String> = phase
        .tasks
        .iter()
        .filter_map(|t| t.roadmap_item_id.clone())
        .collect();

    // Reset phase fields
    phase.status = PhaseStatus::Pending;
    phase.review_cycles_completed = 0;
    phase.baseline_commit = None;
    phase.implem_completed_at = None;

    let task_count = phase.tasks.len();

    // Reset all tasks in the phase
    for task in &mut phase.tasks {
        task.status = TaskStatus::Pending;
        task.attempts.clear();
        task.implem_completed_at = None;
        task.baseline_commit = None;
        task.review_cycles_completed = 0;
    }

    // Save modified state
    save_state(&state, &cli.state)?;

    println!("Phase '{phase_id}' reset to pending state");
    println!("  - Phase status: pending");
    println!("  - {task_count} task(s) reset");

    // Reset roadmap.json items linked to this phase
    let cm_dir = cli
        .state
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let roadmap_json_path = cm_dir.join("roadmap.json");

    if roadmap_json_path.exists() && !roadmap_item_ids.is_empty() {
        let mut roadmap = load_roadmap(&roadmap_json_path)?;
        let mut items_reset = 0;

        // Find and reset linked roadmap items
        for roadmap_phase in &mut roadmap.phases {
            for item in &mut roadmap_phase.items {
                if roadmap_item_ids.contains(&item.id) {
                    if item.completed {
                        item.completed = false;
                        items_reset += 1;
                    }
                    // Also reset all sub-items
                    for sub_item in &mut item.sub_items {
                        if sub_item.completed {
                            sub_item.completed = false;
                        }
                    }
                }
            }
        }

        if items_reset > 0 {
            save_roadmap(&roadmap, &roadmap_json_path)?;

            // Regenerate ROADMAP.md
            let roadmap_md_path = cm_dir.join("ROADMAP.md");
            let roadmap_content = generate_roadmap_md(&roadmap);
            write_md_file(&roadmap_content, &roadmap_md_path)?;

            println!("  - {items_reset} roadmap item(s) reset");
        }
    }

    // Remove phase log file
    let logs_dir = cm_dir.join("logs");
    let phase_log_path = logs_dir.join(format!("{phase_id}.log"));

    if phase_log_path.exists() {
        fs::remove_file(&phase_log_path)?;
        println!("  - Removed phase log: {}", phase_log_path.display());
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
        // Note: plan_file paths are placeholders - these tests don't read plan files
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
                    plan: String::new(),
                    status: PhaseStatus::Completed,
                    review_cycles_completed: 0,
                    baseline_commit: None,
                    implem_completed_at: None,
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
                        plan_file: ".cm/plans/plan-1.md".to_string(),
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
                    plan: String::new(),
                    status: PhaseStatus::InProgress,
                    review_cycles_completed: 0,
                    baseline_commit: None,
                    implem_completed_at: None,
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
                            plan_file: ".cm/plans/plan-2.md".to_string(),
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
                            plan_file: ".cm/plans/plan-2.md".to_string(),
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
                            plan_file: ".cm/plans/plan-2.md".to_string(),
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
    fn test_cli_parse_model_sonnet() {
        let cli = Cli::parse_from(["cm", "--model", "sonnet"]);
        assert_eq!(cli.model, Some(ModelChoice::Sonnet));
    }

    #[test]
    fn test_cli_parse_model_opus() {
        let cli = Cli::parse_from(["cm", "--model", "opus"]);
        assert_eq!(cli.model, Some(ModelChoice::Opus));
    }

    #[test]
    fn test_cli_parse_model_invalid() {
        // This should fail because clap's ValueEnum rejects invalid values
        let result = Cli::try_parse_from(["cm", "--model", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_max_cycles() {
        let cli = Cli::parse_from(["cm", "--max-cycles", "10"]);
        assert_eq!(cli.max_cycles, Some(10));
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
            "--model", "opus",
            "--max-cycles", "10",
            "--working-dir", "/home/user/project",
            "--verbose",
        ]);

        assert_eq!(cli.config, Some(PathBuf::from("/path/to/config.toml")));
        assert_eq!(cli.state, PathBuf::from("/path/to/tasks.json"));
        assert_eq!(cli.model, Some(ModelChoice::Opus));
        assert_eq!(cli.max_cycles, Some(10));
        assert_eq!(cli.working_dir, Some(PathBuf::from("/home/user/project")));
        assert!(cli.verbose);
    }

    #[test]
    fn test_build_manager_config_defaults() {
        let cli = Cli::parse_from(["cm", "--state", "/tmp/tasks.json"]);
        let config = build_manager_config(&cli).unwrap();

        assert_eq!(config.state_path, PathBuf::from("/tmp/tasks.json"));
        assert_eq!(config.model, "sonnet");
        assert_eq!(config.max_cycles, 5);
    }

    #[test]
    fn test_build_manager_config_cli_overrides() {
        let cli = Cli::parse_from([
            "cm",
            "--state", "/tmp/tasks.json",
            "--model", "opus",
            "--max-cycles", "10",
            "--working-dir", "/custom/dir",
        ]);
        let config = build_manager_config(&cli).unwrap();

        assert_eq!(config.model, "opus");
        assert_eq!(config.max_cycles, 10);
        assert_eq!(config.working_dir, PathBuf::from("/custom/dir"));
    }

    #[test]
    fn test_cli_config_error_display() {
        let err = CliError::ConfigError(ConfigError::NotFound(PathBuf::from("/nonexistent")));
        assert!(err.to_string().contains("config error"));
    }

    #[test]
    fn test_build_manager_config_invalid_model_in_file() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        // Write config with invalid model value
        let content = r#"model = "invalid-model""#;
        std::fs::write(&config_path, content).unwrap();

        let cli = Cli::parse_from([
            "cm",
            "--config", config_path.to_str().unwrap(),
            "--state", "/tmp/tasks.json",
        ]);

        let result = build_manager_config(&cli);
        assert!(result.is_err());

        if let Err(CliError::ValidationFailed(msg)) = result {
            assert!(msg.contains("invalid model 'invalid-model'"));
            assert!(msg.contains("must be 'sonnet' or 'opus'"));
        } else {
            panic!("Expected ValidationFailed error");
        }
    }

    #[test]
    fn test_build_manager_config_valid_model_sonnet_in_file() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        // Write config with valid sonnet model
        let content = r#"model = "sonnet""#;
        std::fs::write(&config_path, content).unwrap();

        let cli = Cli::parse_from([
            "cm",
            "--config", config_path.to_str().unwrap(),
            "--state", "/tmp/tasks.json",
        ]);

        let result = build_manager_config(&cli);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.model, "sonnet");
    }

    #[test]
    fn test_build_manager_config_valid_model_opus_in_file() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        // Write config with valid opus model
        let content = r#"model = "opus""#;
        std::fs::write(&config_path, content).unwrap();

        let cli = Cli::parse_from([
            "cm",
            "--config", config_path.to_str().unwrap(),
            "--state", "/tmp/tasks.json",
        ]);

        let result = build_manager_config(&cli);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.model, "opus");
    }
}
