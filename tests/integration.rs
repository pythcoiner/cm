//! Integration tests for the cm (Claude Code Manager) tool.
//!
//! These tests verify that the various components of cm work together correctly.
//! Since we can't actually spawn Claude CLI agents in tests, we focus on testing
//! the state management, recovery, and internal flows.

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use tempfile::TempDir;

use cm::manager::{CheckpointId, ManagerConfig, RecoveryAction, RecoveryManager};
use cm::state::{
    load_state, save_state, AttemptStatus, GlobalContext, Phase, PhaseStatus, Project, Task,
    TaskAttempt, TaskContext, TaskStatus, TaskType, TasksState,
};

// =============================================================================
// Test Setup Helpers
// =============================================================================

/// Create a temporary directory for test isolation.
fn create_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Create a minimal valid TasksState for testing.
fn create_minimal_state() -> TasksState {
    TasksState {
        version: "1.0.0".to_string(),
        project: Project {
            name: "test-project".to_string(),
            description: "Test project for integration testing".to_string(),
            created_at: Some(Utc::now()),
        },
        global_context: None,
        phases: vec![],
        current_phase: None,
        current_task: None,
        agent_history: vec![],
        log_records: vec![],
        interrupted_at: None,
    }
}

/// Create a state with a single phase and task.
fn create_state_with_task() -> TasksState {
    TasksState {
        version: "1.0.0".to_string(),
        project: Project {
            name: "test-project".to_string(),
            description: "Test project".to_string(),
            created_at: Some(Utc::now()),
        },
        global_context: Some(GlobalContext {
            plan_summary: "Test plan summary".to_string(),
        }),
        phases: vec![Phase {
            id: "phase-1".to_string(),
            name: "Test Phase".to_string(),
            plan: String::new(),
            status: PhaseStatus::Pending,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
            tasks: vec![Task {
                id: "phase-1.task-1".to_string(),
                name: "Test Task".to_string(),
                task_type: TaskType::Implement,
                status: TaskStatus::Pending,
                depends_on: vec![],
                context: TaskContext {
                    files_to_read: vec!["src/lib.rs".to_string()],
                    code_style_excerpt: Some("Use thiserror for errors".to_string()),
                    prior_review_issues: vec![],
                },
                instructions: "Implement the test feature".to_string(),
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

/// Create a state with multiple tasks and dependencies.
fn create_state_with_dependencies() -> TasksState {
    TasksState {
        version: "1.0.0".to_string(),
        project: Project {
            name: "test-project".to_string(),
            description: "Test project with dependencies".to_string(),
            created_at: Some(Utc::now()),
        },
        global_context: None,
        phases: vec![Phase {
            id: "phase-1".to_string(),
            name: "Test Phase".to_string(),
            plan: String::new(),
            status: PhaseStatus::InProgress,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
            tasks: vec![
                Task {
                    id: "task-a".to_string(),
                    name: "Task A".to_string(),
                    task_type: TaskType::Implement,
                    status: TaskStatus::Pending,
                    depends_on: vec![],
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Do Task A".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                },
                Task {
                    id: "task-b".to_string(),
                    name: "Task B".to_string(),
                    task_type: TaskType::Implement,
                    status: TaskStatus::Pending,
                    depends_on: vec!["task-a".to_string()],
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Do Task B (depends on A)".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                },
                Task {
                    id: "task-c".to_string(),
                    name: "Task C".to_string(),
                    task_type: TaskType::Review,
                    status: TaskStatus::Pending,
                    depends_on: vec!["task-b".to_string()],
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Review Task B output".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                },
            ],
        }],
        current_phase: Some("phase-1".to_string()),
        current_task: None,
        agent_history: vec![],
        log_records: vec![],
        interrupted_at: None,
    }
}

/// Write a state to the .cm directory in the given temp directory.
fn write_state(dir: &TempDir, state: &TasksState) -> PathBuf {
    let cm_dir = dir.path().join(".cm");
    fs::create_dir_all(&cm_dir).expect("Failed to create .cm directory");
    let path = cm_dir.join("tasks.json");
    save_state(state, &path).expect("Failed to save state");
    path
}

// =============================================================================
// Basic State Flow Tests
// =============================================================================

#[test]
fn test_basic_state_flow() {
    let dir = create_test_dir();
    let mut state = create_minimal_state();

    // Add a simple task
    state.phases.push(Phase {
        id: "phase-1".to_string(),
        name: "Test Phase".to_string(),
        plan: String::new(),
        status: PhaseStatus::Pending,
        review_cycles_completed: 0,
        baseline_commit: None,
        implem_completed_at: None,
        tasks: vec![Task {
            id: "phase-1.task-1".to_string(),
            name: "Test Task".to_string(),
            task_type: TaskType::Implement,
            status: TaskStatus::Pending,
            depends_on: vec![],
            context: TaskContext {
                files_to_read: vec![],
                code_style_excerpt: None,
                prior_review_issues: vec![],
            },
            instructions: "Do the task".to_string(),
            attempts: vec![],
            roadmap_item_id: None,
            implem_completed_at: None,
            baseline_commit: None,
            review_cycles_completed: 0,
        }],
    });

    let path = write_state(&dir, &state);

    // Verify we can load the state
    let loaded = load_state(&path).expect("Failed to load state");
    assert_eq!(loaded.project.name, "test-project");
    assert_eq!(loaded.phases.len(), 1);
    assert_eq!(loaded.phases[0].tasks.len(), 1);
    assert_eq!(loaded.phases[0].tasks[0].id, "phase-1.task-1");
}

#[test]
fn test_state_updates_persist() {
    let dir = create_test_dir();
    let mut state = create_state_with_task();

    // Initially the task is pending
    assert_eq!(state.phases[0].tasks[0].status, TaskStatus::Pending);

    let path = write_state(&dir, &state);

    // Modify task status
    state
        .mark_task_status("phase-1.task-1", TaskStatus::InProgress)
        .expect("Failed to mark status");

    // Save again
    save_state(&state, &path).expect("Failed to save updated state");

    // Reload and verify change persisted
    let loaded = load_state(&path).expect("Failed to reload state");
    assert_eq!(loaded.phases[0].tasks[0].status, TaskStatus::InProgress);
}

#[test]
fn test_state_with_global_context() {
    let dir = create_test_dir();
    let state = create_state_with_task();
    let path = write_state(&dir, &state);

    let loaded = load_state(&path).expect("Failed to load state");

    // Verify global context was saved and loaded
    assert!(loaded.global_context.is_some());
    let ctx = loaded.global_context.unwrap();
    assert_eq!(ctx.plan_summary, "Test plan summary");
}

#[test]
fn test_state_with_task_context() {
    let dir = create_test_dir();
    let state = create_state_with_task();
    let path = write_state(&dir, &state);

    let loaded = load_state(&path).expect("Failed to load state");
    let task = &loaded.phases[0].tasks[0];

    // Verify task context was preserved
    assert_eq!(task.context.files_to_read, vec!["src/lib.rs".to_string()]);
    assert_eq!(
        task.context.code_style_excerpt,
        Some("Use thiserror for errors".to_string())
    );
    assert!(task.context.prior_review_issues.is_empty());
}

// =============================================================================
// Recovery Tests
// =============================================================================

#[test]
fn test_crash_recovery_with_in_progress_task() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);

    // Create state with an in-progress task (simulates a crash)
    let mut state = create_state_with_task();
    state
        .mark_task_status("phase-1.task-1", TaskStatus::InProgress)
        .unwrap();

    // Recovery should suggest retrying the in-progress task
    let action = recovery
        .recover_from_crash(&state)
        .expect("Recovery failed");
    assert_eq!(action, RecoveryAction::Retry);
}

#[test]
fn test_crash_recovery_clean_state() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);

    // Create a clean state with no in-progress tasks
    let state = create_state_with_task();

    // Recovery should suggest continuing
    let action = recovery
        .recover_from_crash(&state)
        .expect("Recovery failed");
    assert_eq!(action, RecoveryAction::Continue);
}

#[test]
fn test_checkpoint_restore() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);
    let state = create_state_with_task();

    // Create checkpoint
    let checkpoint_id = recovery.checkpoint(&state).expect("Checkpoint failed");

    // Modify state after checkpoint
    let mut modified_state = state.clone();
    modified_state
        .mark_task_status("phase-1.task-1", TaskStatus::Completed)
        .unwrap();

    // Restore from checkpoint
    let restored = recovery.restore(&checkpoint_id).expect("Restore failed");

    // Restored state should have the original (Pending) status
    assert_eq!(restored.project.name, state.project.name);
    assert_eq!(restored.phases[0].tasks[0].status, TaskStatus::Pending);
}

#[test]
fn test_multiple_checkpoints() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);
    let mut state = create_state_with_task();

    // Create first checkpoint (task pending)
    let cp1 = recovery.checkpoint(&state).expect("Checkpoint 1 failed");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Modify state and create second checkpoint (task in progress)
    state
        .mark_task_status("phase-1.task-1", TaskStatus::InProgress)
        .unwrap();
    let cp2 = recovery.checkpoint(&state).expect("Checkpoint 2 failed");
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Modify state and create third checkpoint (task completed)
    state
        .mark_task_status("phase-1.task-1", TaskStatus::Completed)
        .unwrap();
    let _cp3 = recovery.checkpoint(&state).expect("Checkpoint 3 failed");

    // Verify we can restore to any checkpoint
    let restored1 = recovery.restore(&cp1).expect("Restore 1 failed");
    assert_eq!(restored1.phases[0].tasks[0].status, TaskStatus::Pending);

    let restored2 = recovery.restore(&cp2).expect("Restore 2 failed");
    assert_eq!(restored2.phases[0].tasks[0].status, TaskStatus::InProgress);
}

#[test]
fn test_checkpoint_cleanup() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);
    let state = create_minimal_state();

    // Create 5 checkpoints
    for _ in 0..5 {
        recovery.checkpoint(&state).expect("Checkpoint failed");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Verify all 5 exist
    let checkpoints = recovery.list_checkpoints().expect("List failed");
    assert_eq!(checkpoints.len(), 5);

    // Cleanup, keeping only 2
    let deleted = recovery.cleanup_checkpoints(2).expect("Cleanup failed");
    assert_eq!(deleted, 3);

    // Verify only 2 remain
    let checkpoints = recovery.list_checkpoints().expect("List failed");
    assert_eq!(checkpoints.len(), 2);
}

