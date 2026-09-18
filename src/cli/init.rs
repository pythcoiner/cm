//! Init subcommand implementation.
//!
//! Creates `.claude/commands/` directory and writes command files.

use std::fs;
use std::path::Path;

use log::info;

use super::CliError;
use crate::command::{
    ACTIONS_TEMPLATE, CLONE_COMMAND, CM_COMMAND, CONFIG_TEMPLATE, END_COMMAND, EXPAND_COMMAND,
    FEAT_COMMAND, FIX_COMMAND, FIX_TEMPLATE, IMPLEMENTER_TEMPLATE, PLANNER_TEMPLATE, PLAN_TEMPLATE,
    REVIEWER_TEMPLATE, RUN_COMMAND, SCHEMA_TEMPLATE, SPLIT_COMMAND, STRUCTURE_TEMPLATE,
    TASK_PLAN_TEMPLATE,
};

/// Command file definition.
struct CommandFile {
    /// Filename without .md extension (e.g., "cm" for .claude/commands/cm.md)
    name: &'static str,
    /// Content of the command file
    content: &'static str,
}

/// Template file definition.
struct TemplateFile {
    /// Relative path from .cm/ directory (e.g., "agents/MANAGER.md")
    path: &'static str,
    /// Content of the template file
    content: &'static str,
}

/// All command files to write.
const COMMANDS: &[CommandFile] = &[
    CommandFile {
        name: "cm",
        content: CM_COMMAND,
    },
    CommandFile {
        name: "feat",
        content: FEAT_COMMAND,
    },
    CommandFile {
        name: "fix",
        content: FIX_COMMAND,
    },
    CommandFile {
        name: "end",
        content: END_COMMAND,
    },
    CommandFile {
        name: "run",
        content: RUN_COMMAND,
    },
    CommandFile {
        name: "split",
        content: SPLIT_COMMAND,
    },
    CommandFile {
        name: "expand",
        content: EXPAND_COMMAND,
    },
    CommandFile {
        name: "clone",
        content: CLONE_COMMAND,
    },
];

/// All template files to write.
const TEMPLATES: &[TemplateFile] = &[
    TemplateFile {
        path: "agents/IMPLEMENTER.md",
        content: IMPLEMENTER_TEMPLATE,
    },
    TemplateFile {
        path: "agents/REVIEWER.md",
        content: REVIEWER_TEMPLATE,
    },
    TemplateFile {
        path: "agents/FIX.md",
        content: FIX_TEMPLATE,
    },
    TemplateFile {
        path: "agents/STRUCTURE.md",
        content: STRUCTURE_TEMPLATE,
    },
    TemplateFile {
        path: "agents/ACTIONS.md",
        content: ACTIONS_TEMPLATE,
    },
    TemplateFile {
        path: "agents/PLAN.md",
        content: PLAN_TEMPLATE,
    },
    TemplateFile {
        path: "agents/PLANNER.md",
        content: PLANNER_TEMPLATE,
    },
    TemplateFile {
        path: "agents/TASK_PLAN_TEMPLATE.md",
        content: TASK_PLAN_TEMPLATE,
    },
    TemplateFile {
        path: "SCHEMA.md",
        content: SCHEMA_TEMPLATE,
    },
    TemplateFile {
        path: "config.toml",
        content: CONFIG_TEMPLATE,
    },
];

/// Execute the init subcommand.
///
/// Creates `.claude/commands/` directory and writes all command files.
/// If `force` is false, existing files will not be overwritten.
///
/// # Errors
///
/// Returns an error if:
/// - Directory creation fails
/// - File writing fails
/// - A file already exists and force is false
pub fn execute_init(force: bool) -> Result<(), CliError> {
    execute_init_in_dir(Path::new("."), force)
}

/// Execute the init subcommand in a specific directory.
///
/// This is the internal implementation that allows specifying the base directory,
/// used for testing.
fn execute_init_in_dir(base_dir: &Path, force: bool) -> Result<(), CliError> {
    let commands_dir = base_dir.join(".claude/commands");
    let cm_dir = base_dir.join(".cm");

    // Create the directories if they don't exist
    if !commands_dir.exists() {
        info!("Creating directory: {commands_dir:?}");
        fs::create_dir_all(&commands_dir)?;
    }
    if !cm_dir.exists() {
        info!("Creating directory: {cm_dir:?}");
        fs::create_dir_all(&cm_dir)?;
    }

    // Write each command file
    for command in COMMANDS {
        let file_path = commands_dir.join(format!("{}.md", command.name));

        if file_path.exists() && !force {
            println!(
                "Skipping {}.md (already exists, use --force to overwrite)",
                command.name
            );
            continue;
        }

        info!("Writing command file: {file_path:?}");
        fs::write(&file_path, command.content)?;
        println!("Created {}", file_path.display());
    }

    // Write each template file
    for template in TEMPLATES {
        let file_path = cm_dir.join(template.path);

        // Create parent directory if needed
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                info!("Creating directory: {parent:?}");
                fs::create_dir_all(parent)?;
            }
        }

        if file_path.exists() && !force {
            println!(
                "Skipping {} (already exists, use --force to overwrite)",
                template.path
            );
            continue;
        }

        info!("Writing template file: {file_path:?}");
        fs::write(&file_path, template.content)?;
        println!("Created {}", file_path.display());
    }

    // Ensure .cm/logs/ is in .gitignore
    ensure_gitignore_entry(base_dir)?;

    // Clean up legacy skills directory if it exists
    cleanup_legacy_skills(base_dir)?;

    println!();
    println!("Commands initialized successfully!");
    println!();
    println!("Available commands:");
    println!("  /cm     - Project setup wizard");
    println!("  /feat   - Feature addition wizard");
    println!("  /fix    - Bug fix wizard");
    println!("  /split  - Refine PLAN.md into detailed phases");
    println!("  /expand - Expand plan files with detailed instructions");
    println!("  /end    - Finalize feat/fix/cm session");
    println!("  /run    - Run cm orchestration");
    println!("  /clone  - Generate spec documents from source code");

    Ok(())
}

