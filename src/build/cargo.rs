//! Cargo command runner for build verification.
//!
//! This module provides functionality for running cargo commands (build, clippy, test)
//! and parsing their output for errors and warnings.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::BuildError;

/// Runs cargo commands in a specific working directory.
pub struct CargoRunner {
    working_dir: PathBuf,
}

/// Output from a cargo build or clippy command.
#[derive(Debug, Clone)]
pub struct BuildOutput {
    /// Whether the command succeeded (exit code 0).
    pub success: bool,
    /// Compiler errors found in the output.
    pub errors: Vec<CompilerMessage>,
    /// Compiler warnings found in the output.
    pub warnings: Vec<CompilerMessage>,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

/// A message from the Rust compiler.
#[derive(Debug, Clone)]
pub struct CompilerMessage {
    /// The severity level of the message.
    pub level: MessageLevel,
    /// The message text.
    pub message: String,
    /// The location of the issue (file:line:col format), if available.
    pub location: Option<String>,
}

/// Severity level of a compiler message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// A compiler error that prevents successful compilation.
    Error,
    /// A compiler warning that does not prevent compilation.
    Warning,
}

/// Output from a cargo test command.
#[derive(Debug, Clone)]
pub struct TestOutput {
    /// Whether all tests passed.
    pub success: bool,
    /// Number of tests that passed.
    pub passed: u32,
    /// Number of tests that failed.
    pub failed: u32,
    /// Number of tests that were ignored.
    pub ignored: u32,
    /// Standard output from the test run.
    pub stdout: String,
    /// Standard error from the test run.
    pub stderr: String,
}

impl CargoRunner {
    /// Create a new `CargoRunner` for the given working directory.
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// Run `cargo build` and return the output.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cargo command is not found
    /// - An I/O error occurs when spawning the process
    pub fn build(&self) -> Result<BuildOutput, BuildError> {
        self.run_cargo_command(&["build"])
    }

    /// Run `cargo clippy` and return the output.
    ///
    /// Runs clippy with `-D warnings` to treat warnings as errors.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cargo command is not found
    /// - An I/O error occurs when spawning the process
    pub fn clippy(&self) -> Result<BuildOutput, BuildError> {
        self.run_cargo_command(&["clippy", "--", "-D", "warnings"])
    }

