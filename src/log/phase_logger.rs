//! Per-phase logging for detailed agent interaction tracking.
//!
//! Each phase gets its own log file in `.cm/logs/` directory containing full
//! prompts and responses for all agents within that phase.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

/// Statistics from a log pruning operation.
#[derive(Debug, Clone)]
pub struct PruneStats {
    /// Number of log lines removed.
    pub removed_count: usize,
    /// Number of log lines kept.
    pub kept_count: usize,
}

/// Parse the timestamp from a log line.
///
/// Expected format: `[YYYY-MM-DD HH:MM:SS.mmm] ...`
/// Returns None if the line doesn't match the expected format.
pub fn parse_log_timestamp(line: &str) -> Option<DateTime<Utc>> {
    if !line.starts_with('[') {
        return None;
    }

    let end_bracket = line.find(']')?;
    let timestamp_str = &line[1..end_bracket];

    chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.3f")
        .ok()
        .map(|naive| naive.and_utc())
}

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

    /// Prune all phase log files older than 24 hours.
    ///
    /// Iterates over all `.log` files in `.cm/logs/` and applies the same
    /// 24-hour pruning logic as the main `cm.log` file.
    ///
    /// # Arguments
    ///
    /// * `logs_dir` - The logs directory (`.cm/logs/`)
    ///
    /// # Returns
    ///
    /// Combined statistics from all pruned files.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the directory or pruning fails.
    pub fn prune_all(logs_dir: &Path) -> Result<PruneStats, PhaseLogError> {
        let duration = Duration::hours(24);
        let cutoff = Utc::now() - duration;

        let mut total_removed = 0;
        let mut total_kept = 0;

        // Ensure directory exists
        if !logs_dir.exists() {
            return Ok(PruneStats {
                removed_count: 0,
                kept_count: 0,
            });
        }

        // Iterate over all .log files in the directory
        for entry in fs::read_dir(logs_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip if not a file or not a .log file
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            // Prune this file
            let stats = prune_log_file(&path, cutoff)?;
            total_removed += stats.removed_count;
            total_kept += stats.kept_count;
        }

        Ok(PruneStats {
            removed_count: total_removed,
            kept_count: total_kept,
        })
    }
}

/// Prune a single log file, removing entries older than the cutoff.
///
/// Lines with unparseable timestamps are kept (conservative approach).
fn prune_log_file(
    path: &Path,
    cutoff: chrono::DateTime<Utc>,
) -> Result<PruneStats, PhaseLogError> {
    let reader = BufReader::new(File::open(path)?);
    let mut kept_lines = Vec::new();
    let mut removed_count = 0;

    for line in reader.lines() {
        let line = line?;
        match parse_log_timestamp(&line) {
            Some(ts) if ts < cutoff => {
                removed_count += 1;
            }
            _ => {
                kept_lines.push(line);
            }
        }
    }

    let kept_count = kept_lines.len();

    // Rewrite the file with only kept lines
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)?;
    for line in &kept_lines {
        writeln!(file, "{}", line)?;
    }
    file.flush()?;

    Ok(PruneStats {
        removed_count,
        kept_count,
    })
}

