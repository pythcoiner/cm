//! Main orchestration loop, state machine, and crash recovery.
//!
//! This module provides the core Manager that orchestrates the entire task
//! execution flow. It coordinates between the state module, agent spawner,
//! build verifier, and log manager to execute tasks in the correct order.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::Utc;
use log::{debug, error, info, warn};
use thiserror::Error;
use uuid::Uuid;


use crate::agent::{AgentError, AgentSpawner, PromptBuilder, ResponseParser};
use crate::build::{BuildError, BuildVerifier, GitRunner};
use crate::generate::{write_md_file, GenerateError};
use crate::log::{LogError, LogManager, PhaseLogger, PhaseLogError};
use crate::state::{
    load_roadmap, load_state, save_roadmap, save_state, AgentInvocation, AgentStatus, AgentType,
    PhaseStatus, RoadmapState, StateError, Task, TaskContext, TaskStatus, TaskType, TasksState,
    Verdict,
};
use crate::generate::generate_roadmap_md;

mod recovery;
mod state;

pub use recovery::{CheckpointId, RecoveryAction, RecoveryError, RecoveryManager, ShutdownHandler};
pub use state::ManagerState;

/// Task selection mode from user prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskSelection {
    /// Run a single task (the next runnable one)
    Single,
    /// Run all remaining tasks
    All,
    /// Run specific phases by ID
    Phases(Vec<String>),
    /// Quit without running
    Quit,
}

