//! Build verification: cargo build/clippy/test and git operations.
//!
//! This module provides functionality for running cargo commands (build, clippy, test)
//! and git operations (status, add, commit) to verify and commit code changes.

use std::path::PathBuf;

use thiserror::Error;

mod cargo;
mod git;

pub use cargo::{BuildOutput, CargoRunner, CompilerMessage, MessageLevel, TestOutput};
pub use git::{CommitId, GitRunner, GitStatus};

/// Errors that can occur during build or git operations.
#[derive(Debug, Error)]
pub enum BuildError {
    /// A command failed with an exit code.
    #[error("command '{command}' failed with exit code {exit_code:?}: {stderr}")]
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    /// A required command was not found in PATH.
    #[error("command not found: {0}")]
    CommandNotFound(String),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// Failed to parse command output.
    #[error("failed to parse output: {0}")]
    ParseError(String),
}

/// Verifies that builds pass and clippy is clean.
///
/// The `BuildVerifier` wraps a `CargoRunner` and provides high-level
/// verification methods for ensuring code quality before commits.
pub struct BuildVerifier {
    cargo: CargoRunner,
}

impl BuildVerifier {
    /// Create a new `BuildVerifier` for the given working directory.
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            cargo: CargoRunner::new(working_dir),
        }
    }

    /// Verify that `cargo build` succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error if the build fails or if cargo cannot be executed.
    pub fn verify_build(&self) -> Result<BuildOutput, BuildError> {
        self.cargo.build()
    }

    /// Verify that `cargo clippy` passes without errors.
    ///
    /// # Errors
    ///
    /// Returns an error if clippy finds errors or if cargo cannot be executed.
    pub fn verify_clippy(&self) -> Result<BuildOutput, BuildError> {
        self.cargo.clippy()
    }

    /// Verify both build and clippy pass.
    ///
    /// Runs `cargo build` first, then `cargo clippy`. If either fails,
    /// returns the appropriate error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `cargo build` fails
    /// - `cargo clippy` finds errors
    pub fn verify_all(&self) -> Result<(), BuildError> {
        let build_output = self.verify_build()?;
        if !build_output.success {
            return Err(BuildError::CommandFailed {
                command: "cargo build".to_string(),
                exit_code: None,
                stderr: build_output.stderr,
            });
        }

        let clippy_output = self.verify_clippy()?;
        if !clippy_output.success {
            return Err(BuildError::CommandFailed {
                command: "cargo clippy".to_string(),
                exit_code: None,
                stderr: clippy_output.stderr,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_error_display() {
        let error = BuildError::CommandFailed {
            command: "cargo build".to_string(),
            exit_code: Some(1),
            stderr: "compilation error".to_string(),
        };
        assert!(error.to_string().contains("cargo build"));
        assert!(error.to_string().contains("compilation error"));

        let error = BuildError::CommandNotFound("cargo".to_string());
        assert!(error.to_string().contains("cargo"));

        let error = BuildError::ParseError("invalid output".to_string());
        assert!(error.to_string().contains("invalid output"));
    }
}
