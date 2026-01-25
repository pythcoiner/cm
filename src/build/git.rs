//! Git command runner for version control operations.
//!
//! This module provides functionality for running git commands (status, add, commit)
//! and parsing their output.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::BuildError;

/// Runs git commands in a specific working directory.
pub struct GitRunner {
    working_dir: PathBuf,
}

/// The status of a git repository.
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Files that have been modified but not staged.
    pub modified: Vec<String>,
    /// Files that are not tracked by git.
    pub untracked: Vec<String>,
    /// Files that have been staged for commit.
    pub staged: Vec<String>,
    /// Whether the working directory is clean (no changes).
    pub clean: bool,
}

/// A git commit identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitId(pub String);

impl GitRunner {
    /// Create a new `GitRunner` for the given working directory.
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// Get the status of the git repository.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The git command is not found
    /// - The working directory is not a git repository
    /// - An I/O error occurs
    pub fn status(&self) -> Result<GitStatus, BuildError> {
        let output = self.run_git_command(&["status", "--porcelain"])?;

        let mut status = GitStatus {
            modified: Vec::new(),
            untracked: Vec::new(),
            staged: Vec::new(),
            clean: true,
        };

        for line in output.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let file = line[3..].to_string();

            // Check if file is staged (index has changes)
            if index_status != ' ' && index_status != '?' {
                status.staged.push(file.clone());
                status.clean = false;
            }

            // Check worktree status
            match worktree_status {
                'M' => {
                    status.modified.push(file);
                    status.clean = false;
                }
                '?' => {
                    // Untracked files have ?? status
                    if index_status == '?' {
                        status.untracked.push(file);
                        status.clean = false;
                    }
                }
                'D' => {
                    status.modified.push(file);
                    status.clean = false;
                }
                _ => {}
            }
        }

        Ok(status)
    }

    /// Stage files for commit.
    ///
    /// # Arguments
    ///
    /// * `files` - The files to stage. Use `["."]` to stage all changes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The git command is not found
    /// - The working directory is not a git repository
    /// - Any of the specified files do not exist
    pub fn add(&self, files: &[&str]) -> Result<(), BuildError> {
        let mut args = vec!["add"];
        args.extend(files);

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::CommandNotFound("git".to_string())
                } else {
                    BuildError::IoError(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BuildError::CommandFailed {
                command: format!("git add {}", files.join(" ")),
                exit_code: output.status.code(),
                stderr,
            });
        }

        Ok(())
    }

    /// Commit staged changes.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The git command is not found
    /// - The working directory is not a git repository
    /// - There are no staged changes to commit
    pub fn commit(&self, message: &str) -> Result<CommitId, BuildError> {
        let output = Command::new("git")
            .args(["commit", "--no-gpg-sign", "-m", message])
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::CommandNotFound("git".to_string())
                } else {
                    BuildError::IoError(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BuildError::CommandFailed {
                command: "git commit".to_string(),
                exit_code: output.status.code(),
                stderr,
            });
        }

        // Get the commit hash
        let commit_hash = self.run_git_command(&["rev-parse", "HEAD"])?;
        Ok(CommitId(commit_hash.trim().to_string()))
    }

    /// Get the current HEAD commit hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn head_commit(&self) -> Result<CommitId, BuildError> {
        let hash = self.run_git_command(&["rev-parse", "HEAD"])?;
        Ok(CommitId(hash.trim().to_string()))
    }

    /// Get the diff between two commits.
    ///
    /// # Arguments
    ///
    /// * `from` - The base commit hash
    /// * `to` - The target commit hash
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn diff_range(&self, from: &str, to: &str) -> Result<String, BuildError> {
        let range = format!("{}..{}", from, to);
        self.run_git_command(&["diff", &range])
    }

    /// Check if the working tree is clean (no uncommitted changes).
    ///
    /// Returns `true` if there are no modified, staged, or untracked files.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn is_clean(&self) -> Result<bool, BuildError> {
        let output = self.run_git_command(&["status", "--porcelain"])?;
        Ok(output.trim().is_empty())
    }

    /// Run a git command and return its stdout.
    fn run_git_command(&self, args: &[&str]) -> Result<String, BuildError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::CommandNotFound("git".to_string())
                } else {
                    BuildError::IoError(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BuildError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                exit_code: output.status.code(),
                stderr,
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_id() {
        let id = CommitId("abc123".to_string());
        assert_eq!(id.0, "abc123");
        assert_eq!(id, CommitId("abc123".to_string()));
    }

    #[test]
    fn test_git_status_default() {
        let status = GitStatus::default();
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
        assert!(!status.clean); // Default is false, gets set to true in status()
    }

    #[test]
    fn test_parse_porcelain_status() {
        // This tests the parsing logic by simulating what status() does
        let porcelain_output = " M src/lib.rs\n?? new_file.rs\nM  staged.rs\nA  added.rs\n";

        let mut status = GitStatus {
            modified: Vec::new(),
            untracked: Vec::new(),
            staged: Vec::new(),
            clean: true,
        };

        for line in porcelain_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let file = line[3..].to_string();

            if index_status != ' ' && index_status != '?' {
                status.staged.push(file.clone());
                status.clean = false;
            }

            match worktree_status {
                'M' => {
                    status.modified.push(file);
                    status.clean = false;
                }
                '?' => {
                    if index_status == '?' {
                        status.untracked.push(file);
                        status.clean = false;
                    }
                }
                _ => {}
            }
        }

        assert_eq!(status.modified, vec!["src/lib.rs"]);
        assert_eq!(status.untracked, vec!["new_file.rs"]);
        assert_eq!(status.staged, vec!["staged.rs", "added.rs"]);
        assert!(!status.clean);
    }
}
