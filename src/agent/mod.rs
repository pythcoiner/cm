//! Agent spawning, prompt building, and response parsing.
//!
//! This module provides functionality for spawning Claude agents as subprocesses,
//! building prompts with task-specific context, and parsing JSON responses.

use std::io::{self, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use log::warn;
use thiserror::Error;

mod prompt;
mod response;

pub use prompt::PromptBuilder;
pub use response::{
    PhaseAgentResponse, PlanAgentResponse, ResponseParser, ReviewAgentResponse,
    ReviewIssueResponse, RunReviewAgentResponse, TaskCompletionInfo,
};

/// Interval between checks of a running agent or a pending rate limit wait.
const CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// Delay before respawning an agent after claude reported a rate limit.
const RATE_LIMIT_RETRY_DELAY: Duration = Duration::from_secs(30);

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

    /// The claude CLI flagged its result as an error (e.g. usage limit reached).
    #[error("claude CLI reported an error: {0}")]
    CliError(String),

    /// The claude CLI reported a rate limit (`api_error_status: 429`).
    #[error("claude CLI hit the rate limit: {0}")]
    RateLimited(String),

    /// Failed to write the prompt to the agent's stdin.
    #[error("failed to write prompt to stdin: {0}")]
    PromptWriteFailed(String),
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
    /// Global shutdown flag, checked while waiting out a rate limit.
    shutdown_flag: Arc<AtomicBool>,
}