#[test]
fn test_recovery_rollback_on_corruption() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);
    let good_state = create_minimal_state();

    // Create a checkpoint with good state
    let checkpoint_id = recovery.checkpoint(&good_state).expect("Checkpoint failed");

    // Create a corrupted state (empty version is invalid)
    let mut corrupted_state = create_minimal_state();
    corrupted_state.version = "".to_string();

    // Recovery should suggest rollback to the checkpoint
    let action = recovery
        .recover_from_crash(&corrupted_state)
        .expect("Recovery failed");
    assert_eq!(action, RecoveryAction::Rollback(checkpoint_id));
}

// =============================================================================
// Task Dependency Tests
// =============================================================================

#[test]
fn test_blocked_task_not_runnable() {
    let state = create_state_with_dependencies();

    // Task A has no dependencies, so it's not blocked
    assert!(!state.is_task_blocked("task-a"));

    // Task B depends on A (which is pending), so B is blocked
    assert!(state.is_task_blocked("task-b"));

    // Task C depends on B (which is pending), so C is blocked
    assert!(state.is_task_blocked("task-c"));

    // Only Task A should be returned by next_runnable_task
    let next = state.next_runnable_task();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, "task-a");
}

#[test]
fn test_dependency_chain_unblocks() {
    let mut state = create_state_with_dependencies();

    // Initially only Task A is runnable
    let next = state.next_runnable_task();
    assert_eq!(next.unwrap().id, "task-a");

    // Complete Task A
    state.mark_task_status("task-a", TaskStatus::Completed).unwrap();

    // Now Task B should be unblocked and runnable
    assert!(!state.is_task_blocked("task-b"));
    let next = state.next_runnable_task();
    assert_eq!(next.unwrap().id, "task-b");

    // Task C should still be blocked
    assert!(state.is_task_blocked("task-c"));

    // Complete Task B
    state.mark_task_status("task-b", TaskStatus::Completed).unwrap();

    // Now Task C should be unblocked
    assert!(!state.is_task_blocked("task-c"));
    let next = state.next_runnable_task();
    assert_eq!(next.unwrap().id, "task-c");
}

