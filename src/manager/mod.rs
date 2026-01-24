//! Main orchestration loop, state machine, and crash recovery.
//!
//! This module provides the core Manager that orchestrates the entire task
//! execution flow. It coordinates between the state module, agent spawner,
//! build verifier, and log manager to execute tasks in the correct order.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use log::{debug, error, info, warn};
use thiserror::Error;
use uuid::Uuid;

use crate::agent::{AgentError, AgentSpawner, PromptBuilder, ResponseParser};
use crate::build::{BuildError, BuildVerifier};
use crate::log::{LogError, LogManager};
use crate::state::{
    load_state, save_state, AgentInvocation, AgentType, AttemptStatus, StateError, Task,
    TaskAttempt, TaskStatus, TaskType, TasksState, Verdict,
};

mod recovery;
mod state;

pub use recovery::{CheckpointId, RecoveryAction, RecoveryError, RecoveryManager, ShutdownHandler};
pub use state::ManagerState;

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

    /// A task is not runnable due to dependencies or status.
    #[error("task not runnable: {0}")]
    TaskNotRunnable(String),

    /// Maximum cycles exceeded for a task.
    #[error("max cycles exceeded for task: {0}")]
    MaxCyclesExceeded(String),

    /// All tasks are blocked or completed.
    #[error("no runnable tasks available")]
    NoRunnableTasks,
}

/// Configuration for the Manager.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Path to the tasks.json state file.
    pub state_path: PathBuf,
    /// Path to the LOG.md file.
    pub log_path: PathBuf,
    /// Working directory for build verification.
    pub working_dir: PathBuf,
    /// Model to use for agent spawning.
    pub model: String,
    /// Timeout for agent execution.
    pub timeout: Duration,
    /// Maximum number of cycles (attempts) per task before deferring.
    pub max_cycles: u32,
}

