//! Crash recovery and graceful shutdown for the manager.
//!
//! This module provides checkpointing, state restoration, crash recovery,
//! and graceful signal handling for the manager. It ensures that interrupted
//! executions can be safely resumed.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use log::{debug, error, info, warn};
use thiserror::Error;

use crate::state::{StateError, TaskStatus, TasksState};

/// Errors that can occur during recovery operations.
#[derive(Debug, Error)]
pub enum RecoveryError {
    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// A state error occurred.
    #[error("state error: {0}")]
    StateError(#[from] StateError),

    /// The specified checkpoint was not found.
    #[error("checkpoint not found: {0}")]
    CheckpointNotFound(String),

    /// The checkpoint file is corrupted or invalid.
    #[error("corrupted checkpoint: {0}")]
    CorruptedCheckpoint(String),
}

/// Unique identifier for a checkpoint.
///
/// Checkpoints are named with timestamps to allow ordering and easy identification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckpointId(pub String);

impl CheckpointId {
    /// Create a new checkpoint ID with the current timestamp.
    pub fn now() -> Self {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
        Self(timestamp)
    }

    /// Get the filename for this checkpoint.
    pub fn filename(&self) -> String {
        format!("{}.json", self.0)
    }
}

impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Action to take when recovering from a crash or interrupted state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Continue from the current state without changes.
    Continue,
    /// Retry the task that was in progress.
    Retry,
    /// Skip the interrupted task and move to the next one.
    Skip,
    /// Rollback to a specific checkpoint.
    Rollback(CheckpointId),
}

impl std::fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryAction::Continue => write!(f, "Continue"),
            RecoveryAction::Retry => write!(f, "Retry"),
            RecoveryAction::Skip => write!(f, "Skip"),
            RecoveryAction::Rollback(id) => write!(f, "Rollback to {id}"),
        }
    }
}

/// Manages checkpoints and state recovery.
///
/// The RecoveryManager handles creating checkpoints of the task state,
/// restoring from checkpoints, and determining the appropriate recovery
/// action after a crash or interruption.
pub struct RecoveryManager {
    /// Directory where checkpoints are stored.
    checkpoints_dir: PathBuf,
}

impl RecoveryManager {
    /// Create a new RecoveryManager with the specified checkpoints directory.
    ///
    /// # Arguments
    ///
    /// * `checkpoints_dir` - Path to the directory for storing checkpoints.
    ///   This is typically `.cm/checkpoints/`.
    pub fn new(checkpoints_dir: PathBuf) -> Self {
        Self { checkpoints_dir }
    }

    /// Create a checkpoint of the current state.
    ///
    /// Saves the state to a timestamped JSON file in the checkpoints directory.
    ///
    /// # Arguments
    ///
    /// * `state` - The current tasks state to checkpoint
    ///
    /// # Returns
    ///
    /// The ID of the created checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The checkpoints directory cannot be created
    /// - The state cannot be serialized
    /// - The file cannot be written
    pub fn checkpoint(&self, state: &TasksState) -> Result<CheckpointId, RecoveryError> {
        // Ensure the checkpoints directory exists
        fs::create_dir_all(&self.checkpoints_dir)?;

        // Create checkpoint ID with current timestamp
        let checkpoint_id = CheckpointId::now();
        let checkpoint_path = self.checkpoints_dir.join(checkpoint_id.filename());

        // Serialize and save the state
        let content = serde_json::to_string_pretty(state)
            .map_err(|e| RecoveryError::CorruptedCheckpoint(e.to_string()))?;

        fs::write(&checkpoint_path, content)?;

        info!("Created checkpoint {checkpoint_id} at {checkpoint_path:?}");

        Ok(checkpoint_id)
    }

    /// Restore state from a specific checkpoint.
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - The ID of the checkpoint to restore
    ///
    /// # Returns
    ///
    /// The restored tasks state.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The checkpoint file does not exist
    /// - The file cannot be read
    /// - The JSON is invalid or corrupted
    pub fn restore(&self, checkpoint_id: &CheckpointId) -> Result<TasksState, RecoveryError> {
        let checkpoint_path = self.checkpoints_dir.join(checkpoint_id.filename());

        if !checkpoint_path.exists() {
            return Err(RecoveryError::CheckpointNotFound(checkpoint_id.0.clone()));
        }

        let content = fs::read_to_string(&checkpoint_path)?;

        let state: TasksState = serde_json::from_str(&content)
            .map_err(|e| RecoveryError::CorruptedCheckpoint(e.to_string()))?;

        info!("Restored state from checkpoint {checkpoint_id}");

        Ok(state)
    }

