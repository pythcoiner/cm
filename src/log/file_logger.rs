//! Persistent file logging for operational debugging and auditing.
//!
//! Writes timestamped, leveled log entries to `.cm/cm.log`. This is independent
//! from LogManager (LOG.md audit trail) and env_logger (stdout). Thread-safe
//! via Mutex for concurrent access from signal handlers.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

/// Log level for file logger entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Detailed debug information (state saves, prompt previews, blocked tasks).
    Debug,
    /// Major milestones (task start/complete, agent spawn).
    Info,
    /// Build failures, task deferrals, shutdown signals.
    Warn,
    /// Fatal errors.
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Errors that can occur during file logging.
#[derive(Debug, Error)]
pub enum FileLogError {
    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// A parse error occurred (e.g., invalid timestamp in log line).
    #[error("parse error: {0}")]
    ParseError(String),
}

/// Statistics from a log pruning operation.
#[derive(Debug, Clone)]
pub struct PruneStats {
    /// Number of log lines removed.
    pub removed_count: usize,
    /// Number of log lines kept.
    pub kept_count: usize,
}

/// Persistent file logger for structured operational logging.
///
/// Writes timestamped, leveled log entries to a file. Thread-safe via Mutex.
/// This is independent from LogManager (LOG.md) and env_logger (stdout).
pub struct FileLogger {
    path: PathBuf,
    file: Mutex<File>,
    min_level: LogLevel,
}

