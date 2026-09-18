//! Log gathering for the post-run review agent.
//!
//! Provides functions to read per-phase log files from `.cm/logs/` and format
//! them into a single string suitable for embedding in a review prompt.

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, Utc};
use thiserror::Error;

use crate::log::parse_log_timestamp;

/// Upper bound on the formatted review input embedded in the prompt.
pub const MAX_REVIEW_INPUT_BYTES: usize = 300_000;

/// Prefix of the review input when older content was cut to fit the cap.
const TRUNCATION_MARKER: &str = "[earlier review input truncated]\n";

/// Timestamp format of phase log entry headers.
const PHASE_LOG_TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f UTC";

/// Width of the `=` lines framing each phase log entry header.
const PHASE_LOG_SEPARATOR_WIDTH: usize = 80;

/// Start of the raw JSON block closing a phase log RESPONSE entry.
const RAW_RESPONSE_MARKER: &str = "<details>";

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

/// Kind of a phase log entry.
#[derive(Debug, PartialEq)]
enum EntryKind {
    Prompt,
    Response,
}

/// Header line of a phase log entry.
struct EntryHeader {
    timestamp: DateTime<Utc>,
    kind: EntryKind,
}

/// Parse `[<timestamp> UTC] PROMPT: ...` or `[<timestamp> UTC] RESPONSE: ...`.
fn parse_entry_header(line: &str) -> Option<EntryHeader> {
    let (timestamp, tail) = line.strip_prefix('[')?.split_once("] ")?;
    let timestamp = NaiveDateTime::parse_from_str(timestamp, PHASE_LOG_TIMESTAMP_FORMAT)
        .ok()?
        .and_utc();
    let kind = match tail.split_once(": ")?.0 {
        "PROMPT" => EntryKind::Prompt,
        "RESPONSE" => EntryKind::Response,
        _ => return None,
    };
    Some(EntryHeader { timestamp, kind })
}

/// Keep the phase log entries written at or after `since`.
///
/// PROMPT entries are reduced to their header, RESPONSE entries lose their raw JSON block.
fn filter_phase_log(content: &str, since: DateTime<Utc>) -> String {
    let separator = "=".repeat(PHASE_LOG_SEPARATOR_WIDTH);
    let mut output = Vec::new();
    let mut keep_body = false;

    for line in content.lines() {
        match parse_entry_header(line) {
            Some(header) => {
                let kept = header.timestamp >= since;
                if kept {
                    output.push(line);
                }
                keep_body = kept && header.kind == EntryKind::Response;
            }
            None if line.starts_with(RAW_RESPONSE_MARKER) => keep_body = false,
            None if keep_body && line != separator => output.push(line),
            None => {}
        }
    }

    output.join("\n")
}

/// Keep the most recent `max_bytes` of `input`, cut at a line start when possible.
fn truncate_to_tail(input: String, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input;
    }
    let budget = max_bytes.saturating_sub(TRUNCATION_MARKER.len());
    let mut start = input.len() - budget;
    while !input.is_char_boundary(start) {
        start += 1;
    }
    if let Some(newline) = input[start..].find('\n') {
        start += newline + 1;
    }
    format!("{TRUNCATION_MARKER}{}", &input[start..])
}

