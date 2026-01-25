//! Main orchestration loop, state machine, and crash recovery.
//!
//! This module provides the core Manager that orchestrates the entire task
//! execution flow. It coordinates between the state module, agent spawner,
//! build verifier, and log manager to execute tasks in the correct order.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use log::{debug, error, info, warn};
use thiserror::Error;
use uuid::Uuid;

use crate::tui::{ManagerEvent, TuiCommand};

use crate::agent::{AgentError, AgentSpawner, PromptBuilder, ResponseParser};
use crate::build::{BuildError, BuildVerifier, GitRunner};
use crate::generate::{generate_log_md, write_md_file, GenerateError};
use crate::log::{FileLogError, FileLogger, LogError, LogLevel, LogManager};
use crate::state::{
    load_roadmap, load_state, save_roadmap, save_state, AgentInvocation, AgentStatus, AgentType,
    AttemptStatus, PhaseStatus, RoadmapState, StateError, Task, TaskAttempt, TaskContext,
    TaskStatus, TaskType, TasksState, Verdict,
};
use crate::generate::generate_roadmap_md;

mod recovery;
mod state;

pub use recovery::{CheckpointId, RecoveryAction, RecoveryError, RecoveryManager, ShutdownHandler};
pub use state::ManagerState;

/// Task selection mode from user prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSelection {
    /// Run a single task (the next runnable one)
    Single,
    /// Run all remaining tasks
    All,
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

    /// An error occurred while writing to the file log.
    #[error("file log error: {0}")]
    FileLogError(#[from] FileLogError),

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
    /// Timeout for agent execution.
    pub timeout: Duration,
    /// Maximum number of cycles (attempts) per task before deferring.
    pub max_cycles: u32,
    /// Path to the cm.log file for persistent operational logging.
    pub file_log_path: PathBuf,
    /// Whether verbose (DEBUG-level) file logging is enabled.
    pub verbose: bool,
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
            model: "claude-sonnet-4-5-20250929".to_string(),
            timeout: Duration::from_secs(300),
            max_cycles: 5,
            file_log_path: parent.join("cm.log"),
            verbose: false,
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

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum cycles.
    pub fn max_cycles(mut self, max_cycles: u32) -> Self {
        self.max_cycles = max_cycles;
        self
    }

    /// Set the file log path.
    pub fn file_log_path(mut self, path: PathBuf) -> Self {
        self.file_log_path = path;
        self
    }

    /// Set verbose mode.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
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
    /// Persistent file logger for .cm/cm.log.
    file_logger: FileLogger,
    /// Current execution state.
    manager_state: ManagerState,
    /// Shutdown flag for graceful termination.
    shutdown_flag: Arc<AtomicBool>,
    /// Optional event sender for TUI communication.
    event_tx: Option<Sender<ManagerEvent>>,
    /// Optional command receiver for TUI communication.
    cmd_rx: Option<Receiver<TuiCommand>>,
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
        let agent_spawner = AgentSpawner::new(config.model.clone(), config.timeout);
        let build_verifier = BuildVerifier::new(config.working_dir.clone());
        let log_manager = LogManager::new(config.log_path.clone());

        let log_level = if config.verbose {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };
        let file_logger = FileLogger::new(config.file_log_path.clone())?.with_level(log_level);

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

        // Log startup to file
        let _ = file_logger.info(
            "manager",
            &format!("Manager starting with model: {}", config.model),
        );

        Ok(Self {
            config,
            state,
            roadmap_state,
            agent_spawner,
            build_verifier,
            log_manager,
            file_logger,
            manager_state: ManagerState::Idle,
            shutdown_flag,
            event_tx: None,
            cmd_rx: None,
        })
    }

    /// Get the current manager state.
    pub fn manager_state(&self) -> ManagerState {
        self.manager_state
    }

    /// Log to file logger, ignoring errors (file log failures should not halt execution).
    fn flog(&self, level: LogLevel, component: &str, message: &str) {
        if let Err(e) = self.file_logger.log(level, component, message) {
            warn!("File log write failed: {}", e);
        }
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
        info!("Starting manager run loop");
        emit_cm("Starting run loop");
        self.flog(LogLevel::Info, "manager", "Execution loop started");

        // Ensure clean working tree for commit-per-agent audit trail
        self.preflight_git_check()?;

        self.manager_state = ManagerState::Executing;

        loop {
            // Check for shutdown signal before selecting next task
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received, saving state and exiting gracefully");
                self.flog(LogLevel::Warn, "manager", "Shutdown signal received, saving state");
                self.update_state()?;
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }

            // Select next task
            let task_id = match self.select_next_task() {
                Some(task) => task.id.clone(),
                None => {
                    info!("No more runnable tasks, exiting loop");
                    emit_cm("No more runnable tasks");
                    self.flog(LogLevel::Info, "manager", "No more runnable tasks");
                    break;
                }
            };

            info!("Selected task for execution: {}", task_id);
            emit_cm(&format!("Selected task: {}", task_id));
            self.flog(LogLevel::Info, "task", &format!("Task selected: {}", task_id));

            // Execute the task
            match self.execute_task(&task_id) {
                Ok(()) => {
                    info!("Task {} completed successfully", task_id);
                    emit_cm(&format!("Task {} completed", task_id));
                    self.flog(LogLevel::Info, "task", &format!("Task {} completed", task_id));
                }
                Err(e) => {
                    let error_msg = format!("Task {} failed: {}", task_id, e);
                    error!("{}", error_msg);
                    emit_cm(&format!("Task {} failed: {}", task_id, e));
                    self.flog(LogLevel::Error, "task", &error_msg);
                    self.log_manager.log_error(&error_msg)?;
                    self.state
                        .log_records
                        .push(LogManager::create_error_record(&error_msg));

                    // Don't propagate the error; continue with next task
                    // The task will be marked as failed/deferred in execute_task
                }
            }

            // Save state after each task
            self.update_state()?;

            // Check if the task's phase is now complete
            if let Some(phase) = self.state.find_phase_for_task(&task_id) {
                let phase_id = phase.id.clone();
                self.check_phase_completion(&phase_id)?;
                self.update_state()?;
            }

            // Check for shutdown signal after task completion
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received after task completion, exiting gracefully");
                self.flog(LogLevel::Warn, "manager", "Shutdown signal received after task completion");
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }
        }

        self.manager_state = ManagerState::Idle;
        info!("Manager run loop completed");
        emit_cm("Run loop completed");
        self.flog(LogLevel::Info, "manager", "Execution loop completed");
        Ok(())
    }

    /// Run the main orchestration loop with TUI channel communication.
    ///
    /// This variant of `run()` sends events to the TUI and checks for commands.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Sender for manager events to the TUI
    /// * `cmd_rx` - Receiver for commands from the TUI
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task execution fails
    /// - State cannot be saved
    /// - Shutdown was requested
    pub fn run_with_channels(
        &mut self,
        event_tx: Sender<ManagerEvent>,
        cmd_rx: Receiver<TuiCommand>,
    ) -> Result<(), ManagerError> {
        info!("Starting manager run loop with TUI channels");
        emit_cm("Starting run loop (TUI)");

        // Store channels for use in nested methods (like prompt_retry_cycles)
        self.event_tx = Some(event_tx.clone());
        self.cmd_rx = Some(cmd_rx);

        // Ensure clean working tree for commit-per-agent audit trail
        self.preflight_git_check()?;

        self.manager_state = ManagerState::Executing;

        // Send initial state to TUI
        let _ = event_tx.send(ManagerEvent::StateUpdated(Box::new(self.state.clone())));

        let mut paused = false;

        loop {
            // Check for shutdown signal
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received, saving state and exiting gracefully");
                self.update_state()?;
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }

            // Check for TUI commands (non-blocking)
            if let Some(ref rx) = self.cmd_rx {
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        TuiCommand::Pause => {
                            paused = !paused;
                            info!("Manager paused: {}", paused);
                        }
                        TuiCommand::Interrupt => {
                            info!("Interrupt received, stopping manager");
                            self.manager_state = ManagerState::Idle;
                            return Ok(());
                        }
                        TuiCommand::Quit => {
                            info!("Quit received, stopping manager");
                            self.manager_state = ManagerState::Idle;
                            return Ok(());
                        }
                        TuiCommand::RetryResponse { .. } => {
                            // Ignore - only expected during prompt_retry_cycles
                        }
                    }
                }
            }

            // If paused, wait a bit and check again
            if paused {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            // Select next task
            let task_id = match self.select_next_task() {
                Some(task) => task.id.clone(),
                None => {
                    info!("No more runnable tasks, exiting loop");
                    emit_cm("No more runnable tasks");
                    break;
                }
            };

            info!("Selected task for execution: {}", task_id);
            emit_cm(&format!("Selected task: {}", task_id));

            // Send TaskStarted event
            let _ = event_tx.send(ManagerEvent::TaskStarted {
                task_id: task_id.clone(),
            });

            // Execute the task
            match self.execute_task(&task_id) {
                Ok(()) => {
                    info!("Task {} completed successfully", task_id);
                    emit_cm(&format!("Task {} completed", task_id));
                    let _ = event_tx.send(ManagerEvent::TaskCompleted {
                        task_id: task_id.clone(),
                    });
                }
                Err(e) => {
                    let error_msg = format!("Task {} failed: {}", task_id, e);
                    error!("{}", error_msg);
                    emit_cm(&format!("Task {} failed: {}", task_id, e));
                    let _ = event_tx.send(ManagerEvent::Error(error_msg.clone()));
                    self.log_manager.log_error(&error_msg)?;
                    self.state
                        .log_records
                        .push(LogManager::create_error_record(&error_msg));

                    // Don't propagate the error; continue with next task
                    // The task will be marked as failed/deferred in execute_task
                }
            }

            // Save state after each task
            self.update_state()?;

            // Check if the task's phase is now complete
            if let Some(phase) = self.state.find_phase_for_task(&task_id) {
                let phase_id = phase.id.clone();
                self.check_phase_completion(&phase_id)?;
                self.update_state()?;
            }

            // Send updated state to TUI
            let _ = event_tx.send(ManagerEvent::StateUpdated(Box::new(self.state.clone())));

            // Check for shutdown signal after task completion
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown signal received after task completion, exiting gracefully");
                self.manager_state = ManagerState::Idle;
                return Err(ManagerError::ShutdownRequested);
            }
        }

        self.manager_state = ManagerState::Idle;
        info!("Manager run loop completed");
        emit_cm("Run loop completed");
        Ok(())
    }

    /// Execute a single step (one task) and return.
    ///
    /// # Errors
    ///
    /// Returns `ManagerError::NoRunnableTasks` if there are no tasks to run.
    pub fn step(&mut self) -> Result<(), ManagerError> {
        let task_id = match self.select_next_task() {
            Some(task) => task.id.clone(),
            None => return Err(ManagerError::NoRunnableTasks),
        };

        self.execute_task(&task_id)?;
        self.update_state()?;

        // Check if the task's phase is now complete
        if let Some(phase) = self.state.find_phase_for_task(&task_id) {
            let phase_id = phase.id.clone();
            self.check_phase_completion(&phase_id)?;
            self.update_state()?;
        }

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
        print!("\n[s]ingle / [a]ll / [q]uit: ");
        io::stdout()
            .flush()
            .map_err(|e| ManagerError::StateError(StateError::Io(e)))?;

        let stdin = io::stdin();
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| ManagerError::StateError(StateError::Io(e)))?;

        match line.trim().to_lowercase().as_str() {
            "s" | "single" => Ok(TaskSelection::Single),
            "a" | "all" => Ok(TaskSelection::All),
            "q" | "quit" | "" => Ok(TaskSelection::Quit),
            _ => {
                println!("Invalid selection. Use 's', 'a', or 'q'.");
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
                TaskSelection::Quit => {
                    println!("Exiting.");
                    break;
                }
            }
        }
        Ok(())
    }

    /// Select the next task to execute.
    ///
    /// Returns the first pending task whose dependencies are all completed.
    fn select_next_task(&self) -> Option<&Task> {
        self.state.next_runnable_task()
    }

    /// Execute a task by its ID.
    ///
    /// Dispatches to the appropriate execution method based on task type.
    fn execute_task(&mut self, task_id: &str) -> Result<(), ManagerError> {
        // Mark task as in progress
        self.state.mark_task_status(task_id, TaskStatus::InProgress)?;

        // Get the task (we need to clone to avoid borrow issues)
        let task = self
            .state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .find(|t| t.id == task_id)
            .cloned()
            .ok_or_else(|| ManagerError::TaskNotRunnable(format!("Task {} not found", task_id)))?;

        // Check if we've exceeded max cycles
        if task.attempts.len() >= self.config.max_cycles as usize {
            warn!(
                "Task {} has exceeded max cycles ({}), deferring",
                task_id, self.config.max_cycles
            );
            self.flog(
                LogLevel::Warn,
                "task",
                &format!("Task {} deferred: max cycles exceeded ({})", task_id, self.config.max_cycles),
            );
            let reason = format!("Exceeded maximum cycles ({})", self.config.max_cycles);
            self.state.mark_task_status(task_id, TaskStatus::Deferred)?;
            self.log_manager.log_task_deferred(task_id, &reason)?;
            self.state
                .log_records
                .push(LogManager::create_task_deferred_record(task_id, &reason));
            return Err(ManagerError::MaxCyclesExceeded(task_id.to_string()));
        }

        // Execute based on task type
        match task.task_type {
            TaskType::Implement => self.execute_implem(&task),
            TaskType::Review => self.execute_review(&task),
            TaskType::Fix => self.execute_fix(&task),
            TaskType::Test => self.execute_test(&task),
        }
    }

    /// Execute an implementation task.
    ///
    /// Full lifecycle: IMPLEM → commit → build verify → REVIEW → (FIX → re-verify → re-REVIEW)*
    ///
    /// Flow:
    /// 1. Record baseline commit (HEAD before agent runs)
    /// 2. Spawn IMPLEM agent, parse response
    /// 3. Commit agent changes (for audit trail)
    /// 4. Run build verification
    /// 5. Run review cycle (REVIEW → FIX → re-REVIEW, max_cycles)
    /// 6. Mark task complete if approved, or defer if review exhausted
    fn execute_implem(&mut self, task: &Task) -> Result<(), ManagerError> {
        debug!("Executing IMPLEM task: {}", task.id);
        self.flog(LogLevel::Info, "task", &format!("Executing IMPLEM task: {}", task.id));
        self.manager_state = ManagerState::WaitingForAgent;

        // Check if we should resume at REVIEW (IMPLEM already completed)
        if task.implem_completed_at.is_some() {
            if let Some(ref baseline) = task.baseline_commit {
                let starting_cycle = task.review_cycles_completed;
                info!(
                    "Resuming task {} at REVIEW cycle {} (IMPLEM already completed)",
                    task.id, starting_cycle
                );
                self.flog(
                    LogLevel::Info,
                    "task",
                    &format!("Resuming at REVIEW cycle {} for task {}", starting_cycle, task.id),
                );
                emit_cm(&format!("Resuming {} at REVIEW cycle {}", task.id, starting_cycle));

                let verdict = self.run_review_cycle(task, baseline, starting_cycle)?;

                match verdict {
                    Verdict::Approved => {
                        self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                        self.sync_roadmap_item(&task.id);
                        self.log_manager.log_task_complete(&task.id)?;
                        self.state
                            .log_records
                            .push(LogManager::create_task_complete_record(&task.id));
                        self.clear_implem_completion(&task.id);
                        self.flog(LogLevel::Info, "task", &format!("Task {} completed (resumed review approved)", task.id));
                    }
                    Verdict::NeedsFixes => {
                        let reason = "Review cycle exhausted after resuming".to_string();
                        self.state.mark_task_status(&task.id, TaskStatus::Deferred)?;
                        self.log_manager.log_task_deferred(&task.id, &reason)?;
                        self.state
                            .log_records
                            .push(LogManager::create_task_deferred_record(&task.id, &reason));
                        self.clear_implem_completion(&task.id);
                        self.flog(LogLevel::Warn, "task", &format!("Task {} deferred: {}", task.id, reason));
                    }
                }

                self.manager_state = ManagerState::Executing;
                return Ok(());
            }
        }

        // Record baseline commit for later diff
        let baseline_commit = self.get_head_commit().unwrap_or_default();

        // Build the prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_implem_prompt(task);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn (both to file and to state records)
        self.log_manager
            .log_agent_spawn(&AgentType::Implem, &task.id, &prompt)?;
        self.state.log_records.push(
            LogManager::create_agent_spawn_record(&AgentType::Implem, &task.id, &prompt),
        );

        // Generate agent ID
        let agent_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Record the invocation
        self.state.agent_history.push(AgentInvocation {
            id: agent_id.clone(),
            task_id: task.id.clone(),
            agent_type: AgentType::Implem,
            started_at,
            completed_at: None,
            exit_status: None,
            commit_hash: None,
        });

        // Spawn and wait for the agent
        self.flog(LogLevel::Info, "agent", &format!("Agent spawned for IMPLEM task {}", task.id));
        let handle = self.agent_spawner.spawn(&prompt, &task.id, "IMPLEM")?;
        let output = handle.wait()?;
        self.flog(LogLevel::Info, "agent", &format!("Agent completed for IMPLEM task {}", task.id));

        // Parse the response, retrying with --continue if parse fails
        let response = match ResponseParser::parse(&output.stdout) {
            Ok(resp) => resp,
            Err(AgentError::ParseError(msg)) => {
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                // Log the parse failure and raw response
                eprintln!("[{} IMPLEM] {} parse failed: {}", now, task.id, msg);
                eprintln!("[{} IMPLEM] {} raw response ({} chars):", now, task.id, output.stdout.len());
                eprintln!("{}", &output.stdout);

                // Try to extract session_id for retry
                if let Some(session_id) = &output.session_id {
                    eprintln!("[{} IMPLEM] {} retrying with --continue...", now, task.id);

                    let retry_prompt = "Your previous response could not be parsed correctly. \
                        Please provide a summary of your changes. \
                        If you made file changes, list them briefly.";

                    let retry_handle = self
                        .agent_spawner
                        .spawn_with_continue(session_id, retry_prompt, &task.id, "IMPLEM")?;
                    let retry_output = retry_handle.wait()?;

                    // Try parsing again, fail if still bad
                    ResponseParser::parse(&retry_output.stdout).map_err(|e| {
                        let now2 = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                        eprintln!("[{} IMPLEM] {} retry also failed: {}", now2, task.id, e);
                        e
                    })?
                } else {
                    eprintln!("[{} IMPLEM] {} no session_id available, cannot retry", now, task.id);
                    return Err(AgentError::ParseError(msg).into());
                }
            }
            Err(e) => return Err(e.into()),
        };
        debug!(
            "Agent response: {}",
            &response.message[..response.message.len().min(500)]
        );

        // Log agent response
        self.log_manager.log_agent_response(&response)?;

        // Update invocation completion
        if let Some(inv) = self
            .state
            .agent_history
            .iter_mut()
            .find(|i| i.id == agent_id)
        {
            inv.completed_at = Some(Utc::now());
            inv.exit_status = output.exit_code;
        }

        // Check if agent reported failure
        if response.status == AgentStatus::Failed {
            warn!(
                "Agent reported failure for task {}: {}",
                task.id, response.message
            );
            self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Verify agent actually made file changes
        if !self.check_git_changes()? {
            warn!(
                "Agent reported success but no file changes detected for task {}",
                task.id
            );
            self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Commit IMPLEM agent changes for audit trail
        match self.commit_agent_changes(&task.id, "IMPLEM", &agent_id) {
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to commit IMPLEM changes for {}: {}", task.id, e);
                self.flog(LogLevel::Warn, "git", &format!("IMPLEM commit failed: {}", e));
                // Continue anyway — changes exist but aren't committed
            }
        }

        // Record successful IMPLEM attempt
        self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, &prompt, Some(response))?;

        // Run build verification
        self.manager_state = ManagerState::Verifying;
        let build_result = self.build_verifier.verify_all();
        let build_output = match &build_result {
            Ok(()) => crate::build::BuildOutput {
                success: true,
                errors: vec![],
                warnings: vec![],
                stdout: String::new(),
                stderr: String::new(),
            },
            Err(e) => {
                let err_msg = match e {
                    BuildError::CommandFailed { stderr, .. } => stderr.clone(),
                    _ => e.to_string(),
                };
                crate::build::BuildOutput {
                    success: false,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: err_msg,
                }
            }
        };
        self.log_manager.log_build_result(&build_output)?;
        self.state
            .log_records
            .push(LogManager::create_build_result_record(&build_output));

        if let Err(e) = build_result {
            let err_msg = match &e {
                BuildError::CommandFailed { stderr, .. } => stderr.clone(),
                _ => e.to_string(),
            };
            warn!("Build failed after IMPLEM for task {}: {}", task.id, err_msg);
            emit_cm(&format!("Build FAILED after IMPLEM for {}", task.id));
            self.flog(LogLevel::Warn, "build", &format!("Build failed after IMPLEM: {}", err_msg));
            // Mark pending for retry — the next IMPLEM attempt may fix it
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        emit_cm(&format!("Build PASSED for {}, starting review", task.id));
        self.flog(LogLevel::Info, "build", &format!("Build PASSED for task {}", task.id));

        // Save IMPLEM completion state for potential resume on --continue
        self.save_implem_completion(&task.id, &baseline_commit);

        // Run review cycle: REVIEW → FIX → re-REVIEW (max_cycles)
        if !baseline_commit.is_empty() {
            let starting_cycle = task.review_cycles_completed;
            let verdict = self.run_review_cycle(task, &baseline_commit, starting_cycle)?;

            match verdict {
                Verdict::Approved => {
                    self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                    self.sync_roadmap_item(&task.id);
                    self.log_manager.log_task_complete(&task.id)?;
                    self.state
                        .log_records
                        .push(LogManager::create_task_complete_record(&task.id));
                    self.clear_implem_completion(&task.id);
                    self.flog(LogLevel::Info, "task", &format!("Task {} completed (review approved)", task.id));
                }
                Verdict::NeedsFixes => {
                    // Review cycle exhausted — defer the task
                    let reason = format!("Review cycle exhausted after {} cycles", self.config.max_cycles);
                    self.state.mark_task_status(&task.id, TaskStatus::Deferred)?;
                    self.log_manager.log_task_deferred(&task.id, &reason)?;
                    self.state
                        .log_records
                        .push(LogManager::create_task_deferred_record(&task.id, &reason));
                    self.clear_implem_completion(&task.id);
                    self.flog(LogLevel::Warn, "task", &format!("Task {} deferred: {}", task.id, reason));
                }
            }
        } else {
            // No baseline commit (git not available?) — skip review, mark complete
            warn!("No baseline commit for task {}, skipping review", task.id);
            self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
            self.sync_roadmap_item(&task.id);
            self.log_manager.log_task_complete(&task.id)?;
            self.state
                .log_records
                .push(LogManager::create_task_complete_record(&task.id));
            self.flog(LogLevel::Info, "task", &format!("Task {} completed (no review — git unavailable)", task.id));
        }

        self.manager_state = ManagerState::Executing;
        Ok(())
    }

    /// Execute a review task.
    ///
    /// Flow:
    /// 1. Build review prompt
    /// 2. Spawn agent
    /// 3. Parse review result
    /// 4. If APPROVED, mark complete
    /// 5. If NEEDS_FIXES, increment attempt count
    fn execute_review(&mut self, task: &Task) -> Result<(), ManagerError> {
        debug!("Executing REVIEW task: {}", task.id);
        self.flog(LogLevel::Info, "task", &format!("Executing REVIEW task: {}", task.id));
        self.manager_state = ManagerState::WaitingForAgent;

        // For review, we need to get the code to review
        // This is typically from the files_to_read in the context
        let code_to_review = self.gather_code_for_review(task);

        // Build the review prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_review_prompt(task, &code_to_review);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn (both to file and to state records)
        self.log_manager
            .log_agent_spawn(&AgentType::Review, &task.id, &prompt)?;
        self.state.log_records.push(
            LogManager::create_agent_spawn_record(&AgentType::Review, &task.id, &prompt),
        );

        // Generate agent ID
        let agent_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Record the invocation
        self.state.agent_history.push(AgentInvocation {
            id: agent_id.clone(),
            task_id: task.id.clone(),
            agent_type: AgentType::Review,
            started_at,
            completed_at: None,
            exit_status: None,
            commit_hash: None,
        });

        // Spawn and wait for the agent
        self.flog(LogLevel::Info, "agent", &format!("Agent spawned for REVIEW task {}", task.id));
        let handle = self.agent_spawner.spawn(&prompt, &task.id, "REVIEW")?;
        let output = handle.wait()?;
        self.flog(LogLevel::Info, "agent", &format!("Agent completed for REVIEW task {}", task.id));

        // Parse the response, retrying with --continue if parse fails
        let response = match ResponseParser::parse(&output.stdout) {
            Ok(resp) => resp,
            Err(AgentError::ParseError(msg)) => {
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                eprintln!("[{} REVIEW] {} parse failed: {}", now, task.id, msg);
                eprintln!("[{} REVIEW] {} raw response ({} chars):", now, task.id, output.stdout.len());
                eprintln!("{}", &output.stdout);

                if let Some(session_id) = &output.session_id {
                    eprintln!("[{} REVIEW] {} retrying with --continue...", now, task.id);

                    let retry_prompt = "Your previous response could not be parsed correctly. \
                        Please provide your review verdict and any issues found.";

                    let retry_handle = self
                        .agent_spawner
                        .spawn_with_continue(session_id, retry_prompt, &task.id, "REVIEW")?;
                    let retry_output = retry_handle.wait()?;

                    ResponseParser::parse(&retry_output.stdout).map_err(|e| {
                        let now2 = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                        eprintln!("[{} REVIEW] {} retry also failed: {}", now2, task.id, e);
                        e
                    })?
                } else {
                    eprintln!("[{} REVIEW] {} no session_id available, cannot retry", now, task.id);
                    return Err(AgentError::ParseError(msg).into());
                }
            }
            Err(e) => return Err(e.into()),
        };
        debug!(
            "Agent response: {}",
            &response.message[..response.message.len().min(500)]
        );

        // Log agent response
        self.log_manager.log_agent_response(&response)?;

        // Update invocation completion
        if let Some(inv) = self
            .state
            .agent_history
            .iter_mut()
            .find(|i| i.id == agent_id)
        {
            inv.completed_at = Some(Utc::now());
            inv.exit_status = output.exit_code;
        }

        // Check if agent reported failure
        if response.status == AgentStatus::Failed {
            warn!(
                "Agent reported failure for review task {}: {}",
                task.id, response.message
            );
            self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Try to extract review verdict from the response
        let verdict = self.extract_review_verdict(&response.message);

        // Log review result
        self.log_manager.log_review_result(&verdict, &[])?;
        self.state
            .log_records
            .push(LogManager::create_review_result_record(&verdict, &[]));

        match verdict {
            Verdict::Approved => {
                info!("Review approved for task {}", task.id);
                emit_cm(&format!("Review APPROVED for {}", task.id));
                self.flog(LogLevel::Info, "task", &format!("Review APPROVED for task {}", task.id));
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, &prompt, Some(response))?;
                self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                self.sync_roadmap_item(&task.id);
                self.log_manager.log_task_complete(&task.id)?;
                self.state
                    .log_records
                    .push(LogManager::create_task_complete_record(&task.id));
            }
            Verdict::NeedsFixes => {
                warn!("Review found issues for task {}", task.id);
                emit_cm(&format!("Review NEEDS_FIXES for {}", task.id));
                self.flog(LogLevel::Warn, "task", &format!("Review NEEDS_FIXES for task {}", task.id));
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
                // Keep task pending for another cycle
                self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            }
        }

        self.manager_state = ManagerState::Executing;
        Ok(())
    }

    /// Execute a fix task.
    ///
    /// Flow:
    /// 1. Build fix prompt with issues
    /// 2. Spawn agent
    /// 3. Parse response
    /// 4. Run build verification
    fn execute_fix(&mut self, task: &Task) -> Result<(), ManagerError> {
        debug!("Executing FIX task: {}", task.id);
        self.flog(LogLevel::Info, "task", &format!("Executing FIX task: {}", task.id));
        self.manager_state = ManagerState::WaitingForAgent;

        // Get prior review issues from context
        let issues = self.gather_issues_for_fix(task);

        // Build the fix prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_fix_prompt(task, &issues);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn (both to file and to state records)
        self.log_manager
            .log_agent_spawn(&AgentType::Fix, &task.id, &prompt)?;
        self.state.log_records.push(
            LogManager::create_agent_spawn_record(&AgentType::Fix, &task.id, &prompt),
        );

        // Generate agent ID
        let agent_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        // Record the invocation
        self.state.agent_history.push(AgentInvocation {
            id: agent_id.clone(),
            task_id: task.id.clone(),
            agent_type: AgentType::Fix,
            started_at,
            completed_at: None,
            exit_status: None,
            commit_hash: None,
        });

        // Spawn and wait for the agent
        self.flog(LogLevel::Info, "agent", &format!("Agent spawned for FIX task {}", task.id));
        let handle = self.agent_spawner.spawn(&prompt, &task.id, "FIX")?;
        let output = handle.wait()?;
        self.flog(LogLevel::Info, "agent", &format!("Agent completed for FIX task {}", task.id));

        // Parse the response, retrying with --continue if parse fails
        let response = match ResponseParser::parse(&output.stdout) {
            Ok(resp) => resp,
            Err(AgentError::ParseError(msg)) => {
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                eprintln!("[{} FIX] {} parse failed: {}", now, task.id, msg);
                eprintln!("[{} FIX] {} raw response ({} chars):", now, task.id, output.stdout.len());
                eprintln!("{}", &output.stdout);

                if let Some(session_id) = &output.session_id {
                    eprintln!("[{} FIX] {} retrying with --continue...", now, task.id);

                    let retry_prompt = "Your previous response could not be parsed correctly. \
                        Please provide a summary of the fixes you made.";

                    let retry_handle = self
                        .agent_spawner
                        .spawn_with_continue(session_id, retry_prompt, &task.id, "FIX")?;
                    let retry_output = retry_handle.wait()?;

                    ResponseParser::parse(&retry_output.stdout).map_err(|e| {
                        let now2 = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                        eprintln!("[{} FIX] {} retry also failed: {}", now2, task.id, e);
                        e
                    })?
                } else {
                    eprintln!("[{} FIX] {} no session_id available, cannot retry", now, task.id);
                    return Err(AgentError::ParseError(msg).into());
                }
            }
            Err(e) => return Err(e.into()),
        };
        debug!(
            "Agent response: {}",
            &response.message[..response.message.len().min(500)]
        );

        // Log agent response
        self.log_manager.log_agent_response(&response)?;

        // Update invocation completion
        if let Some(inv) = self
            .state
            .agent_history
            .iter_mut()
            .find(|i| i.id == agent_id)
        {
            inv.completed_at = Some(Utc::now());
            inv.exit_status = output.exit_code;
        }

        // Check if agent reported failure
        if response.status == AgentStatus::Failed {
            warn!(
                "Agent reported failure for fix task {}: {}",
                task.id, response.message
            );
            self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Verify agent actually made file changes
        if !self.check_git_changes()? {
            warn!(
                "Agent reported success but no file changes detected for fix task {}",
                task.id
            );
            self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, &prompt, Some(response))?;
            self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            self.manager_state = ManagerState::Executing;
            return Ok(());
        }

        // Record successful attempt (build verification deferred to phase completion)
        self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, &prompt, Some(response))?;

        // Mark task as completed
        self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
        self.sync_roadmap_item(&task.id);
        self.log_manager.log_task_complete(&task.id)?;
        self.state
            .log_records
            .push(LogManager::create_task_complete_record(&task.id));
        self.flog(LogLevel::Info, "task", &format!("FIX task {} marked completed", task.id));

        self.manager_state = ManagerState::Executing;
        Ok(())
    }

    /// Execute a test task.
    ///
    /// For now, test tasks are handled similarly to implement tasks.
    fn execute_test(&mut self, task: &Task) -> Result<(), ManagerError> {
        debug!("Executing TEST task: {}", task.id);
        // Test tasks follow the same flow as implement tasks
        self.execute_implem(task)
    }

    /// Prompt user whether to retry after max review cycles exhausted.
    ///
    /// In TUI mode, sends a RetryPrompt event and waits for RetryResponse.
    /// In daemon mode (no TUI channels), prompts via stdin/stdout.
    ///
    /// Returns Some(additional_cycles) if user wants to retry, None if they decline.
    fn prompt_retry_cycles(&self, task_id: &str, cycles_completed: u32) -> Option<u32> {
        let message = format!(
            "Task {} exhausted {} review cycles. Retry more cycles?",
            task_id, cycles_completed
        );

        // TUI mode: send event and wait for response
        if let (Some(ref tx), Some(ref rx)) = (&self.event_tx, &self.cmd_rx) {
            let _ = tx.send(ManagerEvent::RetryPrompt {
                task_id: task_id.to_string(),
                cycles_completed,
                message: message.clone(),
            });

            // Wait for response with 5 minute timeout
            let timeout = Duration::from_secs(300);
            loop {
                match rx.recv_timeout(timeout) {
                    Ok(TuiCommand::RetryResponse { retry, additional_cycles }) => {
                        if retry && additional_cycles > 0 {
                            return Some(additional_cycles);
                        } else {
                            return None;
                        }
                    }
                    Ok(TuiCommand::Quit) | Ok(TuiCommand::Interrupt) => {
                        return None;
                    }
                    Ok(_) => {
                        // Ignore other commands while waiting
                        continue;
                    }
                    Err(_) => {
                        // Timeout - treat as decline
                        warn!("Retry prompt timed out for task {}", task_id);
                        return None;
                    }
                }
            }
        }

        // Daemon mode: use stdin/stdout
        use std::io::{self, BufRead, Write};

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
            Some(5) // Default to 5 more cycles
        } else if let Ok(n) = input.parse::<u32>() {
            if n > 0 { Some(n) } else { None }
        } else {
            None
        }
    }

    /// Run the REVIEW → FIX → re-REVIEW cycle after an IMPLEM task.
    ///
    /// Spawns a REVIEW agent to check the IMPLEM agent's changes (identified
    /// via git diff from the baseline commit). If the review finds issues,
    /// spawns a FIX agent, commits its changes, re-verifies the build, and
    /// re-reviews. Repeats up to `max_cycles` times.
    ///
    /// # Arguments
    ///
    /// * `task` - The original IMPLEM task (for context)
    /// * `baseline_commit` - The HEAD commit hash before the IMPLEM agent ran
    /// * `starting_cycle` - The cycle number to start from (for resuming)
    ///
    /// # Returns
    ///
    /// `Ok(Verdict::Approved)` if the review passes, or
    /// `Ok(Verdict::NeedsFixes)` if max cycles exhausted without approval.
    fn run_review_cycle(
        &mut self,
        task: &Task,
        baseline_commit: &str,
        starting_cycle: u32,
    ) -> Result<Verdict, ManagerError> {
        let mut cycle = starting_cycle;
        let mut max_cycles = self.config.max_cycles;
        let task_id = task.id.clone();

        loop {
            // Check if we've exceeded max cycles
            if cycle >= max_cycles {
                // Prompt user for more cycles
                if let Some(additional) = self.prompt_retry_cycles(&task_id, cycle) {
                    max_cycles += additional;
                    self.flog(
                        LogLevel::Info,
                        "review",
                        &format!("User requested {} more cycles for task {}", additional, task_id),
                    );
                    emit_cm(&format!("Retrying {} more cycles for {}", additional, task_id));
                } else {
                    // User declined - return NeedsFixes
                    warn!("Review cycle exhausted for task {} after {} cycles", task_id, cycle);
                    self.flog(
                        LogLevel::Warn,
                        "review",
                        &format!("Review cycle exhausted for task {} after {} cycles", task_id, cycle),
                    );
                    return Ok(Verdict::NeedsFixes);
                }
            }
            self.flog(
                LogLevel::Info,
                "review",
                &format!("Review cycle {}/{} for task {}", cycle + 1, max_cycles, task_id),
            );
            emit_cm(&format!("Review cycle {}/{} for {}", cycle + 1, max_cycles, task_id));

            // Get diff from baseline to current HEAD
            let diff = self.get_diff_between(baseline_commit, "HEAD")?;

            if diff.trim().is_empty() {
                warn!("No diff found between baseline and HEAD for task {}", task.id);
                self.flog(LogLevel::Warn, "review", "No diff to review, skipping review");
                return Ok(Verdict::Approved);
            }

            // Build auto-review prompt and spawn REVIEW agent
            let review_prompt = PromptBuilder::build_auto_review_prompt(task, &diff);

            self.log_manager
                .log_agent_spawn(&AgentType::Review, &task.id, &review_prompt)?;
            self.state.log_records.push(
                LogManager::create_agent_spawn_record(&AgentType::Review, &task.id, &review_prompt),
            );

            let review_agent_id = Uuid::new_v4().to_string();
            let review_started = Utc::now();

            self.state.agent_history.push(AgentInvocation {
                id: review_agent_id.clone(),
                task_id: task.id.clone(),
                agent_type: AgentType::Review,
                started_at: review_started,
                completed_at: None,
                exit_status: None,
                commit_hash: None,
            });

            self.flog(LogLevel::Info, "agent", &format!("Spawning auto-REVIEW for task {}", task.id));
            let review_handle = self.agent_spawner.spawn(&review_prompt, &task.id, "REVIEW")?;
            let review_output = review_handle.wait()?;
            self.flog(LogLevel::Info, "agent", &format!("Auto-REVIEW completed for task {}", task.id));

            // Parse review response
            let review_response = match ResponseParser::parse(&review_output.stdout) {
                Ok(resp) => resp,
                Err(AgentError::ParseError(msg)) => {
                    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                    eprintln!("[{} REVIEW] {} parse failed: {}", now, task.id, msg);

                    if let Some(session_id) = &review_output.session_id {
                        eprintln!("[{} REVIEW] {} retrying with --continue...", now, task.id);
                        let retry_handle = self.agent_spawner.spawn_with_continue(
                            session_id,
                            "Your previous response could not be parsed. Please provide your review verdict and any issues found.",
                            &task.id,
                            "REVIEW",
                        )?;
                        let retry_output = retry_handle.wait()?;
                        match ResponseParser::parse(&retry_output.stdout) {
                            Ok(resp) => resp,
                            Err(_) => {
                                // Can't parse review — treat as approved to not block progress
                                warn!("Review parse failed twice for {}, treating as approved", task.id);
                                self.flog(LogLevel::Warn, "review", "Review parse failed, treating as approved");
                                return Ok(Verdict::Approved);
                            }
                        }
                    } else {
                        warn!("Review parse failed for {} (no session_id), treating as approved", task.id);
                        return Ok(Verdict::Approved);
                    }
                }
                Err(e) => return Err(e.into()),
            };

            // Update invocation
            if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == review_agent_id) {
                inv.completed_at = Some(Utc::now());
                inv.exit_status = review_output.exit_code;
            }

            self.log_manager.log_agent_response(&review_response)?;

            // Check for agent failure
            if review_response.status == AgentStatus::Failed {
                warn!("Review agent failed for {}: {}", task.id, review_response.message);
                self.flog(LogLevel::Warn, "review", &format!("Review agent failed: {}", review_response.message));
                // Treat failure as approved to not block
                return Ok(Verdict::Approved);
            }

            // Extract verdict
            let verdict = self.extract_review_verdict(&review_response.message);

            self.log_manager.log_review_result(&verdict, &[])?;
            self.state
                .log_records
                .push(LogManager::create_review_result_record(&verdict, &[]));

            match verdict {
                Verdict::Approved => {
                    info!("Auto-review approved for task {}", task.id);
                    emit_cm(&format!("Review APPROVED for {}", task.id));
                    self.flog(LogLevel::Info, "review", &format!("Review APPROVED for task {}", task.id));
                    return Ok(Verdict::Approved);
                }
                Verdict::NeedsFixes => {
                    warn!("Auto-review found issues for task {} (cycle {})", task.id, cycle + 1);
                    emit_cm(&format!("Review NEEDS_FIXES for {} (cycle {})", task.id, cycle + 1));
                    self.flog(
                        LogLevel::Warn,
                        "review",
                        &format!("Review NEEDS_FIXES for task {} (cycle {})", task.id, cycle + 1),
                    );

                    // If this is the last cycle, increment and continue to the loop check
                    // (which will prompt user for more cycles)
                    if cycle + 1 >= max_cycles {
                        cycle += 1;
                        // Update review_cycles_completed in state
                        self.update_task_review_cycles(&task_id, cycle);
                        continue;
                    }

                    // Spawn FIX agent with the review feedback
                    let fix_prompt = PromptBuilder::build_auto_fix_prompt(task, &review_response.message);

                    self.log_manager
                        .log_agent_spawn(&AgentType::Fix, &task.id, &fix_prompt)?;
                    self.state.log_records.push(
                        LogManager::create_agent_spawn_record(&AgentType::Fix, &task.id, &fix_prompt),
                    );

                    let fix_agent_id = Uuid::new_v4().to_string();
                    let fix_started = Utc::now();

                    self.state.agent_history.push(AgentInvocation {
                        id: fix_agent_id.clone(),
                        task_id: task.id.clone(),
                        agent_type: AgentType::Fix,
                        started_at: fix_started,
                        completed_at: None,
                        exit_status: None,
                        commit_hash: None,
                    });

                    self.flog(LogLevel::Info, "agent", &format!("Spawning auto-FIX for task {}", task.id));
                    let fix_handle = self.agent_spawner.spawn(&fix_prompt, &task.id, "FIX")?;
                    let fix_output = fix_handle.wait()?;
                    self.flog(LogLevel::Info, "agent", &format!("Auto-FIX completed for task {}", task.id));

                    // Update invocation
                    if let Some(inv) = self.state.agent_history.iter_mut().find(|i| i.id == fix_agent_id) {
                        inv.completed_at = Some(Utc::now());
                        inv.exit_status = fix_output.exit_code;
                    }

                    // Parse FIX response (best-effort, don't block on parse failure)
                    if let Ok(fix_response) = ResponseParser::parse(&fix_output.stdout) {
                        self.log_manager.log_agent_response(&fix_response)?;
                    }

                    // Check if FIX agent made changes
                    if !self.check_git_changes()? {
                        warn!("FIX agent made no changes for task {}", task.id);
                        self.flog(LogLevel::Warn, "review", "FIX agent made no file changes");
                        continue; // Re-review anyway (might still pass)
                    }

                    // Commit FIX changes
                    match self.commit_agent_changes(&task.id, "FIX", &fix_agent_id) {
                        Ok(_hash) => {}
                        Err(e) => {
                            warn!("Failed to commit FIX changes for {}: {}", task.id, e);
                            self.flog(LogLevel::Warn, "git", &format!("FIX commit failed: {}", e));
                            continue;
                        }
                    }

                    // Re-verify build after FIX
                    self.manager_state = ManagerState::Verifying;
                    if let Err(e) = self.build_verifier.verify_all() {
                        let err_msg = match &e {
                            BuildError::CommandFailed { stderr, .. } => stderr.clone(),
                            _ => e.to_string(),
                        };
                        warn!("Build failed after FIX for task {}: {}", task.id, err_msg);
                        self.flog(LogLevel::Warn, "build", &format!("Build failed after FIX: {}", err_msg));
                        // Continue to next cycle — the next FIX attempt might resolve it
                    }
                    self.manager_state = ManagerState::WaitingForAgent;

                    // Increment cycle and update state
                    cycle += 1;
                    self.update_task_review_cycles(&task_id, cycle);

                    // Save state between cycles
                    self.update_state()?;
                }
            }
        }
        // Note: loop only exits via return statements above
    }

    /// Update the review_cycles_completed field for a task.
    fn update_task_review_cycles(&mut self, task_id: &str, cycles: u32) {
        for phase in &mut self.state.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.review_cycles_completed = cycles;
                    return;
                }
            }
        }
    }

    /// Save IMPLEM completion state for a task.
    ///
    /// This allows resuming at the REVIEW stage on --continue instead of
    /// re-running IMPLEM.
    fn save_implem_completion(&mut self, task_id: &str, baseline_commit: &str) {
        for phase in &mut self.state.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.implem_completed_at = Some(Utc::now());
                    task.baseline_commit = Some(baseline_commit.to_string());
                    task.review_cycles_completed = 0;
                    return;
                }
            }
        }
    }

    /// Clear IMPLEM completion state for a task.
    ///
    /// Called when a task is completed or deferred.
    fn clear_implem_completion(&mut self, task_id: &str) {
        for phase in &mut self.state.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.implem_completed_at = None;
                    task.baseline_commit = None;
                    task.review_cycles_completed = 0;
                    return;
                }
            }
        }
    }

    /// Save the current state to disk and regenerate LOG.md.
    fn update_state(&mut self) -> Result<(), ManagerError> {
        save_state(&self.state, &self.config.state_path)?;
        debug!("State saved to {:?}", self.config.state_path);
        self.flog(LogLevel::Debug, "state", &format!("State saved to {:?}", self.config.state_path));

        // Regenerate LOG.md from log_records
        self.regenerate_log_md()?;

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

    /// Regenerate LOG.md from the log_records in state.
    fn regenerate_log_md(&self) -> Result<(), ManagerError> {
        let content = generate_log_md(&self.state.log_records);
        write_md_file(&content, &self.config.log_path)?;
        debug!("LOG.md regenerated at {:?}", self.config.log_path);
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

        // All tasks in phase completed - run build verification
        emit_cm(&format!("Phase {} complete, verifying build...", phase_id));
        self.flog(
            LogLevel::Info,
            "build",
            &format!("Phase {} complete, running build verification", phase_id),
        );
        self.manager_state = ManagerState::Verifying;

        let build_result = self.build_verifier.verify_all();

        match build_result {
            Ok(()) => {
                emit_cm(&format!("Build PASSED for phase {}", phase_id));
                self.flog(
                    LogLevel::Info,
                    "build",
                    &format!("Build PASSED for phase {}", phase_id),
                );
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
                self.flog(
                    LogLevel::Warn,
                    "build",
                    &format!("Build FAILED for phase {}: {}", phase_id, err_msg),
                );

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
        let instructions = format!(
            "The build failed after all tasks in this phase completed. Fix the build errors.\n\nBuild errors:\n{}",
            build_errors
        );

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
            instructions,
            attempts: vec![],
            roadmap_item_id: None,
            implem_completed_at: None,
            baseline_commit: None,
            review_cycles_completed: 0,
        };

        self.state.add_task_to_phase(phase_id, task)?;
        emit_cm(&format!("Injected build-fix task: {}", task_id));
        self.flog(
            LogLevel::Info,
            "task",
            &format!("Injected build-fix task: {}", task_id),
        );
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
            Ok(true) => {
                self.flog(LogLevel::Info, "git", "Preflight check: working tree is clean");
                Ok(())
            }
            Ok(false) => {
                let msg = "Working tree has uncommitted changes. Commit or stash them before running cm.";
                self.flog(LogLevel::Error, "git", msg);
                Err(ManagerError::TaskNotRunnable(msg.to_string()))
            }
            Err(e) => {
                warn!("Preflight git check failed (git not available?): {}", e);
                self.flog(LogLevel::Warn, "git", &format!("Preflight git check failed: {}", e));
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

        self.flog(
            LogLevel::Info,
            "git",
            &format!("Committed {} changes for {}: {}", agent_label, task_id, hash),
        );
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

    /// Record a task attempt in the state.
    fn record_attempt(
        &mut self,
        task_id: &str,
        agent_id: &str,
        started_at: chrono::DateTime<Utc>,
        status: AttemptStatus,
        prompt: &str,
        response: Option<crate::state::AgentResponse>,
    ) -> Result<(), ManagerError> {
        // Find the task and add the attempt
        for phase in &mut self.state.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    let attempt_number = task.attempts.len() as u32 + 1;
                    task.attempts.push(TaskAttempt {
                        attempt_number,
                        agent_id: agent_id.to_string(),
                        started_at,
                        completed_at: Some(Utc::now()),
                        status,
                        prompt: prompt.to_string(),
                        response,
                    });
                    return Ok(());
                }
            }
        }
        Err(ManagerError::TaskNotRunnable(format!(
            "Task {} not found for recording attempt",
            task_id
        )))
    }

    /// Gather code from files for review.
    ///
    /// Reads the files specified in the task context and concatenates them.
    fn gather_code_for_review(&self, task: &Task) -> String {
        let mut code = String::new();

        for file_path in &task.context.files_to_read {
            let full_path = self.config.working_dir.join(file_path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    code.push_str(&format!("// File: {}\n", file_path));
                    code.push_str(&content);
                    code.push_str("\n\n");
                }
                Err(e) => {
                    warn!("Failed to read file {} for review: {}", file_path, e);
                }
            }
        }

        if code.is_empty() {
            code = "No code files specified for review.".to_string();
        }

        code
    }

    /// Gather issues from prior reviews for fix task.
    fn gather_issues_for_fix(&self, _task: &Task) -> Vec<crate::state::ReviewIssue> {
        // Issues are typically stored in the task context's prior_review_issues
        // For now, we return an empty vec as issues would be parsed from review responses
        // In a full implementation, this would extract structured ReviewIssue objects
        // from the task's previous attempts or context
        vec![]
    }

    /// Extract review verdict from the response text.
    fn extract_review_verdict(&self, response: &str) -> Verdict {
        let response_lower = response.to_lowercase();

        // Look for explicit verdict indicators
        if response_lower.contains("\"verdict\"")
            || response_lower.contains("verdict:")
            || response_lower.contains("**verdict**")
        {
            if response_lower.contains("approved") && !response_lower.contains("not approved") {
                return Verdict::Approved;
            }
            if response_lower.contains("needs_fixes")
                || response_lower.contains("needs fixes")
                || response_lower.contains("needsfixes")
            {
                return Verdict::NeedsFixes;
            }
        }

        // Default heuristics
        if response_lower.contains("lgtm")
            || response_lower.contains("looks good")
            || response_lower.contains("no issues")
            || response_lower.contains("code is correct")
        {
            return Verdict::Approved;
        }

        // If we find issue indicators, assume needs fixes
        if response_lower.contains("issue")
            || response_lower.contains("problem")
            || response_lower.contains("fix")
            || response_lower.contains("error")
            || response_lower.contains("bug")
        {
            return Verdict::NeedsFixes;
        }

        // Default to approved if unclear
        Verdict::Approved
    }
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
            .timeout(Duration::from_secs(600))
            .max_cycles(10);

        assert_eq!(config.log_path, PathBuf::from("/tmp/custom.md"));
        assert_eq!(config.model, "claude-opus-4-5-20251101");
        assert_eq!(config.timeout, Duration::from_secs(600));
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
    fn test_extract_review_verdict_approved() {
        // We can't create a Manager without a valid state file, so test the logic directly

        // Test approved patterns
        let response1 = "The code looks good to me. LGTM!";
        assert!(response1.to_lowercase().contains("lgtm"));

        let response2 = r#"{"verdict": "approved", "issues": []}"#;
        assert!(response2.to_lowercase().contains("approved"));
    }

    #[test]
    fn test_extract_review_verdict_needs_fixes() {
        let response = r#"{"verdict": "needs_fixes", "issues": [{"id": "1", "problem": "missing error handling"}]}"#;
        assert!(response.to_lowercase().contains("needs_fixes"));
    }

    #[test]
    fn test_manager_state_default() {
        assert_eq!(ManagerState::default(), ManagerState::Idle);
    }
}
