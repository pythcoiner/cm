//! LogRecord types for structured log storage.
//!
//! This module provides types for storing log entries as structured data
//! in tasks.json. These records are the source of truth for LOG.md generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single structured log record.
///
/// LogRecords are stored in TasksState and used to deterministically
/// generate LOG.md content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// Unique identifier for this record.
    pub id: String,
    /// When this record was created.
    pub timestamp: DateTime<Utc>,
    /// The action being logged.
    pub action: LogAction,
    /// ID of the phase this record relates to (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    /// ID of the task this record relates to (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// ID of the agent this record relates to (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Data payload for this record.
    pub data: LogData,
}

/// Actions that can be recorded in a log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

/// Data payload for log records.
///
/// Each variant corresponds to a LogAction and contains relevant data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum LogData {
    /// Data for PhaseStart action.
    PhaseStart {
        /// Name of the phase.
        name: String,
    },
    /// Data for TaskStart action.
    TaskStart {
        /// Name of the task.
        name: String,
    },
    /// Data for AgentSpawn action.
    AgentSpawn {
        /// Type of agent (implem, review, fix).
        agent_type: String,
        /// Preview of the prompt (truncated).
        prompt_preview: String,
    },
    /// Data for AgentComplete action.
    AgentComplete {
        /// Exit code of the agent process.
        exit_code: i32,
        /// Duration in seconds.
        duration_secs: u64,
    },
    /// Data for BuildResult action.
    BuildResult {
        /// Whether the build succeeded.
        success: bool,
        /// Error messages.
        errors: Vec<String>,
        /// Warning messages.
        warnings: Vec<String>,
    },
    /// Data for ReviewResult action.
    ReviewResult {
        /// Review verdict (approved, needs_fixes).
        verdict: String,
        /// Number of issues found.
        issues_count: usize,
    },
    /// Data for TaskComplete action.
    TaskComplete {
        /// Final status of the task.
        status: String,
    },
    /// Data for TaskDeferred action.
    TaskDeferred {
        /// Reason for deferral.
        reason: String,
    },
    /// Data for Error action.
    Error {
        /// Error message.
        message: String,
    },
    /// Data for Shutdown action.
    Shutdown {
        /// Reason for shutdown.
        reason: String,
    },
}

impl LogRecord {
    /// Create a new log record with generated ID and current timestamp.
    ///
    /// # Arguments
    ///
    /// * `action` - The action being logged
    /// * `data` - The data payload for this record
    pub fn new(action: LogAction, data: LogData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            action,
            phase_id: None,
            task_id: None,
            agent_id: None,
            data,
        }
    }

    /// Set the phase ID for this record.
    pub fn with_phase_id(mut self, phase_id: String) -> Self {
        self.phase_id = Some(phase_id);
        self
    }

    /// Set the task ID for this record.
    pub fn with_task_id(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Set the agent ID for this record.
    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_record_new() {
        let record = LogRecord::new(
            LogAction::PhaseStart,
            LogData::PhaseStart {
                name: "Phase 1".to_string(),
            },
        );

        assert!(!record.id.is_empty());
        assert_eq!(record.action, LogAction::PhaseStart);
        assert!(record.phase_id.is_none());
        assert!(record.task_id.is_none());
        assert!(record.agent_id.is_none());
    }

    #[test]
    fn test_log_record_builder() {
        let record = LogRecord::new(
            LogAction::AgentSpawn,
            LogData::AgentSpawn {
                agent_type: "implem".to_string(),
                prompt_preview: "Implement feature X".to_string(),
            },
        )
        .with_phase_id("phase-1".to_string())
        .with_task_id("task-1".to_string())
        .with_agent_id("agent-123".to_string());

        assert_eq!(record.phase_id, Some("phase-1".to_string()));
        assert_eq!(record.task_id, Some("task-1".to_string()));
        assert_eq!(record.agent_id, Some("agent-123".to_string()));
    }

    #[test]
    fn test_log_action_serialization() {
        let action = LogAction::BuildResult;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"build_result\"");

        let deserialized: LogAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LogAction::BuildResult);
    }

    #[test]
    fn test_log_data_serialization() {
        let data = LogData::BuildResult {
            success: true,
            errors: vec![],
            warnings: vec!["unused variable".to_string()],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"type\":\"build_result\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"warnings\""));

        let deserialized: LogData = serde_json::from_str(&json).unwrap();
        match deserialized {
            LogData::BuildResult {
                success,
                errors,
                warnings,
            } => {
                assert!(success);
                assert!(errors.is_empty());
                assert_eq!(warnings.len(), 1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_log_record_round_trip() {
        let record = LogRecord::new(
            LogAction::TaskComplete,
            LogData::TaskComplete {
                status: "completed".to_string(),
            },
        )
        .with_task_id("phase-1.task-1".to_string());

        let json = serde_json::to_string_pretty(&record).unwrap();
        let deserialized: LogRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, record.id);
        assert_eq!(deserialized.action, record.action);
        assert_eq!(deserialized.task_id, record.task_id);
    }
}