#[test]
fn test_no_runnable_tasks_when_all_blocked() {
    let mut state = create_state_with_dependencies();

    // Put Task A in progress (not pending, not completed)
    state
        .mark_task_status("task-a", TaskStatus::InProgress)
        .unwrap();

    // Now no tasks are runnable:
    // - Task A is InProgress (not Pending)
    // - Task B is blocked by A
    // - Task C is blocked by B
    let next = state.next_runnable_task();
    assert!(next.is_none());
}

#[test]
fn test_no_runnable_tasks_when_all_completed() {
    let mut state = create_state_with_dependencies();

    // Complete all tasks
    state.mark_task_status("task-a", TaskStatus::Completed).unwrap();
    state.mark_task_status("task-b", TaskStatus::Completed).unwrap();
    state.mark_task_status("task-c", TaskStatus::Completed).unwrap();

    // No tasks are runnable because all are completed
    let next = state.next_runnable_task();
    assert!(next.is_none());
}

// =============================================================================
// Deferred Task Tests
// =============================================================================

#[test]
fn test_task_deferred_status() {
    let mut state = create_state_with_task();

    // Mark task as deferred
    state
        .mark_task_status("phase-1.task-1", TaskStatus::Deferred)
        .unwrap();

    // Deferred tasks should not be runnable
    let next = state.next_runnable_task();
    assert!(next.is_none());

    // Verify the status was set
    let task = &state.phases[0].tasks[0];
    assert_eq!(task.status, TaskStatus::Deferred);
}