/// Extract phase ID from a task ID or phase ID.
///
/// Handles both:
/// - Task IDs: `phase-21.task-1` -> Some("phase-21")
/// - Phase IDs: `phase-21` -> Some("phase-21")
/// - Decimal phases: `phase-0.5.task-1` -> Some("phase-0.5"), `phase-0.5` -> Some("phase-0.5")
/// - Invalid: `invalid` -> None
///
/// # Arguments
///
/// * `id` - The task ID or phase ID to parse
///
/// # Returns
///
/// The phase ID if valid, None otherwise.
pub fn extract_phase_id(id: &str) -> Option<&str> {
    // Must start with "phase-"
    if !id.starts_with("phase-") {
        return None;
    }

    // If it contains ".task-", extract the phase part
    if let Some(split_index) = id.rfind(".task-") {
        return Some(&id[..split_index]);
    }

    // Otherwise it's already a phase ID (e.g., "phase-27" or "phase-0.5")
    Some(id)
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
    fn test_extract_phase_id_direct() {
        // Phase IDs passed directly (not as part of task ID) should work
        assert_eq!(extract_phase_id("phase-21"), Some("phase-21"));
        assert_eq!(extract_phase_id("phase-1"), Some("phase-1"));
        assert_eq!(extract_phase_id("phase-0.5"), Some("phase-0.5"));
    }

    #[test]
    fn test_extract_phase_id_invalid() {
        assert_eq!(extract_phase_id("invalid"), None);
        assert_eq!(extract_phase_id("task-1"), None);
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

    #[test]
    fn test_prune_all_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        let stats = PhaseLogger::prune_all(&logs_dir).unwrap();
        assert_eq!(stats.removed_count, 0);
        assert_eq!(stats.kept_count, 0);
    }

    #[test]
    fn test_prune_all_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("nonexistent");

        let stats = PhaseLogger::prune_all(&logs_dir).unwrap();
        assert_eq!(stats.removed_count, 0);
        assert_eq!(stats.kept_count, 0);
    }

    #[test]
    fn test_prune_all_keeps_recent() {
        let temp_dir = TempDir::new().unwrap();
        let logger = PhaseLogger::new(temp_dir.path()).unwrap();

        // Add recent entries to multiple phases
        logger
            .log_prompt("phase-1.task-1", "IMPLEM", "Recent prompt 1")
            .unwrap();
        logger
            .log_prompt("phase-2.task-1", "IMPLEM", "Recent prompt 2")
            .unwrap();

        let logs_dir = temp_dir.path().join("logs");
        let stats = PhaseLogger::prune_all(&logs_dir).unwrap();

        // All recent entries should be kept (no removal)
        assert_eq!(stats.removed_count, 0);
        // Phase logger writes multi-line entries, so we just verify no lines were removed
    }

    #[test]
    fn test_prune_all_removes_old() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        // Write old entries directly with past timestamps (format matches parse_log_timestamp)
        let phase1_log = logs_dir.join("phase-1.log");
        let mut file1 = File::create(&phase1_log).unwrap();
        writeln!(
            file1,
            "[2020-01-01 00:00:00.000] Old line 1"
        )
        .unwrap();
        writeln!(
            file1,
            "[2020-01-01 00:00:01.000] Old line 2"
        )
        .unwrap();
        drop(file1);

        let phase2_log = logs_dir.join("phase-2.log");
        let mut file2 = File::create(&phase2_log).unwrap();
        writeln!(
            file2,
            "[2020-01-01 00:00:00.000] Old line 3"
        )
        .unwrap();
        drop(file2);

        let stats = PhaseLogger::prune_all(&logs_dir).unwrap();

        // Should remove 3 old lines, keep 0
        assert_eq!(stats.removed_count, 3);
        assert_eq!(stats.kept_count, 0);

        // Verify files are empty
        let content1 = fs::read_to_string(&phase1_log).unwrap();
        assert!(content1.is_empty());

        let content2 = fs::read_to_string(&phase2_log).unwrap();
        assert!(content2.is_empty());
    }

    #[test]
    fn test_prune_all_ignores_non_log_files() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        // Create a non-.log file
        let other_file = logs_dir.join("readme.txt");
        fs::write(&other_file, "This is a readme").unwrap();

        // Create a .log file with old timestamp
        let log_file = logs_dir.join("phase-1.log");
        fs::write(&log_file, "[2020-01-01 00:00:00.000] Old line\n").unwrap();

        let stats = PhaseLogger::prune_all(&logs_dir).unwrap();

        // Should only process .log files (1 old line removed)
        assert_eq!(stats.removed_count, 1);
        assert_eq!(stats.kept_count, 0);

        // Non-.log file should be untouched
        assert!(other_file.exists());
        let content = fs::read_to_string(&other_file).unwrap();
        assert_eq!(content, "This is a readme");
    }
}
