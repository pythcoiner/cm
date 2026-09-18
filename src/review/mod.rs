//! Log gathering for the post-run review agent.
//!
//! Provides functions to read per-phase log files from `.cm/logs/` and format
//! them into a single string suitable for embedding in a review prompt.

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::log::parse_log_timestamp;

/// Errors that can occur during review log gathering.
#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

/// Gathered log content for one phase.
pub struct PhaseLogContent {
    pub phase_id: String,
    pub log_path: PathBuf,
    pub content: String, // empty string if file not found
}

/// Gather log files for the given phase IDs from `.cm/logs/<phase_id>.log`.
///
/// Missing log files silently return empty content — graceful degradation.
pub fn gather_phase_logs(
    cm_dir: &Path,
    phase_ids: &[String],
) -> Result<Vec<PhaseLogContent>, ReviewError> {
    let mut result = Vec::with_capacity(phase_ids.len());
    for id in phase_ids {
        let log_path = cm_dir.join("logs").join(format!("{id}.log"));
        let content = fs::read_to_string(&log_path).unwrap_or_default();
        result.push(PhaseLogContent {
            phase_id: id.clone(),
            log_path,
            content,
        });
    }
    Ok(result)
}

/// Read `<cm_dir>/cm.log`, returning only the lines whose timestamp is >= `since`.
///
/// Lines whose timestamp cannot be parsed are treated as continuation lines:
/// they are kept only when they immediately follow a kept line.
pub fn gather_cm_log_since(cm_dir: &Path, since: DateTime<Utc>) -> String {
    let log_path = cm_dir.join("cm.log");
    let file = match fs::File::open(&log_path) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };

    let reader = io::BufReader::new(file);
    let mut output = Vec::<String>::new();
    let mut last_kept = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        match parse_log_timestamp(&line) {
            Some(ts) => {
                if ts >= since {
                    output.push(line);
                    last_kept = true;
                } else {
                    last_kept = false;
                }
            }
            None => {
                // Continuation line — keep only if the previous line was kept
                if last_kept {
                    output.push(line);
                }
                // last_kept stays the same
            }
        }
    }

    output.join("\n")
}

/// Concatenate all gathered logs into a single string for prompt embedding.
///
/// Phases with empty content are filtered out.
pub fn format_logs_for_prompt(logs: &[PhaseLogContent]) -> String {
    logs.iter()
        .filter(|l| !l.content.is_empty())
        .map(|l| format!("## Phase: {}\n\n{}\n", l.phase_id, l.content))
        .collect::<Vec<_>>()
        .join("\n---\n\n")
}