    /// List all available checkpoints.
    ///
    /// Returns checkpoints sorted by timestamp (oldest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoints directory cannot be read.
    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointId>, RecoveryError> {
        if !self.checkpoints_dir.exists() {
            return Ok(vec![]);
        }

        let mut checkpoints = Vec::new();

        for entry in fs::read_dir(&self.checkpoints_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(stem) = path.file_stem() {
                        if let Some(name) = stem.to_str() {
                            checkpoints.push(CheckpointId(name.to_string()));
                        }
                    }
                }
            }
        }

        // Sort by timestamp (lexicographically works for our format)
        checkpoints.sort();

        Ok(checkpoints)
    }

    /// Get the latest (most recent) checkpoint.
    ///
    /// # Returns
    ///
    /// The most recent checkpoint ID, or None if no checkpoints exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoints directory cannot be read.
    pub fn latest_checkpoint(&self) -> Result<Option<CheckpointId>, RecoveryError> {
        let checkpoints = self.list_checkpoints()?;
        Ok(checkpoints.into_iter().last())
    }

    /// Determine the appropriate recovery action based on the current state.
    ///
    /// This analyzes the state to determine what happened before the interruption
    /// and suggests the best course of action:
    ///
    /// - If a task is in_progress, suggest retrying it
    /// - If the state looks corrupted, suggest rolling back to the latest checkpoint
    /// - Otherwise, suggest continuing normally
    ///
    /// # Arguments
    ///
    /// * `state` - The current tasks state to analyze
    ///
    /// # Returns
    ///
    /// The recommended recovery action.
    ///
    /// # Errors
    ///
    /// Returns an error if checkpoint information cannot be read.
    pub fn recover_from_crash(&self, state: &TasksState) -> Result<RecoveryAction, RecoveryError> {
        // Check for in_progress tasks - these indicate an interrupted execution
        let in_progress_tasks: Vec<&str> = state
            .phases
            .iter()
            .flat_map(|p| &p.tasks)
            .filter(|t| t.status == TaskStatus::InProgress)
            .map(|t| t.id.as_str())
            .collect();

        if !in_progress_tasks.is_empty() {
            info!(
                "Found {} in_progress task(s): {:?}",
                in_progress_tasks.len(),
                in_progress_tasks
            );
            return Ok(RecoveryAction::Retry);
        }

        // Check for state corruption
        if self.is_state_corrupted(state) {
            warn!("State appears to be corrupted");

            // Try to find a checkpoint to rollback to
            if let Some(checkpoint_id) = self.latest_checkpoint()? {
                info!("Suggesting rollback to checkpoint {checkpoint_id}");
                return Ok(RecoveryAction::Rollback(checkpoint_id));
            } else {
                warn!("No checkpoints available for rollback");
            }
        }

        // State looks fine, continue normally
        debug!("State looks clean, suggesting continue");
        Ok(RecoveryAction::Continue)
    }

    /// Check if the state appears to be corrupted.
    ///
    /// This performs basic integrity checks on the state.
    fn is_state_corrupted(&self, state: &TasksState) -> bool {
        // Check for missing or invalid version
        if state.version.is_empty() {
            error!("State has empty version");
            return true;
        }

        // Check for empty project name
        if state.project.name.is_empty() {
            error!("State has empty project name");
            return true;
        }

        // Check for phases with no ID
        for phase in &state.phases {
            if phase.id.is_empty() {
                error!("Found phase with empty ID");
                return true;
            }

            // Check for tasks with no ID
            for task in &phase.tasks {
                if task.id.is_empty() {
                    error!("Found task with empty ID in phase {}", phase.id);
                    return true;
                }
            }
        }

        // Check for invalid current_phase reference
        if let Some(ref current_phase) = state.current_phase {
            if !state.phases.iter().any(|p| &p.id == current_phase) {
                error!("current_phase '{current_phase}' not found in phases");
                return true;
            }
        }

        // Check for invalid current_task reference
        if let Some(ref current_task) = state.current_task {
            let task_exists = state
                .phases
                .iter()
                .flat_map(|p| &p.tasks)
                .any(|t| &t.id == current_task);

            if !task_exists {
                error!("current_task '{current_task}' not found in any phase");
                return true;
            }
        }

        false
    }

    /// Delete old checkpoints, keeping only the most recent N.
    ///
    /// # Arguments
    ///
    /// * `keep` - Number of recent checkpoints to keep
    ///
    /// # Errors
    ///
    /// Returns an error if checkpoints cannot be read or deleted.
    pub fn cleanup_checkpoints(&self, keep: usize) -> Result<usize, RecoveryError> {
        let checkpoints = self.list_checkpoints()?;

        if checkpoints.len() <= keep {
            return Ok(0);
        }

        let to_delete = checkpoints.len() - keep;
        let mut deleted = 0;

        for checkpoint_id in checkpoints.into_iter().take(to_delete) {
            let checkpoint_path = self.checkpoints_dir.join(checkpoint_id.filename());

            match fs::remove_file(&checkpoint_path) {
                Ok(()) => {
                    debug!("Deleted old checkpoint {checkpoint_id}");
                    deleted += 1;
                }
                Err(e) => {
                    warn!("Failed to delete checkpoint {checkpoint_id}: {e}");
                }
            }
        }

        info!("Cleaned up {deleted} old checkpoint(s)");
        Ok(deleted)
    }
}

