//! Per-phase detailed logging.
//!
//! This module provides functionality for the manager (cm) to log detailed information
//! about task execution on a per-phase basis. Logs include agent prompts, responses,
//! build results, and review verdicts. Only the manager writes logs - agents never touch them directly.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::build::BuildOutput;
use crate::state::{
    AgentResponse, AgentType, LogAction as StateLogAction, LogData, LogRecord, Phase, ReviewIssue,
    Verdict,
};

mod phase_logger;
mod tee_writer;

pub use phase_logger::{parse_log_timestamp, PhaseLogError, PhaseLogger, PruneStats};
pub use tee_writer::{init_log_file, tee_eprintln, tee_print, tee_println, TeeWriter};

/// Maximum length for prompts and responses before truncation.
const MAX_CONTENT_LENGTH: usize = 2000;

/// Errors that can occur during log operations.
#[derive(Debug, Error)]
pub enum LogError {
    /// An I/O error occurred while writing to the log file.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// A formatting error occurred while creating a log entry.
    #[error("format error: {0}")]
    FormatError(String),
}

/// Actions that can be logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogAction {
    /// A phase has started.
    PhaseStart,
    /// A task has started.
    TaskStart,
    /// An agent has been spawned.
    AgentSpawn,
    /// An agent has completed its work.
    AgentComplete,
    /// Build verification result.
    BuildResult,
    /// Code review result.
    ReviewResult,
    /// A task has been completed.
    TaskComplete,
    /// A task has been deferred.
    TaskDeferred,
    /// An error occurred.
    Error,
    /// A graceful shutdown was initiated.
    Shutdown,
}

impl std::fmt::Display for LogAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogAction::PhaseStart => write!(f, "PHASE_START"),
            LogAction::TaskStart => write!(f, "TASK_START"),
            LogAction::AgentSpawn => write!(f, "AGENT_SPAWN"),
            LogAction::AgentComplete => write!(f, "AGENT_COMPLETE"),
            LogAction::BuildResult => write!(f, "BUILD_RESULT"),
            LogAction::ReviewResult => write!(f, "REVIEW_RESULT"),
            LogAction::TaskComplete => write!(f, "TASK_COMPLETE"),
            LogAction::TaskDeferred => write!(f, "TASK_DEFERRED"),
            LogAction::Error => write!(f, "ERROR"),
            LogAction::Shutdown => write!(f, "SHUTDOWN"),
        }
    }
}

/// A single log entry to be appended to LOG.md.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp when the entry was created.
    pub timestamp: DateTime<Utc>,
    /// The phase this entry relates to (if any).
    pub phase: Option<String>,
    /// The task this entry relates to (if any).
    pub task: Option<String>,
    /// The agent ID this entry relates to (if any).
    pub agent_id: Option<String>,
    /// The action being logged.
    pub action: LogAction,
    /// Details about the action.
    pub details: String,
}

impl LogEntry {
    /// Create a new log entry with the current timestamp.
    pub fn new(action: LogAction, details: String) -> Self {
        Self {
            timestamp: Utc::now(),
            phase: None,
            task: None,
            agent_id: None,
            action,
            details,
        }
    }

    /// Set the phase for this entry.
    pub fn with_phase(mut self, phase: String) -> Self {
        self.phase = Some(phase);
        self
    }

    /// Set the task for this entry.
    pub fn with_task(mut self, task: String) -> Self {
        self.task = Some(task);
        self
    }

    /// Set the agent ID for this entry.
    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

/// Manages the cm.log file.
///
/// The `LogManager` is responsible for appending entries to `.cm/logs/cm.log`.
/// It ensures that the log is append-only and properly formatted.
pub struct LogManager {
    /// Path to the cm.log file.
    path: PathBuf,
}

impl LogManager {
    /// Create a new `LogManager` for the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the cm.log file
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Append an entry to the log file.
    ///
    /// Creates the file if it doesn't exist. Always appends to the end of the file.
    ///
    /// # Arguments
    ///
    /// * `entry` - The log entry to append
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened or created
    /// - Writing to the file fails
    pub fn append_entry(&mut self, entry: &LogEntry) -> Result<(), LogError> {
        let formatted = Self::format_entry(entry);
        self.append_raw(&formatted)
    }