    /// Run `cargo test` and return the test results.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The cargo command is not found
    /// - An I/O error occurs when spawning the process
    pub fn test(&self) -> Result<TestOutput, BuildError> {
        let output = Command::new("cargo")
            .args(["test"])
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::CommandNotFound("cargo".to_string())
                } else {
                    BuildError::IoError(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (passed, failed, ignored) = Self::parse_test_results(&stdout, &stderr);

        Ok(TestOutput {
            success: output.status.success(),
            passed,
            failed,
            ignored,
            stdout,
            stderr,
        })
    }

    /// Run a cargo command with the given arguments.
    fn run_cargo_command(&self, args: &[&str]) -> Result<BuildOutput, BuildError> {
        let output = Command::new("cargo")
            .args(args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::CommandNotFound("cargo".to_string())
                } else {
                    BuildError::IoError(e)
                }
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (errors, warnings) = Self::parse_compiler_messages(&stderr);

        Ok(BuildOutput {
            success: output.status.success(),
            errors,
            warnings,
            stdout,
            stderr,
        })
    }

    /// Parse compiler messages from stderr.
    ///
    /// Looks for lines starting with "error" or "warning" and extracts
    /// the message and location information.
    fn parse_compiler_messages(stderr: &str) -> (Vec<CompilerMessage>, Vec<CompilerMessage>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for line in stderr.lines() {
            let trimmed = line.trim();

            // Parse error lines: "error[E0432]: unresolved import" or "error: message"
            if trimmed.starts_with("error[") || trimmed.starts_with("error:") {
                let message = Self::extract_message(trimmed, "error");
                errors.push(CompilerMessage {
                    level: MessageLevel::Error,
                    message,
                    location: None,
                });
            }
            // Parse warning lines: "warning: unused variable" or "warning[...]"
            else if trimmed.starts_with("warning[") || trimmed.starts_with("warning:") {
                let message = Self::extract_message(trimmed, "warning");
                warnings.push(CompilerMessage {
                    level: MessageLevel::Warning,
                    message,
                    location: None,
                });
            }
            // Parse location lines: " --> src/main.rs:10:5"
            else if trimmed.starts_with("-->") {
                let location = trimmed.trim_start_matches("-->").trim().to_string();
                // Attach location to the most recent message
                if let Some(last_error) = errors.last_mut() {
                    if last_error.location.is_none() {
                        last_error.location = Some(location.clone());
                    }
                }
                if let Some(last_warning) = warnings.last_mut() {
                    if last_warning.location.is_none() {
                        last_warning.location = Some(location);
                    }
                }
            }
        }

        (errors, warnings)
    }

    /// Extract the message part from an error/warning line.
    fn extract_message(line: &str, prefix: &str) -> String {
        // Handle "error[E0432]: message" format
        if let Some(bracket_pos) = line.find('[') {
            if let Some(colon_pos) = line[bracket_pos..].find(':') {
                return line[bracket_pos + colon_pos + 1..].trim().to_string();
            }
        }
        // Handle "error: message" format
        let prefix_with_colon = format!("{}:", prefix);
        if let Some(pos) = line.find(&prefix_with_colon) {
            return line[pos + prefix_with_colon.len()..].trim().to_string();
        }
        line.to_string()
    }

    /// Parse test results from the output.
    ///
    /// Looks for lines like "test result: ok. 5 passed; 0 failed; 2 ignored"
    fn parse_test_results(stdout: &str, stderr: &str) -> (u32, u32, u32) {
        let combined = format!("{}\n{}", stdout, stderr);

        for line in combined.lines() {
            if line.contains("test result:") {
                let mut passed = 0u32;
                let mut failed = 0u32;
                let mut ignored = 0u32;

                // Parse "N passed"
                if let Some(pos) = line.find(" passed") {
                    if let Some(start) = line[..pos].rfind(|c: char| !c.is_ascii_digit()) {
                        if let Ok(n) = line[start + 1..pos].parse() {
                            passed = n;
                        }
                    }
                }

                // Parse "N failed"
                if let Some(pos) = line.find(" failed") {
                    if let Some(start) = line[..pos].rfind(|c: char| !c.is_ascii_digit()) {
                        if let Ok(n) = line[start + 1..pos].parse() {
                            failed = n;
                        }
                    }
                }

                // Parse "N ignored"
                if let Some(pos) = line.find(" ignored") {
                    if let Some(start) = line[..pos].rfind(|c: char| !c.is_ascii_digit()) {
                        if let Ok(n) = line[start + 1..pos].parse() {
                            ignored = n;
                        }
                    }
                }

                return (passed, failed, ignored);
            }
        }

        (0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compiler_messages() {
        let stderr = r#"
error[E0432]: unresolved import `foo`
 --> src/main.rs:1:5
  |
1 | use foo;
  |     ^^^ no `foo` in the root

warning: unused variable: `x`
 --> src/main.rs:5:9
  |
5 |     let x = 1;
  |         ^ help: if this is intentional, prefix it with an underscore: `_x`
"#;

        let (errors, warnings) = CargoRunner::parse_compiler_messages(stderr);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].level, MessageLevel::Error);
        assert!(errors[0].message.contains("unresolved import"));
        assert_eq!(errors[0].location, Some("src/main.rs:1:5".to_string()));

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].level, MessageLevel::Warning);
        assert!(warnings[0].message.contains("unused variable"));
        assert_eq!(warnings[0].location, Some("src/main.rs:5:9".to_string()));
    }

    #[test]
    fn test_parse_test_results() {
        let stdout = "test result: ok. 10 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out";
        let (passed, failed, ignored) = CargoRunner::parse_test_results(stdout, "");

        assert_eq!(passed, 10);
        assert_eq!(failed, 2);
        assert_eq!(ignored, 3);
    }

    #[test]
    fn test_parse_test_results_no_match() {
        let (passed, failed, ignored) = CargoRunner::parse_test_results("no results", "");
        assert_eq!(passed, 0);
        assert_eq!(failed, 0);
        assert_eq!(ignored, 0);
    }

    #[test]
    fn test_message_level_equality() {
        assert_eq!(MessageLevel::Error, MessageLevel::Error);
        assert_eq!(MessageLevel::Warning, MessageLevel::Warning);
        assert_ne!(MessageLevel::Error, MessageLevel::Warning);
    }
}
