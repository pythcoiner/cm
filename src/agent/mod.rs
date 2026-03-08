//! Agent spawning, prompt building, and response parsing.
//!
//! This module provides functionality for spawning Claude agents as subprocesses,
//! building prompts with task-specific context, and parsing JSON responses.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use thiserror::Error;

mod prompt;
mod response;

pub use prompt::PromptBuilder;
pub use response::{PhaseAgentResponse, PlanAgentResponse, ResponseParser, ReviewAgentResponse, ReviewIssueResponse, TaskCompletionInfo};

/// Errors that can occur during agent operations.
#[derive(Debug, Error)]
pub enum AgentError {
    /// The claude CLI was not found in PATH.
    #[error("claude CLI not found - ensure it is installed and in PATH")]
    CliNotFound,

    /// Failed to spawn the agent process.
    #[error("failed to spawn agent: {0}")]
    SpawnFailed(String),

    /// Failed to read output from the agent.
    #[error("failed to read agent output: {0}")]
    OutputError(String),

    /// The agent was interrupted.
    #[error("agent was interrupted")]
    Interrupted,

    /// Failed to parse the agent response.
    #[error("failed to parse response: {0}")]
    ParseError(String),
}

/// Output from an agent execution.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// Standard output from the agent.
    pub stdout: String,
    /// Standard error from the agent.
    pub stderr: String,
    /// Exit code of the agent process (None if killed/not available).
    pub exit_code: Option<i32>,
    /// Duration of the agent execution.
    pub duration: Duration,
    /// Session ID from the Claude CLI response (for --continue).
    pub session_id: Option<String>,
}

/// Spawns and manages Claude agent processes.
///
/// The spawner configures agent execution parameters like model,
/// then spawns agent processes that run in separate threads.
pub struct AgentSpawner {
    /// Model to use for the agent.
    model: String,
}

impl AgentSpawner {
    /// Create a new agent spawner with the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier to use (e.g., "claude-sonnet-4-5-20250929")
    pub fn new(model: String) -> Self {
        Self { model }
    }

    /// Spawn a new agent process with the given prompt.
    ///
    /// The agent runs in a separate thread and can be managed via the returned handle.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to send to the agent
    /// * `task_id` - The ID of the task this agent is executing
    /// * `agent_label` - The label to use in progress logs
    ///
    /// # Errors
    ///
    /// Returns `AgentError::CliNotFound` if the claude CLI is not found.
    /// Returns `AgentError::SpawnFailed` if the process cannot be started.
    pub fn spawn(&self, prompt: &str, task_id: &str, agent_label: &str) -> Result<AgentHandle, AgentError> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let started_at = Utc::now();
        let task_id_owned = task_id.to_string();
        let prompt_owned = prompt.to_string();
        let model_owned = self.model.clone();
        let stop_flag_clone = stop_flag.clone();
        let agent_label_owned = agent_label.to_string();

        // Spawn the child process
        let mut child = Command::new("claude")
            .args([
                "--output-format",
                "json",
                "--model",
                &model_owned,
                "--dangerously-skip-permissions",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    AgentError::CliNotFound
                } else {
                    AgentError::SpawnFailed(e.to_string())
                }
            })?;

        // Write prompt to stdin then close it so claude sees EOF
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt_owned.as_bytes()).map_err(|e| {
                AgentError::SpawnFailed(format!("Failed to write prompt to stdin: {}", e))
            })?;
        }

        // Run the process in a separate thread
        let task_id_for_thread = task_id_owned.clone();
        let thread_handle = thread::spawn(move || {
            run_agent_thread(&mut child, stop_flag_clone, &task_id_for_thread, &agent_label_owned)
        });

        Ok(AgentHandle {
            thread_handle: Some(thread_handle),
            stop_flag,
            task_id: task_id_owned,
            started_at,
        })
    }

    /// Spawn a new agent process that continues an existing conversation.
    ///
    /// This is used to retry after a parse failure, asking the agent
    /// to provide a properly formatted response.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID from the previous agent run
    /// * `prompt` - The follow-up prompt (e.g., asking for proper format)
    /// * `task_id` - The ID of the task this agent is executing
    /// * `agent_label` - The label to use in progress logs
    pub fn spawn_with_continue(
        &self,
        session_id: &str,
        prompt: &str,
        task_id: &str,
        agent_label: &str,
    ) -> Result<AgentHandle, AgentError> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let started_at = Utc::now();
        let task_id_owned = task_id.to_string();
        let prompt_owned = prompt.to_string();
        let session_id_owned = session_id.to_string();
        let stop_flag_clone = stop_flag.clone();
        let agent_label_owned = agent_label.to_string();

        // Spawn the child process with --continue flag
        let mut child = Command::new("claude")
            .args([
                "--continue",
                &session_id_owned,
                "--output-format",
                "json",
                "--dangerously-skip-permissions",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    AgentError::CliNotFound
                } else {
                    AgentError::SpawnFailed(e.to_string())
                }
            })?;

        // Write prompt to stdin then close it so claude sees EOF
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt_owned.as_bytes()).map_err(|e| {
                AgentError::SpawnFailed(format!("Failed to write prompt to stdin: {}", e))
            })?;
        }

        // Run the process in a separate thread
        let task_id_for_thread = task_id_owned.clone();
        let thread_handle = thread::spawn(move || {
            run_agent_thread(&mut child, stop_flag_clone, &task_id_for_thread, &agent_label_owned)
        });

        Ok(AgentHandle {
            thread_handle: Some(thread_handle),
            stop_flag,
            task_id: task_id_owned,
            started_at,
        })
    }

    /// Get the model being used by this spawner.
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Handle to a running agent process.
///
/// Provides methods to wait for completion or interrupt the agent.
pub struct AgentHandle {
    /// Handle to the thread running the agent.
    thread_handle: Option<JoinHandle<Result<AgentOutput, AgentError>>>,
    /// Flag to signal the agent should stop.
    stop_flag: Arc<AtomicBool>,
    /// ID of the task this agent is executing.
    task_id: String,
    /// When the agent was started.
    started_at: DateTime<Utc>,
}

