//! TASKS.md generation from TasksState.
//!
//! This module generates TASKS.md content from a TasksState.
//! The generation is deterministic: the same state always produces the same output.

use crate::state::{Phase, PhaseStatus, Task, TaskStatus, TasksState};
use std::fs;

/// Generate TASKS.md content from a tasks state.
///
/// The generated markdown shows:
/// - Each phase with its plan
/// - Tasks within each phase with status indicators
/// - A summary table at the end
///
/// # Arguments
///
/// * `state` - The tasks state to generate markdown from
///
/// # Returns
///
/// A string containing the complete TASKS.md content.
pub fn generate_tasks_md(state: &TasksState) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("# {} - Tasks\n\n", state.project.name));
    output.push_str("This document shows phase plans and task status. Generated from tasks.json.\n\n");

    // Phases
    for phase in &state.phases {
        output.push_str(&format_phase(phase));
        output.push_str("\n---\n\n");
    }

    // Summary table
    output.push_str(&generate_summary_table(state));

    output
}

/// Format a single phase as markdown.
fn format_phase(phase: &Phase) -> String {
    let mut output = String::new();

    // Phase header
    output.push_str(&format!("## {}: {}\n\n", phase.id, phase.name));

    // Phase status
    let completed_tasks = phase.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let total_tasks = phase.tasks.len();
    let status_str = match phase.status {
        PhaseStatus::Completed => "Complete",
        PhaseStatus::InProgress => "In Progress",
        PhaseStatus::Pending => "Pending",
    };
    output.push_str(&format!(
        "**Status:** {} ({}/{})\n\n",
        status_str, completed_tasks, total_tasks
    ));

    // Phase plan (if present)
    if !phase.plan.is_empty() {
        output.push_str("### Plan\n\n");
        output.push_str(&phase.plan);
        output.push_str("\n\n");
    }

    // Tasks
    if !phase.tasks.is_empty() {
        output.push_str("### Tasks\n\n");
        for task in &phase.tasks {
            output.push_str(&format_task(task));
        }
    }

    output
}

/// Format a single task as markdown.
fn format_task(task: &Task) -> String {
    let status_icon = match task.status {
        TaskStatus::Completed => "[x]",
        TaskStatus::InProgress => "[~]",
        TaskStatus::Pending => "[ ]",
        TaskStatus::Deferred => "[!]",
    };

    // Load plan from file and get first line as summary
    let plan_content = fs::read_to_string(&task.plan_file)
        .unwrap_or_else(|_| "(plan file missing)".to_string());
    let summary = plan_content.lines().next().unwrap_or(&plan_content);

    format!(
        "- {} **{}**: {} - {}\n",
        status_icon,
        task.id,
        task.name,
        summary
    )
}

/// Generate the summary table.
fn generate_summary_table(state: &TasksState) -> String {
    let mut output = String::new();

    output.push_str("## Summary\n\n");
    output.push_str("| Phase | Status | Tasks | Completed |\n");
    output.push_str("|-------|--------|-------|----------:|\n");

    let mut total_tasks = 0;
    let mut total_completed = 0;

    for phase in &state.phases {
        let completed = phase.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let total = phase.tasks.len();
        total_tasks += total;
        total_completed += completed;

        let status_str = match phase.status {
            PhaseStatus::Completed => "Complete",
            PhaseStatus::InProgress => "In Progress",
            PhaseStatus::Pending => "Pending",
        };

        output.push_str(&format!(
            "| {}: {} | {} | {} | {} |\n",
            phase.id,
            truncate(&phase.name, 30),
            status_str,
            total,
            completed
        ));
    }

    // Total row
    output.push_str(&format!(
        "| **Total** | | **{}** | **{}** |\n",
        total_tasks,
        total_completed
    ));

    output
}