    /// Append raw markdown content to the log file.
    ///
    /// # Arguments
    ///
    /// * `content` - The raw markdown content to append
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the file fails.
    fn append_raw(&mut self, content: &str) -> Result<(), LogError> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;

        writeln!(file, "{content}")?;
        Ok(())
    }

    /// Format a log entry as markdown.
    fn format_entry(entry: &LogEntry) -> String {
        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
        let mut parts = vec![format!("[{}] **{}**", timestamp, entry.action)];

        if let Some(ref phase) = entry.phase {
            parts.push(format!("Phase: {phase}"));
        }

        if let Some(ref task) = entry.task {
            parts.push(format!("Task: {task}"));
        }

        if let Some(ref agent_id) = entry.agent_id {
            parts.push(format!("Agent: {agent_id}"));
        }

        format!("{}\n\n{}\n", parts.join(" | "), entry.details)
    }

    // -------------------------------------------------------------------------
    // Formatting functions for specific log types
    // -------------------------------------------------------------------------

    /// Format a phase start entry.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase that is starting
    pub fn format_phase_start(phase: &Phase) -> String {
        format!(
            "## Phase: {}\n\n**ID:** {}\n**Status:** {:?}\n**Tasks:** {}",
            phase.name,
            phase.id,
            phase.status,
            phase.tasks.len()
        )
    }

    /// Format an agent spawn entry.
    ///
    /// # Arguments
    ///
    /// * `agent_type` - The type of agent being spawned
    /// * `task_id` - The ID of the task the agent is working on
    /// * `prompt` - The prompt given to the agent (will be truncated if too long)
    pub fn format_agent_spawn(agent_type: &AgentType, task_id: &str, prompt: &str) -> String {
        let truncated_prompt = truncate_content(prompt, MAX_CONTENT_LENGTH);
        format!(
            "### Agent Spawn\n\n**Type:** {agent_type:?}\n**Task:** {task_id}\n\n<details>\n<summary>Prompt</summary>\n\n```\n{truncated_prompt}\n```\n\n</details>"
        )
    }

    /// Format an agent response entry.
    ///
    /// # Arguments
    ///
    /// * `response` - The response from the agent
    pub fn format_agent_response(response: &AgentResponse) -> String {
        let truncated_response = truncate_content(&response.message, MAX_CONTENT_LENGTH);

        let mut output = String::from("### Agent Response\n\n");

        if !response.files_created.is_empty() {
            output.push_str("**Files Created:**\n");
            for file in &response.files_created {
                output.push_str(&format!("- `{file}`\n"));
            }
            output.push('\n');
        }

        if !response.files_modified.is_empty() {
            output.push_str("**Files Modified:**\n");
            for file in &response.files_modified {
                output.push_str(&format!("- `{file}`\n"));
            }
            output.push('\n');
        }

        if !response.commands_run.is_empty() {
            output.push_str("**Commands Run:**\n");
            for cmd in &response.commands_run {
                output.push_str(&format!("- `{cmd}`\n"));
            }
            output.push('\n');
        }

        output.push_str(&format!(
            "<details>\n<summary>Raw Response</summary>\n\n```\n{truncated_response}\n```\n\n</details>"
        ));

        output
    }