#[test]
fn test_task_with_max_failed_attempts_scenario() {
    // This test verifies the state representation for a task that should be deferred
    // after too many failed attempts (the actual logic is in Manager)

    let mut state = create_state_with_task();

    // Add 5 failed attempts to the task
    for i in 1..=5 {
        state.phases[0].tasks[0].attempts.push(TaskAttempt {
            attempt_number: i,
            agent_id: format!("agent-{}", i),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            status: AttemptStatus::Failed,
            prompt: String::new(),
            response: None,
        });
    }

    // Verify we can save and load state with attempts
    let dir = create_test_dir();
    let path = write_state(&dir, &state);
    let loaded = load_state(&path).expect("Failed to load state");

    assert_eq!(loaded.phases[0].tasks[0].attempts.len(), 5);
    assert_eq!(
        loaded.phases[0].tasks[0].attempts[4].status,
        AttemptStatus::Failed
    );
}

// =============================================================================
// Manager Config Tests
// =============================================================================

#[test]
fn test_manager_config_defaults() {
    let config = ManagerConfig::new(PathBuf::from("/tmp/.cm/tasks.json"));

    assert_eq!(config.state_path, PathBuf::from("/tmp/.cm/tasks.json"));
    assert_eq!(config.log_path, PathBuf::from("/tmp/.cm/LOG.md"));
    assert_eq!(config.max_cycles, 5);
}

#[test]
fn test_manager_config_builder_pattern() {
    let config = ManagerConfig::new(PathBuf::from("/tmp/.cm/tasks.json"))
        .log_path(PathBuf::from("/tmp/.cm/custom.log.md"))
        .model("claude-opus-4-5-20251101".to_string())
        .max_cycles(10);

    assert_eq!(config.log_path, PathBuf::from("/tmp/.cm/custom.log.md"));
    assert_eq!(config.model, "claude-opus-4-5-20251101");
    assert_eq!(config.max_cycles, 10);
}

// =============================================================================
// State Interruption Tests
// =============================================================================

#[test]
fn test_interrupted_state_flag() {
    let mut state = create_minimal_state();

    // Initially not interrupted
    assert!(!state.was_interrupted());
    assert!(state.interrupted_at.is_none());

    // Mark as interrupted
    state.mark_interrupted();
    assert!(state.was_interrupted());
    assert!(state.interrupted_at.is_some());

    // Verify timestamp is recent
    let interrupted_at = state.interrupted_at.unwrap();
    let now = Utc::now();
    let diff = now.signed_duration_since(interrupted_at);
    assert!(diff.num_seconds() < 5); // Should be within 5 seconds

    // Clear the interrupted flag
    state.clear_interrupted();
    assert!(!state.was_interrupted());
    assert!(state.interrupted_at.is_none());
}

#[test]
fn test_interrupted_state_persists() {
    let dir = create_test_dir();
    let mut state = create_minimal_state();

    // Mark as interrupted and save
    state.mark_interrupted();
    let path = write_state(&dir, &state);

    // Load and verify
    let loaded = load_state(&path).expect("Failed to load state");
    assert!(loaded.was_interrupted());
    assert!(loaded.interrupted_at.is_some());
}

// =============================================================================
// Phase and Task Type Tests
// =============================================================================

