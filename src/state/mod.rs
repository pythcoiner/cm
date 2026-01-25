//! State management for tasks.json.
//!
//! This module handles loading, saving, and manipulating the tasks.json state file
//! that tracks all phases, tasks, and agent invocations in a cm project.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

mod log_record;
mod roadmap;
mod tasks;
mod validate;

pub use log_record::{LogAction, LogData, LogRecord};
pub use roadmap::{
    load_roadmap, save_roadmap, RoadmapItem, RoadmapPhase, RoadmapState, RoadmapSubItem,
};
pub use tasks::{
    AgentInvocation, AgentResponse, AgentStatus, AgentType, AttemptStatus, GlobalContext, Phase,
    PhaseStatus, Project, ReviewIssue, ReviewResult, Severity, Task, TaskAttempt, TaskContext,
    TaskStatus, TaskType, TasksState, Verdict,
};
pub use validate::{
    validate_all, validate_cross_references, validate_roadmap_json, validate_tasks_json,
    SanityError, ValidationResult,
};

/// Errors that can occur during state operations.
#[derive(Debug, Error)]
pub enum StateError {
    /// The tasks.json file was not found at the specified path.
    #[error("tasks.json not found at {0}")]
    NotFound(PathBuf),

    /// Failed to parse the tasks.json file.
    #[error("invalid JSON: {0}")]
    ParseError(String),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The specified task was not found.
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// The specified phase was not found.
    #[error("phase not found: {0}")]
    PhaseNotFound(String),
}

/// Load the tasks state from a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the tasks.json file
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist (`StateError::NotFound`)
/// - The file cannot be read (`StateError::Io`)
/// - The JSON is invalid (`StateError::ParseError`)
pub fn load_state(path: &Path) -> Result<TasksState, StateError> {
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            StateError::NotFound(path.to_path_buf())
        } else {
            StateError::Io(e)
        }
    })?;

    serde_json::from_str(&content).map_err(|e| StateError::ParseError(e.to_string()))
}

/// Save the tasks state to a JSON file.
///
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `state` - The state to save
/// * `path` - Path to write the tasks.json file
///
/// # Errors
///
/// Returns an error if:
/// - Parent directories cannot be created (`StateError::Io`)
/// - The file cannot be written (`StateError::Io`)
/// - Serialization fails (`StateError::ParseError`)
pub fn save_state(state: &TasksState, path: &Path) -> Result<(), StateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content =
        serde_json::to_string_pretty(state).map_err(|e| StateError::ParseError(e.to_string()))?;

    fs::write(path, content)?;
    Ok(())
}

impl TasksState {
    /// Get the currently active phase, if any.
    pub fn current_phase(&self) -> Option<&Phase> {
        self.current_phase
            .as_ref()
            .and_then(|id| self.phases.iter().find(|p| &p.id == id))
    }

    /// Get the currently active task, if any.
    pub fn current_task(&self) -> Option<&Task> {
        self.current_task.as_ref().and_then(|task_id| {
            self.phases
                .iter()
                .flat_map(|p| &p.tasks)
                .find(|t| &t.id == task_id)
        })
    }

    /// Find the next task that can be run.
    ///
    /// A task is runnable if:
    /// - Its status is `Pending`
    /// - All tasks it depends on are `Completed`
    pub fn next_runnable_task(&self) -> Option<&Task> {
        self.phases
            .iter()
            .flat_map(|p| &p.tasks)
            .find(|t| t.status == TaskStatus::Pending && !self.is_task_blocked(&t.id))
    }

    /// Check if a task is blocked by uncompleted dependencies.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to check
    ///
    /// Returns `true` if any dependency is not completed, `false` otherwise.
    /// Returns `false` if the task is not found.
    pub fn is_task_blocked(&self, task_id: &str) -> bool {
        let task = self
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .find(|t| t.id == task_id);

        let Some(task) = task else {
            return false;
        };

        task.depends_on.iter().any(|dep_id| {
            let dep_task = self
                .phases
                .iter()
                .flat_map(|p| &p.tasks)
                .find(|t| &t.id == dep_id);

            match dep_task {
                Some(t) => t.status != TaskStatus::Completed,
                None => true, // Missing dependency counts as blocked
            }
        })
    }