    /// Format a build result entry.
    ///
    /// # Arguments
    ///
    /// * `output` - The output from the build command
    pub fn format_build_result(output: &BuildOutput) -> String {
        let status = if output.success { "PASS" } else { "FAIL" };
        let mut result = format!("### Build Result: {status}\n\n");

        if !output.errors.is_empty() {
            result.push_str("**Errors:**\n");
            for error in &output.errors {
                if let Some(ref loc) = error.location {
                    result.push_str(&format!("- `{}`: {}\n", loc, error.message));
                } else {
                    result.push_str(&format!("- {}\n", error.message));
                }
            }
            result.push('\n');
        }

        if !output.warnings.is_empty() {
            result.push_str("**Warnings:**\n");
            for warning in &output.warnings {
                if let Some(ref loc) = warning.location {
                    result.push_str(&format!("- `{}`: {}\n", loc, warning.message));
                } else {
                    result.push_str(&format!("- {}\n", warning.message));
                }
            }
            result.push('\n');
        }

        if !output.success {
            let truncated_stderr = truncate_content(&output.stderr, MAX_CONTENT_LENGTH / 2);
            result.push_str(&format!(
                "<details>\n<summary>Stderr</summary>\n\n```\n{truncated_stderr}\n```\n\n</details>"
            ));
        }

        result
    }

    /// Format a review result entry.
    ///
    /// # Arguments
    ///
    /// * `verdict` - The review verdict
    /// * `issues` - List of issues found during review
    pub fn format_review_result(verdict: &Verdict, issues: &[ReviewIssue]) -> String {
        let verdict_str = match verdict {
            Verdict::Approved => "APPROVED",
            Verdict::NeedsFixes => "NEEDS_FIXES",
        };

        let mut result = format!("### Review Result: {verdict_str}\n\n");

        if issues.is_empty() {
            result.push_str("No issues found.\n");
        } else {
            result.push_str(&format!("**Issues ({}):**\n\n", issues.len()));
            for issue in issues {
                let resolved = if issue.resolved { " [RESOLVED]" } else { "" };
                result.push_str(&format!(
                    "- **[{}]** `{}`: {}{}\n  - *Fix:* {}\n\n",
                    format!("{:?}", issue.severity).to_uppercase(),
                    issue.location,
                    issue.problem,
                    resolved,
                    issue.suggested_fix
                ));
            }
        }

        result
    }

    // -------------------------------------------------------------------------
    // Helper methods for common logging operations
    // -------------------------------------------------------------------------

    /// Log the start of a phase.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase that is starting
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_phase_start(&mut self, phase: &Phase) -> Result<(), LogError> {
        let details = Self::format_phase_start(phase);
        let entry = LogEntry::new(LogAction::PhaseStart, details).with_phase(phase.id.clone());
        self.append_entry(&entry)
    }

    /// Log the spawning of an agent.
    ///
    /// # Arguments
    ///
    /// * `agent_type` - The type of agent being spawned
    /// * `task_id` - The ID of the task the agent is working on
    /// * `prompt` - The prompt given to the agent
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_agent_spawn(
        &mut self,
        agent_type: &AgentType,
        task_id: &str,
        prompt: &str,
    ) -> Result<(), LogError> {
        let details = Self::format_agent_spawn(agent_type, task_id, prompt);
        let entry = LogEntry::new(LogAction::AgentSpawn, details).with_task(task_id.to_string());
        self.append_entry(&entry)
    }

    /// Log an agent's response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response from the agent
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_agent_response(&mut self, response: &AgentResponse) -> Result<(), LogError> {
        let details = Self::format_agent_response(response);
        let entry = LogEntry::new(LogAction::AgentComplete, details);
        self.append_entry(&entry)
    }

    /// Log a build result.
    ///
    /// # Arguments
    ///
    /// * `output` - The output from the build command
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_build_result(&mut self, output: &BuildOutput) -> Result<(), LogError> {
        let details = Self::format_build_result(output);
        let entry = LogEntry::new(LogAction::BuildResult, details);
        self.append_entry(&entry)
    }

    /// Log a review result.
    ///
    /// # Arguments
    ///
    /// * `verdict` - The review verdict
    /// * `issues` - List of issues found during review
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_review_result(
        &mut self,
        verdict: &Verdict,
        issues: &[ReviewIssue],
    ) -> Result<(), LogError> {
        let details = Self::format_review_result(verdict, issues);
        let entry = LogEntry::new(LogAction::ReviewResult, details);
        self.append_entry(&entry)
    }

