//! Prompt building for agent invocations.
//!
//! This module provides functionality to build prompts for different types
//! of agent tasks, ensuring context isolation - agents only receive
//! task-specific context, never global knowledge.

use crate::state::{ReviewIssue, Task};

/// Builds prompts for different types of agent tasks.
///
/// Prompts are built with context isolation in mind - agents only receive
/// the context necessary for their specific task, never global knowledge
/// that could leak across task boundaries.
pub struct PromptBuilder;

impl PromptBuilder {
    /// Build a prompt for an implementation task.
    ///
    /// The prompt includes:
    /// - Task instructions
    /// - Files to read for context
    /// - Code style excerpt (if provided)
    ///
    /// # Arguments
    ///
    /// * `task` - The task to build a prompt for
    pub fn build_implem_prompt(task: &Task) -> String {
        let mut prompt = String::new();

        // Header
        prompt.push_str("You are an IMPLEMENTATION agent. Your task is to implement the following:\n\n");

        // Task name and instructions
        prompt.push_str(&format!("## Task: {}\n\n", task.name));
        prompt.push_str("### Instructions\n\n");
        prompt.push_str(&task.instructions);
        prompt.push_str("\n\n");

        // Files to read for context
        if !task.context.files_to_read.is_empty() {
            prompt.push_str("### Files to Read for Context\n\n");
            prompt.push_str("Read the following files to understand the existing codebase:\n\n");
            for file in &task.context.files_to_read {
                prompt.push_str(&format!("- {}\n", file));
            }
            prompt.push('\n');
        }

        // Code style excerpt
        if let Some(ref style) = task.context.code_style_excerpt {
            prompt.push_str("### Code Style Guidelines\n\n");
            prompt.push_str("Follow these code style guidelines:\n\n");
            prompt.push_str(style);
            prompt.push_str("\n\n");
        }

        // Output format instructions
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("Respond with a JSON object containing:\n");
        prompt.push_str("- `files_created`: list of files you created\n");
        prompt.push_str("- `files_modified`: list of files you modified\n");
        prompt.push_str("- `commands_run`: list of commands you executed\n");
        prompt.push_str("- `summary`: brief summary of what you did\n");

        prompt
    }

    /// Build a prompt for a review task.
    ///
    /// The prompt includes:
    /// - Task instructions
    /// - Code to review
    /// - Files to read for context
    /// - Code style excerpt (if provided)
    ///
    /// # Arguments
    ///
    /// * `task` - The task to build a prompt for
    /// * `code_to_review` - The code that needs to be reviewed
    pub fn build_review_prompt(task: &Task, code_to_review: &str) -> String {
        let mut prompt = String::new();

        // Header
        prompt.push_str("You are a REVIEW agent. Your task is to review the following code:\n\n");

        // Task name and instructions
        prompt.push_str(&format!("## Review Task: {}\n\n", task.name));
        prompt.push_str("### Review Instructions\n\n");
        prompt.push_str(&task.instructions);
        prompt.push_str("\n\n");

        // Code to review
        prompt.push_str("### Code to Review\n\n");
        prompt.push_str("```\n");
        prompt.push_str(code_to_review);
        prompt.push_str("\n```\n\n");

        // Files to read for context
        if !task.context.files_to_read.is_empty() {
            prompt.push_str("### Reference Files\n\n");
            prompt.push_str("These files provide context for the review:\n\n");
            for file in &task.context.files_to_read {
                prompt.push_str(&format!("- {}\n", file));
            }
            prompt.push('\n');
        }

        // Code style excerpt
        if let Some(ref style) = task.context.code_style_excerpt {
            prompt.push_str("### Code Style Guidelines\n\n");
            prompt.push_str("Check the code against these style guidelines:\n\n");
            prompt.push_str(style);
            prompt.push_str("\n\n");
        }

        // Output format instructions
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("Respond with a JSON object containing:\n");
        prompt.push_str("- `verdict`: either \"approved\" or \"needs_fixes\"\n");
        prompt.push_str("- `issues`: array of issues found (if any), each with:\n");
        prompt.push_str("  - `id`: unique identifier for the issue\n");
        prompt.push_str("  - `severity`: \"critical\", \"high\", \"medium\", or \"low\"\n");
        prompt.push_str("  - `location`: file:line or description of location\n");
        prompt.push_str("  - `problem`: description of the problem\n");
        prompt.push_str("  - `suggested_fix`: how to fix the issue\n");
        prompt.push_str("- `summary`: brief summary of the review\n");

        prompt
    }