    /// Update the status of a task.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The ID of the task to update
    /// * `status` - The new status
    ///
    /// # Errors
    ///
    /// Returns `StateError::TaskNotFound` if the task does not exist.
    pub fn mark_task_status(
        &mut self,
        task_id: &str,
        status: TaskStatus,
    ) -> Result<(), StateError> {
        for phase in &mut self.phases {
            for task in &mut phase.tasks {
                if task.id == task_id {
                    task.status = status;
                    return Ok(());
                }
            }
        }

        Err(StateError::TaskNotFound(task_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_state() -> TasksState {
        TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "test".to_string(),
                description: "test project".to_string(),
                created_at: None,
            },
            global_context: None,
            phases: vec![Phase {
                id: "phase-1".to_string(),
                name: "Phase 1".to_string(),
                status: PhaseStatus::InProgress,
                tasks: vec![
                    Task {
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
                    },
                    Task {
                        id: "task-2".to_string(),
                        name: "Task 2".to_string(),
                        task_type: TaskType::Implement,
                        status: TaskStatus::Pending,
                        depends_on: vec!["task-1".to_string()],
                        context: TaskContext {
                            files_to_read: vec![],
                            code_style_excerpt: None,
                            prior_review_issues: vec![],
                        },
                        instructions: "Do task 2".to_string(),
                        attempts: vec![],
                        roadmap_item_id: None,
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
                    },
                ],
            }],
            current_phase: Some("phase-1".to_string()),
            current_task: Some("task-1".to_string()),
            agent_history: vec![],
            log_records: vec![],
            interrupted_at: None,
        }
    }

    #[test]
    fn test_load_save_round_trip() {
        let state = create_test_state();
        let file = NamedTempFile::new().unwrap();

        save_state(&state, file.path()).unwrap();

        let loaded = load_state(file.path()).unwrap();

        assert_eq!(loaded.version, state.version);
        assert_eq!(loaded.project.name, state.project.name);
        assert_eq!(loaded.phases.len(), state.phases.len());
    }

    #[test]
    fn test_load_not_found() {
        let result = load_state(Path::new("/nonexistent/path/tasks.json"));
        assert!(matches!(result, Err(StateError::NotFound(_))));
    }

    #[test]
    fn test_load_invalid_json() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{{ invalid json }}").unwrap();

        let result = load_state(file.path());
        assert!(matches!(result, Err(StateError::ParseError(_))));
    }

    #[test]
    fn test_current_phase() {
        let state = create_test_state();
        let phase = state.current_phase().unwrap();
        assert_eq!(phase.id, "phase-1");
    }

    #[test]
    fn test_current_task() {
        let state = create_test_state();
        let task = state.current_task().unwrap();
        assert_eq!(task.id, "task-1");
    }

    #[test]
    fn test_next_runnable_task() {
        let state = create_test_state();
        // task-1 is completed, task-2 depends on task-1 (completed), so task-2 is runnable
        let next = state.next_runnable_task().unwrap();
        assert_eq!(next.id, "task-2");
    }

    #[test]
    fn test_is_task_blocked() {
        let state = create_test_state();

        // task-2 depends on task-1 which is completed, so not blocked
        assert!(!state.is_task_blocked("task-2"));

        // task-3 depends on task-2 which is pending, so blocked
        assert!(state.is_task_blocked("task-3"));

        // task-1 has no dependencies, so not blocked
        assert!(!state.is_task_blocked("task-1"));
    }

    #[test]
    fn test_mark_task_status() {
        let mut state = create_test_state();

        state
            .mark_task_status("task-2", TaskStatus::InProgress)
            .unwrap();

        let task = state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .find(|t| t.id == "task-2")
            .unwrap();

        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_mark_task_status_not_found() {
        let mut state = create_test_state();
        let result = state.mark_task_status("nonexistent", TaskStatus::Completed);
        assert!(matches!(result, Err(StateError::TaskNotFound(_))));
    }
}