    /// Log the completion of a task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the completed task
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_task_complete(&mut self, task_id: &str) -> Result<(), LogError> {
        let details = format!("Task `{task_id}` completed successfully.");
        let entry =
            LogEntry::new(LogAction::TaskComplete, details).with_task(task_id.to_string());
        self.append_entry(&entry)
    }

    /// Log that a task was deferred.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the deferred task
    /// * `reason` - The reason for deferring the task
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_task_deferred(&mut self, task_id: &str, reason: &str) -> Result<(), LogError> {
        let details = format!("Task `{task_id}` deferred.\n\n**Reason:** {reason}");
        let entry =
            LogEntry::new(LogAction::TaskDeferred, details).with_task(task_id.to_string());
        self.append_entry(&entry)
    }

    /// Log an error.
    ///
    /// # Arguments
    ///
    /// * `error` - Description of the error
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_error(&mut self, error: &str) -> Result<(), LogError> {
        let details = format!("**Error:** {error}");
        let entry = LogEntry::new(LogAction::Error, details);
        self.append_entry(&entry)
    }

    /// Log a graceful shutdown event.
    ///
    /// This records that a shutdown signal was received and the manager
    /// is stopping gracefully. Optionally includes information about
    /// the current phase/task that was interrupted.
    ///
    /// # Arguments
    ///
    /// * `current_phase` - The ID of the phase that was active during shutdown (if any)
    /// * `current_task` - The ID of the task that was active during shutdown (if any)
    /// * `reason` - Optional reason for the shutdown
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the log file fails.
    pub fn log_shutdown(
        &mut self,
        current_phase: Option<&str>,
        current_task: Option<&str>,
        reason: Option<&str>,
    ) -> Result<(), LogError> {
        let mut details = String::from("### Graceful Shutdown\n\n");

        details.push_str("**Status:** Manager shutting down gracefully.\n\n");

        if let Some(phase) = current_phase {
            details.push_str(&format!("**Interrupted Phase:** {phase}\n"));
        }

        if let Some(task) = current_task {
            details.push_str(&format!("**Interrupted Task:** {task}\n"));
        }

        if let Some(r) = reason {
            details.push_str(&format!("\n**Reason:** {r}\n"));
        }

        details.push_str(
            "\nExecution can be resumed by running the manager again. \
             The state has been saved and any in-progress tasks will be retried.",
        );

        let mut entry = LogEntry::new(LogAction::Shutdown, details);

        if let Some(phase) = current_phase {
            entry = entry.with_phase(phase.to_string());
        }

        if let Some(task) = current_task {
            entry = entry.with_task(task.to_string());
        }

        self.append_entry(&entry)
    }

    // -------------------------------------------------------------------------
    // LogRecord creation methods
    //
    // These methods create LogRecord objects that can be stored in TasksState
    // for deterministic LOG.md generation.
    // -------------------------------------------------------------------------

    /// Create a LogRecord for a phase start event.
    pub fn create_phase_start_record(phase: &Phase) -> LogRecord {
        LogRecord::new(
            StateLogAction::PhaseStart,
            LogData::PhaseStart {
                name: phase.name.clone(),
            },
        )
        .with_phase_id(phase.id.clone())
    }

    /// Create a LogRecord for an agent spawn event.
    pub fn create_agent_spawn_record(
        agent_type: &AgentType,
        task_id: &str,
        prompt: &str,
    ) -> LogRecord {
        let prompt_preview = truncate_content(prompt, MAX_CONTENT_LENGTH);
        LogRecord::new(
            StateLogAction::AgentSpawn,
            LogData::AgentSpawn {
                agent_type: format!("{agent_type:?}").to_lowercase(),
                prompt_preview,
            },
        )
        .with_task_id(task_id.to_string())
    }

    /// Create a LogRecord for an agent complete event.
    pub fn create_agent_complete_record(
        task_id: &str,
        agent_id: &str,
        exit_code: i32,
        duration_secs: u64,
    ) -> LogRecord {
        LogRecord::new(
            StateLogAction::AgentComplete,
            LogData::AgentComplete {
                exit_code,
                duration_secs,
            },
        )
        .with_task_id(task_id.to_string())
        .with_agent_id(agent_id.to_string())
    }