#[test]
fn test_current_phase_retrieval() {
    let state = create_state_with_task();

    let current_phase = state.current_phase();
    assert!(current_phase.is_some());
    assert_eq!(current_phase.unwrap().id, "phase-1");
}

#[test]
fn test_task_types() {
    let mut state = create_minimal_state();

    // Add tasks of different types
    state.phases.push(Phase {
        id: "phase-1".to_string(),
        name: "Mixed Phase".to_string(),
        plan: String::new(),
        status: PhaseStatus::Pending,
        review_cycles_completed: 0,
        baseline_commit: None,
        implem_completed_at: None,
        tasks: vec![
            Task {
                id: "t1".to_string(),
                name: "Implement Task".to_string(),
                task_type: TaskType::Implement,
                status: TaskStatus::Pending,
                depends_on: vec![],
                context: TaskContext {
                    files_to_read: vec![],
                    code_style_excerpt: None,
                    prior_review_issues: vec![],
                },
                instructions: "Implement something".to_string(),
                attempts: vec![],
                roadmap_item_id: None,
                implem_completed_at: None,
                baseline_commit: None,
                review_cycles_completed: 0,
            },
            Task {
                id: "t2".to_string(),
                name: "Review Task".to_string(),
                task_type: TaskType::Review,
                status: TaskStatus::Pending,
                depends_on: vec!["t1".to_string()],
                context: TaskContext {
                    files_to_read: vec![],
                    code_style_excerpt: None,
                    prior_review_issues: vec![],
                },
                instructions: "Review the implementation".to_string(),
                attempts: vec![],
                roadmap_item_id: None,
                implem_completed_at: None,
                baseline_commit: None,
                review_cycles_completed: 0,
            },
            Task {
                id: "t3".to_string(),
                name: "Fix Task".to_string(),
                task_type: TaskType::Fix,
                status: TaskStatus::Pending,
                depends_on: vec!["t2".to_string()],
                context: TaskContext {
                    files_to_read: vec![],
                    code_style_excerpt: None,
                    prior_review_issues: vec!["Issue from review".to_string()],
                },
                instructions: "Fix the issues".to_string(),
                attempts: vec![],
                roadmap_item_id: None,
                implem_completed_at: None,
                baseline_commit: None,
                review_cycles_completed: 0,
            },
            Task {
                id: "t4".to_string(),
                name: "Test Task".to_string(),
                task_type: TaskType::Test,
                status: TaskStatus::Pending,
                depends_on: vec!["t3".to_string()],
                context: TaskContext {
                    files_to_read: vec![],
                    code_style_excerpt: None,
                    prior_review_issues: vec![],
                },
                instructions: "Write tests".to_string(),
                attempts: vec![],
                roadmap_item_id: None,
                implem_completed_at: None,
                baseline_commit: None,
                review_cycles_completed: 0,
            },
        ],
    });

    let dir = create_test_dir();
    let path = write_state(&dir, &state);
    let loaded = load_state(&path).expect("Failed to load state");

    // Verify all task types are preserved
    assert_eq!(loaded.phases[0].tasks[0].task_type, TaskType::Implement);
    assert_eq!(loaded.phases[0].tasks[1].task_type, TaskType::Review);
    assert_eq!(loaded.phases[0].tasks[2].task_type, TaskType::Fix);
    assert_eq!(loaded.phases[0].tasks[3].task_type, TaskType::Test);

    // Verify prior_review_issues are preserved
    assert_eq!(
        loaded.phases[0].tasks[2].context.prior_review_issues,
        vec!["Issue from review".to_string()]
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_mark_nonexistent_task() {
    let mut state = create_state_with_task();

    let result = state.mark_task_status("nonexistent-task", TaskStatus::Completed);
    assert!(result.is_err());
}

#[test]
fn test_load_nonexistent_file() {
    let result = load_state(&PathBuf::from("/nonexistent/path/tasks.json"));
    assert!(result.is_err());
}

#[test]
fn test_checkpoint_nonexistent_id() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);

    let result = recovery.restore(&CheckpointId("nonexistent".to_string()));
    assert!(result.is_err());
}

// =============================================================================
// Complex Integration Scenarios
// =============================================================================

