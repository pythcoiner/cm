//! Deterministic markdown generation from JSON state.
//!
//! This module provides functions to generate markdown files (LOG.md, ROADMAP.md)
//! from their JSON source of truth files. The generation is deterministic:
//! the same input always produces the same output.

mod log_md;
mod roadmap_md;

pub use log_md::generate_log_md;
pub use roadmap_md::generate_roadmap_md;

use std::fs;
use std::path::Path;

use thiserror::Error;

/// Errors that can occur during markdown generation.
#[derive(Debug, Error)]
pub enum GenerateError {
    /// An I/O error occurred.
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    /// State loading error.
    #[error("state error: {0}")]
    StateError(#[from] crate::state::StateError),
}

/// Write generated markdown content to a file.
///
/// Creates parent directories if they don't exist.
///
/// # Arguments
///
/// * `content` - The markdown content to write
/// * `path` - Path to write the file
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn write_md_file(content: &str, path: &Path) -> Result<(), GenerateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_md_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("subdir/test.md");

        write_md_file("# Test\n\nContent here.", &path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Test"));
        assert!(content.contains("Content here."));
    }

    #[test]
    fn test_generate_error_display() {
        let err = GenerateError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("io error"));
    }
}