    /// Build a prompt for a fix task.
    ///
    /// The prompt includes:
    /// - Task instructions
    /// - Issues to fix (from previous review)
    /// - Files to read for context
    /// - Code style excerpt (if provided)
    ///
    /// # Arguments
    ///
    /// * `task` - The task to build a prompt for
    /// * `issues` - The issues from the review that need to be fixed
    pub fn build_fix_prompt(task: &Task, issues: &[ReviewIssue]) -> String {
        let mut prompt = String::new();

        // Header
        prompt.push_str("You are a FIX agent. Your task is to fix the issues found in the code review:\n\n");

        // Task name and instructions
        prompt.push_str(&format!("## Fix Task: {}\n\n", task.name));
        prompt.push_str("### Fix Instructions\n\n");
        prompt.push_str(&task.instructions);
        prompt.push_str("\n\n");

        // Issues to fix
        prompt.push_str("### Issues to Fix\n\n");
        if issues.is_empty() {
            prompt.push_str("No specific issues provided. Review the code and fix any problems.\n\n");
        } else {
            for issue in issues {
                prompt.push_str(&format!("#### Issue: {} ({})\n\n", issue.id, issue.severity_str()));
                prompt.push_str(&format!("**Location:** {}\n\n", issue.location));
                prompt.push_str(&format!("**Problem:** {}\n\n", issue.problem));
                prompt.push_str(&format!("**Suggested Fix:** {}\n\n", issue.suggested_fix));
            }
        }

        // Files to read for context
        if !task.context.files_to_read.is_empty() {
            prompt.push_str("### Files to Read for Context\n\n");
            prompt.push_str("Read the following files to understand the existing codebase:\n\n");
            for file in &task.context.files_to_read {
                prompt.push_str(&format!("- {}\n", file));
            }
            prompt.push('\n');
        }

        // Code style excerpt
        if let Some(ref style) = task.context.code_style_excerpt {
            prompt.push_str("### Code Style Guidelines\n\n");
            prompt.push_str("Ensure fixes follow these style guidelines:\n\n");
            prompt.push_str(style);
            prompt.push_str("\n\n");
        }

        // Output format instructions
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("Respond with a JSON object containing:\n");
        prompt.push_str("- `files_modified`: list of files you modified\n");
        prompt.push_str("- `issues_fixed`: list of issue IDs that you fixed\n");
        prompt.push_str("- `summary`: brief summary of the fixes applied\n");

        prompt
    }
}

/// Extension trait to get severity as a string.
trait SeverityExt {
    fn severity_str(&self) -> &'static str;
}

impl SeverityExt for ReviewIssue {
    fn severity_str(&self) -> &'static str {
        match self.severity {
            crate::state::Severity::Critical => "critical",
            crate::state::Severity::High => "high",
            crate::state::Severity::Medium => "medium",
            crate::state::Severity::Low => "low",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Severity, TaskContext, TaskStatus, TaskType};

    fn create_test_task() -> Task {
        Task {
            id: "test-task".to_string(),
            name: "Test Task".to_string(),
            task_type: TaskType::Implement,
            status: TaskStatus::Pending,
            depends_on: vec![],
            context: TaskContext {
                files_to_read: vec!["src/lib.rs".to_string(), "src/main.rs".to_string()],
                code_style_excerpt: Some("Use thiserror for errors".to_string()),
                prior_review_issues: vec![],
            },
            instructions: "Implement the foo function that does bar".to_string(),
            attempts: vec![],
            roadmap_item_id: None,
        }
    }

    #[test]
    fn test_build_implem_prompt_contains_task_name() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        assert!(prompt.contains("Test Task"));
        assert!(prompt.contains("IMPLEMENTATION agent"));
    }

    #[test]
    fn test_build_implem_prompt_contains_instructions() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        assert!(prompt.contains("Implement the foo function that does bar"));
    }

    #[test]
    fn test_build_implem_prompt_contains_files_to_read() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("src/main.rs"));
    }

    #[test]
    fn test_build_implem_prompt_contains_code_style() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        assert!(prompt.contains("Use thiserror for errors"));
    }

    #[test]
    fn test_build_implem_prompt_contains_output_format() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        assert!(prompt.contains("files_created"));
        assert!(prompt.contains("files_modified"));
    }

    #[test]
    fn test_build_review_prompt_contains_code_to_review() {
        let task = create_test_task();
        let code = "fn foo() { bar(); }";
        let prompt = PromptBuilder::build_review_prompt(&task, code);

        assert!(prompt.contains("REVIEW agent"));
        assert!(prompt.contains(code));
    }

    #[test]
    fn test_build_review_prompt_contains_verdict_instructions() {
        let task = create_test_task();
        let code = "fn foo() {}";
        let prompt = PromptBuilder::build_review_prompt(&task, code);

        assert!(prompt.contains("verdict"));
        assert!(prompt.contains("approved"));
        assert!(prompt.contains("needs_fixes"));
    }

    #[test]
    fn test_build_fix_prompt_contains_issues() {
        let task = create_test_task();
        let issues = vec![ReviewIssue {
            id: "issue-1".to_string(),
            severity: Severity::High,
            location: "src/lib.rs:42".to_string(),
            problem: "Missing error handling".to_string(),
            suggested_fix: "Add Result return type".to_string(),
            resolved: false,
        }];

        let prompt = PromptBuilder::build_fix_prompt(&task, &issues);

        assert!(prompt.contains("FIX agent"));
        assert!(prompt.contains("issue-1"));
        assert!(prompt.contains("Missing error handling"));
        assert!(prompt.contains("Add Result return type"));
        assert!(prompt.contains("src/lib.rs:42"));
    }

    #[test]
    fn test_build_fix_prompt_handles_empty_issues() {
        let task = create_test_task();
        let issues: Vec<ReviewIssue> = vec![];

        let prompt = PromptBuilder::build_fix_prompt(&task, &issues);

        assert!(prompt.contains("No specific issues provided"));
    }

    #[test]
    fn test_build_implem_prompt_no_files_to_read() {
        let mut task = create_test_task();
        task.context.files_to_read = vec![];

        let prompt = PromptBuilder::build_implem_prompt(&task);

        // Should not contain the "Files to Read" section header when empty
        assert!(!prompt.contains("### Files to Read for Context"));
    }

    #[test]
    fn test_build_implem_prompt_no_code_style() {
        let mut task = create_test_task();
        task.context.code_style_excerpt = None;

        let prompt = PromptBuilder::build_implem_prompt(&task);

        // Should not contain code style section when None
        assert!(!prompt.contains("### Code Style Guidelines"));
    }
}
