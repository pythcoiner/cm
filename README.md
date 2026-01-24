# Claude Code Manager (cm)

Automated agent coordination for software development.

`cm` orchestrates Claude AI agents to execute multi-phase development tasks
autonomously. Agents work in isolation - each receives only task-specific context,
ensuring independent verification and preventing context pollution.

## Features

- **Multi-phase task orchestration** - Define phases with dependent tasks
- **Agent context isolation** - No direct agent-to-agent communication
- **Build verification** - Automatic cargo build + clippy checks
- **Crash recovery** - Checkpointing with state restoration
- **Audit log** - Append-only LOG.md tracks all actions

### Planned Features

- **Real-time TUI** - Terminal UI for monitoring (not yet wired up)
- **Config file support** - `--config` flag (not yet implemented)
- **Signal handling** - Graceful Ctrl+C shutdown (not yet enabled)

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- [Claude CLI](https://github.com/anthropics/claude-code) installed and configured

### Build from source

```bash
git clone https://github.com/your-repo/cm.git
cd cm
cargo build --release
```

### Install to PATH (optional)

```bash
cargo install --path .
```

## Usage

```bash
cm                  # Run all tasks until completion
cm --continue       # Resume from interrupted state
cm --step           # Execute one task, then pause
cm --status         # Show progress summary
cm --validate       # Validate tasks.json schema
cm --dry-run        # Preview without executing
cm -v, --verbose    # Enable debug logging
cm --state FILE     # Custom state file (default: .cm/tasks.json)
```

## Project Setup

Create a `.cm` directory in your project root with a `tasks.json` file:

```
your-project/
├── .cm/
│   ├── tasks.json      # Task definitions & state
│   ├── LOG.md          # Audit trail (auto-generated)
│   └── checkpoints/    # Recovery snapshots (auto-generated)
├── src/
└── Cargo.toml
```

### tasks.json Format

```json
{
  "version": "1.0.0",
  "project": {
    "name": "my-project",
    "description": "Project description"
  },
  "phases": [
    {
      "id": "phase-1",
      "name": "Setup",
      "status": "pending",
      "tasks": [
        {
          "id": "task-1",
          "name": "Create module structure",
          "task_type": "implement",
          "status": "pending",
          "depends_on": [],
          "context": {
            "files_to_read": ["src/lib.rs"],
            "code_style_excerpt": null,
            "prior_review_issues": []
          },
          "instructions": "Create the basic module structure...",
          "attempts": []
        }
      ]
    }
  ],
  "current_phase": null,
  "current_task": null,
  "agent_history": []
}
```

### Task Types

- `implement` - Write new code
- `review` - Review implemented code
- `fix` - Fix issues from review
- `test` - Write or run tests

### Task Status

- `pending` - Not yet started
- `in_progress` - Currently executing
- `completed` - Successfully finished
- `deferred` - Skipped after max retry cycles

## TUI Controls (Planned)

> **Note:** The TUI is implemented but not yet wired into the CLI. This section
> documents the planned interface.

When running in interactive mode, these keyboard shortcuts will be available:

| Key | Action |
|-----|--------|
| `p` | Pause after current task completes |
| `i` | Interrupt immediately and save state |
| `q` | Quit (same as interrupt) |
| `Up/Down` | Scroll output view |

## How It Works

1. **Task Selection** - Manager picks the next runnable task (respects dependencies)
2. **Implementation** - Spawns IMPLEM agent with task-only context
3. **Verification** - Runs `cargo build` and `cargo clippy`
4. **Review** - Spawns REVIEW agent with fresh context (same files, no prior
knowledge)
5. **Iteration** - If review finds issues, creates FIX task and re-reviews
6. **Completion** - After approval, marks task complete and commits changes

Tasks are retried up to 5 times before being deferred. All actions are logged to
`LOG.md`.

## License

MIT
