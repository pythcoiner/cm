//! Prompt building for agent invocations.
//!
//! This module provides functionality to build prompts for different types
//! of agent tasks, ensuring context isolation - agents only receive
//! task-specific context, never global knowledge.

use crate::state::{ReviewIssue, Task};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Ensures an agent template file exists on disk.
///
/// If the file doesn't exist, writes the default content to it first.
/// Returns the contents of the file.
///
/// # Arguments
///
/// * `path` - Path to the template file
/// * `default_content` - Default content to write if file doesn't exist
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
fn ensure_template(path: &Path, default_content: &str) -> io::Result<String> {
    // If file doesn't exist, create it with default content
    if !path.exists() {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, default_content)?;
    }

    // Read and return the file contents
    fs::read_to_string(path)
}

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

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/IMPLEMENTER.md");
        let template = ensure_template(&template_path, crate::command::IMPLEMENTER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::warn!("Failed to load IMPLEMENTER template: {}, using embedded default", e);
                crate::command::IMPLEMENTER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

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

        // Output format instructions (kept inline for consistency)
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully completed the task:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"summary\": \"Brief description of what you did\",\n");
        prompt.push_str("  \"files_created\": [\"list\", \"of\", \"new\", \"files\"],\n");
        prompt.push_str("  \"files_modified\": [\"list\", \"of\", \"modified\", \"files\"]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT complete the task:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not complete the task\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

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

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/REVIEWER.md");
        let template = ensure_template(&template_path, crate::command::REVIEWER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::warn!("Failed to load REVIEWER template: {}, using embedded default", e);
                crate::command::REVIEWER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

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
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully completed the review:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"verdict\": \"approved\" or \"needs_fixes\",\n");
        prompt.push_str("  \"summary\": \"Brief review summary\",\n");
        prompt.push_str("  \"issues\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"id\": \"unique-issue-id\",\n");
        prompt.push_str("      \"severity\": \"critical\" or \"high\" or \"medium\" or \"low\",\n");
        prompt.push_str("      \"location\": \"file:line\",\n");
        prompt.push_str("      \"problem\": \"description of the problem\",\n");
        prompt.push_str("      \"suggested_fix\": \"how to fix the issue\"\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT complete the review:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not complete the review\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

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

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/FIX.md");
        let template = ensure_template(&template_path, crate::command::FIX_TEMPLATE)
            .unwrap_or_else(|e| {
                log::warn!("Failed to load FIX template: {}, using embedded default", e);
                crate::command::FIX_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

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
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully fixed the issues:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"summary\": \"Brief description of the fixes applied\",\n");
        prompt.push_str("  \"files_modified\": [\"list\", \"of\", \"modified\", \"files\"]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT fix the issues:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not fix the issues\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

        prompt
    }

    /// Build a prompt for automatic review after an IMPLEM task.
    ///
    /// Unlike `build_review_prompt()` which is for explicit review tasks,
    /// this builds a review prompt using the git diff of the agent's committed
    /// changes, with the original task context for understanding intent.
    ///
    /// The diff is reliable because cm enforces a clean working tree before
    /// execution and commits after each agent, so the diff is scoped to
    /// exactly one agent's changes.
    ///
    /// # Arguments
    ///
    /// * `task` - The original IMPLEM task (for context about what was implemented)
    /// * `diff` - The git diff of the agent's committed changes
    pub fn build_auto_review_prompt(task: &Task, diff: &str) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/REVIEWER.md");
        let template = ensure_template(&template_path, crate::command::REVIEWER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::warn!("Failed to load REVIEWER template: {}, using embedded default", e);
                crate::command::REVIEWER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Original task context
        prompt.push_str(&format!("## Original Task: {}\n\n", task.name));
        prompt.push_str("### What was requested\n\n");
        prompt.push_str(&task.instructions);
        prompt.push_str("\n\n");

        // Code changes to review
        prompt.push_str("### Code Changes (git diff)\n\n");
        prompt.push_str("```diff\n");
        prompt.push_str(diff);
        prompt.push_str("\n```\n\n");

        // Review criteria
        prompt.push_str("### Review Criteria\n\n");
        prompt.push_str("1. **Correctness**: Do the changes correctly implement the requested task?\n");
        prompt.push_str("2. **Code quality**: Is the code clean, well-structured, and idiomatic?\n");
        prompt.push_str("3. **Error handling**: Are errors handled appropriately?\n");
        prompt.push_str("4. **Style**: Does the code follow the project's style conventions?\n");
        prompt.push_str("5. **Completeness**: Are all requirements addressed?\n\n");

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
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully completed the review:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"verdict\": \"approved\" or \"needs_fixes\",\n");
        prompt.push_str("  \"summary\": \"Brief review summary\",\n");
        prompt.push_str("  \"issues\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"id\": \"unique-issue-id\",\n");
        prompt.push_str("      \"severity\": \"critical\" or \"high\" or \"medium\" or \"low\",\n");
        prompt.push_str("      \"location\": \"file:line\",\n");
        prompt.push_str("      \"problem\": \"description of the problem\",\n");
        prompt.push_str("      \"suggested_fix\": \"how to fix the issue\"\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT complete the review:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not complete the review\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

        prompt
    }

    /// Build a prompt for automatic fix after a review found issues.
    ///
    /// Unlike `build_fix_prompt()` which takes structured `ReviewIssue` objects,
    /// this takes the raw review response text, since auto-review responses
    /// may not be fully parsed into structured issues.
    ///
    /// # Arguments
    ///
    /// * `task` - The original IMPLEM task (for context)
    /// * `review_feedback` - The raw review agent response containing issues
    pub fn build_auto_fix_prompt(task: &Task, review_feedback: &str) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/FIX.md");
        let template = ensure_template(&template_path, crate::command::FIX_TEMPLATE)
            .unwrap_or_else(|e| {
                log::warn!("Failed to load FIX template: {}, using embedded default", e);
                crate::command::FIX_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Original task context
        prompt.push_str(&format!("## Original Task: {}\n\n", task.name));
        prompt.push_str("### Original Instructions\n\n");
        prompt.push_str(&task.instructions);
        prompt.push_str("\n\n");

        // Review feedback
        prompt.push_str("### Review Feedback\n\n");
        prompt.push_str("The following issues were found during review. Fix all of them:\n\n");
        prompt.push_str(review_feedback);
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
            prompt.push_str("Ensure fixes follow these style guidelines:\n\n");
            prompt.push_str(style);
            prompt.push_str("\n\n");
        }

        // Output format instructions
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully fixed the issues:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"summary\": \"Brief description of the fixes applied\",\n");
        prompt.push_str("  \"files_modified\": [\"list\", \"of\", \"modified\", \"files\"]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT fix the issues:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not fix the issues\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

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
    use std::fs;

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
        // Now loading from template, so check for template content or task header
        assert!(prompt.contains("## Task: Test Task"));
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

        // Now loading from template, so check for code and task header instead
        assert!(prompt.contains(code));
        assert!(prompt.contains("## Review Task: Test Task"));
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

        // Should contain Fix Agent from template (case-insensitive match)
        let prompt_lower = prompt.to_lowercase();
        assert!(prompt_lower.contains("fix agent"));
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

    #[test]
    fn test_ensure_template_creates_file_if_missing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("test_template.md");
        let default_content = "# Test Template\n\nThis is a test.";

        // File should not exist initially
        assert!(!template_path.exists());

        // Call ensure_template
        let result = ensure_template(&template_path, default_content);

        // Should succeed
        assert!(result.is_ok());

        // File should now exist
        assert!(template_path.exists());

        // Contents should match default
        let contents = result.unwrap();
        assert_eq!(contents, default_content);
    }

    #[test]
    fn test_ensure_template_reads_existing_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("existing_template.md");
        let existing_content = "# Existing Content\n\nThis already exists.";
        let default_content = "# Default Content\n\nThis should not be used.";

        // Create the file with existing content
        fs::write(&template_path, existing_content).unwrap();

        // Call ensure_template
        let result = ensure_template(&template_path, default_content);

        // Should succeed
        assert!(result.is_ok());

        // Contents should be the existing content, not the default
        let contents = result.unwrap();
        assert_eq!(contents, existing_content);
    }

    #[test]
    fn test_ensure_template_creates_parent_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("subdir/nested/template.md");
        let default_content = "# Test Template";

        // Parent directories should not exist
        assert!(!template_path.parent().unwrap().exists());

        // Call ensure_template
        let result = ensure_template(&template_path, default_content);

        // Should succeed
        assert!(result.is_ok());

        // Parent directories and file should now exist
        assert!(template_path.parent().unwrap().exists());
        assert!(template_path.exists());
    }

    #[test]
    fn test_build_implem_prompt_loads_template() {
        let task = create_test_task();
        let prompt = PromptBuilder::build_implem_prompt(&task);

        // Should contain content from IMPLEMENTER template or fallback
        // We can't guarantee the exact content, but we can check that a prompt was built
        assert!(!prompt.is_empty());
        assert!(prompt.contains("## Task: Test Task"));
        assert!(prompt.contains("Implement the foo function that does bar"));
    }

    #[test]
    fn test_build_review_prompt_loads_template() {
        let task = create_test_task();
        let code = "fn foo() { bar(); }";
        let prompt = PromptBuilder::build_review_prompt(&task, code);

        // Should contain the code to review
        assert!(prompt.contains(code));
        assert!(prompt.contains("## Review Task: Test Task"));
    }

    #[test]
    fn test_build_auto_review_prompt_loads_template() {
        let task = create_test_task();
        let diff = "+fn new_function() {}\n-fn old_function() {}";
        let prompt = PromptBuilder::build_auto_review_prompt(&task, diff);

        // Should contain the diff
        assert!(prompt.contains(diff));
        assert!(prompt.contains("## Original Task: Test Task"));
    }

    #[test]
    fn test_build_fix_prompt_loads_template() {
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

        // Should contain the issues and task information
        assert!(!prompt.is_empty());
        assert!(prompt.contains("issue-1"));
        assert!(prompt.contains("Missing error handling"));
    }

    #[test]
    fn test_build_auto_fix_prompt_loads_template() {
        let task = create_test_task();
        let review_feedback = "The code has several issues that need to be fixed:\n1. Missing error handling\n2. Incorrect type usage";

        let prompt = PromptBuilder::build_auto_fix_prompt(&task, review_feedback);

        // Should contain the review feedback
        assert!(prompt.contains(review_feedback));
        assert!(prompt.contains("## Original Task: Test Task"));
    }
}
