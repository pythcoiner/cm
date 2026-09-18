//! Tee writer for mirroring terminal output to cm.log.
//!
//! This module provides a global log file that mirrors all stdout/stderr output
//! to `.cm/logs/cm.log` so users can review execution history.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::am::{send_output, send_output_line, Stream};

/// Global log file for tee output.
static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Initialize the global log file.
///
/// Must be called early in program startup, before any logging occurs.
/// Creates the logs directory if it doesn't exist.
///
/// # Arguments
///
/// * `cm_dir` - Path to the .cm directory
///
/// # Errors
///
/// Returns an error if the log file cannot be created.
pub fn init_log_file(cm_dir: &Path) -> io::Result<()> {
    let logs_dir = cm_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    let log_path = logs_dir.join("cm.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    LOG_FILE.set(Mutex::new(file)).map_err(|_| {
        io::Error::new(io::ErrorKind::AlreadyExists, "log file already initialized")
    })?;

    Ok(())
}

/// Write a line to the log file (if initialized).
///
/// This is a no-op if the log file hasn't been initialized.
fn write_to_log(s: &str) {
    if let Some(file_mutex) = LOG_FILE.get() {
        if let Ok(mut file) = file_mutex.lock() {
            // Ignore write errors - logging shouldn't crash the program
            let _ = writeln!(file, "{s}");
        }
    }
}

/// Write to the log file without a newline.
fn write_to_log_raw(s: &str) {
    if let Some(file_mutex) = LOG_FILE.get() {
        if let Ok(mut file) = file_mutex.lock() {
            let _ = write!(file, "{s}");
            let _ = file.flush();
        }
    }
}

/// Print to stdout and mirror to log file.
///
/// Use this instead of `println!` to capture output in cm.log.
pub fn tee_println(s: &str) {
    println!("{s}");
    write_to_log(s);
    send_output(Stream::Stdout, format!("{s}\n").as_bytes());
}

/// Print to stdout without newline and mirror to log file.
pub fn tee_print(s: &str) {
    print!("{s}");
    let _ = io::stdout().flush();
    write_to_log_raw(s);
    send_output_line(Stream::Stdout, s.as_bytes());
}

/// Print to stderr and mirror to log file.
///
/// Use this instead of `eprintln!` to capture output in cm.log.
pub fn tee_eprintln(s: &str) {
    eprintln!("{s}");
    write_to_log(s);
    send_output(Stream::Stderr, format!("{s}\n").as_bytes());
}

/// A writer that tees output to both the original destination and the log file.
///
/// This is used for env_logger's custom target.
pub struct TeeWriter {
    /// Whether to write to stderr (true) or stdout (false).
    use_stderr: bool,
}

impl TeeWriter {
    /// Create a new TeeWriter that writes to stderr.
    pub fn stderr() -> Self {
        Self { use_stderr: true }
    }

    /// Create a new TeeWriter that writes to stdout.
    #[allow(dead_code)]
    pub fn stdout() -> Self {
        Self { use_stderr: false }
    }
}

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Write to the original destination
        let written = if self.use_stderr {
            io::stderr().write(buf)?
        } else {
            io::stdout().write(buf)?
        };

        // Also write to log file (ignore errors)
        if let Some(file_mutex) = LOG_FILE.get() {
            if let Ok(mut file) = file_mutex.lock() {
                let _ = file.write_all(&buf[..written]);
            }
        }

        let stream = if self.use_stderr {
            Stream::Stderr
        } else {
            Stream::Stdout
        };
        send_output(stream, &buf[..written]);

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.use_stderr {
            io::stderr().flush()?;
        } else {
            io::stdout().flush()?;
        }

        if let Some(file_mutex) = LOG_FILE.get() {
            if let Ok(mut file) = file_mutex.lock() {
                let _ = file.flush();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tee_writer_write() {
        let tmp = TempDir::new().unwrap();

        // Initialize log file
        init_log_file(tmp.path()).unwrap();

        // Write through TeeWriter
        let mut writer = TeeWriter::stderr();
        writer.write_all(b"test message\n").unwrap();
        writer.flush().unwrap();

        // Check log file contents
        let log_path = tmp.path().join("logs/cm.log");
        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("test message"));
    }
}