    /// Create a LogRecord for a build result event.
    pub fn create_build_result_record(output: &BuildOutput) -> LogRecord {
        let errors: Vec<String> = output
            .errors
            .iter()
            .map(|e| {
                if let Some(ref loc) = e.location {
                    format!("{}: {}", loc, e.message)
                } else {
                    e.message.clone()
                }
            })
            .collect();

        let warnings: Vec<String> = output
            .warnings
            .iter()
            .map(|w| {
                if let Some(ref loc) = w.location {
                    format!("{}: {}", loc, w.message)
                } else {
                    w.message.clone()
                }
            })
            .collect();

        LogRecord::new(
            StateLogAction::BuildResult,
            LogData::BuildResult {
                success: output.success,
                errors,
                warnings,
            },
        )
    }

    /// Create a LogRecord for a review result event.
    pub fn create_review_result_record(verdict: &Verdict, issues: &[ReviewIssue]) -> LogRecord {
        let verdict_str = match verdict {
            Verdict::Approved => "approved".to_string(),
            Verdict::NeedsFixes => "needs_fixes".to_string(),
        };

        LogRecord::new(
            StateLogAction::ReviewResult,
            LogData::ReviewResult {
                verdict: verdict_str,
                issues_count: issues.len(),
            },
        )
    }

    /// Create a LogRecord for a task complete event.
    pub fn create_task_complete_record(task_id: &str) -> LogRecord {
        LogRecord::new(
            StateLogAction::TaskComplete,
            LogData::TaskComplete {
                status: "completed".to_string(),
            },
        )
        .with_task_id(task_id.to_string())
    }

    /// Create a LogRecord for a task deferred event.
    pub fn create_task_deferred_record(task_id: &str, reason: &str) -> LogRecord {
        LogRecord::new(
            StateLogAction::TaskDeferred,
            LogData::TaskDeferred {
                reason: reason.to_string(),
            },
        )
        .with_task_id(task_id.to_string())
    }

    /// Create a LogRecord for an error event.
    pub fn create_error_record(message: &str) -> LogRecord {
        LogRecord::new(
            StateLogAction::Error,
            LogData::Error {
                message: message.to_string(),
            },
        )
    }

    /// Create a LogRecord for a shutdown event.
    pub fn create_shutdown_record(
        current_phase: Option<&str>,
        current_task: Option<&str>,
        reason: Option<&str>,
    ) -> LogRecord {
        let reason_str = reason.unwrap_or("User requested shutdown").to_string();
        let mut record = LogRecord::new(
            StateLogAction::Shutdown,
            LogData::Shutdown { reason: reason_str },
        );

        if let Some(phase) = current_phase {
            record = record.with_phase_id(phase.to_string());
        }

        if let Some(task) = current_task {
            record = record.with_task_id(task.to_string());
        }

        record
    }
}