/// Ensure `.cm/logs/` is in the root .gitignore file.
///
/// Creates the .gitignore file if it doesn't exist.
/// Appends the entry if not already present.
fn ensure_gitignore_entry(base_dir: &Path) -> Result<(), CliError> {
    let gitignore_path = base_dir.join(".gitignore");
    let entry = ".cm/logs/";

    // Read existing content if file exists
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // Check if entry already exists (as a complete line)
    let already_present = content.lines().any(|line| line.trim() == entry);

    if already_present {
        return Ok(());
    }

    // Append the entry
    let mut new_content = content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(entry);
    new_content.push('\n');

    fs::write(&gitignore_path, new_content)?;
    info!("Added '{entry}' to .gitignore");
    println!("Added '{entry}' to .gitignore");

    Ok(())
}

/// Clean up legacy `.claude/skills/` directory structure.
///
/// Removes known legacy skill files and empty directories from the old
/// `.claude/skills/{name}/SKILL.md` structure.
fn cleanup_legacy_skills(base_dir: &Path) -> Result<(), CliError> {
    let skills_dir = base_dir.join(".claude/skills");

    if !skills_dir.exists() {
        return Ok(());
    }

    let legacy_skills = ["cm", "feat", "fix", "end", "run"];
    let mut removed_any = false;

    for skill_name in &legacy_skills {
        let skill_dir = skills_dir.join(skill_name);
        let skill_file = skill_dir.join("SKILL.md");

        // Remove legacy SKILL.md file if it exists
        if skill_file.exists() {
            info!("Removing legacy file: {skill_file:?}");
            fs::remove_file(&skill_file)?;
            removed_any = true;
        }

        // Try to remove the skill subdirectory if it's empty
        if skill_dir.exists() {
            if let Ok(()) = fs::remove_dir(&skill_dir) {
                info!("Removed empty directory: {skill_dir:?}");
            }
        }
    }

    // Try to remove the .claude/skills directory if it's empty
    if let Ok(()) = fs::remove_dir(&skills_dir) {
        info!("Removed empty directory: {skills_dir:?}");
    }

    if removed_any {
        println!();
        println!("Migrated from legacy .claude/skills/ to .claude/commands/");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_execute_init_creates_directory() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let commands_dir = tmp.path().join(".claude/commands");
        assert!(commands_dir.exists());
        assert!(commands_dir.is_dir());
    }

    #[test]
    fn test_execute_init_creates_all_command_files() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let commands_dir = tmp.path().join(".claude/commands");

        for command in COMMANDS {
            let file_path = commands_dir.join(format!("{}.md", command.name));
            assert!(file_path.exists(), "Expected {}.md to exist", command.name);
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, command.content);
        }
    }

    #[test]
    fn test_execute_init_skips_existing_without_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a file with different content
        let commands_dir = tmp.path().join(".claude/commands");
        fs::create_dir_all(&commands_dir).unwrap();
        let cm_path = commands_dir.join("cm.md");
        let original_content = "original content";
        fs::write(&cm_path, original_content).unwrap();

        // Run init without force
        execute_init_in_dir(tmp.path(), false).unwrap();

        // Check that cm.md was not overwritten
        let content = fs::read_to_string(&cm_path).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_execute_init_overwrites_with_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a file with different content
        let commands_dir = tmp.path().join(".claude/commands");
        fs::create_dir_all(&commands_dir).unwrap();
        let cm_path = commands_dir.join("cm.md");
        fs::write(&cm_path, "original content").unwrap();

        // Run init with force
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check that cm.md was overwritten
        let content = fs::read_to_string(&cm_path).unwrap();
        assert_eq!(content, CM_COMMAND);
    }

    #[test]
    fn test_execute_init_idempotent() {
        let tmp = TempDir::new().unwrap();

        // Run init twice with force
        execute_init_in_dir(tmp.path(), true).unwrap();
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check files still exist and have correct content
        let commands_dir = tmp.path().join(".claude/commands");
        for command in COMMANDS {
            let file_path = commands_dir.join(format!("{}.md", command.name));
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, command.content);
        }
    }

    #[test]
    fn test_cleanup_legacy_skills() {
        let tmp = TempDir::new().unwrap();

        // Create legacy skills structure
        let skills_dir = tmp.path().join(".claude/skills");
        let cm_dir = skills_dir.join("cm");
        let feat_dir = skills_dir.join("feat");
        fs::create_dir_all(&cm_dir).unwrap();
        fs::create_dir_all(&feat_dir).unwrap();
        fs::write(cm_dir.join("SKILL.md"), "legacy cm").unwrap();
        fs::write(feat_dir.join("SKILL.md"), "legacy feat").unwrap();

        // Run init
        execute_init_in_dir(tmp.path(), false).unwrap();

        // Check that legacy files are removed
        assert!(!cm_dir.join("SKILL.md").exists());
        assert!(!feat_dir.join("SKILL.md").exists());

        // Check that legacy directories are removed if empty
        // (They might still exist if there are other files)

        // Check that new commands exist
        let commands_dir = tmp.path().join(".claude/commands");
        assert!(commands_dir.join("cm.md").exists());
        assert!(commands_dir.join("feat.md").exists());
    }

    #[test]
    fn test_cleanup_legacy_skills_removes_empty_dirs() {
        let tmp = TempDir::new().unwrap();

        // Create legacy skills structure with only SKILL.md files
        let skills_dir = tmp.path().join(".claude/skills");
        let cm_dir = skills_dir.join("cm");
        fs::create_dir_all(&cm_dir).unwrap();
        fs::write(cm_dir.join("SKILL.md"), "legacy cm").unwrap();

        // Run cleanup
        cleanup_legacy_skills(tmp.path()).unwrap();

        // Check that the directory structure is removed
        assert!(!cm_dir.join("SKILL.md").exists());
        assert!(!cm_dir.exists());
    }

    #[test]
    fn test_cleanup_legacy_skills_no_op_when_not_exists() {
        let tmp = TempDir::new().unwrap();

        // Run cleanup when .claude/skills doesn't exist
        let result = cleanup_legacy_skills(tmp.path());

        // Should succeed without error
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_init_creates_cm_directory() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let cm_dir = tmp.path().join(".cm");
        assert!(cm_dir.exists());
        assert!(cm_dir.is_dir());
    }

    #[test]
    fn test_execute_init_creates_agents_directory() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let agents_dir = tmp.path().join(".cm/agents");
        assert!(agents_dir.exists());
        assert!(agents_dir.is_dir());
    }

    #[test]
    fn test_execute_init_creates_all_template_files() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let cm_dir = tmp.path().join(".cm");

        for template in TEMPLATES {
            let file_path = cm_dir.join(template.path);
            assert!(file_path.exists(), "Expected {} to exist", template.path);
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, template.content);
        }
    }

    #[test]
    fn test_execute_init_skips_existing_templates_without_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a template file with different content
        let agents_dir = tmp.path().join(".cm/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        let implementer_path = agents_dir.join("IMPLEMENTER.md");
        let original_content = "original implementer content";
        fs::write(&implementer_path, original_content).unwrap();

        // Run init without force
        execute_init_in_dir(tmp.path(), false).unwrap();

        // Check that IMPLEMENTER.md was not overwritten
        let content = fs::read_to_string(&implementer_path).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_execute_init_overwrites_templates_with_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a template file with different content
        let agents_dir = tmp.path().join(".cm/agents");
        fs::create_dir_all(&agents_dir).unwrap();
        let implementer_path = agents_dir.join("IMPLEMENTER.md");
        fs::write(&implementer_path, "original content").unwrap();

        // Run init with force
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check that IMPLEMENTER.md was overwritten
        let content = fs::read_to_string(&implementer_path).unwrap();
        assert_eq!(content, IMPLEMENTER_TEMPLATE);
    }

    #[test]
    fn test_execute_init_creates_both_commands_and_templates() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        // Verify commands exist
        let commands_dir = tmp.path().join(".claude/commands");
        assert!(commands_dir.join("cm.md").exists());
        assert!(commands_dir.join("feat.md").exists());

        // Verify templates exist
        let cm_dir = tmp.path().join(".cm");
        assert!(cm_dir.join("agents/IMPLEMENTER.md").exists());
    }

    #[test]
    fn test_ensure_gitignore_creates_file() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let gitignore = tmp.path().join(".gitignore");
        assert!(gitignore.exists());
        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains(".cm/logs/"));
    }

    #[test]
    fn test_ensure_gitignore_appends_to_existing() {
        let tmp = TempDir::new().unwrap();
        let gitignore = tmp.path().join(".gitignore");
        fs::write(&gitignore, "node_modules/\n").unwrap();

        execute_init_in_dir(tmp.path(), false).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".cm/logs/"));
    }

    #[test]
    fn test_ensure_gitignore_idempotent() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let gitignore = tmp.path().join(".gitignore");
        let content = fs::read_to_string(&gitignore).unwrap();
        // Should only appear once
        assert_eq!(content.matches(".cm/logs/").count(), 1);
    }
}