impl AgentSpawner {
    /// Create a new agent spawner with the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The model identifier to use (e.g., "claude-sonnet-4-5-20250929")
    /// * `shutdown_flag` - Global shutdown flag, stops a pending rate limit retry
    pub fn new(model: String, shutdown_flag: Arc<AtomicBool>) -> Self {
        Self {
            model,
            shutdown_flag,
        }
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
    pub fn spawn(
        &self,
        prompt: &str,
        task_id: &str,
        agent_label: &str,
    ) -> Result<AgentHandle, AgentError> {
        let args = [
            "--print",
            "--output-format",
            "json",
            "--model",
            self.model.as_str(),
            "--dangerously-skip-permissions",
        ]
        .map(String::from)
        .to_vec();
        self.start(args, prompt, task_id, agent_label)
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
        let args = [
            "--print",
            "--continue",
            session_id,
            "--output-format",
            "json",
            "--dangerously-skip-permissions",
        ]
        .map(String::from)
        .to_vec();
        self.start(args, prompt, task_id, agent_label)
    }

    /// Spawn the claude process with `args` and run it in a separate thread.
    fn start(
        &self,
        args: Vec<String>,
        prompt: &str,
        task_id: &str,
        agent_label: &str,
    ) -> Result<AgentHandle, AgentError> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let started_at = Utc::now();
        let mut child = spawn_claude(&args)?;
        let prompt_writer = write_prompt(&mut child, prompt.to_string());

        let run = AgentRun {
            args,
            prompt: prompt.to_string(),
            stop_flag: stop_flag.clone(),
            shutdown_flag: self.shutdown_flag.clone(),
            task_id: task_id.to_string(),
            agent_label: agent_label.to_string(),
        };
        let thread_handle = thread::spawn(move || run.run(child, prompt_writer));

        Ok(AgentHandle {
            thread_handle: Some(thread_handle),
            stop_flag,
            task_id: task_id.to_string(),
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
    /// - The claude CLI flagged its result as an error (`AgentError::CliError`)
    /// - A stop or shutdown was requested during a rate limit wait (`AgentError::Interrupted`)
    /// - The prompt could not be written to stdin (`AgentError::PromptWriteFailed`)
    pub fn wait(mut self) -> Result<AgentOutput, AgentError> {
        let handle = self
            .thread_handle
            .take()
            .expect("thread handle should exist");

        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(AgentError::OutputError("agent thread panicked".to_string())),
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

/// Spawn the claude CLI with `args` and piped stdio.
fn spawn_claude(args: &[String]) -> Result<Child, AgentError> {
    Command::new("claude")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                AgentError::CliNotFound
            } else {
                AgentError::SpawnFailed(e.to_string())
            }
        })
}

/// Write the prompt on its own thread so an early claude exit cannot block or fail the spawn.
fn write_prompt(child: &mut Child, prompt: String) -> Option<JoinHandle<io::Result<()>>> {
    child
        .stdin
        .take()
        .map(|mut stdin| thread::spawn(move || stdin.write_all(prompt.as_bytes())))
}

/// Wait for the prompt writer thread and report a failed write.
fn join_prompt_writer(prompt_writer: Option<JoinHandle<io::Result<()>>>) -> Result<(), AgentError> {
    match prompt_writer.map(JoinHandle::join) {
        None | Some(Ok(Ok(()))) => Ok(()),
        Some(Ok(Err(e))) => Err(AgentError::PromptWriteFailed(e.to_string())),
        Some(Err(_)) => Err(AgentError::OutputError(
            "prompt writer thread panicked".to_string(),
        )),
    }
}

/// Everything needed to respawn the same agent step.
struct AgentRun {
    args: Vec<String>,
    prompt: String,
    stop_flag: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    task_id: String,
    agent_label: String,
}

impl AgentRun {
    /// Run the agent, respawning it after `RATE_LIMIT_RETRY_DELAY` while claude reports a rate limit.
    fn run(
        self,
        mut child: Child,
        mut prompt_writer: Option<JoinHandle<io::Result<()>>>,
    ) -> Result<AgentOutput, AgentError> {
        loop {
            match run_child(
                &mut child,
                prompt_writer,
                &self.stop_flag,
                &self.task_id,
                &self.agent_label,
            ) {
                Err(AgentError::RateLimited(msg)) => {
                    self.wait_rate_limit(&msg)?;
                    child = spawn_claude(&self.args)?;
                    prompt_writer = write_prompt(&mut child, self.prompt.clone());
                }
                result => return result,
            }
        }
    }

    /// Wait `RATE_LIMIT_RETRY_DELAY`, returning `Interrupted` if a stop or shutdown is requested.
    fn wait_rate_limit(&self, msg: &str) -> Result<(), AgentError> {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let line = format!(
            "[{now} {}] {} hit the rate limit: {msg}, retrying in {}s",
            self.agent_label,
            self.task_id,
            RATE_LIMIT_RETRY_DELAY.as_secs()
        );
        warn!("{line}");
        crate::log::tee_eprintln(&line);

        let start = Instant::now();
        while start.elapsed() < RATE_LIMIT_RETRY_DELAY {
            if self.stop_flag.load(Ordering::SeqCst) || self.shutdown_flag.load(Ordering::SeqCst) {
                return Err(AgentError::Interrupted);
            }
            thread::sleep(CHECK_INTERVAL);
        }
        Ok(())
    }
}

/// Run one claude process to completion, handling interruption.
fn run_child(
    child: &mut Child,
    prompt_writer: Option<JoinHandle<io::Result<()>>>,
    stop_flag: &AtomicBool,
    task_id: &str,
    agent_label: &str,
) -> Result<AgentOutput, AgentError> {
    let start = Instant::now();
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
            eprint!("\r[{now} {agent_label}] {task_id} running for {elapsed_secs}s...");
            std::io::stderr().flush().ok();
            last_progress_secs = elapsed_secs;
        }

        // Check if process has completed
        match child.try_wait() {
            Ok(Some(status)) => {
                let duration = start.elapsed();

                // Erase the progress ticker line (terminal only), then print
                // the clean completion line through tee so cm.log and the am
                // socket get it too.
                let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                eprint!("\r{:80}\r", "");
                std::io::stderr().flush().ok();
                crate::log::tee_eprintln(&format!(
                    "[{now} {agent_label}] {task_id} completed in {}s",
                    duration.as_secs()
                ));

                // Collect output from reader threads
                let stdout = stdout_thread
                    .map(|t| t.join().unwrap_or_default())
                    .unwrap_or_default();
                let stderr = stderr_thread
                    .map(|t| t.join().unwrap_or_default())
                    .unwrap_or_default();

                if let Some(err) = response::ResponseParser::extract_cli_error(&stdout) {
                    return Err(err);
                }
                join_prompt_writer(prompt_writer)?;

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
                thread::sleep(CHECK_INTERVAL);
            }
            Err(e) => {
                return Err(AgentError::OutputError(format!(
                    "failed to check process status: {e}"
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
        let spawner = AgentSpawner::new(
            "claude-sonnet-4-5-20250929".to_string(),
            Arc::new(AtomicBool::new(false)),
        );

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

        let err = AgentError::CliError("You've hit your limit".to_string());
        assert_eq!(
            err.to_string(),
            "claude CLI reported an error: You've hit your limit"
        );

        let err = AgentError::RateLimited("You've hit your limit".to_string());
        assert_eq!(
            err.to_string(),
            "claude CLI hit the rate limit: You've hit your limit"
        );

        let err = AgentError::PromptWriteFailed("Broken pipe (os error 32)".to_string());
        assert_eq!(
            err.to_string(),
            "failed to write prompt to stdin: Broken pipe (os error 32)"
        );
    }

    #[test]
    fn test_join_prompt_writer_ok() {
        assert!(join_prompt_writer(None).is_ok());

        let writer = thread::spawn(|| Ok(()));
        assert!(join_prompt_writer(Some(writer)).is_ok());
    }

    #[test]
    fn test_join_prompt_writer_failed_write() {
        let writer =
            thread::spawn(|| Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe")));

        match join_prompt_writer(Some(writer)) {
            Err(AgentError::PromptWriteFailed(msg)) => assert_eq!(msg, "broken pipe"),
            other => panic!("expected PromptWriteFailed, got {other:?}"),
        }
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

        let debug_str = format!("{output:?}");
        assert!(debug_str.contains("test output"));
        assert!(debug_str.contains("exit_code"));
        assert!(debug_str.contains("session_id"));
    }
}