#[test]
fn test_full_workflow_simulation() {
    let dir = create_test_dir();
    let checkpoints_dir = dir.path().join(".cm/checkpoints");
    fs::create_dir_all(&checkpoints_dir).expect("Failed to create checkpoints dir");

    let recovery = RecoveryManager::new(checkpoints_dir);
    let mut state = create_state_with_dependencies();

    // Step 1: Create initial checkpoint
    let _cp1 = recovery.checkpoint(&state).expect("Checkpoint failed");

    // Step 2: Start executing Task A
    state.mark_task_status("task-a", TaskStatus::InProgress).unwrap();
    state.current_task = Some("task-a".to_string());

    // Simulate "crash" - verify recovery would suggest retry
    let action = recovery.recover_from_crash(&state).expect("Recovery failed");
    assert_eq!(action, RecoveryAction::Retry);

    // Step 3: Complete Task A
    state.mark_task_status("task-a", TaskStatus::Completed).unwrap();
    state.phases[0].tasks[0].attempts.push(TaskAttempt {
        attempt_number: 1,
        agent_id: "agent-1".to_string(),
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        status: AttemptStatus::Success,
        prompt: String::new(),
        response: None,
    });
    let _cp2 = recovery.checkpoint(&state).expect("Checkpoint failed");

    // Step 4: Verify Task B is now unblocked
    assert!(!state.is_task_blocked("task-b"));
    let next = state.next_runnable_task();
    assert_eq!(next.unwrap().id, "task-b");

    // Step 5: Complete Task B
    state.mark_task_status("task-b", TaskStatus::InProgress).unwrap();
    state.mark_task_status("task-b", TaskStatus::Completed).unwrap();

    // Step 6: Verify Task C is now unblocked
    assert!(!state.is_task_blocked("task-c"));
    let next = state.next_runnable_task();
    assert_eq!(next.unwrap().id, "task-c");

    // Step 7: Complete Task C
    state.mark_task_status("task-c", TaskStatus::Completed).unwrap();

    // Step 8: No more runnable tasks
    assert!(state.next_runnable_task().is_none());

    // Step 9: Save final state and verify persistence
    let path = write_state(&dir, &state);
    let loaded = load_state(&path).expect("Failed to load state");

    // Verify all tasks are completed
    for task in &loaded.phases[0].tasks {
        assert_eq!(task.status, TaskStatus::Completed);
    }
}

#[test]
fn test_multi_phase_workflow() {
    let mut state = create_minimal_state();

    // Add two phases
    state.phases = vec![
        Phase {
            id: "phase-1".to_string(),
            name: "Phase 1".to_string(),
            plan: String::new(),
            status: PhaseStatus::Completed,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
            tasks: vec![Task {
                id: "p1-t1".to_string(),
                name: "P1 Task".to_string(),
                task_type: TaskType::Implement,
                status: TaskStatus::Completed,
                depends_on: vec![],
                context: TaskContext {
                    files_to_read: vec![],
                    code_style_excerpt: None,
                    prior_review_issues: vec![],
                },
                instructions: "Phase 1 task".to_string(),
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
            status: PhaseStatus::Pending,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
            tasks: vec![
                Task {
                    id: "p2-t1".to_string(),
                    name: "P2 Task 1".to_string(),
                    task_type: TaskType::Implement,
                    status: TaskStatus::Pending,
                    depends_on: vec!["p1-t1".to_string()], // Depends on phase 1 task
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Phase 2 task 1".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                },
                Task {
                    id: "p2-t2".to_string(),
                    name: "P2 Task 2".to_string(),
                    task_type: TaskType::Test,
                    status: TaskStatus::Pending,
                    depends_on: vec!["p2-t1".to_string()],
                    context: TaskContext {
                        files_to_read: vec![],
                        code_style_excerpt: None,
                        prior_review_issues: vec![],
                    },
                    instructions: "Phase 2 task 2".to_string(),
                    attempts: vec![],
                    roadmap_item_id: None,
                    implem_completed_at: None,
                    baseline_commit: None,
                    review_cycles_completed: 0,
                },
            ],
        },
    ];

    state.current_phase = Some("phase-2".to_string());

    // Cross-phase dependency: p2-t1 depends on p1-t1 which is completed
    assert!(!state.is_task_blocked("p2-t1"));
    assert!(state.is_task_blocked("p2-t2"));

    // Next runnable should be from phase 2
    let next = state.next_runnable_task();
    assert!(next.is_some());
    assert_eq!(next.unwrap().id, "p2-t1");
}