/// Truncate a string to a maximum length, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Project, TaskContext, TaskType};
    use std::path::Path;

    fn create_test_task(id: &str, name: &str, status: TaskStatus) -> Task {
        // Create plan file for test
        let plan_file = format!(".cm/plans/plan-test-{}.md", id);
        let plan_dir = Path::new(".cm/plans");
        if !plan_dir.exists() {
            fs::create_dir_all(plan_dir).ok();
        }
        fs::write(&plan_file, "Do something important").ok();

        Task {
            id: id.to_string(),
            name: name.to_string(),
            task_type: TaskType::Implement,
            status,
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
        }
    }

    fn create_test_phase(id: &str, name: &str, status: PhaseStatus, tasks: Vec<Task>) -> Phase {
        Phase {
            id: id.to_string(),
            name: name.to_string(),
            plan: String::new(),
            status,
            tasks,
            review_cycles_completed: 0,
            baseline_commit: None,
            implem_completed_at: None,
        }
    }

    fn create_test_state() -> TasksState {
        TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "Test Project".to_string(),
                description: "A test project".to_string(),
                created_at: None,
            },
            global_context: None,
            phases: vec![
                create_test_phase(
                    "phase-1",
                    "Setup",
                    PhaseStatus::Completed,
                    vec![
                        create_test_task("phase-1.task-1", "Create structure", TaskStatus::Completed),
                        create_test_task("phase-1.task-2", "Add dependencies", TaskStatus::Completed),
                    ],
                ),
                create_test_phase(
                    "phase-2",
                    "Implementation",
                    PhaseStatus::InProgress,
                    vec![
                        create_test_task("phase-2.task-1", "Implement core", TaskStatus::Completed),
                        create_test_task("phase-2.task-2", "Add tests", TaskStatus::InProgress),
                    ],
                ),
            ],
            current_phase: Some("phase-2".to_string()),
            current_task: Some("phase-2.task-2".to_string()),
            agent_history: vec![],
            log_records: vec![],
            interrupted_at: None,
        }
    }

    #[test]
    fn test_generate_tasks_md_basic() {
        let state = create_test_state();
        let output = generate_tasks_md(&state);

        assert!(output.contains("# Test Project - Tasks"));
        assert!(output.contains("## phase-1: Setup"));
        assert!(output.contains("## phase-2: Implementation"));
        assert!(output.contains("## Summary"));
    }

    #[test]
    fn test_generate_tasks_md_contains_tasks() {
        let state = create_test_state();
        let output = generate_tasks_md(&state);

        assert!(output.contains("phase-1.task-1"));
        assert!(output.contains("Create structure"));
        assert!(output.contains("[x]")); // Completed task
        assert!(output.contains("[~]")); // In progress task
    }

    #[test]
    fn test_generate_tasks_md_summary_table() {
        let state = create_test_state();
        let output = generate_tasks_md(&state);

        assert!(output.contains("| Phase | Status | Tasks | Completed |"));
        assert!(output.contains("| **Total** |"));
    }

    #[test]
    fn test_format_phase_with_plan() {
        let mut phase = create_test_phase(
            "phase-1",
            "Setup",
            PhaseStatus::InProgress,
            vec![create_test_task("phase-1.task-1", "Do stuff", TaskStatus::Pending)],
        );
        phase.plan = "## Objective\n\nSet up the project structure.".to_string();

        let output = format_phase(&phase);

        assert!(output.contains("### Plan"));
        assert!(output.contains("## Objective"));
        assert!(output.contains("Set up the project structure"));
    }

    #[test]
    fn test_format_task_status_icons() {
        let completed = create_test_task("t1", "Completed", TaskStatus::Completed);
        let in_progress = create_test_task("t2", "In Progress", TaskStatus::InProgress);
        let pending = create_test_task("t3", "Pending", TaskStatus::Pending);
        let mut deferred = create_test_task("t4", "Deferred", TaskStatus::Deferred);
        deferred.status = TaskStatus::Deferred;

        assert!(format_task(&completed).contains("[x]"));
        assert!(format_task(&in_progress).contains("[~]"));
        assert!(format_task(&pending).contains("[ ]"));
        assert!(format_task(&deferred).contains("[!]"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_empty_phases() {
        let state = TasksState {
            version: "1.0.0".to_string(),
            project: Project {
                name: "Empty".to_string(),
                description: "No phases".to_string(),
                created_at: None,
            },
            global_context: None,
            phases: vec![],
            current_phase: None,
            current_task: None,
            agent_history: vec![],
            log_records: vec![],
            interrupted_at: None,
        };

        let output = generate_tasks_md(&state);

        assert!(output.contains("# Empty - Tasks"));
        assert!(output.contains("## Summary"));
        assert!(output.contains("| **Total** | | **0** | **0** |"));
    }
}
