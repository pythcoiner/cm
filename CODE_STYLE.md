# Claude Code Manager - Code Style Guide

This document defines coding patterns and conventions for the cm project.

---

## Error Handling

Use `thiserror` for all error types. Never use `anyhow`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("tasks.json not found at {0}")]
    NotFound(PathBuf),

    #[error("invalid JSON: {0}")]
    ParseError(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

Each module has its own error type. Use `#[from]` for automatic conversion.

---

## Struct Patterns

### Builder Pattern

Use builder pattern for structs with many optional fields:

```rust
pub struct Config {
    state_path: PathBuf,
    log_path: PathBuf,
    model: String,
    timeout: Duration,
    max_retries: u32,
    verbose: bool,
}

impl Config {
    pub fn new(state_path: PathBuf) -> Self {
        Self {
            state_path,
            log_path: PathBuf::from(".cm/LOG.md"),
            model: "claude-sonnet-4-5-20250929".to_string(),
            timeout: Duration::from_secs(300),
            max_retries: 5,
            verbose: false,
        }
    }

    pub fn log_path(mut self, path: PathBuf) -> Self {
        self.log_path = path;
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}
```

### Persistence Pattern

For types that persist to disk:

```rust
impl TasksState {
    pub fn load(path: &Path) -> Result<Self, StateError> {
        let content = fs::read_to_string(path)
            .map_err(|_| StateError::NotFound(path.to_path_buf()))?;
        serde_json::from_str(&content)
            .map_err(|e| StateError::ParseError(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| StateError::ParseError(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }
}
```

---

## Serde Conventions

### Enum Serialization

Use snake_case for JSON:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Deferred,
}
```

### DateTime Handling

Use chrono with serde:

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskAttempt {
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}
```

### Skip Serialization

For computed or transient fields:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Manager {
    state: TasksState,
    #[serde(skip)]
    process_handle: Option<Child>,
}
```

---

## Process Spawning

Use std::process for spawning claude:

```rust
use std::process::{Command, Stdio};

pub fn spawn_agent(prompt: &str) -> Result<Child, AgentError> {
    Command::new("claude")
        .args(["-p", prompt, "--output-format", "json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AgentError::CliNotFound)
}
```

---

## Threading

Use std::thread, std::sync::mpsc, NO tokio/async.

```rust
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

// Agent runs in separate thread, communicates via channel
pub fn spawn_agent(prompt: &str, tx: Sender<AgentEvent>) -> AgentHandle {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop = stop_flag.clone();

    let handle = thread::spawn(move || {
        let child = Command::new("claude")
            .args(["-p", &prompt, "--output-format", "json"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn claude");

        // Stream output to channel
        // Check stop_flag periodically
        // ...
    });

    AgentHandle { handle, stop_flag }
}

// TUI and Manager communicate via channels
pub struct Manager {
    event_tx: Sender<ManagerEvent>,
    command_rx: Receiver<Command>,
}

pub enum Command {
    Pause,
    Interrupt,
    Quit,
}
```

---

## Logging

Use the `log` crate with env_logger:

```rust
use log::{info, warn, error, debug};

pub fn run(&mut self) -> Result<()> {
    info!("Starting manager");

    while let Some(task) = self.next_task() {
        debug!("Executing task: {}", task.id);

        if let Err(e) = self.execute_task(&task) {
            warn!("Task {} failed: {}", task.id, e);
        }
    }

    info!("Manager completed");
    Ok(())
}
```

---

## File Organization

### Module Structure

Each module has:
- `mod.rs` - Public API, error types
- `types.rs` - Data structures (if many)
- Implementation files

```rust
// src/state/mod.rs
mod tasks;

pub use tasks::{TasksState, Task, Phase, TaskStatus};

#[derive(Debug, thiserror::Error)]
pub enum StateError { ... }

pub fn load_state(path: &Path) -> Result<TasksState, StateError> { ... }
```

### Import Order

1. std imports
2. External crate imports
3. Internal module imports

```rust
use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::state::{TasksState, Task};
use crate::agent::AgentSpawner;
```

---

## Testing

### Unit Tests

Place in same file with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"in_progress\"");
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration.rs
use cm::Manager;
use tempfile::TempDir;

#[test]
fn test_basic_flow() {
    let tmp = TempDir::new().unwrap();
    // ...
}
```

---

## Documentation

### Public Items

Document all public types and functions:

```rust
/// Manages the execution of tasks.
///
/// The manager loads state from tasks.json, executes tasks
/// by spawning agents, and updates state after each task.
pub struct Manager {
    // ...
}

/// Execute all pending tasks until completion or error.
///
/// # Errors
///
/// Returns an error if:
/// - State cannot be loaded
/// - An agent fails to spawn
/// - Build verification fails
pub fn run(&mut self) -> Result<(), ManagerError> {
    // ...
}
```

### Internal Comments

Use comments sparingly, only for non-obvious logic:

```rust
// After 5 failed attempts, defer the task instead of blocking
if task.attempts.len() >= 5 {
    self.defer_task(&task.id)?;
    continue;
}
```
