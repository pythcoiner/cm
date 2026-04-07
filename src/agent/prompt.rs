//! Prompt building for agent invocations.
//!
//! This module provides functionality to build prompts for different types
//! of agent tasks, ensuring context isolation - agents only receive
//! task-specific context, never global knowledge.

use crate::state::{Phase, ReviewIssue, Task};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Load task plan content from the plan file.
///
/// The plan file path is relative to the project root (current working directory).
///
/// # Arguments
///
/// * `task` - The task containing the plan_file path
///
/// # Returns
///
/// The contents of the plan file, or an error message if it couldn't be read.
fn load_task_plan(task: &Task) -> String {
    fs::read_to_string(&task.plan_file).unwrap_or_else(|e| {
        log::error!("Failed to load plan file '{}': {}", task.plan_file, e);
        format!("(Error loading plan file '{}': {})", task.plan_file, e)
    })
}

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
            fs::create_dir_all(parent).map_err(|e| {
                log::error!("Failed to create parent directory for template: {e}");
                e
            })?;
        }
        fs::write(path, default_content).map_err(|e| {
            log::error!("Failed to write default template content: {e}");
            e
        })?;
    }

    // Read and return the file contents
    fs::read_to_string(path).map_err(|e| {
        log::error!("Failed to read template file: {e}");
        e
    })
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
                log::error!("Failed to load IMPLEMENTER template: {e}, using embedded default");
                crate::command::IMPLEMENTER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Task name and instructions (loaded from plan file)
        prompt.push_str(&format!("## Task: {}\n\n", task.name));
        prompt.push_str("### Instructions\n\n");
        let plan_content = load_task_plan(task);
        prompt.push_str(&plan_content);
        prompt.push_str("\n\n");

        // Files to read for context
        if !task.context.files_to_read.is_empty() {
            prompt.push_str("### Files to Read for Context\n\n");
            prompt.push_str("Read the following files to understand the existing codebase:\n\n");
            for file in &task.context.files_to_read {
                prompt.push_str(&format!("- {file}\n"));
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
                log::error!("Failed to load REVIEWER template: {e}, using embedded default");
                crate::command::REVIEWER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Task name and instructions (loaded from plan file)
        prompt.push_str(&format!("## Review Task: {}\n\n", task.name));
        prompt.push_str("### Review Instructions\n\n");
        let plan_content = load_task_plan(task);
        prompt.push_str(&plan_content);
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
                prompt.push_str(&format!("- {file}\n"));
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
                log::error!("Failed to load FIX template: {e}, using embedded default");
                crate::command::FIX_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Task name and instructions (loaded from plan file)
        prompt.push_str(&format!("## Fix Task: {}\n\n", task.name));
        prompt.push_str("### Fix Instructions\n\n");
        let plan_content = load_task_plan(task);
        prompt.push_str(&plan_content);
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
                prompt.push_str(&format!("- {file}\n"));
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
                log::error!("Failed to load REVIEWER template: {e}, using embedded default");
                crate::command::REVIEWER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Original task context (loaded from plan file)
        prompt.push_str(&format!("## Original Task: {}\n\n", task.name));
        prompt.push_str("### What was requested\n\n");
        let plan_content = load_task_plan(task);
        prompt.push_str(&plan_content);
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
                prompt.push_str(&format!("- {file}\n"));
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
                log::error!("Failed to load FIX template: {e}, using embedded default");
                crate::command::FIX_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Original task context (loaded from plan file)
        prompt.push_str(&format!("## Original Task: {}\n\n", task.name));
        prompt.push_str("### Original Instructions\n\n");
        let plan_content = load_task_plan(task);
        prompt.push_str(&plan_content);
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
                prompt.push_str(&format!("- {file}\n"));
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

    /// Build a prompt for a plan agent to evaluate an initial plan.
    ///
    /// The plan agent reviews the initial plan and optionally creates a more detailed version.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase to plan for
    /// * `initial_plan` - The initial plan to evaluate
    pub fn build_phase_plan_prompt(phase: &Phase, initial_plan: &str) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/PLANNER.md");
        let template = ensure_template(&template_path, crate::command::PLANNER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::error!("Failed to load PLANNER template: {e}, using embedded default");
                crate::command::PLANNER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Phase context
        prompt.push_str(&format!("# Phase: {}\n\n", phase.name));

        // Add initial plan
        prompt.push_str("## Initial Plan\n\n");
        prompt.push_str(initial_plan);
        prompt.push_str("\n\n");

        // Add task list for context
        prompt.push_str("## Tasks in This Phase\n\n");
        for task in &phase.tasks {
            prompt.push_str(&format!("- **{}**: {}\n", task.id, task.name));
        }
        prompt.push('\n');

        prompt
    }

    /// Build a prompt for implementing ALL tasks in a phase with an explicit plan.
    ///
    /// This variant accepts an explicit plan string instead of using phase.plan,
    /// allowing the manager to substitute a detailed plan from the PLAN agent.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase being implemented
    /// * `tasks` - All pending tasks in the phase to implement
    /// * `plan` - The explicit plan to use (from PLAN agent or original)
    pub fn build_phase_implem_prompt_with_plan(
        phase: &Phase,
        tasks: &[&Task],
        plan: &str,
    ) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/IMPLEMENTER.md");
        let template = ensure_template(&template_path, crate::command::IMPLEMENTER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::error!("Failed to load IMPLEMENTER template: {e}, using embedded default");
                crate::command::IMPLEMENTER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Phase context
        prompt.push_str(&format!("## Phase: {} ({})\n\n", phase.name, phase.id));
        prompt.push_str(&format!(
            "You are implementing **{} tasks** in this phase. Complete all of them in order.\n\n",
            tasks.len()
        ));

        // Include the provided plan if present
        if !plan.is_empty() {
            prompt.push_str("### Phase Plan\n\n");
            prompt.push_str(plan);
            prompt.push_str("\n\n");
        }

        // List all tasks with their instructions (loaded from plan files)
        // Deduplicate: if multiple tasks share the same plan_file, only include content once
        prompt.push_str("### Tasks to Implement\n\n");
        let mut seen_plan_files = HashSet::new();
        for (i, task) in tasks.iter().enumerate() {
            prompt.push_str(&format!(
                "#### Task {}: {} ({})\n\n",
                i + 1,
                task.name,
                task.id
            ));
            if seen_plan_files.insert(task.plan_file.clone()) {
                let plan_content = load_task_plan(task);
                prompt.push_str(&format!("**Instructions:**\n{plan_content}\n\n"));
            } else {
                prompt.push_str(&format!(
                    "**Instructions:** (Same plan file as above: `{}`)\n\n",
                    task.plan_file
                ));
            }

            // Include task-specific context files
            if !task.context.files_to_read.is_empty() {
                prompt.push_str("**Files to read:**\n");
                for file in &task.context.files_to_read {
                    prompt.push_str(&format!("- {file}\n"));
                }
                prompt.push('\n');
            }
        }

        // Aggregate code style excerpt (use first non-empty one)
        let code_style = tasks
            .iter()
            .filter_map(|t| t.context.code_style_excerpt.as_ref())
            .next();
        if let Some(style) = code_style {
            prompt.push_str("### Code Style Guidelines\n\n");
            prompt.push_str("Follow these code style guidelines:\n\n");
            prompt.push_str(style);
            prompt.push_str("\n\n");
        }

        // Output format instructions for multi-task response
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully completed all tasks:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"summary\": \"Brief description of what you did for the entire phase\",\n");
        prompt.push_str("  \"tasks_completed\": [\n");
        prompt.push_str("    {\n");
        prompt.push_str("      \"task_id\": \"phase-X.task-Y\",\n");
        prompt.push_str("      \"summary\": \"What was done for this task\"\n");
        prompt.push_str("    }\n");
        prompt.push_str("  ],\n");
        prompt.push_str("  \"files_created\": [\"list\", \"of\", \"new\", \"files\"],\n");
        prompt.push_str("  \"files_modified\": [\"list\", \"of\", \"modified\", \"files\"]\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n\n");
        prompt.push_str("If you could NOT complete the tasks:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"failed\",\n");
        prompt.push_str("  \"error\": \"Detailed explanation of why you could not complete the tasks\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

        prompt
    }

    /// Build a prompt for implementing ALL tasks in a phase.
    ///
    /// This is the phase-level equivalent of `build_implem_prompt()`.
    /// The agent receives all tasks in the phase and should implement
    /// them in sequence within a single session.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase being implemented
    /// * `tasks` - All pending tasks in the phase to implement
    pub fn build_phase_implem_prompt(phase: &Phase, tasks: &[&Task]) -> String {
        Self::build_phase_implem_prompt_with_plan(phase, tasks, &phase.plan)
    }

    /// Build a prompt for reviewing ALL changes in a phase.
    ///
    /// This is the phase-level equivalent of `build_auto_review_prompt()`.
    /// The reviewer sees the complete diff of all task implementations
    /// and the original task descriptions for context.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase being reviewed
    /// * `diff` - The git diff of all changes since baseline
    pub fn build_phase_review_prompt(phase: &Phase, diff: &str) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/REVIEWER.md");
        let template = ensure_template(&template_path, crate::command::REVIEWER_TEMPLATE)
            .unwrap_or_else(|e| {
                log::error!("Failed to load REVIEWER template: {e}, using embedded default");
                crate::command::REVIEWER_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Phase context
        prompt.push_str(&format!("## Phase Review: {} ({})\n\n", phase.name, phase.id));
        prompt.push_str(&format!(
            "This phase contains **{} tasks**. Review all changes together.\n\n",
            phase.tasks.len()
        ));

        // Include phase plan if present
        if !phase.plan.is_empty() {
            prompt.push_str("### Phase Plan\n\n");
            prompt.push_str("This is the implementation plan the agent was given:\n\n");
            prompt.push_str(&phase.plan);
            prompt.push_str("\n\n");
        }

        // List all tasks with their full instructions (loaded from plan files)
        // Deduplicate: if multiple tasks share the same plan_file, only include content once
        prompt.push_str("### Tasks in This Phase\n\n");
        let mut seen_plan_files = HashSet::new();
        for (i, task) in phase.tasks.iter().enumerate() {
            prompt.push_str(&format!(
                "#### Task {}: {} ({})\n\n",
                i + 1,
                task.name,
                task.id
            ));
            if seen_plan_files.insert(task.plan_file.clone()) {
                let plan_content = load_task_plan(task);
                prompt.push_str(&format!("**Instructions:**\n{plan_content}\n\n"));
            } else {
                prompt.push_str(&format!(
                    "**Instructions:** (Same plan file as above: `{}`)\n\n",
                    task.plan_file
                ));
            }
        }

        // Code changes to review
        prompt.push_str("### Code Changes (git diff)\n\n");
        prompt.push_str("```diff\n");
        prompt.push_str(diff);
        prompt.push_str("\n```\n\n");

        // Review criteria
        prompt.push_str("### Review Criteria\n\n");
        prompt.push_str("1. **Correctness**: Do the changes correctly implement all requested tasks?\n");
        prompt.push_str("2. **Code quality**: Is the code clean, well-structured, and idiomatic?\n");
        prompt.push_str("3. **Error handling**: Are errors handled appropriately?\n");
        prompt.push_str("4. **Style**: Does the code follow the project's style conventions?\n");
        prompt.push_str("5. **Completeness**: Are all phase requirements addressed?\n\n");

        // Output format instructions
        prompt.push_str("### Output Format\n\n");
        prompt.push_str("When you are done, you MUST end your response with a JSON code block in this exact format.\n\n");
        prompt.push_str("If you successfully completed the review:\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"status\": \"success\",\n");
        prompt.push_str("  \"verdict\": \"approved\" or \"needs_fixes\",\n");
        prompt.push_str("  \"summary\": \"Brief review summary for the entire phase\",\n");
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

    /// Build a prompt for fixing issues found in a phase review.
    ///
    /// This is the phase-level equivalent of `build_auto_fix_prompt()`.
    /// The fixer receives the review feedback for the entire phase
    /// and should address all issues.
    ///
    /// # Arguments
    ///
    /// * `phase` - The phase being fixed
    /// * `review_feedback` - The raw review agent response containing issues
    pub fn build_phase_fix_prompt(phase: &Phase, review_feedback: &str) -> String {
        let mut prompt = String::new();

        // Load template from disk (or create it if it doesn't exist)
        let template_path = PathBuf::from(".cm/agents/FIX.md");
        let template = ensure_template(&template_path, crate::command::FIX_TEMPLATE)
            .unwrap_or_else(|e| {
                log::error!("Failed to load FIX template: {e}, using embedded default");
                crate::command::FIX_TEMPLATE.to_string()
            });

        // Add template header
        prompt.push_str(&template);
        prompt.push_str("\n\n---\n\n");

        // Phase context
        prompt.push_str(&format!("## Phase Fix: {} ({})\n\n", phase.name, phase.id));
        prompt.push_str(&format!(
            "This phase contains **{} tasks**. Fix all issues found in the review.\n\n",
            phase.tasks.len()
        ));

        // Reference to plan file (don't embed full plan to keep prompt focused)
        prompt.push_str("If you need context about the original implementation plan, read: `.cm/PLAN.md`\n\n");

        // Review feedback
        prompt.push_str("### Review Feedback\n\n");
        prompt.push_str("The following issues were found during phase review. Fix all of them:\n\n");
        prompt.push_str(review_feedback);
        prompt.push_str("\n\n");

        // Extract files from issue locations (e.g., "**Location:** src/main.rs:42")
        let issue_files = extract_files_from_feedback(review_feedback);
        if !issue_files.is_empty() {
            prompt.push_str("### Files to Modify\n\n");
            prompt.push_str("Based on the issues above, these files need changes:\n\n");
            for file in &issue_files {
                prompt.push_str(&format!("- {file}\n"));
            }
            prompt.push('\n');
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

    /// Build a prompt for the post-run review agent.
    ///
    /// The agent receives concatenated phase logs and must produce a markdown
    /// report followed by a trailing JSON verdict line.
    ///
    /// # Arguments
    ///
    /// * `run_started_at` - When the run started (for context)
    /// * `phase_logs` - Concatenated log content from all touched phases
    pub fn build_run_review_prompt(
        run_started_at: &chrono::DateTime<chrono::Utc>,
        phase_logs: &str,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("# Run Review Agent\n\n");
        prompt.push_str("You are a post-run auditor. Review the following execution logs from a cm run\n");
        prompt.push_str(&format!(
            "started at {}.\n\n",
            run_started_at.format("%Y-%m-%dT%H:%M:%SZ")
        ));

        prompt.push_str("## Instructions\n\n");
        prompt.push_str("- Identify any errors, build failures, deferred phases, or unresolved review issues.\n");
        prompt.push_str("- Look for orchestration bugs, tasks marked completed when they should not be, and missing edge cases.\n");
        prompt.push_str("- Report your findings as a markdown document.\n");
        prompt.push_str("- At the end of your response, include a JSON block with this exact structure:\n\n");
        prompt.push_str("```json\n{\"has_issues\": true}\n```\n\n");
        prompt.push_str("or\n\n");
        prompt.push_str("```json\n{\"has_issues\": false}\n```\n\n");
        prompt.push_str("If any issues are found, use `true`. If the run looks clean, use `false`.\n\n");

        prompt.push_str("## Phase Logs\n\n");
        if phase_logs.is_empty() {
            prompt.push_str("(No phase logs available.)\n");
        } else {
            prompt.push_str(phase_logs);
        }

        prompt
    }
}

/// Extract unique file paths from review feedback issue locations.
/// Parses patterns like "**Location:** src/file.rs:42" and returns deduplicated file paths.
fn extract_files_from_feedback(feedback: &str) -> Vec<String> {
    use std::collections::HashSet;

    let mut files: HashSet<String> = HashSet::new();
    for line in feedback.lines() {
        if line.starts_with("**Location:**") {
            // Format: "**Location:** path/to/file.rs:line"
            if let Some(loc) = line.strip_prefix("**Location:**") {
                let loc = loc.trim();
                // Split on ':' to remove line number
                if let Some(path) = loc.split(':').next() {
                    let path = path.trim();
                    if !path.is_empty() {
                        files.insert(path.to_string());
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = files.into_iter().collect();
    result.sort();
    result
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
        // Create a unique temporary plan file for testing (avoid race conditions)
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);

        // Use system temp directory to avoid polluting project directory
        let temp_dir = std::env::temp_dir().join("cm-tests");
        fs::create_dir_all(&temp_dir).expect("Failed to create temp directory for test");
        let plan_file = temp_dir.join(format!("plan-{}.md", unique_id));
        fs::write(&plan_file, "Implement the foo function that does bar")
            .expect("Failed to write plan file for test");

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
            plan_file: plan_file.to_string_lossy().to_string(),
            attempts: vec![],
            roadmap_item_id: None,
            implem_completed_at: None,
            baseline_commit: None,
            review_cycles_completed: 0,
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

    #[test]
    fn test_extract_files_from_feedback() {
        let feedback = r#"## Review Summary

Found 2 issues

## Issues to Fix

### Issue: issue-1 (high)
**Location:** src/main.rs:42
**Problem:** Missing error handling
**Suggested Fix:** Add ? operator

### Issue: issue-2 (medium)
**Location:** src/lib.rs:100
**Problem:** Unused variable
**Suggested Fix:** Remove or use it
"#;

        let files = extract_files_from_feedback(feedback);
        assert_eq!(files, vec!["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn test_extract_files_from_feedback_empty() {
        let feedback = "## Review Summary\n\nNo issues found.";
        let files = extract_files_from_feedback(feedback);
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_files_from_feedback_duplicate_files() {
        let feedback = r#"### Issue: issue-1 (high)
**Location:** src/main.rs:10
**Problem:** Problem 1

### Issue: issue-2 (high)
**Location:** src/main.rs:20
**Problem:** Problem 2
"#;

        let files = extract_files_from_feedback(feedback);
        // Should deduplicate
        assert_eq!(files, vec!["src/main.rs"]);
    }

}
