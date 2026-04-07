//! Log gathering for the post-run review agent.
//!
//! Provides functions to read per-phase log files from `.cm/logs/` and format
//! them into a single string suitable for embedding in a review prompt.

use std::fs;
use std::path::{Path, PathBuf};

/// Gathered log content for one phase.
pub struct PhaseLogContent {
    pub phase_id: String,
    pub log_path: PathBuf,
    pub content: String, // empty string if file not found
}

/// Gather log files for the given phase IDs from `.cm/logs/<phase_id>.log`.
///
/// Missing log files silently return empty content — graceful degradation.
pub fn gather_phase_logs(cm_dir: &Path, phase_ids: &[String]) -> Vec<PhaseLogContent> {
    phase_ids
        .iter()
        .map(|id| {
            let log_path = cm_dir.join("logs").join(format!("{id}.log"));
            let content = fs::read_to_string(&log_path).unwrap_or_default();
            PhaseLogContent {
                phase_id: id.clone(),
                log_path,
                content,
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_gather_phase_logs_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        let logs = gather_phase_logs(dir.path(), &["phase-1".to_string()]);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].content.is_empty());
    }

    #[test]
    fn test_gather_phase_logs_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logs")).unwrap();
        fs::write(dir.path().join("logs/phase-1.log"), "log content").unwrap();
        let logs = gather_phase_logs(dir.path(), &["phase-1".to_string()]);
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
            &["phase-1".to_string(), "phase-2".to_string(), "phase-3".to_string()],
        );
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
}
