//! Init subcommand implementation.
//!
//! Creates `.claude/skills/` directory and writes skill files (cm.md, feat.md, fix.md).

use std::fs;
use std::path::Path;

use log::info;

use super::CliError;
use crate::skill::{CM_SKILL, FEAT_SKILL, FIX_SKILL};

/// Skill file definition.
struct SkillFile {
    name: &'static str,
    content: &'static str,
}

/// All skill files to write.
const SKILLS: &[SkillFile] = &[
    SkillFile {
        name: "cm.md",
        content: CM_SKILL,
    },
    SkillFile {
        name: "feat.md",
        content: FEAT_SKILL,
    },
    SkillFile {
        name: "fix.md",
        content: FIX_SKILL,
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
        let file_path = skills_dir.join(skill.name);

        if file_path.exists() && !force {
            println!(
                "Skipping {} (already exists, use --force to overwrite)",
                skill.name
            );
            continue;
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
            let file_path = skills_dir.join(skill.name);
            assert!(file_path.exists(), "Expected {} to exist", skill.name);
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, skill.content);
        }
    }

    #[test]
    fn test_execute_init_skips_existing_without_force() {
        let tmp = TempDir::new().unwrap();

        // Create directory and a file with different content
        let skills_dir = tmp.path().join(".claude/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        let cm_path = skills_dir.join("cm.md");
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
        let skills_dir = tmp.path().join(".claude/skills");
        fs::create_dir_all(&skills_dir).unwrap();
        let cm_path = skills_dir.join("cm.md");
        fs::write(&cm_path, "original content").unwrap();

        // Run init with force
        execute_init_in_dir(tmp.path(), true).unwrap();

        // Check that cm.md was overwritten
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
            let file_path = skills_dir.join(skill.name);
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, skill.content);
        }
    }
}
