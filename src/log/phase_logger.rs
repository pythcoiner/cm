//! Per-phase TRACE logging for detailed agent interaction tracking.
//!
//! Each phase gets its own log file in `.cm/logs/` directory containing full
//! prompts and responses for all agents within that phase. These are separate
//! from the main `cm.log` (operational debug log) and `LOG.md` (audit trail).

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use thiserror::Error;

/// Errors that can occur during phase logging.
#[derive(Debug, Error)]
pub enum PhaseLogError {
    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid task ID format.
    #[error("invalid task id: {0}")]
    InvalidTaskId(String),
}

/// Per-phase logger that writes full prompts and responses to separate files.
///
/// Each phase gets a file like `.cm/logs/phase-21.log` containing all agent
/// interactions for that phase. Thread-safe via Mutex.
pub struct PhaseLogger {
    /// Directory where phase logs are stored (`.cm/logs/`).
    logs_dir: PathBuf,
    /// Open file handles for each phase, keyed by phase ID.
    files: Mutex<HashMap<String, File>>,
}

impl PhaseLogger {
    /// Create a new PhaseLogger that writes to `.cm/logs/` directory.
    ///
    /// Creates the logs directory if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `cm_dir` - The `.cm` directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the logs directory cannot be created.
    pub fn new(cm_dir: &Path) -> Result<Self, PhaseLogError> {
        let logs_dir = cm_dir.join("logs");
        fs::create_dir_all(&logs_dir)?;

        Ok(Self {
            logs_dir,
            files: Mutex::new(HashMap::new()),
        })
    }

    /// Get or create a file handle for a given phase.
    ///
    /// # Arguments
    ///
    /// * `phase_id` - The phase ID (e.g., "phase-21")
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or created.
    fn get_or_create_file(&self, phase_id: &str) -> Result<(), PhaseLogError> {
        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());

        if !files.contains_key(phase_id) {
            let file_path = self.logs_dir.join(format!("{}.log", phase_id));
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(file_path)?;
            files.insert(phase_id.to_string(), file);
        }

        Ok(())
    }

    /// Write content to a phase log file.
    ///
    /// # Arguments
    ///
    /// * `phase_id` - The phase ID
    /// * `content` - The content to write
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn write_to_phase(&self, phase_id: &str, content: &str) -> Result<(), PhaseLogError> {
        self.get_or_create_file(phase_id)?;

        let mut files = self.files.lock().unwrap_or_else(|e| e.into_inner());
        let file = files.get_mut(phase_id).expect("file should exist");

        writeln!(file, "{}", content)?;
        file.flush()?;

        Ok(())
    }

    /// Log a prompt sent to an agent.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID (e.g., "phase-21.task-1")
    /// * `agent_type` - The type of agent (e.g., "IMPLEM", "REVIEW")
    /// * `prompt` - The full prompt text
    ///
    /// # Errors
    ///
    /// Returns an error if the task ID is invalid or writing fails.
    pub fn log_prompt(
        &self,
        task_id: &str,
        agent_type: &str,
        prompt: &str,
    ) -> Result<(), PhaseLogError> {
        let phase_id = extract_phase_id(task_id)
            .ok_or_else(|| PhaseLogError::InvalidTaskId(task_id.to_string()))?;

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
        let separator = "=".repeat(80);

        let content = format!(
            "{}\n[{}] PROMPT: {} for {}\n{}\n\n{}\n",
            separator, timestamp, agent_type, task_id, separator, prompt
        );

        self.write_to_phase(phase_id, &content)
    }

    /// Log a response from an agent.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID (e.g., "phase-21.task-1")
    /// * `agent_type` - The type of agent (e.g., "IMPLEM", "REVIEW")
    /// * `response` - The full response text
    /// * `duration_secs` - How long the agent took to respond
    /// * `exit_code` - The exit code from the agent process (if available)
    ///
    /// # Errors
    ///
    /// Returns an error if the task ID is invalid or writing fails.
    pub fn log_response(
        &self,
        task_id: &str,
        agent_type: &str,
        response: &str,
        duration_secs: u64,
        exit_code: Option<i32>,
    ) -> Result<(), PhaseLogError> {
        let phase_id = extract_phase_id(task_id)
            .ok_or_else(|| PhaseLogError::InvalidTaskId(task_id.to_string()))?;

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
        let separator = "=".repeat(80);

        let exit_code_str = exit_code
            .map(|code| format!("Exit Code: {}", code))
            .unwrap_or_else(|| "Exit Code: N/A".to_string());

        let content = format!(
            "{}\n[{}] RESPONSE: {} for {} (Duration: {}s, {})\n{}\n\n{}\n",
            separator, timestamp, agent_type, task_id, duration_secs, exit_code_str, separator, response
        );

        self.write_to_phase(phase_id, &content)
    }
}