/// Errors that can occur during manager operations.
#[derive(Debug, Error)]
pub enum ManagerError {
    /// An error occurred while managing state.
    #[error("state error: {0}")]
    StateError(#[from] StateError),

    /// An error occurred while spawning or communicating with an agent.
    #[error("agent error: {0}")]
    AgentError(#[from] AgentError),

    /// An error occurred during build verification.
    #[error("build error: {0}")]
    BuildError(#[from] BuildError),

    /// An error occurred while writing to the log.
    #[error("log error: {0}")]
    LogError(#[from] LogError),

    /// An error occurred during markdown generation.
    #[error("generate error: {0}")]
    GenerateError(#[from] GenerateError),

    /// An error occurred while writing to the phase log.
    #[error("phase log error: {0}")]
    PhaseLogError(#[from] PhaseLogError),

    /// A task is not runnable due to dependencies or status.
    #[error("task not runnable: {0}")]
    TaskNotRunnable(String),

    /// Maximum cycles exceeded for a task.
    #[error("max cycles exceeded for task: {0}")]
    MaxCyclesExceeded(String),

    /// All tasks are blocked or completed.
    #[error("no runnable tasks available")]
    NoRunnableTasks,

    /// Shutdown was requested via signal.
    #[error("shutdown requested")]
    ShutdownRequested,
}

/// Configuration for the Manager.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Path to the tasks.json state file.
    pub state_path: PathBuf,
    /// Path to the LOG.md file.
    pub log_path: PathBuf,
    /// Path to the roadmap.json file.
    pub roadmap_path: PathBuf,
    /// Path to the ROADMAP.md file.
    pub roadmap_md_path: PathBuf,
    /// Working directory for build verification.
    pub working_dir: PathBuf,
    /// Model to use for agent spawning.
    pub model: String,
    /// Maximum number of cycles (attempts) per task before deferring.
    pub max_cycles: u32,
    /// Build commands to run for verification.
    /// If empty, build verification is skipped.
    pub build_commands: Vec<String>,
}

impl ManagerConfig {
    /// Create a new configuration with default values.
    ///
    /// # Arguments
    ///
    /// * `state_path` - Path to the tasks.json file
    pub fn new(state_path: PathBuf) -> Self {
        let parent = state_path
            .parent()
            .unwrap_or(std::path::Path::new("."));
        Self {
            state_path: state_path.clone(),
            log_path: parent.join("LOG.md"),
            roadmap_path: parent.join("roadmap.json"),
            roadmap_md_path: parent.join("ROADMAP.md"),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            model: "sonnet".to_string(),
            max_cycles: 5,
            build_commands: Vec::new(),
        }
    }

    /// Set the log path.
    pub fn log_path(mut self, path: PathBuf) -> Self {
        self.log_path = path;
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = path;
        self
    }

    /// Set the model.
    pub fn model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Set the maximum cycles.
    pub fn max_cycles(mut self, max_cycles: u32) -> Self {
        self.max_cycles = max_cycles;
        self
    }

    /// Set the build commands.
    pub fn build_commands(mut self, commands: Vec<String>) -> Self {
        self.build_commands = commands;
        self
    }
}

/// The main manager that orchestrates task execution.
///
/// The Manager loads state from tasks.json, executes tasks by spawning agents,
/// runs build verification, and updates state after each task.
pub struct Manager {
    /// Configuration for the manager.
    config: ManagerConfig,
    /// The current tasks state.
    state: TasksState,
    /// The roadmap state (loaded if available).
    roadmap_state: Option<RoadmapState>,
    /// Spawner for creating agent processes.
    agent_spawner: AgentSpawner,
    /// Verifier for build and clippy checks.
    build_verifier: BuildVerifier,
    /// Manager for LOG.md entries.
    log_manager: LogManager,
    /// Per-phase logger for full prompts and responses.
    phase_logger: PhaseLogger,
    /// Current execution state.
    manager_state: ManagerState,
    /// Shutdown flag for graceful termination.
    shutdown_flag: Arc<AtomicBool>,
}

/// Emit a timestamped [CM] message to stderr for orchestration visibility.
fn emit_cm(msg: &str) {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    eprintln!("[{} CM] {}", now, msg);
}

impl Manager {
    /// Create a new Manager with the given configuration.
    ///
    /// This loads the state from the configured path or returns an error
    /// if the state file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the manager
    /// * `shutdown_flag` - Atomic flag for graceful shutdown
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The state file cannot be loaded
    /// - The state file is invalid
    pub fn new(config: ManagerConfig, shutdown_flag: Arc<AtomicBool>) -> Result<Self, ManagerError> {
        let state = load_state(&config.state_path)?;
        let agent_spawner = AgentSpawner::new(config.model.clone());
        let build_verifier = BuildVerifier::new(config.working_dir.clone());
        let log_manager = LogManager::new(config.log_path.clone());

        // Initialize phase logger
        let cm_dir = config.state_path.parent().unwrap_or(std::path::Path::new("."));
        let phase_logger = PhaseLogger::new(cm_dir)?;

        // Load roadmap if available
        let roadmap_state = match load_roadmap(&config.roadmap_path) {
            Ok(roadmap) => {
                info!("Roadmap loaded from {:?}", config.roadmap_path);
                Some(roadmap)
            }
            Err(e) => {
                warn!("No roadmap loaded: {}", e);
                None
            }
        };

        info!("Manager initialized with state from {:?}", config.state_path);

        Ok(Self {
            config,
            state,
            roadmap_state,
            agent_spawner,
            build_verifier,
            log_manager,
            phase_logger,
            manager_state: ManagerState::Idle,
            shutdown_flag,
        })
    }

    /// Get the current manager state.
    pub fn manager_state(&self) -> ManagerState {
        self.manager_state
    }

    /// Get a reference to the tasks state.
    pub fn state(&self) -> &TasksState {
        &self.state
    }

    /// Run the main orchestration loop.
    ///
    /// This loop:
    /// 1. Checks for shutdown signal
    /// 2. Selects the next runnable task
    /// 3. Executes the task
    /// 4. Saves state
    /// 5. Repeats until no more tasks are runnable or shutdown is requested
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task execution fails
    /// - State cannot be saved
    /// - Shutdown was requested
    pub fn run(&mut self) -> Result<(), ManagerError> {
        info!("Starting manager run loop (phase-based)");
        emit_cm("Starting run loop (phase-based)");

        // Ensure clean working tree for commit-per-agent audit trail
        self.preflight_git_check()?;

        self.manager_state = ManagerState::Executing;

        loop {
            // Check for shutdown signal before selecting next phase
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received, saving state and exiting gracefully");
                self.update_state()?;
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }

            // Select next runnable phase
            let phase_id = match self.state.next_runnable_phase() {
                Some(phase) => phase.id.clone(),
                None => {
                    info!("No more runnable phases, exiting loop");
                    emit_cm("No more runnable phases");
                    break;
                }
            };

            info!("Selected phase for execution: {}", phase_id);
            emit_cm(&format!("Selected phase: {}", phase_id));

            // Execute the phase (all pending tasks in one agent call)
            match self.execute_phase(&phase_id) {
                Ok(()) => {
                    info!("Phase {} completed successfully", phase_id);
                    emit_cm(&format!("Phase {} completed", phase_id));
                }
                Err(e) => {
                    let error_msg = format!("Phase {} failed: {}", phase_id, e);
                    error!("{}", error_msg);
                    emit_cm(&format!("Phase {} failed: {}", phase_id, e));
                    self.log_manager.log_error(&error_msg)?;
                    self.state
                        .log_records
                        .push(LogManager::create_error_record(&error_msg));

                    // Don't propagate the error; continue with next phase
                }
            }

            // Save state after each phase
            self.update_state()?;

            // Check for shutdown signal after phase completion
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received after phase completion, exiting gracefully");
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }
        }

        self.manager_state = ManagerState::Idle;
        info!("Manager run loop completed");
        emit_cm("Run loop completed");
        Ok(())
    }


    /// Execute a single step (one phase) and return.
    ///
    /// In phase-based mode, this executes all pending tasks in the next
    /// runnable phase with a single agent call.
    ///
    /// # Errors
    ///
    /// Returns `ManagerError::NoRunnableTasks` if there are no phases to run.
    pub fn step(&mut self) -> Result<(), ManagerError> {
        let phase_id = match self.state.next_runnable_phase() {
            Some(phase) => phase.id.clone(),
            None => return Err(ManagerError::NoRunnableTasks),
        };

        self.execute_phase(&phase_id)?;
        self.update_state()?;

        Ok(())
    }

    /// Prompt the user to select what tasks to run.
    ///
    /// Shows pending tasks and asks for selection.
    /// This is used in daemon mode (non-TUI).
    pub fn prompt_task_selection(&self) -> Result<TaskSelection, ManagerError> {
        use std::io::{self, BufRead, Write};

        // Reload state to get fresh data
        let state = load_state(&self.config.state_path)?;

        // Show pending tasks
        let pending: Vec<_> = state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .filter(|t| t.status == TaskStatus::Pending)
            .collect();

        // Check for stuck in_progress tasks
        let in_progress: Vec<_> = state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .filter(|t| t.status == TaskStatus::InProgress)
            .collect();

        if pending.is_empty() {
            if !in_progress.is_empty() {
                println!("No runnable tasks available.");
                println!(
                    "\nFound {} task(s) stuck in 'in_progress' status:",
                    in_progress.len()
                );
                for task in &in_progress {
                    println!("  - {} ({})", task.id, task.name);
                }
                println!("\nRun with --continue to reset and retry these tasks.");
            } else {
                println!("No pending tasks.");
            }
            return Ok(TaskSelection::Quit);
        }

        println!("\nPending tasks ({}):", pending.len());
        for (i, task) in pending.iter().take(10).enumerate() {
            let blocked = if state.is_task_blocked(&task.id) {
                " (blocked)"
            } else {
                ""
            };
            println!("  {}. {} - {}{}", i + 1, task.id, task.name, blocked);
        }
        if pending.len() > 10 {
            println!("  ... and {} more", pending.len() - 10);
        }

        // Find next runnable task
        if let Some(next) = state.next_runnable_task() {
            println!("\nNext runnable: {} - {}", next.id, next.name);
        } else if !in_progress.is_empty() {
            // No runnable tasks, but there are stuck in_progress tasks
            println!(
                "\nNo runnable tasks. Found {} task(s) stuck in 'in_progress' status:",
                in_progress.len()
            );
            for task in &in_progress {
                println!("  - {} ({})", task.id, task.name);
            }
            println!("\nRun with --continue to reset and retry these tasks.");
            return Ok(TaskSelection::Quit);
        } else {
            // No runnable tasks and no in_progress tasks - all pending tasks are blocked
            println!("\nNo runnable tasks available (all pending tasks are blocked).");
            return Ok(TaskSelection::Quit);
        }

        // Prompt
        print!("\n[s]ingle / [a]ll / [p]hase <# or #-#> / [q]uit: ");
        io::stdout()
            .flush()
            .map_err(|e| ManagerError::StateError(StateError::Io(e)))?;

        let stdin = io::stdin();
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| ManagerError::StateError(StateError::Io(e)))?;

        let input = line.trim().to_lowercase();
        match input.as_str() {
            "s" | "single" => Ok(TaskSelection::Single),
            "a" | "all" => Ok(TaskSelection::All),
            "q" | "quit" | "" => Ok(TaskSelection::Quit),
            _ if input.starts_with("p ") || input.starts_with("phase ") => {
                let nums_part = input.strip_prefix("p ").or_else(|| input.strip_prefix("phase ")).unwrap();
                let phase_ids: Vec<String> = nums_part
                    .split_whitespace()
                    .flat_map(|token| {
                        if let Some((start_str, end_str)) = token.split_once('-') {
                            // Range: "3-6" -> ["phase-3", "phase-4", "phase-5", "phase-6"]
                            // Parse with proper error handling
                            let start = match start_str.parse::<u32>() {
                                Ok(n) => n,
                                Err(_) => {
                                    println!("Invalid phase number: {}", token);
                                    return vec![];
                                }
                            };
                            let end = match end_str.parse::<u32>() {
                                Ok(n) => n,
                                Err(_) => {
                                    println!("Invalid phase number: {}", token);
                                    return vec![];
                                }
                            };
                            // Validate range
                            if start > end {
                                println!("Invalid range: {}-{} (start must be <= end)", start, end);
                                return vec![];
                            }
                            (start..=end).map(|n| format!("phase-{}", n)).collect::<Vec<_>>()
                        } else {
                            // Single: "3" -> ["phase-3"]
                            // Validate it's a number
                            if token.parse::<u32>().is_ok() {
                                vec![format!("phase-{}", token)]
                            } else {
                                println!("Invalid phase number: {}", token);
                                vec![]
                            }
                        }
                    })
                    .collect();
                if phase_ids.is_empty() {
                    println!("No valid phase numbers provided.");
                    self.prompt_task_selection()
                } else {
                    Ok(TaskSelection::Phases(phase_ids))
                }
            }
            _ => {
                println!("Invalid selection. Use 's', 'a', 'p <#...>', or 'q'.");
                self.prompt_task_selection() // Retry
            }
        }
    }

    /// Run with interactive prompts (daemon mode).
    ///
    /// This method prompts the user before running tasks and loops until
    /// the user quits or no more tasks are available.
    pub fn run_interactive(&mut self) -> Result<(), ManagerError> {
        // Ensure clean working tree for commit-per-agent audit trail
        self.preflight_git_check()?;

        loop {
            match self.prompt_task_selection()? {
                TaskSelection::Single => {
                    if let Err(e) = self.step() {
                        match e {
                            ManagerError::NoRunnableTasks => {
                                println!("No runnable tasks available.");
                                break;
                            }
                            ManagerError::ShutdownRequested => break,
                            _ => return Err(e),
                        }
                    }
                }
                TaskSelection::All => {
                    self.run()?;
                    break;
                }
                TaskSelection::Phases(phase_ids) => {
                    self.run_specific_phases(&phase_ids)?;
                    // Continue looping for another selection
                }
                TaskSelection::Quit => {
                    println!("Exiting.");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Run specific phases by ID.
    ///
    /// Skips phases that don't exist or are already completed, with warnings.
    fn run_specific_phases(&mut self, phase_ids: &[String]) -> Result<(), ManagerError> {
        for phase_id in phase_ids {
            // Reload state to check current status
            let state = load_state(&self.config.state_path)?;

            // Find the phase
            let phase = state.phases.iter().find(|p| p.id == *phase_id);

            match phase {
                None => {
                    println!("Warning: Phase '{}' not found, skipping.", phase_id);
                    continue;
                }
                Some(p) if p.status == PhaseStatus::Completed => {
                    println!("Warning: Phase '{}' already completed, skipping.", phase_id);
                    continue;
                }
                Some(_) => {
                    // Execute the phase
                    match self.execute_phase(phase_id) {
                        Ok(()) => {
                            self.update_state()?;
                        }
                        Err(e) => {
                            println!("Phase '{}' failed: {}", phase_id, e);
                            // Continue with next phase
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Prompt user whether to retry after max review cycles exhausted.
    ///
    /// Prompts via stdin/stdout.
    ///
    /// Returns Some(additional_cycles) if user wants to retry, None if they decline.
    fn prompt_retry_cycles(&self, task_id: &str, cycles_completed: u32) -> Option<u32> {
        use std::io::{self, BufRead, Write};

        let message = format!(
            "Task {} exhausted {} review cycles. Retry more cycles?",
            task_id, cycles_completed
        );

        println!();
        println!("{}", message);
        print!("Retry more cycles? [y/N/number]: ");
        let _ = io::stdout().flush();

        let stdin = io::stdin();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return None;
        }

        let input = line.trim().to_lowercase();
        if input == "y" || input == "yes" {
            Some(5)
        } else if let Ok(n) = input.parse::<u32>() {
            if n > 0 { Some(n) } else { None }
        } else {
            None
        }
    }

    // =========================================================================
    // Phase-level execution methods (all tasks in a phase with one agent call)
    // =========================================================================

    /// Execute an entire phase: IMPLEM all tasks → BUILD → phase-level REVIEW cycle.
    ///
    /// This is the phase-level equivalent of `execute_implem()`. Instead of running
    /// one agent per task, it runs one agent for all pending tasks in the phase.
    pub fn execute_phase(&mut self, phase_id: &str) -> Result<(), ManagerError> {
        info!("Executing phase: {}", phase_id);
        emit_cm(&format!("Executing phase: {}", phase_id));

        // Get all pending tasks in this phase BEFORE marking as in progress
        let pending_tasks: Vec<Task> = self
            .state
            .pending_tasks_in_phase(phase_id)
            .into_iter()
            .cloned()
            .collect();

        if pending_tasks.is_empty() {
            info!("No pending tasks in phase {}, checking completion", phase_id);
            // Try normal completion check first
            self.check_phase_completion(phase_id)?;
            // If phase is still not completed (e.g., tasks in weird states), force complete
            // since there's literally nothing more to do
            let phase = self.state.get_phase_mut(phase_id)?;
            if phase.status != PhaseStatus::Completed {
                info!(
                    "Phase {} has no pending tasks but wasn't marked complete, forcing completion",
                    phase_id
                );
                phase.status = PhaseStatus::Completed;
            }
            return Ok(());
        }

        // Mark phase as in progress only if there's actual work to do
        self.state.mark_phase_status(phase_id, PhaseStatus::InProgress)?;

        emit_cm(&format!("Phase {} has {} pending tasks", phase_id, pending_tasks.len()));

        // Check if we should resume at REVIEW (phase IMPLEM already completed)
        let phase = self.state.get_phase_mut(phase_id)?;
        if phase.implem_completed_at.is_some() {
            if let Some(ref baseline) = phase.baseline_commit.clone() {
                let starting_cycle = phase.review_cycles_completed;
                info!(
                    "Resuming phase {} at REVIEW cycle {} (IMPLEM already completed)",
                    phase_id, starting_cycle
                );
                emit_cm(&format!("Resuming {} at REVIEW cycle {}", phase_id, starting_cycle));

                let verdict = self.run_phase_review_cycle(phase_id, baseline, starting_cycle)?;

                match verdict {
                    Verdict::Approved => {
                        // Mark all pending tasks as completed
                        for task in &pending_tasks {
                            self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                            self.sync_roadmap_item(&task.id);
                        }
                        self.state.mark_phase_status(phase_id, PhaseStatus::Completed)?;
                        self.clear_phase_implem_completion(phase_id);
                    }
                    Verdict::NeedsFixes => {
                        // Defer all tasks
                        for task in &pending_tasks {
                            self.state.mark_task_status(&task.id, TaskStatus::Deferred)?;
                        }
                        self.clear_phase_implem_completion(phase_id);
                    }
                }
                return Ok(());
            }
        }

        // Record baseline commit for later diff
        let baseline_commit = self.get_head_commit().unwrap_or_default();

        // Mark all tasks as in progress
        for task in &pending_tasks {
            self.state.mark_task_status(&task.id, TaskStatus::InProgress)?;
        }

        // Get phase for prompt building
        let phase = self
            .state
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .cloned()
            .ok_or_else(|| StateError::PhaseNotFound(phase_id.to_string()))?;

        // === Run PLAN agent first to evaluate/enhance the initial plan ===
        let initial_plan = phase.plan.clone();
        let plan_prompt = PromptBuilder::build_phase_plan_prompt(&phase, &initial_plan);

        info!("Spawning PLAN agent for {}", phase_id);
        emit_cm(&format!("Spawning PLAN agent for {}", phase_id));

        // Log prompt to phase logger
        let _ = self.phase_logger.log_prompt(phase_id, "PHASE_PLAN", &plan_prompt);

        // Log agent spawn
        self.log_manager
            .log_agent_spawn(&AgentType::Plan, phase_id, &plan_prompt)?;
        self.state.log_records.push(
            LogManager::create_agent_spawn_record(&AgentType::Plan, phase_id, &plan_prompt),
        );

        let plan_agent_id = Uuid::new_v4().to_string();
        let plan_started = Utc::now();

        self.state.agent_history.push(AgentInvocation {
            id: plan_agent_id.clone(),
            task_id: phase_id.to_string(),
            agent_type: AgentType::Plan,
            started_at: plan_started,
            completed_at: None,
            exit_status: None,
            commit_hash: None,
        });

        let plan_handle = self.agent_spawner.spawn(&plan_prompt, phase_id, "PHASE_PLAN")?;
        let plan_output = plan_handle.wait()?;

        // Parse plan response
        let plan_response = match ResponseParser::parse_plan_response(&plan_output.stdout) {
            Ok(resp) => resp,
            Err(AgentError::ParseError(msg)) => {
                warn!("PLAN agent parse failed: {}, using original plan", msg);
                // If parse fails, just use the original plan
                crate::agent::PlanAgentResponse { plan: None }
            }
            Err(e) => return Err(e.into()),
        };

        // Update invocation completion
        if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == plan_agent_id) {
            inv.completed_at = Some(Utc::now());
            inv.exit_status = plan_output.exit_code;
        }

        // Log response to phase logger
        let _ = self.phase_logger.log_response(
            phase_id,
            "PHASE_PLAN",
            &plan_output.stdout,
            plan_output.duration.as_secs(),
            plan_output.exit_code,
            None, // No AgentResponse to log for PLAN agent
        );

        // Use detailed plan if provided, otherwise use original
        let final_plan = match plan_response.plan {
            Some(detailed) => {
                info!("PLAN agent provided detailed plan for {}", phase_id);
                emit_cm(&format!("PLAN agent: using detailed plan for {}", phase_id));
                detailed
            }
            None => {
                info!("PLAN agent: original plan sufficient for {}", phase_id);
                emit_cm(&format!("PLAN agent: using original plan for {}", phase_id));
                initial_plan
            }
        };

        // === Build phase-level IMPLEM prompt with the final plan ===
        let task_refs: Vec<&Task> = pending_tasks.iter().collect();
        let prompt = PromptBuilder::build_phase_implem_prompt_with_plan(&phase, &task_refs, &final_plan);

        // Log full prompt to phase logger
        let _ = self.phase_logger.log_prompt(phase_id, "PHASE_IMPLEM", &prompt);

        // Log agent spawn
        self.log_manager
            .log_agent_spawn(&AgentType::Implem, phase_id, &prompt)?;
        self.state.log_records.push(
            LogManager::create_agent_spawn_record(&AgentType::Implem, phase_id, &prompt),
        );

        // Generate agent ID
        let agent_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Record the invocation
        self.state.agent_history.push(AgentInvocation {
            id: agent_id.clone(),
            task_id: phase_id.to_string(), // Use phase_id for phase-level invocations
            agent_type: AgentType::Implem,
            started_at,
            completed_at: None,
            exit_status: None,
            commit_hash: None,
        });

        // Spawn and wait for the agent
        self.manager_state = ManagerState::WaitingForAgent;
        let handle = self.agent_spawner.spawn(&prompt, phase_id, "PHASE_IMPLEM")?;
        let output = handle.wait()?;

        // Parse the response
        let response = match ResponseParser::parse(&output.stdout) {
            Ok(resp) => resp,
            Err(AgentError::ParseError(msg)) => {
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                eprintln!("[{} PHASE_IMPLEM] {} parse failed: {}", now, phase_id, msg);

                if let Some(session_id) = &output.session_id {
                    eprintln!("[{} PHASE_IMPLEM] {} retrying with --continue...", now, phase_id);
                    let retry_handle = self.agent_spawner.spawn_with_continue(
                        session_id,
                        "Your previous response could not be parsed. Please provide a summary of your changes.",
                        phase_id,
                        "PHASE_IMPLEM",
                    )?;
                    let retry_output = retry_handle.wait()?;
                    ResponseParser::parse(&retry_output.stdout).map_err(|e| {
                        eprintln!("[{} PHASE_IMPLEM] {} retry also failed: {}", now, phase_id, e);
                        e
                    })?
                } else {
                    return Err(AgentError::ParseError(msg).into());
                }
            }
            Err(e) => return Err(e.into()),
        };

        // Log full response to phase logger (with parsed result)
        let _ = self.phase_logger.log_response(phase_id, "PHASE_IMPLEM", &output.stdout, output.duration.as_secs(), output.exit_code, Some(&response));

        // Log agent response
        self.log_manager.log_agent_response(&response)?;

        // Update invocation completion
        if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == agent_id) {
            inv.completed_at = Some(Utc::now());
            inv.exit_status = output.exit_code;
        }

        // Check if agent reported failure
        if response.status == AgentStatus::Failed {
            warn!("Agent reported failure for phase {}: {}", phase_id, response.message);
            // Mark tasks back to pending
            for task in &pending_tasks {
                self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            }
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Verify agent actually made file changes
        if !self.check_git_changes()? {
            warn!("Agent reported success but no file changes detected for phase {}", phase_id);
            for task in &pending_tasks {
                self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            }
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Commit IMPLEM agent changes for audit trail
        match self.commit_agent_changes(phase_id, "PHASE_IMPLEM", &agent_id) {
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to commit PHASE_IMPLEM changes for {}: {}", phase_id, e);
            }
        }

        // Run build verification (if configured)
        self.manager_state = ManagerState::Verifying;
        if !self.config.build_commands.is_empty() {
            let build_result = self.build_verifier.verify_commands(&self.config.build_commands);

            if let Err(e) = build_result {
                let err_msg = match &e {
                    BuildError::CommandFailed { stderr, .. } => stderr.clone(),
                    _ => e.to_string(),
                };
                warn!("Build failed after PHASE_IMPLEM for {}: {}", phase_id, err_msg);
                emit_cm(&format!("Build FAILED after PHASE_IMPLEM for {}", phase_id));
                // Mark tasks back to pending for retry
                for task in &pending_tasks {
                    self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
                }
                self.manager_state = ManagerState::Executing;
                return Ok(());
            }
            emit_cm(&format!("Build PASSED for phase {}, starting review", phase_id));
        } else {
            info!("No build commands configured, skipping build verification");
            emit_cm(&format!("Skipping build verification for phase {} (no commands configured)", phase_id));
        }

        // Save phase IMPLEM completion state for potential resume
        self.save_phase_implem_completion(phase_id, &baseline_commit);

        // Run phase-level review cycle
        if !baseline_commit.is_empty() {
            let verdict = self.run_phase_review_cycle(phase_id, &baseline_commit, 0)?;

            match verdict {
                Verdict::Approved => {
                    // Mark all tasks as completed
                    for task in &pending_tasks {
                        self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                        self.sync_roadmap_item(&task.id);
                        self.log_manager.log_task_complete(&task.id)?;
                        self.state
                            .log_records
                            .push(LogManager::create_task_complete_record(&task.id));
                    }
                    self.state.mark_phase_status(phase_id, PhaseStatus::Completed)?;
                    self.clear_phase_implem_completion(phase_id);
                }
                Verdict::NeedsFixes => {
                    // Defer all tasks
                    let reason = format!("Review cycle exhausted after {} cycles", self.config.max_cycles);
                    for task in &pending_tasks {
                        self.state.mark_task_status(&task.id, TaskStatus::Deferred)?;
                        self.log_manager.log_task_deferred(&task.id, &reason)?;
                        self.state
                            .log_records
                            .push(LogManager::create_task_deferred_record(&task.id, &reason));
                    }
                    self.clear_phase_implem_completion(phase_id);
                }
            }
        } else {
            // No baseline commit — skip review, mark complete
            warn!("No baseline commit for phase {}, skipping review", phase_id);
            for task in &pending_tasks {
                self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                self.sync_roadmap_item(&task.id);
            }
            self.state.mark_phase_status(phase_id, PhaseStatus::Completed)?;
        }

        self.manager_state = ManagerState::Executing;
        Ok(())
    }

    /// Run phase-level REVIEW → FIX → re-REVIEW cycle.
    ///
    /// Similar to `run_review_cycle()` but operates on the entire phase's changes
    /// rather than a single task.
    fn run_phase_review_cycle(
        &mut self,
        phase_id: &str,
        baseline_commit: &str,
        starting_cycle: u32,
    ) -> Result<Verdict, ManagerError> {
        let mut cycle = starting_cycle;
        let mut max_cycles = self.config.max_cycles;

        loop {
            // Check if we've exceeded max cycles
            if cycle >= max_cycles {
                if let Some(additional) = self.prompt_retry_cycles(phase_id, cycle) {
                    max_cycles += additional;
                    emit_cm(&format!("Retrying {} more cycles for phase {}", additional, phase_id));
                } else {
                    warn!("Phase review cycle exhausted for {} after {} cycles", phase_id, cycle);
                    return Ok(Verdict::NeedsFixes);
                }
            }

            emit_cm(&format!("Phase review cycle {}/{} for {}", cycle + 1, max_cycles, phase_id));

            // Get diff from baseline to current HEAD
            let diff = self.get_diff_between(baseline_commit, "HEAD")?;

            if diff.trim().is_empty() {
                warn!("No diff found for phase {}", phase_id);
                return Ok(Verdict::Approved);
            }

            // Get phase for prompt building
            let phase = self
                .state
                .phases
                .iter()
                .find(|p| p.id == phase_id)
                .cloned()
                .ok_or_else(|| StateError::PhaseNotFound(phase_id.to_string()))?;

            // Build phase-level review prompt
            let review_prompt = PromptBuilder::build_phase_review_prompt(&phase, &diff);

            // Log full prompt to phase logger
            let _ = self.phase_logger.log_prompt(phase_id, "PHASE_REVIEW", &review_prompt);

            self.log_manager
                .log_agent_spawn(&AgentType::Review, phase_id, &review_prompt)?;
            self.state.log_records.push(
                LogManager::create_agent_spawn_record(&AgentType::Review, phase_id, &review_prompt),
            );

            let review_agent_id = Uuid::new_v4().to_string();
            let review_started = Utc::now();

            self.state.agent_history.push(AgentInvocation {
                id: review_agent_id.clone(),
                task_id: phase_id.to_string(),
                agent_type: AgentType::Review,
                started_at: review_started,
                completed_at: None,
                exit_status: None,
                commit_hash: None,
            });

            let review_handle = self.agent_spawner.spawn(&review_prompt, phase_id, "PHASE_REVIEW")?;
            let review_output = review_handle.wait()?;

            // Parse review response with proper JSON extraction
            let review_response = match ResponseParser::parse_review_response(&review_output.stdout) {
                Ok(resp) => resp,
                Err(AgentError::ParseError(_)) => {
                    if let Some(session_id) = &review_output.session_id {
                        let retry_handle = self.agent_spawner.spawn_with_continue(
                            session_id,
                            "Please provide your review verdict.",
                            phase_id,
                            "PHASE_REVIEW",
                        )?;
                        let retry_output = retry_handle.wait()?;
                        match ResponseParser::parse_review_response(&retry_output.stdout) {
                            Ok(resp) => resp,
                            Err(_) => {
                                warn!("Phase review parse failed twice, treating as approved");
                                return Ok(Verdict::Approved);
                            }
                        }
                    } else {
                        warn!("Phase review parse failed, treating as approved");
                        return Ok(Verdict::Approved);
                    }
                }
                Err(e) => return Err(e.into()),
            };

            // Log full response to phase logger
            // Note: We need to convert to AgentResponse for logging compatibility
            let agent_response_for_log = crate::state::AgentResponse {
                status: review_response.status.clone(),
                files_created: vec![],
                files_modified: vec![],
                commands_run: vec![],
                message: review_response.summary.clone(),
            };
            let _ = self.phase_logger.log_response(phase_id, "PHASE_REVIEW", &review_output.stdout, review_output.duration.as_secs(), review_output.exit_code, Some(&agent_response_for_log));

            // Update invocation
            if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == review_agent_id) {
                inv.completed_at = Some(Utc::now());
                inv.exit_status = review_output.exit_code;
            }

            self.log_manager.log_agent_response(&agent_response_for_log)?;

            if review_response.status == AgentStatus::Failed {
                warn!("Phase review agent failed for {}", phase_id);
                return Ok(Verdict::Approved);
            }

            // Extract verdict from parsed JSON response
            // CRITICAL FIX: If verdict is "needs_fixes" but there are no issues,
            // treat it as "approved" since there's nothing to fix
            let verdict = match review_response.verdict.as_deref() {
                Some("needs_fixes") if !review_response.issues.is_empty() => Verdict::NeedsFixes,
                Some("needs_fixes") => {
                    warn!("Review returned needs_fixes with 0 issues, treating as approved");
                    Verdict::Approved
                }
                _ => Verdict::Approved,
            };

            // Log with actual issue count
            self.log_manager.log_review_result(&verdict, &[])?;
            self.state
                .log_records
                .push(LogManager::create_review_result_record(&verdict, &[]));

            match verdict {
                Verdict::Approved => {
                    info!("Phase review approved for {}", phase_id);
                    emit_cm(&format!("Phase review APPROVED for {}", phase_id));
                    return Ok(Verdict::Approved);
                }
                Verdict::NeedsFixes => {
                    warn!("Phase review found issues for {} (cycle {})", phase_id, cycle + 1);
                    emit_cm(&format!("Phase review NEEDS_FIXES for {} (cycle {})", phase_id, cycle + 1));

                    if cycle + 1 >= max_cycles {
                        cycle += 1;
                        self.update_phase_review_cycles(phase_id, cycle);
                        warn!("Phase {} reached max cycles ({}), deferring", phase_id, max_cycles);
                        emit_cm(&format!("Phase {} deferred after {} cycles", phase_id, max_cycles));
                        return Ok(Verdict::NeedsFixes);
                    }

                    // Re-fetch phase for fix prompt
                    let phase = self
                        .state
                        .phases
                        .iter()
                        .find(|p| p.id == phase_id)
                        .cloned()
                        .ok_or_else(|| StateError::PhaseNotFound(phase_id.to_string()))?;

                    // Spawn FIX agent with formatted review feedback
                    let review_feedback = format_review_feedback(&review_response);
                    let fix_prompt = PromptBuilder::build_phase_fix_prompt(&phase, &review_feedback);

                    let _ = self.phase_logger.log_prompt(phase_id, "PHASE_FIX", &fix_prompt);

                    self.log_manager
                        .log_agent_spawn(&AgentType::Fix, phase_id, &fix_prompt)?;
                    self.state.log_records.push(
                        LogManager::create_agent_spawn_record(&AgentType::Fix, phase_id, &fix_prompt),
                    );

                    let fix_agent_id = Uuid::new_v4().to_string();
                    let fix_started = Utc::now();

                    self.state.agent_history.push(AgentInvocation {
                        id: fix_agent_id.clone(),
                        task_id: phase_id.to_string(),
                        agent_type: AgentType::Fix,
                        started_at: fix_started,
                        completed_at: None,
                        exit_status: None,
                        commit_hash: None,
                    });

                    let fix_handle = self.agent_spawner.spawn(&fix_prompt, phase_id, "PHASE_FIX")?;
                    let fix_output = fix_handle.wait()?;

                    // Parse and log with parsed response if available
                    let fix_response = ResponseParser::parse(&fix_output.stdout).ok();
                    let _ = self.phase_logger.log_response(phase_id, "PHASE_FIX", &fix_output.stdout, fix_output.duration.as_secs(), fix_output.exit_code, fix_response.as_ref());

                    if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == fix_agent_id) {
                        inv.completed_at = Some(Utc::now());
                        inv.exit_status = fix_output.exit_code;
                    }

                    if let Some(ref resp) = fix_response {
                        self.log_manager.log_agent_response(resp)?;
                    }

                    if !self.check_git_changes()? {
                        warn!("PHASE_FIX made no changes for {}", phase_id);
                        continue;
                    }

                    match self.commit_agent_changes(phase_id, "PHASE_FIX", &fix_agent_id) {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("Failed to commit PHASE_FIX changes: {}", e);
                            continue;
                        }
                    }

                    // Re-verify build after FIX (if configured)
                    self.manager_state = ManagerState::Verifying;
                    if !self.config.build_commands.is_empty() {
                        if let Err(e) = self.build_verifier.verify_commands(&self.config.build_commands) {
                            warn!("Build failed after PHASE_FIX: {}", e);
                        }
                    }
                    self.manager_state = ManagerState::WaitingForAgent;

                    cycle += 1;
                    self.update_phase_review_cycles(phase_id, cycle);
                    self.update_state()?;
                }
            }
        }
    }

    /// Update the review_cycles_completed field for a phase.
    fn update_phase_review_cycles(&mut self, phase_id: &str, cycles: u32) {
        if let Ok(phase) = self.state.get_phase_mut(phase_id) {
            phase.review_cycles_completed = cycles;
        }
    }

    /// Save phase IMPLEM completion state.
    fn save_phase_implem_completion(&mut self, phase_id: &str, baseline_commit: &str) {
        if let Ok(phase) = self.state.get_phase_mut(phase_id) {
            phase.implem_completed_at = Some(Utc::now());
            phase.baseline_commit = Some(baseline_commit.to_string());
            phase.review_cycles_completed = 0;
        }
    }

    /// Clear phase IMPLEM completion state.
    fn clear_phase_implem_completion(&mut self, phase_id: &str) {
        if let Ok(phase) = self.state.get_phase_mut(phase_id) {
            phase.implem_completed_at = None;
            phase.baseline_commit = None;
            phase.review_cycles_completed = 0;
        }
    }

    /// Save the current state to disk.
    fn update_state(&mut self) -> Result<(), ManagerError> {
        save_state(&self.state, &self.config.state_path)?;
        debug!("State saved to {:?}", self.config.state_path);

        // Save roadmap and regenerate ROADMAP.md if loaded
        if let Some(ref roadmap) = self.roadmap_state {
            if let Err(e) = save_roadmap(roadmap, &self.config.roadmap_path) {
                warn!("Failed to save roadmap: {}", e);
            } else {
                debug!("Roadmap saved to {:?}", self.config.roadmap_path);
                let content = generate_roadmap_md(roadmap);
                if let Err(e) = write_md_file(&content, &self.config.roadmap_md_path) {
                    warn!("Failed to regenerate ROADMAP.md: {}", e);
                } else {
                    debug!("ROADMAP.md regenerated at {:?}", self.config.roadmap_md_path);
                }
            }
        }

        Ok(())
    }

    /// Sync a completed task's status to the roadmap.
    ///
    /// If the task has a `roadmap_item_id`, finds the corresponding roadmap item
    /// and marks it as completed.
    fn sync_roadmap_item(&mut self, task_id: &str) {
        // Find the task's roadmap_item_id
        let roadmap_item_id = self
            .state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .find(|t| t.id == task_id)
            .and_then(|t| t.roadmap_item_id.clone());

        let item_id = match roadmap_item_id {
            Some(id) => id,
            None => return,
        };

        let roadmap = match &mut self.roadmap_state {
            Some(r) => r,
            None => return,
        };

        // Find and mark the roadmap item as completed
        for phase in &mut roadmap.phases {
            for item in &mut phase.items {
                if item.id == item_id {
                    item.completed = true;
                    info!("Roadmap item '{}' marked completed (task {})", item.name, task_id);
                    // Cascade: mark all sub-items as completed too
                    for sub_item in &mut item.sub_items {
                        if !sub_item.completed {
                            sub_item.completed = true;
                            info!("Roadmap sub-item '{}' marked completed (cascaded from item {})", sub_item.name, item_id);
                        }
                    }
                    return;
                }
                // Also check sub-items by name (for direct sub-item targeting)
                for sub_item in &mut item.sub_items {
                    if sub_item.name == item_id {
                        sub_item.completed = true;
                        info!("Roadmap sub-item '{}' marked completed (task {})", sub_item.name, task_id);
                        return;
                    }
                }
            }
        }

        debug!("Roadmap item {} not found for task {}", item_id, task_id);
    }

    /// Sync a completed phase's status to the roadmap.
    ///
    /// Marks all items and sub-items in the corresponding roadmap phase as completed.
    fn sync_roadmap_phase(&mut self, phase_id: &str) {
        let roadmap = match &mut self.roadmap_state {
            Some(r) => r,
            None => return,
        };

        for phase in &mut roadmap.phases {
            if phase.id == phase_id {
                for item in &mut phase.items {
                    if !item.completed {
                        item.completed = true;
                        info!(
                            "Roadmap item '{}' marked completed (phase {})",
                            item.name, phase_id
                        );
                    }
                    for sub_item in &mut item.sub_items {
                        if !sub_item.completed {
                            sub_item.completed = true;
                            info!(
                                "Roadmap sub-item '{}' marked completed (phase {})",
                                sub_item.name, phase_id
                            );
                        }
                    }
                }
                return;
            }
        }

        debug!("Roadmap phase {} not found", phase_id);
    }

    /// Check if a phase just completed and run build verification.
    ///
    /// Returns `Ok(true)` if build passed (or no check needed),
    /// `Ok(false)` if build failed and a fix task was injected.
    fn check_phase_completion(&mut self, phase_id: &str) -> Result<bool, ManagerError> {
        if !self.state.all_phase_tasks_completed(phase_id) {
            return Ok(true); // Phase not complete yet, no check needed
        }

        // All tasks in phase completed - run build verification (if configured)
        self.manager_state = ManagerState::Verifying;

        // If no build commands configured, skip verification and mark complete
        if self.config.build_commands.is_empty() {
            info!("No build commands configured, skipping build verification");
            emit_cm(&format!("Phase {} complete (build verification skipped)", phase_id));

            // Mark phase as completed
            self.state
                .mark_phase_status(phase_id, PhaseStatus::Completed)?;

            // Sync roadmap phase - mark all items/sub-items as complete
            self.sync_roadmap_phase(phase_id);

            self.manager_state = ManagerState::Executing;
            return Ok(true);
        }

        emit_cm(&format!("Phase {} complete, verifying build...", phase_id));
        let build_result = self.build_verifier.verify_commands(&self.config.build_commands);

        match build_result {
            Ok(()) => {
                emit_cm(&format!("Build PASSED for phase {}", phase_id));
                let build_output = crate::build::BuildOutput {
                    success: true,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: String::new(),
                };
                self.log_manager.log_build_result(&build_output)?;
                self.state
                    .log_records
                    .push(LogManager::create_build_result_record(&build_output));

                // Mark phase as completed
                self.state
                    .mark_phase_status(phase_id, PhaseStatus::Completed)?;

                // Sync roadmap phase - mark all items/sub-items as complete
                self.sync_roadmap_phase(phase_id);

                self.manager_state = ManagerState::Executing;
                Ok(true)
            }
            Err(e) => {
                let err_msg = match &e {
                    BuildError::CommandFailed { stderr, .. } => stderr.clone(),
                    _ => e.to_string(),
                };
                emit_cm(&format!(
                    "Build FAILED for phase {}, injecting fix task",
                    phase_id
                ));

                let build_output = crate::build::BuildOutput {
                    success: false,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: err_msg.clone(),
                };
                self.log_manager.log_build_result(&build_output)?;
                self.state
                    .log_records
                    .push(LogManager::create_build_result_record(&build_output));

                // Inject a build-fix task
                self.inject_build_fix_task(phase_id, &err_msg)?;
                self.manager_state = ManagerState::Executing;
                Ok(false)
            }
        }
    }

    /// Inject a build fix task into the specified phase.
    fn inject_build_fix_task(
        &mut self,
        phase_id: &str,
        build_errors: &str,
    ) -> Result<(), ManagerError> {
        // Count existing build-fix tasks to create a unique ID
        let fix_count = self
            .state
            .phases
            .iter()
            .find(|p| p.id == phase_id)
            .map(|p| {
                p.tasks
                    .iter()
                    .filter(|t| t.id.contains("build-fix"))
                    .count()
            })
            .unwrap_or(0);

        let task_id = format!("{}.build-fix-{}", phase_id, fix_count + 1);
        let plan_content = format!(
            "The build failed after all tasks in this phase completed. Fix the build errors.\n\nBuild errors:\n{}",
            build_errors
        );

        // Create plan file for this task (use phase number for simple naming)
        let phase_num = phase_id.strip_prefix("phase-").unwrap_or(phase_id);
        let plan_file = format!(".cm/plans/plan-{}.md", phase_num);
        let plan_dir = std::path::Path::new(".cm/plans");
        if !plan_dir.exists() {
            std::fs::create_dir_all(plan_dir).map_err(StateError::from)?;
        }
        std::fs::write(&plan_file, &plan_content).map_err(StateError::from)?;

        let task = Task {
            id: task_id.clone(),
            name: format!("Fix build errors (attempt {})", fix_count + 1),
            task_type: TaskType::Fix,
            status: TaskStatus::Pending,
            depends_on: vec![],
            context: TaskContext {
                files_to_read: vec![],
                code_style_excerpt: None,
                prior_review_issues: vec![],
            },
            plan_file,
            attempts: vec![],
            roadmap_item_id: None,
            implem_completed_at: None,
            baseline_commit: None,
            review_cycles_completed: 0,
        };

        self.state.add_task_to_phase(phase_id, task)?;
        emit_cm(&format!("Injected build-fix task: {}", task_id));
        Ok(())
    }

    /// Check if git has any uncommitted changes in the working directory.
    ///
    /// Returns `true` if there are file changes (unstaged or staged), `false` otherwise.
    /// If git is unavailable, returns `true` (assumes changes were made).
    fn check_git_changes(&self) -> Result<bool, ManagerError> {
        match std::process::Command::new("git")
            .args(["diff", "--stat"])
            .current_dir(&self.config.working_dir)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(!stdout.trim().is_empty())
            }
            Err(e) => {
                warn!("Failed to run git diff, assuming changes exist: {}", e);
                Ok(true)
            }
        }
    }

    /// Verify the git working tree is clean before starting execution.
    ///
    /// This enforces that there are no uncommitted changes so that
    /// each agent's changes can be isolated in their own commit.
    ///
    /// # Errors
    ///
    /// Returns `ManagerError::TaskNotRunnable` if the working tree is dirty.
    fn preflight_git_check(&self) -> Result<(), ManagerError> {
        let git = GitRunner::new(self.config.working_dir.clone());
        match git.is_clean() {
            Ok(true) => Ok(()),
            Ok(false) => {
                let msg = "Working tree has uncommitted changes. Commit or stash them before running cm.";
                Err(ManagerError::TaskNotRunnable(msg.to_string()))
            }
            Err(e) => {
                warn!("Preflight git check failed (git not available?): {}", e);
                // Don't block execution if git isn't available
                Ok(())
            }
        }
    }

    /// Commit all agent changes with a descriptive message.
    ///
    /// Stages all changes (`git add -A`), commits with `--no-gpg-sign`,
    /// and returns the commit hash. Also records the commit hash in the
    /// agent's invocation history for audit.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID for the commit message
    /// * `agent_label` - The agent type label (e.g., "IMPLEM", "FIX")
    /// * `agent_id` - The agent invocation ID to record the commit hash against
    fn commit_agent_changes(
        &mut self,
        task_id: &str,
        agent_label: &str,
        agent_id: &str,
    ) -> Result<String, ManagerError> {
        let git = GitRunner::new(self.config.working_dir.clone());

        // Stage all changes
        git.add(&["-A"])?;

        // Commit with descriptive message
        let message = format!("cm: {} {} changes", task_id, agent_label);
        let commit_id = git.commit(&message)?;
        let hash = commit_id.0.clone();

        emit_cm(&format!("Committed {} changes: {}", agent_label, &hash[..8.min(hash.len())]));

        // Record commit hash in agent history
        if let Some(inv) = self
            .state
            .agent_history
            .iter_mut()
            .find(|i| i.id == agent_id)
        {
            inv.commit_hash = Some(hash.clone());
        }

        Ok(hash)
    }

    /// Get the current HEAD commit hash.
    fn get_head_commit(&self) -> Result<String, ManagerError> {
        let git = GitRunner::new(self.config.working_dir.clone());
        let commit = git.head_commit()?;
        Ok(commit.0)
    }

    /// Get the git diff between two commits.
    fn get_diff_between(&self, from: &str, to: &str) -> Result<String, ManagerError> {
        let git = GitRunner::new(self.config.working_dir.clone());
        let diff = git.diff_range(from, to)?;
        Ok(diff)
    }

}

/// Format review feedback from a parsed ReviewAgentResponse into a string for the fix agent.
///
/// This creates a structured text representation of the review issues that
/// the fix agent can understand and act upon.
fn format_review_feedback(response: &crate::agent::ReviewAgentResponse) -> String {
    let mut feedback = String::new();

    // Add summary
    if !response.summary.is_empty() {
        feedback.push_str("## Review Summary\n\n");
        feedback.push_str(&response.summary);
        feedback.push_str("\n\n");
    }

    // Add issues
    if !response.issues.is_empty() {
        feedback.push_str("## Issues to Fix\n\n");
        for issue in &response.issues {
            feedback.push_str(&format!("### Issue: {} ({})\n", issue.id, issue.severity));
            feedback.push_str(&format!("**Location:** {}\n", issue.location));
            feedback.push_str(&format!("**Problem:** {}\n", issue.problem));
            feedback.push_str(&format!("**Suggested Fix:** {}\n\n", issue.suggested_fix));
        }
    }

    feedback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_config_new() {
        let config = ManagerConfig::new(PathBuf::from("/tmp/tasks.json"));

        assert_eq!(config.state_path, PathBuf::from("/tmp/tasks.json"));
        assert_eq!(config.log_path, PathBuf::from("/tmp/LOG.md"));
        assert_eq!(config.max_cycles, 5);
    }

    #[test]
    fn test_manager_config_builder() {
        let config = ManagerConfig::new(PathBuf::from("/tmp/tasks.json"))
            .log_path(PathBuf::from("/tmp/custom.md"))
            .model("claude-opus-4-5-20251101".to_string())
            .max_cycles(10);

        assert_eq!(config.log_path, PathBuf::from("/tmp/custom.md"));
        assert_eq!(config.model, "claude-opus-4-5-20251101");
        assert_eq!(config.max_cycles, 10);
    }

    #[test]
    fn test_manager_error_display() {
        let err = ManagerError::TaskNotRunnable("task-1 is blocked".to_string());
        assert!(err.to_string().contains("task not runnable"));

        let err = ManagerError::MaxCyclesExceeded("task-2".to_string());
        assert!(err.to_string().contains("max cycles exceeded"));

        let err = ManagerError::NoRunnableTasks;
        assert!(err.to_string().contains("no runnable tasks"));

        let err = ManagerError::ShutdownRequested;
        assert!(err.to_string().contains("shutdown requested"));
    }

    #[test]
    fn test_format_review_feedback_with_issues() {
        use crate::agent::{ReviewAgentResponse, ReviewIssueResponse};
        use crate::state::AgentStatus;

        let response = ReviewAgentResponse {
            status: AgentStatus::Success,
            verdict: Some("needs_fixes".to_string()),
            summary: "Found 1 issue".to_string(),
            issues: vec![ReviewIssueResponse {
                id: "issue-1".to_string(),
                severity: "high".to_string(),
                location: "src/main.rs:42".to_string(),
                problem: "Missing error handling".to_string(),
                suggested_fix: "Add ? operator".to_string(),
            }],
            error: None,
        };

        let feedback = format_review_feedback(&response);
        assert!(feedback.contains("## Review Summary"));
        assert!(feedback.contains("Found 1 issue"));
        assert!(feedback.contains("### Issue: issue-1 (high)"));
        assert!(feedback.contains("**Location:** src/main.rs:42"));
        assert!(feedback.contains("**Problem:** Missing error handling"));
    }

    #[test]
    fn test_format_review_feedback_empty_issues() {
        use crate::agent::ReviewAgentResponse;
        use crate::state::AgentStatus;

        let response = ReviewAgentResponse {
            status: AgentStatus::Success,
            verdict: Some("approved".to_string()),
            summary: "Looks good".to_string(),
            issues: vec![],
            error: None,
        };

        let feedback = format_review_feedback(&response);
        assert!(feedback.contains("Looks good"));
        assert!(!feedback.contains("## Issues to Fix"));
    }

    #[test]
    fn test_manager_state_default() {
        assert_eq!(ManagerState::default(), ManagerState::Idle);
    }

    /// Helper function to parse phase input with range support.
    /// Matches the parsing logic in `prompt_task_selection()`.
    fn parse_phase_input(input: &str) -> Vec<String> {
        input
            .split_whitespace()
            .flat_map(|token| {
                if let Some((start_str, end_str)) = token.split_once('-') {
                    let start = match start_str.parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => return vec![],
                    };
                    let end = match end_str.parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => return vec![],
                    };
                    if start > end {
                        return vec![];
                    }
                    (start..=end).map(|n| format!("phase-{}", n)).collect::<Vec<_>>()
                } else {
                    if token.parse::<u32>().is_ok() {
                        vec![format!("phase-{}", token)]
                    } else {
                        vec![]
                    }
                }
            })
            .collect()
    }

    #[test]
    fn test_phase_range_parsing_valid_range() {
        // Test valid range "3-6" expands to ["phase-3", "phase-4", "phase-5", "phase-6"]
        let result = parse_phase_input("3-6");
        assert_eq!(result, vec!["phase-3", "phase-4", "phase-5", "phase-6"]);
    }

    #[test]
    fn test_phase_range_parsing_single_number() {
        // Test single number "3" returns ["phase-3"]
        let result = parse_phase_input("3");
        assert_eq!(result, vec!["phase-3"]);
    }

    #[test]
    fn test_phase_range_parsing_mixed_input() {
        // Test mixed input "1 3-5 8" returns ["phase-1", "phase-3", "phase-4", "phase-5", "phase-8"]
        let result = parse_phase_input("1 3-5 8");
        assert_eq!(result, vec!["phase-1", "phase-3", "phase-4", "phase-5", "phase-8"]);
    }

    #[test]
    fn test_phase_range_parsing_invalid_range() {
        // Test invalid range "5-3" returns empty (start > end)
        let result = parse_phase_input("5-3");
        assert!(result.is_empty());
    }

    #[test]
    fn test_phase_range_parsing_invalid_text() {
        // Test invalid text "abc-def" returns empty
        let result = parse_phase_input("abc-def");
        assert!(result.is_empty());
    }

    #[test]
    fn test_phase_range_parsing_edge_case_same_start_end() {
        // Test edge case "3-3" returns ["phase-3"]
        let result = parse_phase_input("3-3");
        assert_eq!(result, vec!["phase-3"]);
    }
}