/// Format gathered phase logs and append the cm.log window as a final section.
pub fn format_logs_for_prompt_with_cm_log(logs: &[PhaseLogContent], cm_log_window: &str) -> String {
    let mut parts = format_logs_for_prompt(logs);
    if !cm_log_window.is_empty() {
        if !parts.is_empty() {
            parts.push_str("\n---\n\n");
        }
        parts.push_str("===== cm.log (run window) =====\n");
        parts.push_str(cm_log_window);
        parts.push('\n');
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_gather_phase_logs_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        let logs = gather_phase_logs(dir.path(), &["phase-1".to_string()]).unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].content.is_empty());
    }

    #[test]
    fn test_gather_phase_logs_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::write(dir.path().join("logs/phase-1.log"), "log content").unwrap();
        let logs = gather_phase_logs(dir.path(), &["phase-1".to_string()]).unwrap();
        assert_eq!(logs[0].content, "log content");
    }

    #[test]
    fn test_gather_phase_logs_multiple() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::write(dir.path().join("logs/phase-1.log"), "content A").unwrap();
        fs::write(dir.path().join("logs/phase-2.log"), "content B").unwrap();
        let logs = gather_phase_logs(
            dir.path(),
            &[
                "phase-1".to_string(),
                "phase-2".to_string(),
                "phase-3".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].content, "content A");
        assert_eq!(logs[1].content, "content B");
        assert!(logs[2].content.is_empty());
    }

    #[test]
    fn test_format_logs_for_prompt_filters_empty() {
        let logs = vec![
            PhaseLogContent {
                phase_id: "p1".into(),
                log_path: PathBuf::new(),
                content: "content".into(),
            },
            PhaseLogContent {
                phase_id: "p2".into(),
                log_path: PathBuf::new(),
                content: String::new(),
            },
        ];
        let result = format_logs_for_prompt(&logs);
        assert!(result.contains("p1"));
        assert!(!result.contains("p2"));
    }

    #[test]
    fn test_format_logs_for_prompt_all_empty() {
        let logs = vec![PhaseLogContent {
            phase_id: "p1".into(),
            log_path: PathBuf::new(),
            content: String::new(),
        }];
        let result = format_logs_for_prompt(&logs);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_logs_for_prompt_content() {
        let logs = vec![PhaseLogContent {
            phase_id: "phase-5".into(),
            log_path: PathBuf::new(),
            content: "some log content".into(),
        }];
        let result = format_logs_for_prompt(&logs);
        assert!(result.contains("## Phase: phase-5"));
        assert!(result.contains("some log content"));
    }

    // --- cm.log time-slicing tests ---

    fn make_log_line(ts: &str, msg: &str) -> String {
        format!("[{ts}] {msg}")
    }

    #[test]
    fn test_cm_log_all_too_old_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let lines = [
            make_log_line("2024-01-01 10:00:00.000", "old line 1"),
            make_log_line("2024-01-01 10:00:01.000", "old line 2"),
        ];
        fs::write(dir.path().join("cm.log"), lines.join("\n")).unwrap();

        let since: DateTime<Utc> = chrono::NaiveDateTime::parse_from_str(
            "2025-01-01 00:00:00.000",
            "%Y-%m-%d %H:%M:%S%.3f",
        )
        .unwrap()
        .and_utc();

        let result = gather_cm_log_since(dir.path(), since);
        assert!(result.is_empty(), "expected empty but got: {result}");
    }

    #[test]
    fn test_cm_log_boundary_line_included() {
        let dir = tempfile::tempdir().unwrap();
        let lines = [
            make_log_line("2025-01-01 10:00:00.000", "before"),
            make_log_line("2025-01-01 10:00:01.000", "boundary"),
            make_log_line("2025-01-01 10:00:02.000", "after"),
        ];
        fs::write(dir.path().join("cm.log"), lines.join("\n")).unwrap();

        let since: DateTime<Utc> = chrono::NaiveDateTime::parse_from_str(
            "2025-01-01 10:00:01.000",
            "%Y-%m-%d %H:%M:%S%.3f",
        )
        .unwrap()
        .and_utc();

        let result = gather_cm_log_since(dir.path(), since);
        assert!(!result.contains("before"), "should not contain 'before'");
        assert!(result.contains("boundary"), "should contain 'boundary'");
        assert!(result.contains("after"), "should contain 'after'");
    }

    #[test]
    fn test_cm_log_continuation_lines_kept() {
        let dir = tempfile::tempdir().unwrap();
        // continuation line has no timestamp
        let lines = [
            make_log_line("2024-01-01 10:00:00.000", "old line"),
            "  continuation of old line".to_string(),
            make_log_line("2025-06-01 10:00:00.000", "new line"),
            "  continuation of new line".to_string(),
        ];
        fs::write(dir.path().join("cm.log"), lines.join("\n")).unwrap();

        let since: DateTime<Utc> = chrono::NaiveDateTime::parse_from_str(
            "2025-01-01 00:00:00.000",
            "%Y-%m-%d %H:%M:%S%.3f",
        )
        .unwrap()
        .and_utc();

        let result = gather_cm_log_since(dir.path(), since);
        assert!(!result.contains("old line"), "should not contain old line");
        assert!(
            !result.contains("continuation of old line"),
            "should not contain old continuation"
        );
        assert!(result.contains("new line"), "should contain new line");
        assert!(
            result.contains("continuation of new line"),
            "should contain new continuation"
        );
    }

    #[test]
    fn test_cm_log_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let since = Utc::now();
        let result = gather_cm_log_since(dir.path(), since);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_logs_with_cm_log_appends_section() {
        let logs = vec![PhaseLogContent {
            phase_id: "phase-1".into(),
            log_path: PathBuf::new(),
            content: "phase content".into(),
        }];
        let cm_log = "some cm.log line";
        let result = format_logs_for_prompt_with_cm_log(&logs, cm_log);
        assert!(result.contains("## Phase: phase-1"));
        assert!(result.contains("===== cm.log (run window) ====="));
        assert!(result.contains("some cm.log line"));
    }

    #[test]
    fn test_format_logs_with_empty_cm_log_no_section() {
        let logs = vec![PhaseLogContent {
            phase_id: "phase-1".into(),
            log_path: PathBuf::new(),
            content: "phase content".into(),
        }];
        let result = format_logs_for_prompt_with_cm_log(&logs, "");
        assert!(!result.contains("cm.log"));
    }
}
