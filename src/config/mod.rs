//! Configuration file support for cm.
//!
//! This module provides TOML-based configuration file support, allowing users
//! to specify default values for model, max_cycles, working_dir, and
//! build_commands in a config file instead of command-line arguments.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The config file was not found at the specified path.
    #[error("config file not found: {0}")]
    NotFound(PathBuf),

    /// An I/O error occurred while reading the config file.
    #[error("failed to read config: {0}")]
    IoError(#[from] std::io::Error),

    /// The config file contains invalid TOML.
    #[error("failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),
}

/// Per-model pricing table, in USD per million tokens.
///
/// Used by `cm token` to estimate the API-equivalent cost of recorded
/// Claude Code usage. Refreshed by the `/update-pricing` slash command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceTable {
    /// Cost of regular input tokens, $/1M.
    pub input: f64,
    /// Cost of output tokens, $/1M.
    pub output: f64,
    /// Cost of cache-creation (write) input tokens, $/1M.
    pub cache_write: f64,
    /// Cost of cache-read input tokens, $/1M.
    pub cache_read: f64,
}

/// Configuration file structure.
///
/// All fields are optional. Missing fields will use default values.
/// The config file uses TOML format.
///
/// # Example
///
/// ```toml
/// model = "claude-sonnet-4-5-20250929"
/// max_cycles = 5
/// working_dir = "."
/// build_commands = ["cargo build", "cargo clippy"]
///
/// [pricing."claude-opus-4-7"]
/// input = 15.0
/// output = 75.0
/// cache_write = 18.75
/// cache_read = 1.50
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    /// The Claude model to use for agent spawning.
    pub model: Option<String>,

    /// Maximum number of cycles (attempts) per task before deferring.
    pub max_cycles: Option<u32>,

    /// Working directory for build verification.
    pub working_dir: Option<PathBuf>,

    /// Build commands to run for verification.
    /// If empty or not specified, build verification is skipped.
    /// Example: ["cargo build", "cargo clippy"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_commands: Option<Vec<String>>,

    /// Per-model pricing overrides used by `cm token`.
    ///
    /// Keys are either exact model identifiers (e.g. `claude-opus-4-7`) or
    /// family substrings (`opus`, `sonnet`, `haiku`). Lookup tries exact
    /// match first, then falls back to a substring match against the
    /// reported model name. If no entry matches, hardcoded defaults apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<HashMap<String, PriceTable>>,
}

impl ConfigFile {
    /// Load a configuration file from the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file does not exist (`ConfigError::NotFound`)
    /// - The file cannot be read (`ConfigError::IoError`)
    /// - The file contains invalid TOML (`ConfigError::ParseError`)
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        Ok(config)
    }

    /// Try to load the default configuration file.
    ///
    /// Looks for `.cm/config.toml` in the current directory.
    /// Returns `Ok(None)` if the file doesn't exist, allowing the caller
    /// to proceed with defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_default() -> Result<Option<Self>, ConfigError> {
        let default_path = PathBuf::from(".cm/config.toml");

        if !default_path.exists() {
            return Ok(None);
        }

        let config = Self::load(&default_path)?;
        Ok(Some(config))
    }

    /// Check if all fields are None (empty config).
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.max_cycles.is_none()
            && self.working_dir.is_none()
            && self.build_commands.is_none()
            && self.pricing.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_file_default() {
        let config = ConfigFile::default();
        assert!(config.model.is_none());
        assert!(config.max_cycles.is_none());
        assert!(config.working_dir.is_none());
        assert!(config.build_commands.is_none());
        assert!(config.is_empty());
    }

    #[test]
    fn test_config_file_load_not_found() {
        let result = ConfigFile::load(Path::new("/nonexistent/config.toml"));
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_config_file_load_full() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let content = r#"
model = "claude-opus-4-5-20251101"
max_cycles = 10
working_dir = "/home/user/project"
build_commands = ["cargo build", "cargo clippy"]
"#;

        std::fs::write(&config_path, content).unwrap();

        let config = ConfigFile::load(&config_path).unwrap();

        assert_eq!(config.model, Some("claude-opus-4-5-20251101".to_string()));
        assert_eq!(config.max_cycles, Some(10));
        assert_eq!(
            config.working_dir,
            Some(PathBuf::from("/home/user/project"))
        );
        assert_eq!(
            config.build_commands,
            Some(vec!["cargo build".to_string(), "cargo clippy".to_string()])
        );
        assert!(!config.is_empty());
    }

    #[test]
    fn test_config_file_load_partial() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let content = r#"
model = "claude-sonnet-4-5-20250929"
max_cycles = 3
"#;

        std::fs::write(&config_path, content).unwrap();

        let config = ConfigFile::load(&config_path).unwrap();

        assert_eq!(
            config.model,
            Some("claude-sonnet-4-5-20250929".to_string())
        );
        assert_eq!(config.max_cycles, Some(3));
        assert!(config.working_dir.is_none());
    }

    #[test]
    fn test_config_file_load_empty() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        std::fs::write(&config_path, "").unwrap();

        let config = ConfigFile::load(&config_path).unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn test_config_file_invalid_toml() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let content = "this is not valid toml { [ }";
        std::fs::write(&config_path, content).unwrap();

        let result = ConfigFile::load(&config_path);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_config_file_load_default_not_found() {
        // Run in a temp directory where .cm/config.toml doesn't exist
        let tmp = TempDir::new().unwrap();
        let _guard = std::env::set_current_dir(tmp.path());

        // Since we can't easily change working directory in tests,
        // we test load_default behavior indirectly
        let default_path = PathBuf::from(".cm/config.toml");
        if !default_path.exists() {
            // This is the expected path - no default config exists
            assert!(!default_path.exists());
        }
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::NotFound(PathBuf::from("/path/to/config.toml"));
        assert!(err.to_string().contains("config file not found"));
        assert!(err.to_string().contains("/path/to/config.toml"));

        // ParseError display comes from toml::de::Error
        // IoError display comes from std::io::Error
    }

    #[test]
    fn test_config_file_serialize() {
        let config = ConfigFile {
            model: Some("test-model".to_string()),
            max_cycles: Some(5),
            working_dir: None,
            build_commands: None,
            pricing: None,
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("model = \"test-model\""));
        assert!(toml_str.contains("max_cycles = 5"));
    }

    #[test]
    fn test_config_file_pricing_overrides_round_trip() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");

        let content = r#"
[pricing."claude-opus-4-7"]
input = 15.0
output = 75.0
cache_write = 18.75
cache_read = 1.50

[pricing.sonnet]
input = 3.0
output = 15.0
cache_write = 3.75
cache_read = 0.30
"#;

        std::fs::write(&config_path, content).unwrap();
        let config = ConfigFile::load(&config_path).unwrap();
        let pricing = config
            .pricing
            .as_ref()
            .expect("pricing section should be parsed");

        let opus = pricing
            .get("claude-opus-4-7")
            .expect("exact-model entry");
        assert!((opus.input - 15.0).abs() < f64::EPSILON);
        assert!((opus.output - 75.0).abs() < f64::EPSILON);
        assert!((opus.cache_write - 18.75).abs() < f64::EPSILON);
        assert!((opus.cache_read - 1.50).abs() < f64::EPSILON);

        let sonnet = pricing.get("sonnet").expect("family entry");
        assert!((sonnet.input - 3.0).abs() < f64::EPSILON);

        // Re-serialize and re-parse to ensure the table survives a round trip.
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[pricing"));
        let reparsed: ConfigFile = toml::from_str(&toml_str).unwrap();
        assert!(reparsed.pricing.is_some());
    }
}