/// Handles graceful shutdown via signal handlers.
///
/// The ShutdownHandler registers handlers for SIGINT (Ctrl+C) and SIGTERM
/// signals, setting an atomic flag that can be checked by the main loop
/// to know when to stop.
pub struct ShutdownHandler {
    /// Atomic flag indicating whether shutdown has been requested.
    shutdown_flag: Arc<AtomicBool>,
}

impl ShutdownHandler {
    /// Create a new ShutdownHandler.
    ///
    /// The handler is created with the shutdown flag initially set to false.
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register signal handlers for graceful shutdown.
    ///
    /// This sets up handlers for SIGINT (Ctrl+C) and SIGTERM signals.
    /// When a signal is received, the shutdown flag is set to true.
    ///
    /// Note: This should only be called once at the start of the application.
    pub fn register_signal_handlers(&self) {
        let flag = self.shutdown_flag.clone();

        ctrlc::set_handler(move || {
            info!("Received shutdown signal");
            flag.store(true, Ordering::SeqCst);
        })
        .expect("Failed to set Ctrl+C handler");

        debug!("Signal handlers registered");
    }

    /// Check if shutdown has been requested.
    ///
    /// This should be called periodically in the main loop to check
    /// if a shutdown signal has been received.
    pub fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Request a shutdown.
    ///
    /// This can be called programmatically to trigger a shutdown,
    /// for example when an unrecoverable error occurs.
    pub fn request_shutdown(&self) {
        info!("Shutdown requested");
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    /// Get a clone of the shutdown flag.
    ///
    /// This can be used to share the flag with other threads.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }

    /// Reset the shutdown flag.
    ///
    /// This is primarily useful for testing.
    pub fn reset(&self) {
        self.shutdown_flag.store(false, Ordering::SeqCst);
    }

    /// Wait for shutdown to complete with a configurable timeout.
    ///
    /// This method blocks until either:
    /// - The shutdown flag is set (returns true)
    /// - The timeout expires (returns false)
    ///
    /// This is useful for waiting for a currently running agent to complete
    /// during graceful shutdown.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration to wait for shutdown
    ///
    /// # Returns
    ///
    /// `true` if shutdown was signaled within the timeout, `false` if timeout expired.
    pub fn wait_for_shutdown(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        while start.elapsed() < timeout {
            if self.shutdown_flag.load(Ordering::SeqCst) {
                return true;
            }
            std::thread::sleep(poll_interval);
        }

        // Final check before returning
        self.shutdown_flag.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GlobalContext, Phase, PhaseStatus, Project, Task, TaskContext, TaskType};
    use tempfile::TempDir;

    fn create_test_state() -> TasksState {
        TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "test".to_string(),
                description: "test project".to_string(),
                created_at: None,
            },
            global_context: Some(GlobalContext {
                plan_summary: "Test plan".to_string(),
            }),
            phases: vec![Phase {
                id: "phase-1".to_string(),
                name: "Phase 1".to_string(),
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
                    plan_file: ".cm/plans/plan-recovery-task-1.md".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                }],
            }],
            current_phase: Some("phase-1".to_string()),
            current_task: None,
            agent_history: vec![],
            log_records: vec![],
            interrupted_at: None,
        }
    }

    #[test]
    fn test_checkpoint_id_now() {
        let id1 = CheckpointId::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = CheckpointId::now();

        // IDs should be different
        assert_ne!(id1, id2);

        // Later ID should be greater
        assert!(id2 > id1);
    }

    #[test]
    fn test_checkpoint_id_filename() {
        let id = CheckpointId("20240101_120000_000".to_string());
        assert_eq!(id.filename(), "20240101_120000_000.json");
    }

    #[test]
    fn test_recovery_manager_new() {
        let dir = PathBuf::from("/tmp/test_checkpoints");
        let manager = RecoveryManager::new(dir.clone());
        assert_eq!(manager.checkpoints_dir, dir);
    }

    #[test]
    fn test_checkpoint_and_restore() {
        let tmp = TempDir::new().unwrap();
        let checkpoints_dir = tmp.path().join("checkpoints");
        let manager = RecoveryManager::new(checkpoints_dir);

        let state = create_test_state();

        // Create checkpoint
        let checkpoint_id = manager.checkpoint(&state).unwrap();
        assert!(!checkpoint_id.0.is_empty());

        // Restore from checkpoint
        let restored = manager.restore(&checkpoint_id).unwrap();
        assert_eq!(restored.version, state.version);
        assert_eq!(restored.project.name, state.project.name);
    }

    #[test]
    fn test_restore_not_found() {
        let tmp = TempDir::new().unwrap();
        let manager = RecoveryManager::new(tmp.path().to_path_buf());

        let result = manager.restore(&CheckpointId("nonexistent".to_string()));
        assert!(matches!(result, Err(RecoveryError::CheckpointNotFound(_))));
    }

    #[test]
    fn test_list_checkpoints() {
        let tmp = TempDir::new().unwrap();
        let checkpoints_dir = tmp.path().join("checkpoints");
        let manager = RecoveryManager::new(checkpoints_dir);

        let state = create_test_state();

        // Create multiple checkpoints
        let id1 = manager.checkpoint(&state).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = manager.checkpoint(&state).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id3 = manager.checkpoint(&state).unwrap();

        // List should return them in order
        let list = manager.list_checkpoints().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], id1);
        assert_eq!(list[1], id2);
        assert_eq!(list[2], id3);
    }

    #[test]
    fn test_latest_checkpoint() {
        let tmp = TempDir::new().unwrap();
        let checkpoints_dir = tmp.path().join("checkpoints");
        let manager = RecoveryManager::new(checkpoints_dir);

        // No checkpoints yet
        assert!(manager.latest_checkpoint().unwrap().is_none());

        let state = create_test_state();

        // Create checkpoints
        manager.checkpoint(&state).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = manager.checkpoint(&state).unwrap();

        // Latest should be id2
        let latest = manager.latest_checkpoint().unwrap();
        assert_eq!(latest, Some(id2));
    }

    #[test]
    fn test_recover_from_crash_continue() {
        let tmp = TempDir::new().unwrap();
        let manager = RecoveryManager::new(tmp.path().to_path_buf());

        let state = create_test_state();

        // Clean state should suggest Continue
        let action = manager.recover_from_crash(&state).unwrap();
        assert_eq!(action, RecoveryAction::Continue);
    }

    #[test]
    fn test_recover_from_crash_retry() {
        let tmp = TempDir::new().unwrap();
        let manager = RecoveryManager::new(tmp.path().to_path_buf());

        let mut state = create_test_state();
        state.phases[0].tasks[0].status = TaskStatus::InProgress;

        // In-progress task should suggest Retry
        let action = manager.recover_from_crash(&state).unwrap();
        assert_eq!(action, RecoveryAction::Retry);
    }

    #[test]
    fn test_recover_from_crash_rollback() {
        let tmp = TempDir::new().unwrap();
        let checkpoints_dir = tmp.path().join("checkpoints");
        let manager = RecoveryManager::new(checkpoints_dir);

        let good_state = create_test_state();

        // Create a checkpoint with good state
        let checkpoint_id = manager.checkpoint(&good_state).unwrap();

        // Create a corrupted state
        let mut bad_state = create_test_state();
        bad_state.version = "".to_string(); // Invalid

        // Should suggest rollback
        let action = manager.recover_from_crash(&bad_state).unwrap();
        assert_eq!(action, RecoveryAction::Rollback(checkpoint_id));
    }

    #[test]
    fn test_cleanup_checkpoints() {
        let tmp = TempDir::new().unwrap();
        let checkpoints_dir = tmp.path().join("checkpoints");
        let manager = RecoveryManager::new(checkpoints_dir);

        let state = create_test_state();

        // Create 5 checkpoints
        for _ in 0..5 {
            manager.checkpoint(&state).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(manager.list_checkpoints().unwrap().len(), 5);

        // Cleanup, keeping only 2
        let deleted = manager.cleanup_checkpoints(2).unwrap();
        assert_eq!(deleted, 3);

        // Should have 2 left
        assert_eq!(manager.list_checkpoints().unwrap().len(), 2);
    }

    #[test]
    fn test_is_state_corrupted() {
        let tmp = TempDir::new().unwrap();
        let manager = RecoveryManager::new(tmp.path().to_path_buf());

        // Good state
        let good_state = create_test_state();
        assert!(!manager.is_state_corrupted(&good_state));

        // Empty version
        let mut bad_state = create_test_state();
        bad_state.version = "".to_string();
        assert!(manager.is_state_corrupted(&bad_state));

        // Empty project name
        let mut bad_state = create_test_state();
        bad_state.project.name = "".to_string();
        assert!(manager.is_state_corrupted(&bad_state));

        // Invalid current_phase reference
        let mut bad_state = create_test_state();
        bad_state.current_phase = Some("nonexistent".to_string());
        assert!(manager.is_state_corrupted(&bad_state));
    }

    #[test]
    fn test_recovery_action_display() {
        assert_eq!(RecoveryAction::Continue.to_string(), "Continue");
        assert_eq!(RecoveryAction::Retry.to_string(), "Retry");
        assert_eq!(RecoveryAction::Skip.to_string(), "Skip");
        assert_eq!(
            RecoveryAction::Rollback(CheckpointId("test".to_string())).to_string(),
            "Rollback to test"
        );
    }

    #[test]
    fn test_shutdown_handler_new() {
        let handler = ShutdownHandler::new();
        assert!(!handler.should_shutdown());
    }

    #[test]
    fn test_shutdown_handler_request_shutdown() {
        let handler = ShutdownHandler::new();
        assert!(!handler.should_shutdown());

        handler.request_shutdown();
        assert!(handler.should_shutdown());
    }

    #[test]
    fn test_shutdown_handler_reset() {
        let handler = ShutdownHandler::new();
        handler.request_shutdown();
        assert!(handler.should_shutdown());

        handler.reset();
        assert!(!handler.should_shutdown());
    }

    #[test]
    fn test_shutdown_handler_default() {
        let handler = ShutdownHandler::default();
        assert!(!handler.should_shutdown());
    }

    #[test]
    fn test_shutdown_handler_flag_clone() {
        let handler = ShutdownHandler::new();
        let flag = handler.shutdown_flag();

        // Set via flag
        flag.store(true, Ordering::SeqCst);

        // Should be visible via handler
        assert!(handler.should_shutdown());
    }

    #[test]
    fn test_wait_for_shutdown_immediate() {
        let handler = ShutdownHandler::new();

        // Set shutdown flag before waiting
        handler.request_shutdown();

        // Should return immediately with true
        let result = handler.wait_for_shutdown(std::time::Duration::from_millis(100));
        assert!(result);
    }

    #[test]
    fn test_wait_for_shutdown_timeout() {
        let handler = ShutdownHandler::new();

        // Don't set the shutdown flag
        // Should timeout and return false
        let start = std::time::Instant::now();
        let result = handler.wait_for_shutdown(std::time::Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert!(!result);
        // Should have waited at least ~100ms (with some tolerance)
        assert!(elapsed >= std::time::Duration::from_millis(90));
    }

    #[test]
    fn test_wait_for_shutdown_signaled_during_wait() {
        use std::thread;

        let handler = ShutdownHandler::new();
        let flag = handler.shutdown_flag();

        // Spawn a thread that will set the shutdown flag after a short delay
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(50));
            flag.store(true, Ordering::SeqCst);
        });

        // Wait for shutdown with a longer timeout
        let start = std::time::Instant::now();
        let result = handler.wait_for_shutdown(std::time::Duration::from_millis(500));
        let elapsed = start.elapsed();

        assert!(result);
        // Should have returned before the full timeout
        assert!(elapsed < std::time::Duration::from_millis(300));
    }
}
