//! Init subcommand implementation.
//!
//! Creates `.claude/skills/` directory and writes skill files (cm.md, feat.md, fix.md).

use std::fs;
use std::path::Path;

use log::info;

use super::CliError;
use crate::skill::{CM_SKILL, END_SKILL, FEAT_SKILL, FIX_SKILL};

/// Skill file definition.
struct SkillFile {
    /// Directory name under .claude/skills/ (e.g., "cm" for .claude/skills/cm/)
    dir_name: &'static str,
    /// Content of the SKILL.md file
    content: &'static str,
}

/// All skill files to write.
const SKILLS: &[SkillFile] = &[
    SkillFile {
        dir_name: "cm",
        content: CM_SKILL,
    },
    SkillFile {
        dir_name: "feat",
        content: FEAT_SKILL,
    },
    SkillFile {
        dir_name: "fix",
        content: FIX_SKILL,
    },
    SkillFile {
        dir_name: "end",
        content: END_SKILL,
    },
];

/// Execute the init subcommand.
///
/// Creates `.claude/skills/` directory and writes all skill files.
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
    let skills_dir = base_dir.join(".claude/skills");

    // Create the directory if it doesn't exist
    if !skills_dir.exists() {
        info!("Creating directory: {:?}", skills_dir);
        fs::create_dir_all(&skills_dir)?;
    }

    // Write each skill file
    for skill in SKILLS {
        let skill_dir = skills_dir.join(skill.dir_name);
        let file_path = skill_dir.join("SKILL.md");

        if file_path.exists() && !force {
            println!(
                "Skipping {}/SKILL.md (already exists, use --force to overwrite)",
                skill.dir_name
            );
            continue;
        }

        // Create the skill directory if it doesn't exist
        if !skill_dir.exists() {
            info!("Creating directory: {:?}", skill_dir);
            fs::create_dir_all(&skill_dir)?;
        }

        info!("Writing skill file: {:?}", file_path);
        fs::write(&file_path, skill.content)?;
        println!("Created {}", file_path.display());
    }

    println!();
    println!("Skills initialized successfully!");
    println!();
    println!("Available skills:");
    println!("  /cm   - Project setup wizard");
    println!("  /feat - Feature addition wizard");
    println!("  /fix  - Bug fix wizard");
    println!("  /end  - Finalize feat/fix session");

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

        let skills_dir = tmp.path().join(".claude/skills");
        assert!(skills_dir.exists());
        assert!(skills_dir.is_dir());
    }

    #[test]
    fn test_execute_init_creates_all_skill_files() {
        let tmp = TempDir::new().unwrap();
        execute_init_in_dir(tmp.path(), false).unwrap();

        let skills_dir = tmp.path().join(".claude/skills");

        for skill in SKILLS {
            let skill_dir = skills_dir.join(skill.dir_name);
            let file_path = skill_dir.join("SKILL.md");
            assert!(
                skill_dir.exists(),
                "Expected {}/SKILL.md directory to exist",
                skill.dir_name
            );
            assert!(
                file_path.exists(),
                "Expected {}/SKILL.md to exist",
                skill.dir_name
            );
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, skill.content);
        }
    }

    #[test]
    fn test_execute_init_skips_existing_without_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a file with different content
        let skills_dir = tmp.path().join(".claude/skills");
        let cm_dir = skills_dir.join("cm");
        fs::create_dir_all(&cm_dir).unwrap();
        let cm_path = cm_dir.join("SKILL.md");
        let original_content = "original content";
        fs::write(&cm_path, original_content).unwrap();

        // Run init without force
        execute_init_in_dir(tmp.path(), false).unwrap();

        // Check that cm/SKILL.md was not overwritten
        let content = fs::read_to_string(&cm_path).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_execute_init_overwrites_with_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a file with different content
        let skills_dir = tmp.path().join(".claude/skills");
        let cm_dir = skills_dir.join("cm");
        fs::create_dir_all(&cm_dir).unwrap();
        let cm_path = cm_dir.join("SKILL.md");
        fs::write(&cm_path, "original content").unwrap();

        // Run init with force
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check that cm/SKILL.md was overwritten
        let content = fs::read_to_string(&cm_path).unwrap();
        assert_eq!(content, CM_SKILL);
    }

    #[test]
    fn test_execute_init_idempotent() {
        let tmp = TempDir::new().unwrap();

        // Run init twice with force
        execute_init_in_dir(tmp.path(), true).unwrap();
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check files still exist and have correct content
        let skills_dir = tmp.path().join(".claude/skills");
        for skill in SKILLS {
            let file_path = skills_dir.join(skill.dir_name).join("SKILL.md");
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, skill.content);
        }
    }
}