impl AgentHandle {
    /// Wait for the agent to complete and return its output.
    ///
    /// This method blocks until the agent finishes or is interrupted.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The agent is interrupted (`AgentError::Interrupted`)
    /// - Output cannot be read (`AgentError::OutputError`)
    pub fn wait(mut self) -> Result<AgentOutput, AgentError> {
        let handle = self
            .thread_handle
            .take()
            .expect("thread handle should exist");

        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(AgentError::OutputError(
                "agent thread panicked".to_string(),
            )),
        }
    }

    /// Signal the agent to stop.
    ///
    /// This sets the stop flag which the agent thread checks periodically.
    /// The process will be killed on the next check.
    pub fn interrupt(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    /// Get the task ID this agent is executing.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Get when the agent was started.
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }
}

/// Run the agent in a thread, handling interruption.
fn run_agent_thread(
    child: &mut Child,
    stop_flag: Arc<AtomicBool>,
    task_id: &str,
    agent_label: &str,
) -> Result<AgentOutput, AgentError> {
    let start = Instant::now();
    let check_interval = std::time::Duration::from_millis(100);
    let mut last_progress_secs = 0u64;

    // Spawn reader threads IMMEDIATELY to avoid pipe buffer overflow
    // This reads stdout/stderr as data arrives, preventing truncation
    let stdout_thread = child.stdout.take().map(|mut handle| {
        thread::spawn(move || {
            let mut output = String::new();
            let _ = handle.read_to_string(&mut output);
            output
        })
    });

    let stderr_thread = child.stderr.take().map(|mut handle| {
        thread::spawn(move || {
            let mut output = String::new();
            let _ = handle.read_to_string(&mut output);
            output
        })
    });

    // Wait for process to complete, checking for stop flag
    loop {
        // Check if we should stop
        if stop_flag.load(Ordering::SeqCst) {
            eprint!("\r{:80}\r", "");
            std::io::stderr().flush().ok();
            let _ = child.kill();
            let _ = child.wait();
            return Err(AgentError::Interrupted);
        }

        // Update progress every second
        let elapsed_secs = start.elapsed().as_secs();
        if elapsed_secs > last_progress_secs {
            let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
            eprint!("\r[{} {}] {} running for {}s...", now, agent_label, task_id, elapsed_secs);
            std::io::stderr().flush().ok();
            last_progress_secs = elapsed_secs;
        }

        // Check if process has completed
        match child.try_wait() {
            Ok(Some(status)) => {
                let duration = start.elapsed();

                // Clear progress line and print completion
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                eprintln!(
                    "\r[{} {}] {} completed in {}s              ",
                    now,
                    agent_label,
                    task_id,
                    duration.as_secs()
                );

                // Collect output from reader threads
                let stdout = stdout_thread
                    .map(|t| t.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_thread
                    .map(|t| t.join().unwrap_or_default())
                    .unwrap_or_default();

                // Extract session_id from stdout if available
                let session_id = response::ResponseParser::extract_session_id(&stdout);

                return Ok(AgentOutput {
                    stdout,
                    stderr,
                    exit_code: status.code(),
                    duration,
                    session_id,
                });
            }
            Ok(None) => {
                thread::sleep(check_interval);
            }
            Err(e) => {
                return Err(AgentError::OutputError(format!(
                    "failed to check process status: {}",
                    e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_agent_spawner_creation() {
        let spawner = AgentSpawner::new("claude-sonnet-4-5-20250929".to_string());

        assert_eq!(spawner.model(), "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::CliNotFound;
        assert!(err.to_string().contains("claude CLI not found"));

        let err = AgentError::SpawnFailed("permission denied".to_string());
        assert!(err.to_string().contains("permission denied"));

        let err = AgentError::Interrupted;
        assert!(err.to_string().contains("interrupted"));

        let err = AgentError::ParseError("invalid json".to_string());
        assert!(err.to_string().contains("invalid json"));
    }

    #[test]
    fn test_agent_output_debug() {
        let output = AgentOutput {
            stdout: "test output".to_string(),
            stderr: "".to_string(),
            exit_code: Some(0),
            duration: Duration::from_secs(1),
            session_id: Some("test-session".to_string()),
        };

        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("test output"));
        assert!(debug_str.contains("exit_code"));
        assert!(debug_str.contains("session_id"));
    }
}