/// Gather this run's entries from `.cm/logs/<phase_id>.log` for the given phase IDs.
///
/// Only entries written at or after `since` are kept, see `filter_phase_log`.
/// Missing log files silently return empty content — graceful degradation.
pub fn gather_phase_logs(
    cm_dir: &Path,
    phase_ids: &[String],
    since: DateTime<Utc>,
) -> Result<Vec<PhaseLogContent>, ReviewError> {
    let mut result = Vec::with_capacity(phase_ids.len());
    for id in phase_ids {
        let log_path = cm_dir.join("logs").join(format!("{id}.log"));
        let content = filter_phase_log(&fs::read_to_string(&log_path).unwrap_or_default(), since);
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
///
/// The result is capped to `MAX_REVIEW_INPUT_BYTES`, keeping the most recent content.
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
    truncate_to_tail(parts, MAX_REVIEW_INPUT_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PROMPT_HEADER: &str = "[2025-01-01 10:00:00.000 UTC] PROMPT: IMPLEM for phase-1";
    const RESPONSE_HEADER: &str =
        "[2025-01-01 10:00:05.000 UTC] RESPONSE: IMPLEM for phase-1 (Duration: 5s, Exit Code: 0)";
    const RESPONSE_BODY: &str = "\n## Assistant Messages\n\nAll done.\n\n## Parsed Result\n\n**Status:** Success\n\n---\n\n<details>\n<summary>Raw Response (click to expand)</summary>\n\n```json\n{\"result\":\"raw json\"}\n```\n\n</details>\n";

    fn log_entry(header: &str, body: &str) -> String {
        let separator = "=".repeat(PHASE_LOG_SEPARATOR_WIDTH);
        format!("{separator}\n{header}\n{separator}\n\n{body}\n")
    }

    fn since(ts: &str) -> DateTime<Utc> {
        chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S%.3f")
            .unwrap()
            .and_utc()
    }

    #[test]
    fn test_gather_phase_logs_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        let logs = gather_phase_logs(
            dir.path(),
            &["phase-1".to_string()],
            since("2025-01-01 00:00:00.000"),
        )
        .unwrap();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].content.is_empty());
    }

    #[test]
    fn test_gather_phase_logs_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::write(
            dir.path().join("logs/phase-1.log"),
            log_entry(PROMPT_HEADER, "prompt body"),
        )
        .unwrap();
        let logs = gather_phase_logs(
            dir.path(),
            &["phase-1".to_string()],
            since("2025-01-01 00:00:00.000"),
        )
        .unwrap();
        assert_eq!(logs[0].content, PROMPT_HEADER);
    }

    #[test]
    fn test_gather_phase_logs_multiple() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::write(
            dir.path().join("logs/phase-1.log"),
            log_entry(PROMPT_HEADER, "content A"),
        )
        .unwrap();
        fs::write(
            dir.path().join("logs/phase-2.log"),
            log_entry(RESPONSE_HEADER, "content B"),
        )
        .unwrap();
        let logs = gather_phase_logs(
            dir.path(),
            &[
                "phase-1".to_string(),
                "phase-2".to_string(),
                "phase-3".to_string(),
            ],
            since("2025-01-01 00:00:00.000"),
        )
        .unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].content, PROMPT_HEADER);
        assert_eq!(logs[1].content, format!("{RESPONSE_HEADER}\n\ncontent B"));
        assert!(logs[2].content.is_empty());
    }

    #[test]
    fn test_filter_phase_log_drops_entries_before_since() {
        let old_prompt = "[2025-01-01 09:59:59.999 UTC] PROMPT: IMPLEM for phase-1";
        let old_response =
            "[2025-01-01 09:59:59.999 UTC] RESPONSE: IMPLEM for phase-1 (Duration: 1s, Exit Code: 0)";
        let content = [
            log_entry(old_prompt, "old prompt"),
            log_entry(old_response, "old response"),
            log_entry(PROMPT_HEADER, "new prompt"),
        ]
        .concat();

        let result = filter_phase_log(&content, since("2025-01-01 10:00:00.000"));
        assert_eq!(result, PROMPT_HEADER);
    }

    #[test]
    fn test_filter_phase_log_all_before_since_is_empty() {
        let content = log_entry(RESPONSE_HEADER, RESPONSE_BODY);

        let result = filter_phase_log(&content, since("2025-01-01 10:00:05.001"));
        assert_eq!(result, "");
    }

    #[test]
    fn test_filter_phase_log_drops_prompt_body() {
        let content = log_entry(
            PROMPT_HEADER,
            "diff --git a/src/lib.rs b/src/lib.rs\n+fn huge() {}",
        );

        let result = filter_phase_log(&content, since("2025-01-01 00:00:00.000"));
        assert_eq!(result, PROMPT_HEADER);
    }

    #[test]
    fn test_filter_phase_log_drops_raw_response() {
        let content = [
            log_entry(PROMPT_HEADER, "prompt body"),
            log_entry(RESPONSE_HEADER, RESPONSE_BODY),
        ]
        .concat();

        let result = filter_phase_log(&content, since("2025-01-01 00:00:00.000"));
        assert_eq!(
            result,
            format!(
                "{PROMPT_HEADER}\n{RESPONSE_HEADER}\n\n\n## Assistant Messages\n\nAll done.\n\n## Parsed Result\n\n**Status:** Success\n\n---\n"
            )
        );
        assert!(!result.contains("raw json"));
    }

    #[test]
    fn test_parse_entry_header() {
        assert!(
            parse_entry_header("[2025-01-01 10:00:00.000] PROMPT: IMPLEM for phase-1").is_none()
        );
        assert!(parse_entry_header("[2025-01-01 10:00:00.000 UTC] NOTE: something").is_none());
        assert!(parse_entry_header("## Assistant Messages").is_none());

        let header = parse_entry_header(RESPONSE_HEADER).unwrap();
        assert_eq!(header.kind, EntryKind::Response);
        assert_eq!(header.timestamp, since("2025-01-01 10:00:05.000"));
    }

    #[test]
    fn test_truncate_to_tail_under_cap_unchanged() {
        let input = "line 1\nline 2\n".to_string();
        assert_eq!(truncate_to_tail(input.clone(), input.len()), input);
    }

    #[test]
    fn test_truncate_to_tail_cuts_at_line_start() {
        let input = "aaaa\nbbbb\ncccc\n".to_string();

        let result = truncate_to_tail(input, TRUNCATION_MARKER.len() + 7);
        assert_eq!(result, format!("{TRUNCATION_MARKER}cccc\n"));
    }

    #[test]
    fn test_truncate_to_tail_cuts_at_char_boundary() {
        let input = "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}".to_string();

        let result = truncate_to_tail(input, TRUNCATION_MARKER.len() + 3);
        assert_eq!(result, format!("{TRUNCATION_MARKER}\u{e9}"));
    }

    #[test]
    fn test_format_logs_with_cm_log_caps_input() {
        let logs = vec![PhaseLogContent {
            phase_id: "phase-1".into(),
            log_path: PathBuf::new(),
            content: "phase content".into(),
        }];
        let cm_log = (0..40_000)
            .map(|i| format!("cm.log line {i:05}"))
            .collect::<Vec<_>>()
            .join("\n");

        let result = format_logs_for_prompt_with_cm_log(&logs, &cm_log);
        assert!(result.len() <= MAX_REVIEW_INPUT_BYTES);
        assert!(result.starts_with(&format!("{TRUNCATION_MARKER}cm.log line ")));
        assert!(result.ends_with("cm.log line 39999\n"));
        assert!(!result.contains("## Phase: phase-1"));
    }

    #[test]
    fn test_format_logs_with_cm_log_under_cap_not_truncated() {
        let logs = vec![PhaseLogContent {
            phase_id: "phase-1".into(),
            log_path: PathBuf::new(),
            content: "phase content".into(),
        }];

        let result = format_logs_for_prompt_with_cm_log(&logs, "cm.log line");
        assert_eq!(
            result,
            "## Phase: phase-1\n\nphase content\n\n---\n\n===== cm.log (run window) =====\ncm.log line\n"
        );
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