/// Extract phase ID from a task ID.
///
/// Handles task IDs like:
/// - `phase-21.task-1` -> Some("phase-21")
/// - `phase-0.5.task-1` -> Some("phase-0.5")
/// - `invalid` -> None
///
/// Uses `rfind(".task-")` to find the split point between phase and task.
///
/// # Arguments
///
/// * `task_id` - The task ID to parse
///
/// # Returns
///
/// The phase ID if the task ID is valid, None otherwise.
pub fn extract_phase_id(task_id: &str) -> Option<&str> {
    let split_index = task_id.rfind(".task-")?;
    let phase_id = &task_id[..split_index];

    // Basic validation: phase ID should start with "phase-"
    if phase_id.starts_with("phase-") {
        Some(phase_id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_phase_id_basic() {
        assert_eq!(extract_phase_id("phase-21.task-1"), Some("phase-21"));
        assert_eq!(extract_phase_id("phase-1.task-5"), Some("phase-1"));
        assert_eq!(extract_phase_id("phase-100.task-99"), Some("phase-100"));
    }

    #[test]
    fn test_extract_phase_id_with_decimal() {
        assert_eq!(extract_phase_id("phase-0.5.task-1"), Some("phase-0.5"));
        assert_eq!(extract_phase_id("phase-1.2.task-3"), Some("phase-1.2"));
        assert_eq!(
            extract_phase_id("phase-10.5.task-10"),
            Some("phase-10.5")
        );
    }

    #[test]
    fn test_extract_phase_id_multiple_task_separators() {
        // Should use rfind, so it gets the rightmost .task-
        assert_eq!(
            extract_phase_id("phase-1.task-2.task-3"),
            Some("phase-1.task-2")
        );
    }

    #[test]
    fn test_extract_phase_id_invalid() {
        assert_eq!(extract_phase_id("invalid"), None);
        assert_eq!(extract_phase_id("task-1"), None);
        assert_eq!(extract_phase_id("phase-21"), None);
        assert_eq!(extract_phase_id("notphase-1.task-1"), None);
        assert_eq!(extract_phase_id(""), None);
    }

    #[test]
    fn test_new_creates_logs_directory() {
        let temp_dir = TempDir::new().unwrap();
        let cm_dir = temp_dir.path();

        let logger = PhaseLogger::new(cm_dir).unwrap();

        let logs_dir = cm_dir.join("logs");
        assert!(logs_dir.exists());
        assert!(logs_dir.is_dir());
        assert_eq!(logger.logs_dir, logs_dir);
    }

    #[test]
    fn test_log_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        logger
            .log_prompt("phase-1.task-1", "IMPLEM", "Implement feature X")
            .unwrap();

        let log_file = temp_dir.path().join("logs/phase-1.log");
        assert!(log_file.exists());

        let content = fs::read_to_string(log_file).unwrap();
        assert!(content.contains("PROMPT: IMPLEM for phase-1.task-1"));
        assert!(content.contains("Implement feature X"));
        assert!(content.contains("===="));
    }

    #[test]
    fn test_log_response() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        logger
            .log_response(
                "phase-2.task-3",
                "REVIEW",
                "Code looks good",
                45,
                Some(0),
            )
            .unwrap();

        let log_file = temp_dir.path().join("logs/phase-2.log");
        assert!(log_file.exists());

        let content = fs::read_to_string(log_file).unwrap();
        assert!(content.contains("RESPONSE: REVIEW for phase-2.task-3"));
        assert!(content.contains("Code looks good"));
        assert!(content.contains("Duration: 45s"));
        assert!(content.contains("Exit Code: 0"));
    }

    #[test]
    fn test_log_response_no_exit_code() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        logger
            .log_response("phase-3.task-1", "FIX", "Fixed the issue", 30, None)
            .unwrap();

        let log_file = temp_dir.path().join("logs/phase-3.log");
        let content = fs::read_to_string(log_file).unwrap();

        assert!(content.contains("Exit Code: N/A"));
    }

    #[test]
    fn test_multiple_entries_same_phase() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        logger
            .log_prompt("phase-1.task-1", "IMPLEM", "First prompt")
            .unwrap();
        logger
            .log_response("phase-1.task-1", "IMPLEM", "First response", 10, Some(0))
            .unwrap();
        logger
            .log_prompt("phase-1.task-2", "IMPLEM", "Second prompt")
            .unwrap();

        let log_file = temp_dir.path().join("logs/phase-1.log");
        let content = fs::read_to_string(log_file).unwrap();

        assert!(content.contains("First prompt"));
        assert!(content.contains("First response"));
        assert!(content.contains("Second prompt"));
    }

    #[test]
    fn test_multiple_phases() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        logger
            .log_prompt("phase-1.task-1", "IMPLEM", "Phase 1 content")
            .unwrap();
        logger
            .log_prompt("phase-2.task-1", "IMPLEM", "Phase 2 content")
            .unwrap();

        let log1 = temp_dir.path().join("logs/phase-1.log");
        let log2 = temp_dir.path().join("logs/phase-2.log");

        assert!(log1.exists());
        assert!(log2.exists());

        let content1 = fs::read_to_string(log1).unwrap();
        let content2 = fs::read_to_string(log2).unwrap();

        assert!(content1.contains("Phase 1 content"));
        assert!(!content1.contains("Phase 2 content"));
        assert!(content2.contains("Phase 2 content"));
        assert!(!content2.contains("Phase 1 content"));
    }

    #[test]
    fn test_invalid_task_id_error() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        let result = logger.log_prompt("invalid-task-id", "IMPLEM", "Test prompt");
        assert!(result.is_err());

        match result {
            Err(PhaseLogError::InvalidTaskId(id)) => {
                assert_eq!(id, "invalid-task-id");
            }
            _ => panic!("Expected InvalidTaskId error"),
        }
    }

    #[test]
    fn test_concurrent_logging() {
        use std::sync::Arc;
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let logger = Arc::new(PhaseLogger::new(temp_dir.path()).unwrap());

        let mut handles = vec![];
        for i in 0..10 {
            let logger = Arc::clone(&logger);
            handles.push(thread::spawn(move || {
                logger
                    .log_prompt(
                        "phase-1.task-1",
                        "IMPLEM",
                        &format!("Prompt from thread {}", i),
                    )
                    .unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let log_file = temp_dir.path().join("logs/phase-1.log");
        let content = fs::read_to_string(log_file).unwrap();

        // Should have 10 entries
        assert_eq!(content.matches("PROMPT: IMPLEM").count(), 10);
    }
}