impl FileLogger {
    /// Create a new FileLogger that writes to the given path.
    ///
    /// Creates the file if it doesn't exist, opens in append mode.
    /// Default minimum level is Info.
    pub fn new(path: PathBuf) -> Result<Self, FileLogError> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)?;

        Ok(Self {
            path,
            file: Mutex::new(file),
            min_level: LogLevel::Info,
        })
    }

    /// Set the minimum log level. Returns self for builder pattern.
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Write a log entry to the file.
    ///
    /// Format: `[YYYY-MM-DD HH:MM:SS.mmm] [LEVEL] [component] message`
    ///
    /// Entries below the minimum level are silently skipped.
    pub fn log(
        &self,
        level: LogLevel,
        component: &str,
        message: &str,
    ) -> Result<(), FileLogError> {
        if level < self.min_level {
            return Ok(());
        }

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let line = format!("[{}] [{}] [{}] {}", timestamp, level, component, message);

        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        writeln!(file, "{}", line)?;
        file.flush()?;

        Ok(())
    }

    /// Log at DEBUG level.
    pub fn debug(&self, component: &str, message: &str) -> Result<(), FileLogError> {
        self.log(LogLevel::Debug, component, message)
    }

    /// Log at INFO level.
    pub fn info(&self, component: &str, message: &str) -> Result<(), FileLogError> {
        self.log(LogLevel::Info, component, message)
    }

    /// Log at WARN level.
    pub fn warn(&self, component: &str, message: &str) -> Result<(), FileLogError> {
        self.log(LogLevel::Warn, component, message)
    }

    /// Log at ERROR level.
    pub fn error(&self, component: &str, message: &str) -> Result<(), FileLogError> {
        self.log(LogLevel::Error, component, message)
    }

    /// Prune log entries older than 24 hours.
    pub fn prune(&self) -> Result<PruneStats, FileLogError> {
        self.prune_older_than(Duration::hours(24))
    }

    /// Prune log entries older than the given duration.
    ///
    /// Lines with unparseable timestamps are kept (conservative approach).
    pub fn prune_older_than(&self, duration: Duration) -> Result<PruneStats, FileLogError> {
        let cutoff = Utc::now() - duration;

        let reader = BufReader::new(File::open(&self.path)?);
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

        {
            let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
            *file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.path)?;
            for line in &kept_lines {
                writeln!(file, "{}", line)?;
            }
            file.flush()?;
            *file = OpenOptions::new().append(true).open(&self.path)?;
        }

        Ok(PruneStats {
            removed_count,
            kept_count,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp_log_path() -> PathBuf {
        let dir = tempfile::tempdir().unwrap();
        // Leak the dir so it isn't cleaned up during the test
        let path = dir.path().join("test.log");
        std::mem::forget(dir);
        path
    }

    #[test]
    fn test_file_creation_and_basic_log() {
        let path = temp_log_path();
        let logger = FileLogger::new(path.clone()).unwrap();
        logger.info("test", "hello world").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[INFO]"));
        assert!(content.contains("[test]"));
        assert!(content.contains("hello world"));
    }

    #[test]
    fn test_log_level_filtering() {
        let path = temp_log_path();
        let logger = FileLogger::new(path.clone())
            .unwrap()
            .with_level(LogLevel::Warn);

        logger.debug("test", "debug msg").unwrap();
        logger.info("test", "info msg").unwrap();
        logger.warn("test", "warn msg").unwrap();
        logger.error("test", "error msg").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("debug msg"));
        assert!(!content.contains("info msg"));
        assert!(content.contains("warn msg"));
        assert!(content.contains("error msg"));
    }

    #[test]
    fn test_log_format() {
        let path = temp_log_path();
        let logger = FileLogger::new(path.clone()).unwrap();
        logger.info("manager", "test message").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let line = content.lines().next().unwrap();

        // Format: [YYYY-MM-DD HH:MM:SS.mmm] [INFO] [manager] test message
        assert!(line.starts_with('['));
        assert!(line.contains("] [INFO] [manager] test message"));
    }

    #[test]
    fn test_parse_log_timestamp_valid() {
        let line = "[2026-01-24 10:30:45.123] [INFO] [manager] test";
        let ts = parse_log_timestamp(line).unwrap();
        assert_eq!(ts.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-01-24 10:30:45");
    }

    #[test]
    fn test_parse_log_timestamp_invalid() {
        assert!(parse_log_timestamp("no bracket").is_none());
        assert!(parse_log_timestamp("[not a timestamp] rest").is_none());
        assert!(parse_log_timestamp("").is_none());
    }

    #[test]
    fn test_prune_keeps_recent() {
        let path = temp_log_path();
        let logger = FileLogger::new(path.clone()).unwrap();

        logger.info("test", "recent entry 1").unwrap();
        logger.info("test", "recent entry 2").unwrap();

        let stats = logger.prune().unwrap();
        assert_eq!(stats.removed_count, 0);
        assert_eq!(stats.kept_count, 2);
    }

    #[test]
    fn test_prune_removes_old() {
        let path = temp_log_path();

        // Write old entries directly with past timestamps
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)
                .unwrap();
            writeln!(file, "[2020-01-01 00:00:00.000] [INFO] [test] old entry 1").unwrap();
            writeln!(file, "[2020-01-01 00:00:01.000] [INFO] [test] old entry 2").unwrap();
        }

        let logger = FileLogger::new(path.clone()).unwrap();
        // Add a recent entry
        logger.info("test", "recent entry").unwrap();

        let stats = logger.prune().unwrap();
        assert_eq!(stats.removed_count, 2);
        assert_eq!(stats.kept_count, 1);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("recent entry"));
        assert!(!content.contains("old entry"));
    }

    #[test]
    fn test_concurrent_logging() {
        let path = temp_log_path();
        let logger = Arc::new(FileLogger::new(path.clone()).unwrap());

        let mut handles = vec![];
        for i in 0..10 {
            let logger = Arc::clone(&logger);
            handles.push(std::thread::spawn(move || {
                logger
                    .info("thread", &format!("message from thread {}", i))
                    .unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let content = std::fs::read_to_string(&path).unwrap();
        let line_count = content.lines().count();
        assert_eq!(line_count, 10);
    }

    #[test]
    fn test_with_level_builder() {
        let path = temp_log_path();
        let logger = FileLogger::new(path).unwrap().with_level(LogLevel::Debug);
        assert_eq!(logger.min_level, LogLevel::Debug);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
}