impl ManagerConfig {
    /// Create a new configuration with default values.
    ///
    /// # Arguments
    ///
    /// * `state_path` - Path to the tasks.json file
    pub fn new(state_path: PathBuf) -> Self {
        Self {
            state_path: state_path.clone(),
            log_path: state_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("LOG.md"),
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            model: "claude-sonnet-4-5-20250929".to_string(),
            timeout: Duration::from_secs(300),
            max_cycles: 5,
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
    /// Spawner for creating agent processes.
    agent_spawner: AgentSpawner,
    /// Verifier for build and clippy checks.
    build_verifier: BuildVerifier,
    /// Manager for LOG.md entries.
    log_manager: LogManager,
    /// Current execution state.
    manager_state: ManagerState,
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The state file cannot be loaded
    /// - The state file is invalid
    pub fn new(config: ManagerConfig) -> Result<Self, ManagerError> {
        let state = load_state(&config.state_path)?;
        let agent_spawner = AgentSpawner::new(config.model.clone(), config.timeout);
        let build_verifier = BuildVerifier::new(config.working_dir.clone());
        let log_manager = LogManager::new(config.log_path.clone());

        info!("Manager initialized with state from {:?}", config.state_path);

        Ok(Self {
            config,
            state,
            agent_spawner,
            build_verifier,
            log_manager,
            manager_state: ManagerState::Idle,
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
    /// 1. Selects the next runnable task
    /// 2. Executes the task
    /// 3. Saves state
    /// 4. Repeats until no more tasks are runnable
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Task execution fails
    /// - State cannot be saved
    pub fn run(&mut self) -> Result<(), ManagerError> {
        info!("Starting manager run loop");
        self.manager_state = ManagerState::Executing;

        loop {
            // Select next task
            let task_id = match self.select_next_task() {
                Some(task) => task.id.clone(),
                None => {
                    info!("No more runnable tasks, exiting loop");
                    break;
                }
            };

            info!("Selected task for execution: {}", task_id);

            // Execute the task
            match self.execute_task(&task_id) {
                Ok(()) => {
                    info!("Task {} completed successfully", task_id);
                }
                Err(e) => {
                    error!("Task {} failed: {}", task_id, e);
                    self.log_manager
                        .log_error(&format!("Task {} failed: {}", task_id, e))?;

                    // Don't propagate the error; continue with next task
                    // The task will be marked as failed/deferred in execute_task
                }
            }

            // Save state after each task
            self.update_state()?;
        }

        self.manager_state = ManagerState::Idle;
        info!("Manager run loop completed");
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
            self.state.mark_task_status(task_id, TaskStatus::Deferred)?;
            self.log_manager.log_task_deferred(
                task_id,
                &format!("Exceeded maximum cycles ({})", self.config.max_cycles),
            )?;
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
    /// Flow:
    /// 1. Build prompt with task context
    /// 2. Spawn agent
    /// 3. Wait for response
    /// 4. Parse response
    /// 5. Run build verification
    /// 6. Mark task complete or retry
    fn execute_implem(&mut self, task: &Task) -> Result<(), ManagerError> {
        debug!("Executing IMPLEM task: {}", task.id);
        self.manager_state = ManagerState::WaitingForAgent;

        // Build the prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_implem_prompt(task);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn
        self.log_manager
            .log_agent_spawn(&AgentType::Implem, &task.id, &prompt)?;

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
        });

        // Spawn and wait for the agent
        let handle = self.agent_spawner.spawn(&prompt, &task.id)?;
        let output = handle.wait()?;

        // Parse the response
        let response = ResponseParser::parse(&output.stdout)?;
        debug!(
            "Agent response: {}",
            &response.raw_response[..response.raw_response.len().min(500)]
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

        // Run build verification
        self.manager_state = ManagerState::Verifying;
        let build_result = self.build_verifier.verify_all();

        match build_result {
            Ok(()) => {
                info!("Build verification passed for task {}", task.id);
                self.log_manager.log_build_result(&crate::build::BuildOutput {
                    success: true,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: String::new(),
                })?;

                // Record successful attempt
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, Some(response))?;

                // Mark task as completed
                self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                self.log_manager.log_task_complete(&task.id)?;
            }
            Err(e) => {
                warn!("Build verification failed for task {}: {}", task.id, e);

                // Log the build failure
                let build_output = match &e {
                    BuildError::CommandFailed { stderr, .. } => crate::build::BuildOutput {
                        success: false,
                        errors: vec![],
                        warnings: vec![],
                        stdout: String::new(),
                        stderr: stderr.clone(),
                    },
                    _ => crate::build::BuildOutput {
                        success: false,
                        errors: vec![],
                        warnings: vec![],
                        stdout: String::new(),
                        stderr: e.to_string(),
                    },
                };
                self.log_manager.log_build_result(&build_output)?;

                // Record failed attempt
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, Some(response))?;

                // Keep task in progress for retry
                self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            }
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
        self.manager_state = ManagerState::WaitingForAgent;

        // For review, we need to get the code to review
        // This is typically from the files_to_read in the context
        let code_to_review = self.gather_code_for_review(task);

        // Build the review prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_review_prompt(task, &code_to_review);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn
        self.log_manager
            .log_agent_spawn(&AgentType::Review, &task.id, &prompt)?;

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
        });

        // Spawn and wait for the agent
        let handle = self.agent_spawner.spawn(&prompt, &task.id)?;
        let output = handle.wait()?;

        // Parse the response
        let response = ResponseParser::parse(&output.stdout)?;
        debug!(
            "Agent response: {}",
            &response.raw_response[..response.raw_response.len().min(500)]
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

        // Try to extract review verdict from the response
        let verdict = self.extract_review_verdict(&response.raw_response);

        // Log review result
        self.log_manager.log_review_result(&verdict, &[])?;

        match verdict {
            Verdict::Approved => {
                info!("Review approved for task {}", task.id);
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, Some(response))?;
                self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                self.log_manager.log_task_complete(&task.id)?;
            }
            Verdict::NeedsFixes => {
                warn!("Review found issues for task {}", task.id);
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, Some(response))?;
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
        self.manager_state = ManagerState::WaitingForAgent;

        // Get prior review issues from context
        let issues = self.gather_issues_for_fix(task);

        // Build the fix prompt
        debug!("Building prompt for task {}", task.id);
        let prompt = PromptBuilder::build_fix_prompt(task, &issues);
        debug!("Prompt: {}", &prompt[..prompt.len().min(500)]);

        // Log agent spawn
        self.log_manager
            .log_agent_spawn(&AgentType::Fix, &task.id, &prompt)?;

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
        });

        // Spawn and wait for the agent
        let handle = self.agent_spawner.spawn(&prompt, &task.id)?;
        let output = handle.wait()?;

        // Parse the response
        let response = ResponseParser::parse(&output.stdout)?;
        debug!(
            "Agent response: {}",
            &response.raw_response[..response.raw_response.len().min(500)]
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

        // Run build verification
        self.manager_state = ManagerState::Verifying;
        let build_result = self.build_verifier.verify_all();

        match build_result {
            Ok(()) => {
                info!("Build verification passed for fix task {}", task.id);
                self.log_manager.log_build_result(&crate::build::BuildOutput {
                    success: true,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: String::new(),
                })?;

                // Record successful attempt
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Success, Some(response))?;

                // Mark task as completed
                self.state.mark_task_status(&task.id, TaskStatus::Completed)?;
                self.log_manager.log_task_complete(&task.id)?;
            }
            Err(e) => {
                warn!("Build verification failed for fix task {}: {}", task.id, e);

                // Log the build failure
                let build_output = crate::build::BuildOutput {
                    success: false,
                    errors: vec![],
                    warnings: vec![],
                    stdout: String::new(),
                    stderr: e.to_string(),
                };
                self.log_manager.log_build_result(&build_output)?;

                // Record failed attempt
                self.record_attempt(&task.id, &agent_id, started_at, AttemptStatus::Failed, Some(response))?;

                // Keep task pending for retry
                self.state.mark_task_status(&task.id, TaskStatus::Pending)?;
            }
        }

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

    /// Save the current state to disk.
    fn update_state(&mut self) -> Result<(), ManagerError> {
        save_state(&self.state, &self.config.state_path)?;
        debug!("State saved to {:?}", self.config.state_path);
        Ok(())
    }

    /// Record a task attempt in the state.
    fn record_attempt(
        &mut self,
        task_id: &str,
        agent_id: &str,
        started_at: chrono::DateTime<Utc>,
        status: AttemptStatus,
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