/// Truncate content to a maximum length, adding an indicator if truncated.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated = &content[..max_len];
        format!("{}\n\n... [truncated, {} more bytes]", truncated, content.len() - max_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::CompilerMessage;
    use crate::build::MessageLevel;
    use crate::state::{AgentStatus, PhaseStatus, Severity, Task, TaskContext, TaskStatus, TaskType};
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_log_action_display() {
        assert_eq!(LogAction::PhaseStart.to_string(), "PHASE_START");
        assert_eq!(LogAction::TaskStart.to_string(), "TASK_START");
        assert_eq!(LogAction::AgentSpawn.to_string(), "AGENT_SPAWN");
        assert_eq!(LogAction::AgentComplete.to_string(), "AGENT_COMPLETE");
        assert_eq!(LogAction::BuildResult.to_string(), "BUILD_RESULT");
        assert_eq!(LogAction::ReviewResult.to_string(), "REVIEW_RESULT");
        assert_eq!(LogAction::TaskComplete.to_string(), "TASK_COMPLETE");
        assert_eq!(LogAction::TaskDeferred.to_string(), "TASK_DEFERRED");
        assert_eq!(LogAction::Error.to_string(), "ERROR");
        assert_eq!(LogAction::Shutdown.to_string(), "SHUTDOWN");
    }

    #[test]
    fn test_log_entry_builder() {
        let entry = LogEntry::new(LogAction::TaskStart, "Starting task".to_string())
            .with_phase("phase-1".to_string())
            .with_task("task-1".to_string())
            .with_agent_id("agent-123".to_string());

        assert_eq!(entry.phase, Some("phase-1".to_string()));
        assert_eq!(entry.task, Some("task-1".to_string()));
        assert_eq!(entry.agent_id, Some("agent-123".to_string()));
        assert_eq!(entry.action, LogAction::TaskStart);
        assert_eq!(entry.details, "Starting task");
    }

    #[test]
    fn test_append_entry() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        let entry = LogEntry::new(LogAction::TaskStart, "Test entry".to_string());
        manager.append_entry(&entry).unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("TASK_START"));
        assert!(content.contains("Test entry"));
    }

    #[test]
    fn test_append_multiple_entries() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        let entry1 = LogEntry::new(LogAction::PhaseStart, "Phase 1".to_string());
        let entry2 = LogEntry::new(LogAction::TaskStart, "Task 1".to_string());

        manager.append_entry(&entry1).unwrap();
        manager.append_entry(&entry2).unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("PHASE_START"));
        assert!(content.contains("TASK_START"));
        assert!(content.contains("Phase 1"));
        assert!(content.contains("Task 1"));
    }

    #[test]
    fn test_format_phase_start() {
        let phase = Phase {
            id: "phase-1".to_string(),
            name: "Setup Phase".to_string(),
            plan: String::new(),
            status: PhaseStatus::InProgress,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
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
                plan_file: ".cm/plans/plan-log-task.md".to_string(),
                attempts: vec![],
                roadmap_item_id: None,
                implem_completed_at: None,
                baseline_commit: None,
                review_cycles_completed: 0,
            }],
        };

        let formatted = LogManager::format_phase_start(&phase);

        assert!(formatted.contains("## Phase: Setup Phase"));
        assert!(formatted.contains("**ID:** phase-1"));
        assert!(formatted.contains("**Tasks:** 1"));
    }

    #[test]
    fn test_format_agent_spawn() {
        let formatted =
            LogManager::format_agent_spawn(&AgentType::Implem, "task-1", "Implement feature X");

        assert!(formatted.contains("### Agent Spawn"));
        assert!(formatted.contains("**Type:** Implem"));
        assert!(formatted.contains("**Task:** task-1"));
        assert!(formatted.contains("Implement feature X"));
    }

    #[test]
    fn test_format_agent_spawn_truncates_long_prompt() {
        let long_prompt = "x".repeat(3000);
        let formatted = LogManager::format_agent_spawn(&AgentType::Implem, "task-1", &long_prompt);

        assert!(formatted.contains("[truncated"));
        assert!(formatted.len() < long_prompt.len() + 500);
    }

    #[test]
    fn test_format_agent_response() {
        let response = AgentResponse {
            status: AgentStatus::Success,
            files_created: vec!["src/new.rs".to_string()],
            files_modified: vec!["src/lib.rs".to_string()],
            commands_run: vec!["cargo build".to_string()],
            message: "Agent completed the task".to_string(),
        };

        let formatted = LogManager::format_agent_response(&response);

        assert!(formatted.contains("### Agent Response"));
        assert!(formatted.contains("**Files Created:**"));
        assert!(formatted.contains("`src/new.rs`"));
        assert!(formatted.contains("**Files Modified:**"));
        assert!(formatted.contains("`src/lib.rs`"));
        assert!(formatted.contains("**Commands Run:**"));
        assert!(formatted.contains("`cargo build`"));
        assert!(formatted.contains("Agent completed the task"));
    }

    #[test]
    fn test_format_build_result_success() {
        let output = BuildOutput {
            success: true,
            errors: vec![],
            warnings: vec![],
            stdout: "Compiling...".to_string(),
            stderr: "".to_string(),
        };

        let formatted = LogManager::format_build_result(&output);

        assert!(formatted.contains("### Build Result: PASS"));
    }

    #[test]
    fn test_format_build_result_failure() {
        let output = BuildOutput {
            success: false,
            errors: vec![CompilerMessage {
                level: MessageLevel::Error,
                message: "unresolved import".to_string(),
                location: Some("src/lib.rs:1:5".to_string()),
            }],
            warnings: vec![],
            stdout: "".to_string(),
            stderr: "error[E0432]: unresolved import".to_string(),
        };

        let formatted = LogManager::format_build_result(&output);

        assert!(formatted.contains("### Build Result: FAIL"));
        assert!(formatted.contains("**Errors:**"));
        assert!(formatted.contains("`src/lib.rs:1:5`"));
        assert!(formatted.contains("unresolved import"));
    }

    #[test]
    fn test_format_review_result_approved() {
        let formatted = LogManager::format_review_result(&Verdict::Approved, &[]);

        assert!(formatted.contains("### Review Result: APPROVED"));
        assert!(formatted.contains("No issues found."));
    }

    #[test]
    fn test_format_review_result_needs_fixes() {
        let issues = vec![ReviewIssue {
            id: "issue-1".to_string(),
            severity: Severity::High,
            location: "src/lib.rs:10".to_string(),
            problem: "Missing error handling".to_string(),
            suggested_fix: "Add Result return type".to_string(),
            resolved: false,
        }];

        let formatted = LogManager::format_review_result(&Verdict::NeedsFixes, &issues);

        assert!(formatted.contains("### Review Result: NEEDS_FIXES"));
        assert!(formatted.contains("**Issues (1):**"));
        assert!(formatted.contains("[HIGH]"));
        assert!(formatted.contains("Missing error handling"));
        assert!(formatted.contains("Add Result return type"));
    }

    #[test]
    fn test_truncate_content() {
        let short = "short content";
        assert_eq!(truncate_content(short, 100), short);

        let long = "x".repeat(200);
        let truncated = truncate_content(&long, 100);
        assert!(truncated.contains("[truncated"));
        assert!(truncated.contains("100 more bytes"));
    }

    #[test]
    fn test_log_task_complete() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        manager.log_task_complete("task-1").unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("TASK_COMPLETE"));
        assert!(content.contains("task-1"));
        assert!(content.contains("completed successfully"));
    }

    #[test]
    fn test_log_task_deferred() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        manager
            .log_task_deferred("task-1", "Too many failed attempts")
            .unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("TASK_DEFERRED"));
        assert!(content.contains("task-1"));
        assert!(content.contains("Too many failed attempts"));
    }

    #[test]
    fn test_log_error() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        manager.log_error("Something went wrong").unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("ERROR"));
        assert!(content.contains("Something went wrong"));
    }

    #[test]
    fn test_log_error_type_display() {
        let io_error = LogError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_error.to_string().contains("io error"));

        let format_error = LogError::FormatError("invalid format".to_string());
        assert!(format_error.to_string().contains("format error"));
        assert!(format_error.to_string().contains("invalid format"));
    }

    #[test]
    fn test_log_shutdown() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        manager
            .log_shutdown(
                Some("phase-1"),
                Some("task-1"),
                Some("Received SIGINT"),
            )
            .unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("SHUTDOWN"));
        assert!(content.contains("Graceful Shutdown"));
        assert!(content.contains("phase-1"));
        assert!(content.contains("task-1"));
        assert!(content.contains("Received SIGINT"));
        assert!(content.contains("resumed"));
    }

    #[test]
    fn test_log_shutdown_minimal() {
        let file = NamedTempFile::new().unwrap();
        let mut manager = LogManager::new(file.path().to_path_buf());

        manager.log_shutdown(None, None, None).unwrap();

        let mut content = String::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        assert!(content.contains("SHUTDOWN"));
        assert!(content.contains("Graceful Shutdown"));
        assert!(content.contains("Manager shutting down gracefully"));
    }
}
